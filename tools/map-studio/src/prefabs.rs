//! Source-backed prefab library. Default Emerald definitions from Porymap.
//! Prefabs are NOT new tilesets. IDs are validated against the active pair.
use crate::{model::{GridPoint,World,PlacedPrefab,PrefabStamp},tiles::{self,TilePair}};
use serde::{Deserialize,Serialize};
use std::sync::OnceLock;

#[derive(Clone,Debug,PartialEq,Eq,Deserialize,Serialize)]
pub struct PrefabTile {
    pub x:u32,
    pub y:u32,
    pub metatile_id:u16,
    pub collision:u8,
    pub elevation:u8,
}

#[derive(Clone,Debug,PartialEq,Eq,Deserialize,Serialize)]
pub struct Prefab {
    pub name:String,
    pub width:u32,
    pub height:u32,
    #[serde(default)]
    pub primary_tileset:String,
    #[serde(default)]
    pub secondary_tileset:String,
    pub metatiles:Vec<PrefabTile>,
}

/// Original vanilla Emerald prefabs from huderlem/porymap, LGPL-3.0.
pub fn emerald()->&'static [Prefab]{
    static PREFABS:OnceLock<Vec<Prefab>>=OnceLock::new();
    PREFABS.get_or_init(||{
        serde_json::from_str(include_str!("../assets/porymap-emerald-prefabs.json"))
            .expect("Bundled Porymap prefabs must parse")
    })
}

/// Original vanilla FireRed prefab definitions from huderlem/porymap, LGPL-3.0.
/// Their general tileset symbol uses the upstream FireRed name; this fork
/// names its native FRLG primary art gTileset_General_Frlg.
pub fn firered()->&'static [Prefab]{
    static PREFABS:OnceLock<Vec<Prefab>>=OnceLock::new();
    PREFABS.get_or_init(||{
        serde_json::from_str(include_str!("../assets/porymap-firered-prefabs.json"))
            .expect("Bundled Porymap FireRed prefabs must parse")
    })
}
pub fn by_collection(collection:&str)->&'static [Prefab]{
    match collection{
        "FireRed/LeafGreen"=>firered(),
        _=>emerald(),
    }
}

/// Source symbol pairing is part of the Porymap format; an empty symbol
/// represents a wildcard. Reject invalid or modified source IDs rather than
/// painting a lookalike with silently different graphics.
pub fn compatible(prefab:&Prefab,pair:&TilePair)->bool{
    let primary_matches=prefab.primary_tileset.is_empty()
        || prefab.primary_tileset==pair.primary_symbol
        || (pair.collection=="FireRed/LeafGreen" && matches!(
            (prefab.primary_tileset.as_str(),pair.primary_symbol),
            ("gTileset_General","gTileset_General_Frlg")
                | ("gTileset_Building","gTileset_BuildingFrlg")));
    primary_matches
        && (prefab.secondary_tileset.is_empty() || prefab.secondary_tileset==pair.secondary_symbol)
        && prefab.width>0 && prefab.height>0
        && prefab.width<=32 && prefab.height<=32
        && !prefab.metatiles.is_empty()
        && prefab.metatiles.iter().all(|p|p.x<prefab.width && p.y<prefab.height
            && p.collision<=3 && p.elevation<=15
            && tiles::valid_in(pair,p.metatile_id))
}
pub fn compatible_indices(pair:&TilePair)->Vec<usize>{
    by_collection(pair.collection).iter().enumerate()
        .filter_map(|(i,p)|compatible(p,pair).then_some(i)).collect()
}

impl World {
    /// Check native source compatibility and every authored matrix cell.
    /// Overlap is an editing decision, not an invisible permanent reservation:
    /// replacement retires only the old owner's affected cells.
    pub fn preview_prefab_placement(&self,at:GridPoint,prefab:&Prefab)
        ->Result<(),String>{
        let map=self.maps.iter().find(|m|m.space_id==self.active_space_id
            && m.contains(at)).ok_or("Choose a point inside the target map")?;
        let pair=tiles::pair_for(&map.primary_tileset,&map.secondary_tileset)
            .ok_or("Unknown native map tileset")?;
        if !compatible(prefab,pair){
            return Err("Prefab requires another original source tileset".into());
        }
        let local_x=(i64::from(at.x)-i64::from(map.origin.x)) as u32;
        let local_y=(i64::from(at.y)-i64::from(map.origin.y)) as u32;
        if local_x.checked_add(prefab.width).is_none_or(|x|x>map.width)
            || local_y.checked_add(prefab.height).is_none_or(|y|y>map.height){
            return Err("The complete prefab must fit inside one map".into());
        }
        let mut seen=std::collections::HashSet::new();
        for tile in &prefab.metatiles{
            if !seen.insert((tile.x,tile.y)){
                return Err("Prefab source defines a duplicate footprint tile".into());
            }
            let x=local_x+tile.x;
            let y=local_y+tile.y;
            map.index(x,y).ok_or("Prefab tile is outside the map")?;
            if map.events.iter().any(|e|e.x==x && e.y==y){
                return Err("Prefab overlaps an existing event".into());
            }
        }
        Ok(())
    }

    /// The true underlay, not the already-visible tile of another prefab.
    /// This prevents a replaced house from reappearing when its successor is
    /// removed. Manually painted/untracked source cells are retained verbatim.
    fn prefab_underlay(&self,map_id:u32,x:u32,y:u32)->Option<crate::model::Cell>{
        if let Some(owner)=self.tracked_prefab_at(map_id,x,y){
            if let Some(stamp)=owner.stamps.iter().find(|st|
                owner.x+st.x==x && owner.y+st.y==y){
                return stamp.before;
            }
        }
        self.maps.iter().find(|m|m.id==map_id)
            .and_then(|m|m.cell(x,y))
    }

    /// Replace the sparse prefab matrix atomically. Unspecified matrix cells
    /// are transparent, never filled with invented background metatiles.
    pub fn try_paint_prefab(&mut self,at:GridPoint,prefab:&Prefab)
        ->Result<usize,String>{
        self.preview_prefab_placement(at,prefab)?;
        let map=self.maps.iter().find(|m|
            m.space_id==self.active_space_id && m.contains(at))
            .ok_or("Target map disappeared")?;
        let map_id=map.id;
        let primary_tileset=map.primary_tileset.clone();
        let secondary_tileset=map.secondary_tileset.clone();
        let local_x=(at.x-map.origin.x) as u32;
        let local_y=(at.y-map.origin.y) as u32;
        let mut stamps=Vec::with_capacity(prefab.metatiles.len());
        let mut changed=0;
        for tile in &prefab.metatiles{
            let x=local_x+tile.x;
            let y=local_y+tile.y;
            let previous=map.cell(x,y);
            let mut painted=previous.unwrap_or_else(||
                crate::model::Cell::new(crate::model::TileKind::Grass));
            painted.metatile_id=tile.metatile_id;
            painted.collision=tile.collision;
            painted.elevation=tile.elevation;
            if previous!=Some(painted){changed+=1;}
            stamps.push(PrefabStamp{
                x:tile.x,y:tile.y,
                before:self.prefab_underlay(map_id,x,y),
                painted,
            });
        }
        // All checks and snapshot reads complete before changing any cell.
        // Release overlapping old references *without* painting their old
        // underlay over the new object. Unaffected object cells survive.
        for stamp in &stamps{
            self.sync_prefab_cell(map_id,local_x+stamp.x,local_y+stamp.y,None);
        }
        let map=self.maps.iter_mut().find(|m|m.id==map_id)
            .ok_or("Target map disappeared")?;
        for stamp in &stamps{
            let index=map.index(local_x+stamp.x,local_y+stamp.y)
                .expect("preflight checked full matrix");
            map.cells[index]=Some(stamp.painted);
        }
        // Original Emerald source identities are stable across edits; imported
        // custom stamps can still paint the native cells without fake IDs.
        if let Some(source_index)=emerald().iter().position(|p|p==prefab){
            self.prefab_instances.push(PlacedPrefab{
                instance_id:uuid::Uuid::new_v4().to_string(),
                map_id,prefab_index:source_index,
                primary_tileset,
                secondary_tileset,
                x:local_x,y:local_y,width:prefab.width,height:prefab.height,
                stamps,
            });
        }
        Ok(changed)
    }

    /// A tracked prefab is a complete editable object, rather than
    /// unrelated tiles. Old stamps remain paintable but untracked.
    pub fn tracked_prefab_at(&self,map_id:u32,x:u32,y:u32)
        ->Option<&PlacedPrefab>{
        self.prefab_instances.iter().rev().find(|p|
            p.map_id==map_id && p.stamps.iter().any(|st|
                p.x+st.x==x && p.y+st.y==y))
    }

    fn verify_prefab_snapshot(&self,record:&PlacedPrefab)->Result<(),String>{
        let map=self.maps.iter().find(|m|m.id==record.map_id)
            .ok_or("Prefab map was deleted")?;
        for stamp in &record.stamps {
            let x=record.x+stamp.x;
            let y=record.y+stamp.y;
            let idx=map.index(x,y).ok_or("Prefab position is outside map")?;
            if map.cells[idx]!=Some(stamp.painted){
                return Err("Building tiles were edited manually; move is blocked to protect those edits".into());
            }
        }
        Ok(())
    }

    /// Remove a whole tracked prefab and restore its original ground.
    pub fn remove_prefab_instance(&mut self,instance_id:&str)->Result<usize,String>{
        let source=self.prefab_instances.iter()
            .find(|p|p.instance_id==instance_id)
            .ok_or("Tracked building not found")?.clone();
        self.verify_prefab_snapshot(&source)?;
        let index=self.maps.iter().position(|m|m.id==source.map_id)
            .ok_or("Tracked building map missing")?;
        for stamp in &source.stamps {
            let ix=self.maps[index].index(source.x+stamp.x,source.y+stamp.y)
                .ok_or("Prefab footprint changed")?;
            self.maps[index].cells[ix]=stamp.before;
        }
        self.prefab_instances.retain(|p|p.instance_id!=instance_id);
        Ok(source.stamps.len())
    }

    /// Move the current (possibly hand-edited or partially erased) matrix.
    /// The original UUID and all remaining tile customizations are preserved.
    pub fn move_prefab_instance(&mut self,instance_id:&str,to:GridPoint)
        ->Result<usize,String>{
        let source=self.prefab_instances.iter()
            .find(|p|p.instance_id==instance_id)
            .ok_or("Tracked building not found")?.clone();
        self.verify_prefab_snapshot(&source)?;
        let map=self.maps.iter().find(|m|m.id==source.map_id)
            .ok_or("Tracked building map missing")?;
        if !map.contains(to){
            return Err("The building must stay within its current map".into());
        }
        if map.primary_tileset!=source.primary_tileset
            || map.secondary_tileset!=source.secondary_tileset{
            return Err("Building has the wrong native source pair".into());
        }
        let local_x=(to.x-map.origin.x) as u32;
        let local_y=(to.y-map.origin.y) as u32;
        if local_x.checked_add(source.width).is_none_or(|x|x>map.width)
            || local_y.checked_add(source.height).is_none_or(|y|y>map.height){
            return Err("The complete building must fit inside one map".into());
        }
        // Transactional draft: failed moves never disturb the actual project.
        let mut draft=self.clone();
        draft.remove_prefab_instance(instance_id)?;
        let target=draft.maps.iter().find(|m|m.id==source.map_id)
            .ok_or("Tracked building map missing")?;
        for stamp in &source.stamps{
            let x=local_x+stamp.x;
            let y=local_y+stamp.y;
            if target.events.iter().any(|e|e.x==x && e.y==y){
                return Err("Building would overlap an existing event".into());
            }
        }
        let mut updated=source.clone();
        updated.x=local_x;
        updated.y=local_y;
        for stamp in &mut updated.stamps{
            let x=local_x+stamp.x;
            let y=local_y+stamp.y;
            stamp.before=draft.prefab_underlay(source.map_id,x,y);
        }
        for stamp in &updated.stamps{
            draft.sync_prefab_cell(source.map_id,
                local_x+stamp.x,local_y+stamp.y,None);
        }
        let target=draft.maps.iter_mut().find(|m|m.id==source.map_id)
            .ok_or("Tracked building map missing")?;
        for stamp in &updated.stamps{
            let index=target.index(local_x+stamp.x,local_y+stamp.y)
                .ok_or("Moved building tile outside map")?;
            target.cells[index]=Some(stamp.painted);
        }
        draft.prefab_instances.push(updated);
        *self=draft;
        Ok(source.stamps.len())
    }

    /// Legacy caller convenience. New UI should use try_paint_prefab so
    /// failed placement can show an exact error.
    pub fn paint_prefab(&mut self,at:GridPoint,prefab:&Prefab)->usize{
        self.try_paint_prefab(at,prefab).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    use crate::model::{GridPoint,World};
    #[test]
    fn movable_native_prefab_restores_original_ground_and_id(){
        let pair=tiles::pair("general/lilycove").unwrap();
        let idx=compatible_indices(pair)[0];
        let prefab=&emerald()[idx];
        let mut world=World::new();
        let map_id=world.add_map("Town",GridPoint::new(0,0),40,40).unwrap();
        world.set_map_tileset(map_id,pair.key).unwrap();
        let original=world.clone();
        assert!(world.try_paint_prefab(GridPoint::new(5,5),prefab).unwrap()>0);
        assert_eq!(world.prefab_instances.len(),1);
        world.check_integrity().unwrap();
        // Re-stamping an existing object is allowed, and its original underlay survives.
        world.try_paint_prefab(GridPoint::new(5,5),prefab).unwrap();
        assert_eq!(world.prefab_instances.len(),1);
        let replacement_id=world.prefab_instances[0].instance_id.clone();
        let count=world.move_prefab_instance(&replacement_id,GridPoint::new(20,20)).unwrap();
        assert_eq!(count,prefab.metatiles.len());
        assert_eq!(world.prefab_instances[0].instance_id,replacement_id);
        assert_eq!(world.prefab_instances[0].x,20);
        let after_move=world.clone();
        assert!(world.move_prefab_instance(&replacement_id,GridPoint::new(39,39)).is_err());
        assert_eq!(world,after_move);
        world.remove_prefab_instance(&replacement_id).unwrap();
        assert_eq!(world,original);
    }
    #[test]
    fn hand_modified_building_moves_with_its_changed_native_tile(){
        let pair=tiles::pair("general/lilycove").unwrap();
        let prefab=&emerald()[compatible_indices(pair)[0]];
        let mut world=World::new();
        let map_id=world.add_map("Town",GridPoint::new(0,0),35,35).unwrap();
        world.set_map_tileset(map_id,pair.key).unwrap();
        world.try_paint_prefab(GridPoint::new(3,4),prefab).unwrap();
        let record=world.prefab_instances[0].clone();
        let stamp=&record.stamps[0];
        let x=record.x+stamp.x;
        let y=record.y+stamp.y;
        let old=world.maps[0].cell(x,y).unwrap();
        let new_collision=if old.collision==0{1}else{0};
        assert!(world.set_collision(GridPoint::new(x as i32,y as i32),new_collision));
        let id=record.instance_id;
        let new_before=world.maps[0].cell(x,y).unwrap();
        world.move_prefab_instance(&id,GridPoint::new(12,12)).unwrap();
        let moved=&world.prefab_instances[0];
        let ix=moved.x+stamp.x;
        let iy=moved.y+stamp.y;
        assert_eq!(world.maps[0].cell(ix,iy),Some(new_before));
        world.remove_prefab_instance(&id).unwrap();
        world.check_integrity().unwrap();
    }
    #[test]
    fn complete_prefab_placement_never_clips_at_map_edges(){
        let pair=tiles::pair(tiles::DEFAULT_PAIR).unwrap();
        let p=&emerald()[compatible_indices(pair)[0]];
        let mut world=World::new();
        let id=world.add_map("Edge",GridPoint::new(0,0),32,32).unwrap();
        world.set_map_tileset(id,pair.key).unwrap();
        let before=world.clone();
        assert!(world.try_paint_prefab(GridPoint::new(31,31),p)
            .unwrap_err().contains("complete prefab"));
        assert_eq!(world,before,"partial building was stamped");
    }
    #[test]
    fn replacement_retains_ground_without_ghost_occupancy(){
        let pair=tiles::pair(tiles::DEFAULT_PAIR).unwrap();
        let p=&emerald()[compatible_indices(pair)[0]];
        let mut world=World::new();
        let map_id=world.add_map("Replace",GridPoint::new(0,0),32,32).unwrap();
        world.set_map_tileset(map_id,pair.key).unwrap();
        let original=world.clone();
        world.try_paint_prefab(GridPoint::new(4,4),p).unwrap();
        let old_id=world.prefab_instances[0].instance_id.clone();
        world.try_paint_prefab(GridPoint::new(4,4),p).unwrap();
        assert!(world.prefab_instances.iter().all(|obj|obj.instance_id!=old_id));
        assert_eq!(world.prefab_instances.len(),1);
        let current=world.prefab_instances[0].instance_id.clone();
        world.remove_prefab_instance(&current).unwrap();
        assert_eq!(world,original);
    }
    #[test]
    fn erasing_house_cells_releases_ownership_for_new_prefab(){
        let pair=tiles::pair(tiles::DEFAULT_PAIR).unwrap();
        let p=&emerald()[compatible_indices(pair)[0]];
        let mut world=World::new();
        let map_id=world.add_map("Erase",GridPoint::new(0,0),32,32).unwrap();
        world.set_map_tileset(map_id,pair.key).unwrap();
        world.try_paint_prefab(GridPoint::new(4,4),p).unwrap();
        let record=world.prefab_instances[0].clone();
        for stamp in record.stamps{
            let x=record.x+stamp.x;
            let y=record.y+stamp.y;
            world.erase(GridPoint::new(x as i32,y as i32));
        }
        assert!(world.prefab_instances.is_empty());
        world.try_paint_prefab(GridPoint::new(4,4),p).unwrap();
        world.check_integrity().unwrap();
    }
    #[test]
    fn events_still_prevent_destructive_prefab_placement(){
        let pair=tiles::pair(tiles::DEFAULT_PAIR).unwrap();
        let p=&emerald()[compatible_indices(pair)[0]];
        let mut world=World::new();
        let map_id=world.add_map("Events",GridPoint::new(0,0),32,32).unwrap();
        world.set_map_tileset(map_id,pair.key).unwrap();
        world.add_event(GridPoint::new(4,4),crate::model::EventKind::Npc).unwrap();
        assert!(world.try_paint_prefab(GridPoint::new(4,4),p).is_err());
        assert!(world.prefab_instances.is_empty());
    }
        #[test]
    fn seventy_one_original_emerald_prefabs_are_parsed(){
        assert_eq!(emerald().len(),71);
        assert!(emerald().iter().any(|p|p.name=="Mart"));
        assert!(emerald().iter().any(|p|p.name=="Player House"));
    }
    #[test]
    fn firered_definitions_are_bundled_and_source_scoped(){
        assert_eq!(firered().len(),45);
        let pairs:Vec<_>=tiles::pairs().iter()
            .filter(|pair|pair.collection=="FireRed/LeafGreen").collect();
        assert!(!pairs.is_empty(),"The fork contains native FRLG source pairs");
        assert!(pairs.iter().any(|pair|!compatible_indices(pair).is_empty()),
            "No bundled FireRed prefab is source-compatible");
        for pair in pairs{
            for index in compatible_indices(pair){
                assert!(compatible(&firered()[index],pair));
            }
        }
    }
    #[test]
    fn built_in_prefabs_are_only_exposed_when_matching_real_pair(){
        let pair=tiles::pair(tiles::DEFAULT_PAIR).unwrap();
        let chosen=compatible_indices(pair);
        assert!(!chosen.is_empty(),"Emerald must have usable prefab selections");
        for index in chosen{
            assert!(compatible(&emerald()[index],pair));
        }
        let mut invalid=emerald()[0].clone();
        invalid.metatiles[0].metatile_id=1024;
        assert!(!compatible(&invalid,pair));
    }
    #[test]
    fn prefab_paint_keeps_native_properties_and_other_maps_untouched(){
        let pair=tiles::pair(tiles::DEFAULT_PAIR).unwrap();
        let index=compatible_indices(pair)[0];
        let p=&emerald()[index];
        let mut world=World::new();
        let id=world.add_map("Test",GridPoint::new(0,0),32,32).unwrap();
        let id2=world.add_map("Neighbor",GridPoint::new(32,0),32,32).unwrap();
        world.set_map_tileset(id,pair.key).unwrap();
        world.set_map_tileset(id2,pair.key).unwrap();
        assert!(world.paint_prefab(GridPoint::new(5,5),p)>0);
        let map=world.maps.iter().find(|m|m.id==id).unwrap();
        for tile in &p.metatiles{
            let cell=map.cell(5+tile.x,5+tile.y).unwrap();
            assert_eq!(cell.metatile_id,tile.metatile_id);
            assert_eq!(cell.collision,tile.collision);
            assert_eq!(cell.elevation,tile.elevation);
        }
        let neighbor=world.maps.iter().find(|m|m.id==id2).unwrap();
        assert!(neighbor.cells.iter().all(|x|x.is_none()));
    }
}
