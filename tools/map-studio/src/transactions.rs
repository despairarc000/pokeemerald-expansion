//! AI-friendly, deterministic edit transactions over the exact same World
//! operations used by the Android touchscreen editor. No alternative tile map.
//! Preview is read-only; commit is performed by the caller only after all
//! operations, source constraints and whole-world integrity pass.

use crate::{emerald_terrain, export, model::{Cell, EventKind, GridPoint, MapFrame, World},
            prefabs, tiles};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub const PROTOCOL_VERSION:u32=1;
pub const MAX_OPERATIONS:usize=128;
pub const MAX_CHANGED_CELLS:usize=20_000;
pub const MAX_DIFF_SAMPLE:usize=512;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransactionRequest {
    pub protocol_version:u32,
    /// The same UUID must be reused when retrying a request.
    pub transaction_id:String,
    /// sha256 of the exact canonical World JSON; checked immediately before
    /// the proposed changes are made. Never treat a GitHub SHA as this value.
    pub base_revision:String,
    pub operations:Vec<MapOperation>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
pub enum SimpleEventKind { Npc, Sign }

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag="op", rename_all="snake_case", deny_unknown_fields)]
pub enum MapOperation {
    CreateMap { name:String, space_id:u32, x:i32, y:i32, width:u32, height:u32,
                native_pair:String },
    FillRect { map_id:u32, x:u32, y:u32, width:u32, height:u32, metatile_id:u16 },
    PaintTile { map_id:u32, x:u32, y:u32, metatile_id:u16,
                #[serde(default)] collision:Option<u8>,
                #[serde(default)] elevation:Option<u8> },
    EraseTile { map_id:u32, x:u32, y:u32 },
    SetCollision { map_id:u32, x:u32, y:u32, collision:u8 },
    SetElevation { map_id:u32, x:u32, y:u32, elevation:u8 },
    PlaceAsset { map_id:u32, asset_id:String, x:u32, y:u32 },
    MoveObject { map_id:u32, instance_id:String, x:u32, y:u32 },
    RemoveObject { map_id:u32, instance_id:String },
    DrawWalkway { map_id:u32, x1:u32, y1:u32, x2:u32, y2:u32 },
    PlaceEvent { map_id:u32, x:u32, y:u32, kind:SimpleEventKind, dialogue:String },
}

#[derive(Clone, Debug, Serialize)]
pub struct CellChange {
    pub map_id:u32,
    pub x:u32,
    pub y:u32,
    pub before:Option<Cell>,
    pub after:Option<Cell>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ObjectChange {
    pub instance_id:String,
    pub asset_id:String,
    pub map_id:u32,
    pub before:Option<[u32;2]>,
    pub after:Option<[u32;2]>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TransactionReport {
    pub protocol_version:u32,
    pub transaction_id:String,
    pub base_revision:String,
    pub result_revision:String,
    pub operations_applied:usize,
    pub changed_cell_count:usize,
    pub changed_cell_sample:Vec<CellChange>,
    pub cell_sample_truncated:bool,
    pub created_map_ids:Vec<u32>,
    pub object_changes:Vec<ObjectChange>,
    pub events_added:usize,
    pub warnings:Vec<String>,
}

pub struct PreparedTransaction {
    pub world:World,
    pub report:TransactionReport,
}

/// Identical bytes produce identical revisions across Android and headless
/// CLI builds. This is the current editor World state, not a Git commit hash.
pub fn revision(world:&World)->Result<String,String>{
    world.check_integrity()?;
    let bytes=serde_json::to_vec(world).map_err(|e|format!("Encode revision: {e}"))?;
    let hash=Sha256::digest(bytes);
    Ok(format!("sha256:{}",hash.iter().map(|byte|format!("{byte:02x}")).collect::<String>()))
}

fn map_point(world:&World,map_id:u32,x:u32,y:u32)->Result<GridPoint,String>{
    let map=world.maps.iter().find(|m|m.id==map_id)
        .ok_or_else(||format!("Map {map_id} does not exist"))?;
    if x>=map.width||y>=map.height {
        return Err(format!("Map {map_id}: local tile ({x},{y}) is outside {}x{}",
            map.width,map.height));
    }
    Ok(GridPoint::new(
        i32::try_from(i64::from(map.origin.x)+i64::from(x))
            .map_err(|_|"Global X is outside editor coordinates")?,
        i32::try_from(i64::from(map.origin.y)+i64::from(y))
            .map_err(|_|"Global Y is outside editor coordinates")?))
}

fn select_map(world:&mut World,map_id:u32)->Result<(),String>{
    let space=world.maps.iter().find(|m|m.id==map_id)
        .ok_or_else(||format!("Map {map_id} does not exist"))?.space_id;
    world.active_space_id=space;
    Ok(())
}

fn editable_pair(world:&World,map_id:u32,metatile_id:u16)->Result<(),String>{
    let map=world.maps.iter().find(|m|m.id==map_id).ok_or("Map not found")?;
    let pair=tiles::pair_for(&map.primary_tileset,&map.secondary_tileset)
        .ok_or("Map has an unknown native tileset pair")?;
    if pair.collection!="Emerald" || !tiles::valid_in(pair,metatile_id){
        return Err(format!("Metatile {metatile_id} is not present in Emerald source {}",pair.key));
    }
    Ok(())
}

fn check_event_overlap(world:&World,map_id:u32,x:u32,y:u32)->Result<(),String>{
    let map=world.maps.iter().find(|m|m.id==map_id).ok_or("Map not found")?;
    if map.events.iter().any(|event|event.x==x && event.y==y){
        return Err(format!("Map {map_id}: tile ({x},{y}) contains an event"));
    }
    Ok(())
}

fn stable_instance_id(transaction_id:&str,op_index:usize)->String {
    let data=format!("MapStudio-v1/{transaction_id}/{op_index}");
    let digest=Sha256::digest(data.as_bytes());
    let mut bytes=[0u8;16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6]=(bytes[6]&0x0f)|0x50;
    bytes[8]=(bytes[8]&0x3f)|0x80;
    Uuid::from_bytes(bytes).to_string()
}

fn apply_one(world:&mut World,request:&TransactionRequest,index:usize,op:&MapOperation)
    ->Result<(),String>{
    match op {
        MapOperation::CreateMap {name,space_id,x,y,width,height,native_pair}=>{
            if name.trim().is_empty() || name.len()>90 || name.chars().any(char::is_control) {
                return Err("Map name must be 1–90 visible characters".into());
            }
            let pair=tiles::pair(native_pair).ok_or("Unknown native tileset pair")?;
            if pair.collection!="Emerald"{return Err("Only Emerald source maps are editable".into());}
            world.set_active_space(*space_id)?;
            let id=world.add_map(name.clone(),GridPoint::new(*x,*y),*width,*height)
                .map_err(str::to_string)?;
            world.set_map_tileset(id,native_pair)?;
        }
        MapOperation::FillRect {map_id,x,y,width,height,metatile_id}=>{
            editable_pair(world,*map_id,*metatile_id)?;
            if *width==0 || *height==0 || u64::from(*width)*u64::from(*height)>10_000{
                return Err("Fill rectangle must contain 1–10,000 cells".into());
            }
            let map=world.maps.iter().find(|m|m.id==*map_id).ok_or("Map not found")?;
            if x.checked_add(*width).is_none_or(|right|right>map.width)
                || y.checked_add(*height).is_none_or(|bottom|bottom>map.height) {
                return Err("Fill rectangle must fit entirely within the map".into());
            }
            for yy in *y..(*y+*height) {for xx in *x..(*x+*width){
                check_event_overlap(world,*map_id,xx,yy)?;
                let pos=map_point(world,*map_id,xx,yy)?;
                select_map(world,*map_id)?;
                world.paint_metatile(pos,*metatile_id);
            }}
        }
        MapOperation::PaintTile{map_id,x,y,metatile_id,collision,elevation}=>{
            editable_pair(world,*map_id,*metatile_id)?;
            if collision.is_some_and(|v|v>3) || elevation.is_some_and(|v|v>15){
                return Err("Collision must be 0–3 and elevation 0–15".into());
            }
            check_event_overlap(world,*map_id,*x,*y)?;
            let pos=map_point(world,*map_id,*x,*y)?;
            select_map(world,*map_id)?;
            world.paint_metatile(pos,*metatile_id);
            if let Some(value)=collision {world.set_collision(pos,*value);}
            if let Some(value)=elevation {world.set_elevation(pos,*value);}
        }
        MapOperation::EraseTile{map_id,x,y}=>{
            check_event_overlap(world,*map_id,*x,*y)?;
            let pos=map_point(world,*map_id,*x,*y)?;
            select_map(world,*map_id)?;
            world.erase(pos);
        }
        MapOperation::SetCollision{map_id,x,y,collision}=>{
            if *collision>3{return Err("Collision must be 0–3".into());}
            let pos=map_point(world,*map_id,*x,*y)?;
            if world.maps.iter().find(|m|m.id==*map_id)
                .and_then(|m|m.cell(*x,*y)).is_none(){
                return Err("Paint the native cell before setting its collision".into());
            }
            select_map(world,*map_id)?;
            world.set_collision(pos,*collision);
        }
        MapOperation::SetElevation{map_id,x,y,elevation}=>{
            if *elevation>15{return Err("Elevation must be 0–15".into());}
            let pos=map_point(world,*map_id,*x,*y)?;
            if world.maps.iter().find(|m|m.id==*map_id)
                .and_then(|m|m.cell(*x,*y)).is_none(){
                return Err("Paint the native cell before setting its elevation".into());
            }
            select_map(world,*map_id)?;
            world.set_elevation(pos,*elevation);
        }
        MapOperation::PlaceAsset{map_id,asset_id,x,y}=>{
            let (asset_index,asset)=prefabs::by_asset_id(asset_id)
                .ok_or_else(||format!("Unknown stable Emerald asset '{asset_id}'"))?;
            let map=world.maps.iter().find(|m|m.id==*map_id).ok_or("Map not found")?;
            let pair=tiles::pair_for(&map.primary_tileset,&map.secondary_tileset)
                .ok_or("Map source pair is missing")?;
            if pair.collection!="Emerald" || !prefabs::compatible(asset,pair) {
                return Err(format!("Asset {asset_id} requires a different native source pair"));
            }
            let pos=map_point(world,*map_id,*x,*y)?;
            select_map(world,*map_id)?;
            world.try_paint_prefab(pos,asset)?;
            let new_id=stable_instance_id(&request.transaction_id,index);
            if world.prefab_instances.iter().any(|p|p.instance_id==new_id) {
                return Err("Transaction would duplicate an object identity".into());
            }
            let object=world.prefab_instances.last_mut()
                .ok_or("Prefab engine did not record the placed source object")?;
            if object.prefab_index!=asset_index || object.map_id!=*map_id {
                return Err("Prefab engine returned an unexpected object reference".into());
            }
            object.instance_id=new_id;
        }
        MapOperation::MoveObject{map_id,instance_id,x,y}=>{
            if !world.prefab_instances.iter().any(|p|
                p.instance_id==*instance_id && p.map_id==*map_id){
                return Err("Object identity does not belong to the requested map".into());
            }
            let pos=map_point(world,*map_id,*x,*y)?;
            select_map(world,*map_id)?;
            world.move_prefab_instance(instance_id,pos)?;
        }
        MapOperation::RemoveObject{map_id,instance_id}=>{
            if !world.prefab_instances.iter().any(|p|
                p.instance_id==*instance_id && p.map_id==*map_id){
                return Err("Object identity does not belong to the requested map".into());
            }
            select_map(world,*map_id)?;
            world.remove_prefab_instance(instance_id)?;
        }
        MapOperation::DrawWalkway{map_id,x1,y1,x2,y2}=>{
            let from=map_point(world,*map_id,*x1,*y1)?;
            let to=map_point(world,*map_id,*x2,*y2)?;
            select_map(world,*map_id)?;
            emerald_terrain::draw_lilycove_walkway(world,*map_id,from,to)?;
        }
        MapOperation::PlaceEvent{map_id,x,y,kind,dialogue}=>{
            let pos=map_point(world,*map_id,*x,*y)?;
            check_event_overlap(world,*map_id,*x,*y)?;
            if dialogue.is_empty() || dialogue.len()>100 || dialogue.chars()
                .any(|c|!c.is_ascii_graphic() && c!=' ' || matches!(c,'"'|'$'|'\\')){
                return Err("NPC/sign dialogue must be 1–100 source-safe characters".into());
            }
            select_map(world,*map_id)?;
            let native_kind=match kind {SimpleEventKind::Npc=>EventKind::Npc,
                                         SimpleEventKind::Sign=>EventKind::Sign};
            let (map_id,event_index)=world.add_event(pos,native_kind)
                .ok_or("Cannot place an event outside the map")?;
            let event=world.event_mut(map_id,event_index).ok_or("Event record disappeared")?;
            event.dialogue=dialogue.clone();
        }
    }
    Ok(())
}

fn changed_cells(before:&World,after:&World)->(usize,Vec<CellChange>){
    let mut count=0;
    let mut sample=Vec::new();
    for map in &after.maps{
        let old=before.maps.iter().find(|m|m.id==map.id);
        for (i,cell) in map.cells.iter().enumerate(){
            let prior=old.and_then(|m|m.cells.get(i)).copied().flatten();
            if prior!=*cell{
                count+=1;
                if sample.len()<MAX_DIFF_SAMPLE{
                    sample.push(CellChange{map_id:map.id,
                        x:(i as u32)%map.width,y:(i as u32)/map.width,
                        before:prior,after:*cell});
                }
            }
        }
    }
    (count,sample)
}

fn changed_objects(before:&World,after:&World)->Vec<ObjectChange>{
    let mut changes=Vec::new();
    for object in &before.prefab_instances{
        let new=after.prefab_instances.iter().find(|p|p.instance_id==object.instance_id);
        let moved=new.is_some_and(|p|p.map_id!=object.map_id || p.x!=object.x || p.y!=object.y);
        if new.is_none()||moved{
            changes.push(ObjectChange{
                instance_id:object.instance_id.clone(),
                asset_id:prefabs::asset_id(object.prefab_index).unwrap_or_default(),
                map_id:object.map_id,
                before:Some([object.x,object.y]),
                after:new.map(|p|[p.x,p.y]),
            });
        }
    }
    for object in &after.prefab_instances{
        if before.prefab_instances.iter().all(|p|p.instance_id!=object.instance_id){
            changes.push(ObjectChange{
                instance_id:object.instance_id.clone(),
                asset_id:prefabs::asset_id(object.prefab_index).unwrap_or_default(),
                map_id:object.map_id,before:None,after:Some([object.x,object.y]),
            });
        }
    }
    changes
}

pub fn prepare(world:&World,request:&TransactionRequest)
    ->Result<PreparedTransaction,String>{
    world.check_integrity()?;
    if request.protocol_version!=PROTOCOL_VERSION{
        return Err(format!("Unsupported transaction protocol version {}",request.protocol_version));
    }
    if Uuid::parse_str(&request.transaction_id).ok()
        .map(|id|id.to_string())!=Some(request.transaction_id.clone()){
        return Err("transaction_id must be a canonical lowercase UUID".into());
    }
    if request.operations.is_empty()||request.operations.len()>MAX_OPERATIONS{
        return Err(format!("Transaction needs 1–{MAX_OPERATIONS} operations"));
    }
    let current=revision(world)?;
    if current!=request.base_revision{
        return Err(format!("STALE_REVISION: current {current}; inspect/preview again"));
    }
    let mut draft=world.clone();
    let saved_space=draft.active_space_id;
    for (i,op) in request.operations.iter().enumerate(){
        apply_one(&mut draft,request,i,op)
            .map_err(|error|format!("Operation {}: {error} (no changes committed)",i+1))?;
    }
    draft.active_space_id=saved_space;
    draft.check_integrity()?;
    let (changed_cell_count,changed_cell_sample)=changed_cells(world,&draft);
    if changed_cell_count>MAX_CHANGED_CELLS{
        return Err(format!("Transaction changes too many cells ({changed_cell_count}); split into smaller edits"));
    }
    let created_map_ids=draft.maps.iter()
        .filter(|m|world.maps.iter().all(|old|old.id!=m.id))
        .map(|m|m.id).collect();
    let object_changes=changed_objects(world,&draft);
    let events_added=draft.maps.iter().map(|map|{
        let prior=world.maps.iter().find(|m|m.id==map.id).map_or(0,|m|m.events.len());
        map.events.len().saturating_sub(prior)
    }).sum();
    let mut warnings=Vec::new();
    if let Err(reason)=export::validate(&draft){
        warnings.push(format!("Native game export not yet ready: {reason}"));
    }
    let result_revision=revision(&draft)?;
    Ok(PreparedTransaction{world:draft,
        report:TransactionReport{
            protocol_version:PROTOCOL_VERSION,
            transaction_id:request.transaction_id.clone(),
            base_revision:current,result_revision,operations_applied:request.operations.len(),
            changed_cell_count,
            cell_sample_truncated:changed_cell_count>changed_cell_sample.len(),
            changed_cell_sample,created_map_ids,object_changes,
            events_added,warnings,
        }})
}

#[derive(Clone,Debug,Serialize)]
pub struct RegionCell {
    pub x:u32,pub y:u32,pub cell:Option<Cell>,
    pub source_label:Option<&'static str>,
    pub source_behavior:Option<&'static str>,
    pub object_id:Option<String>,
}

#[derive(Clone,Debug,Serialize)]
pub struct RegionInspection {
    pub map_id:u32,
    pub map_name:String,
    pub native_pair:String,
    pub revision:String,
    pub origin:[u32;2],
    pub size:[u32;2],
    pub cells:Vec<RegionCell>,
}

pub fn inspect_region(world:&World,map_id:u32,x:u32,y:u32,width:u32,height:u32)
    ->Result<RegionInspection,String>{
    let map:&MapFrame=world.maps.iter().find(|m|m.id==map_id).ok_or("Map not found")?;
    if width==0||height==0||u64::from(width)*u64::from(height)>4096
        || x.checked_add(width).is_none_or(|v|v>map.width)
        || y.checked_add(height).is_none_or(|v|v>map.height){
        return Err("Inspect region must fit in map and contain 1–4,096 cells".into());
    }
    let pair=tiles::pair_for(&map.primary_tileset,&map.secondary_tileset)
        .ok_or("Map has unknown source pair")?;
    let mut cells=Vec::with_capacity((width*height) as usize);
    for row in y..y+height {for col in x..x+width{
        let cell=map.cell(col,row);
        let metatile=cell.map(|c|c.metatile_id);
        let source_label=metatile.and_then(|id|tiles::source_label_in(pair,id));
        let source_behavior=metatile.and_then(|id|tiles::behavior_name_in(pair,id));
        let object_id=world.tracked_prefab_at(map_id,col,row)
            .map(|p|p.instance_id.clone());
        cells.push(RegionCell{x:col,y:row,cell,source_label,source_behavior,object_id});
    }}
    Ok(RegionInspection{
        map_id,map_name:map.name.clone(),native_pair:pair.key.to_owned(),
        revision:revision(world)?,origin:[x,y],size:[width,height],cells,
    })
}

#[cfg(test)]
mod tests{
    use super::*;
    fn world(pair:&str)->(World,u32){
        let mut w=World::new();
        let id=w.add_map("Town",GridPoint::new(10,20),40,40).unwrap();
        w.set_map_tileset(id,pair).unwrap();
        (w,id)
    }
    fn tx(w:&World,operations:Vec<MapOperation>)->TransactionRequest{
        TransactionRequest{
            protocol_version:PROTOCOL_VERSION,
            transaction_id:"963e9aa3-dcf9-4c16-b093-b6597b1e706e".into(),
            base_revision:revision(w).unwrap(),
            operations,
        }
    }
    #[test]
    fn preview_is_pure_and_commit_result_is_deterministic(){
        let (w,id)=world("general/lilycove");
        let command=tx(&w,vec![
            MapOperation::FillRect{map_id:id,x:0,y:0,width:7,height:7,metatile_id:1},
            MapOperation::DrawWalkway{map_id:id,x1:3,y1:3,x2:3,y2:3},
        ]);
        let before=w.clone();
        let a=prepare(&w,&command).unwrap();
        let b=prepare(&w,&command).unwrap();
        assert_eq!(w,before,"Preview must never change original World");
        assert_eq!(a.world,b.world);
        assert_eq!(a.report.result_revision,b.report.result_revision);
        assert_eq!(a.report.changed_cell_count,49);
        assert!(a.world.maps[0].cell(3,3).is_some());
        a.world.check_integrity().unwrap();
    }
    #[test]
    fn source_asset_is_stably_named_and_tracked(){
        let (w,id)=world("general/rustboro");
        let request=tx(&w,vec![MapOperation::PlaceAsset{map_id:id,
            asset_id:"emerald:environment:pond-bank-route104".into(),x:5,y:7}]);
        let a=prepare(&w,&request).unwrap();
        let b=prepare(&w,&request).unwrap();
        assert_eq!(a.report.result_revision,b.report.result_revision);
        assert_eq!(a.world.prefab_instances.len(),1);
        assert_eq!(a.world.prefab_instances[0].instance_id,b.world.prefab_instances[0].instance_id);
        assert_eq!(a.report.object_changes.len(),1);
        let inspect=inspect_region(&a.world,id,5,7,5,5).unwrap();
        assert!(inspect.cells.iter().any(|c|c.object_id.is_some()));
    }
    #[test]
    fn stale_revision_and_invalid_step_leave_world_unchanged(){
        let (w,id)=world("general/lilycove");
        let mut request=tx(&w,vec![
            MapOperation::FillRect{map_id:id,x:0,y:0,width:4,height:4,metatile_id:1},
            MapOperation::PlaceAsset{map_id:id,asset_id:
                "emerald:environment:pond-bank-route104".into(),x:4,y:4}
        ]);
        let before=w.clone();
        assert!(prepare(&w,&request).unwrap_err_or_string().contains("Operation 2"));
        assert_eq!(w,before);
        request.base_revision="sha256:not-current".into();
        assert!(prepare(&w,&request).unwrap_err_or_string().contains("STALE_REVISION"));
        assert_eq!(w,before);
    }
    #[test]
    fn event_and_canvas_reject_unsafe_edit_and_preserve_source_state(){
        let (w,id)=world("general/lilycove");
        let request=tx(&w,vec![MapOperation::PlaceEvent{map_id:id,x:2,y:2,
            kind:SimpleEventKind::Npc,dialogue:"Hello.".into()}]);
        let prepared=prepare(&w,&request).unwrap();
        assert_eq!(prepared.report.events_added,1);
        let invalid=tx(&prepared.world,vec![MapOperation::EraseTile{map_id:id,x:2,y:2}]);
        assert!(prepare(&prepared.world,&invalid).is_err());
        assert!(inspect_region(&prepared.world,id,2,2,1,1).is_ok());
        assert!(inspect_region(&prepared.world,id,39,39,2,2).is_err());
    }
    // The error type does not require PreparedTransaction to implement Debug.
    trait DisplayResult<T>{fn unwrap_err_or_string(self)->String;}
    impl<T> DisplayResult<T> for Result<T,String>{
        fn unwrap_err_or_string(self)->String{match self{Err(e)=>e,Ok(_)=>panic!("expected error")}}
    }
}
