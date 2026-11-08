//! Headless entry point for AI agents, CI, and deterministic map compilation.
//! Each mutating command updates the same versioned JSON project used by Android.
use std::{env, fs::{self,OpenOptions}, path::{Path,PathBuf}};
use serde::{Serialize,Deserialize};
use map_studio::{model::{EventKind,GridPoint,TileKind,World,ExpandDirection}, storage, export, projects::{Workspace,GameTarget,ProjectManifest}, compiled, prefabs, tiles, emerald_terrain, emerald_library, transactions};

const HELP:&str = "Map Studio CLI
  workspace-new <root> <name> <emerald|firered>
  workspace-list <root>
  workspace-open <root> <project-uuid>
  workspace-import <root> <name> <emerald|firered> <project.json|project.zip>
  workspace-validate <root> <project-uuid>
  workspace-publish <root> <project-uuid>
  new <project.json>
  inspect <project.json>
  revision <project.json>             (native World sha256 revision)
  inspect-region <project.json> <map-id> <x> <y> <width> <height>
  render-region <project.json> <map-id> <x> <y> <width> <height> <image.png>
  preview-transaction <project.json> <request.json>
  apply-transaction <project.json> <request.json>
  undo-transaction <project.json>    (single guarded local Undo)
  add-map <project.json> <name> <x> <y> <width> <height>
  place-map <project.json> <name> <fixed-x> <fixed-y> <width> <height> <direction>
  paint <project.json> <x> <y> <metatile-id>
  tilesets <project.json>
  set-tileset <project.json> <map-id> <native-pair-key>
  assets <project.json> <map-id>
  assets-json <project.json> <map-id>     (machine-readable compatible artwork)
  asset-json <project.json> <asset-id>   (full native metatile matrix)
  place-asset <project.json> <map-id> <asset-id> <local-x> <local-y>
  place-prefab <project.json> <map-id> <source-prefab-index> <local-x> <local-y>
  objects <project.json> <map-id>
  move-prefab <project.json> <map-id> <instance-uuid> <local-x> <local-y>
  remove-prefab <project.json> <map-id> <instance-uuid>
  walkway <project.json> <map-id> <local-x1> <local-y1> <local-x2> <local-y2>
  erase <project.json> <x> <y>
  fill <project.json> <map-id> <metatile-id>
  move <project.json> <map-id> <x> <y>
  resize <project.json> <map-id> <width> <height> [direction]
  event <project.json> <npc|sign|warp|trigger> <x> <y>
  validate <project.json>
  export <project.json> <output.zip>
  export-project <project.json> <editable.zip>
  import <new-project.json> <backup.json|source.zip>
  restore <project.json> <backup.json|source.zip>
Notes: metatile IDs are decimal or 0xHEX, game export requires all cells
painted and valid scripted events. Directions: upper-left, upper-right,
lower-left, lower-right. GitHub sync is not part of this CLI yet.";

fn field<'a>(args:&'a [String],index:usize,label:&str)->Result<&'a str,String> {
    args.get(index).map(|s|s.as_str()).ok_or(format!("Missing {label}\n{HELP}"))
}

fn int<T>(args:&[String],index:usize,label:&str)->Result<T,String>
where T:TryFrom<u64> {
    let raw=field(args,index,label)?;
    let n=if let Some(hex)=raw.strip_prefix("0x").or_else(||raw.strip_prefix("0X")){
        u64::from_str_radix(hex,16)
    } else {raw.parse::<u64>()}.map_err(|_|format!("{label} must be an integer: {raw}"))?;
    T::try_from(n).map_err(|_|format!("{label} outside supported range: {raw}"))
}
fn coord(args:&[String],index:usize,label:&str)->Result<i32,String> {
    field(args,index,label)?.parse::<i32>().map_err(|_|format!("{label} must be a signed integer"))
}

fn direction(raw:&str)->Result<ExpandDirection,String> {
    match raw.to_ascii_lowercase().replace('_',"-").as_str(){
        "upper-left"|"ul" => Ok(ExpandDirection::UpperLeft),
        "upper-right"|"ur" => Ok(ExpandDirection::UpperRight),
        "lower-left"|"ll" => Ok(ExpandDirection::LowerLeft),
        "lower-right"|"lr" => Ok(ExpandDirection::LowerRight),
        _ => Err("Direction must be upper-left, upper-right, lower-left or lower-right".into()),
    }
}

/// A local CLI lock prevents two independent headless agents from both
/// committing to the same on-disk project revision at once. The GUI has its
/// own native editing loop, so revision checks remain necessary.
struct TransactionLock { path:PathBuf, file:Option<fs::File> }
impl Drop for TransactionLock {
    fn drop(&mut self) {
        self.file.take(); // Close before unlink (also works on Windows).
        let _=fs::remove_file(&self.path);
    }
}
fn metadata_dir(path:&Path)->Result<PathBuf,String>{
    let folder=path.parent().unwrap_or(Path::new(".")).join(".mapstudio");
    fs::create_dir_all(&folder).map_err(|e|format!("Create transaction metadata folder: {e}"))?;
    Ok(folder)
}
fn transaction_filename(path:&Path,extension:&str)->String{
    let name=path.file_name().unwrap_or_default().to_string_lossy();
    format!("{name}.{extension}")
}
fn lock_transaction(path:&Path)->Result<TransactionLock,String>{
    let folder=metadata_dir(path)?;
    let lockfile=folder.join(transaction_filename(path,"transaction.lock"));
    let file=OpenOptions::new().write(true).create_new(true).open(&lockfile)
        .map_err(|e|format!("Project transaction is already locked ({e}); never delete a live lock"))?;
    Ok(TransactionLock{path:lockfile,file:Some(file)})
}
fn receipt_path(path:&Path)->Result<PathBuf,String>{
    Ok(metadata_dir(path)?.join(transaction_filename(path,"last-transaction.json")))
}
#[derive(Clone,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct LastTransaction {
    transaction_id:String,
    before_revision:String,
    after_revision:String,
    before_world:World,
}
fn read_request(path:&Path)->Result<transactions::TransactionRequest,String>{
    let length=fs::metadata(path).map_err(|e|format!("Read transaction request: {e}"))?.len();
    if length>1024*1024 {return Err("Transaction request exceeds 1 MiB".into());}
    let bytes=fs::read(path).map_err(|e|format!("Read transaction request: {e}"))?;
    serde_json::from_slice(&bytes).map_err(|e|format!("Parse transaction JSON: {e}"))
}
fn write_receipt(path:&Path,value:&LastTransaction)->Result<(),String>{
    let receipt=receipt_path(path)?;
    let temporary=receipt.with_extension("json.tmp");
    let bytes=serde_json::to_vec(value)
        .map_err(|e|format!("Encode transaction Undo: {e}"))?;
    if bytes.len()>storage::MAX_PROJECT_BYTES {
        return Err("Undo snapshot exceeds the project's safety limit".into());
    }
    fs::write(&temporary,&bytes).map_err(|e|format!("Write temporary Undo: {e}"))?;
    if let Err(e)=fs::rename(&temporary,&receipt){
        let _=fs::remove_file(&temporary);
        return Err(format!("Commit Undo snapshot: {e}"));
    }
    Ok(())
}

fn run(args:&[String])->Result<(),String> {
    let action=field(args,1,"command")?;
    if action=="help" || action=="--help" {println!("{HELP}");return Ok(());}
    // Every AI operation uses a stable project UUID; ordinary editor model
    // commands continue to accept the selected Editor/.../project.json path.
    if action.starts_with("workspace-"){
        let root=Path::new(field(args,2,"workspace root")?);
        let store=Workspace{root:root.to_path_buf()};
        let target=|raw:&str|->Result<GameTarget,String>{
            match raw{
                "emerald"|"emerald_expansion"=>Ok(GameTarget::EmeraldExpansion),
                "firered"=>Ok(GameTarget::FireRed),
                _=>Err("Target must be emerald or firered".into()),
            }
        };
        match action {
            "workspace-new"=>{
                let name=field(args,3,"project name")?;
                let game=target(field(args,4,"target GBA")?)?;
                let project=store.create(name,game,&World::new())?;
                // Do not change the currently active project on creation.
                println!("Created {} ({}) in Editor/projects/{}/",
                    project.name,project.target.label(),project.project_id);
            }
            "workspace-list"=>{
                for item in store.list()?{
                    let selected=store.active_id()?.as_deref()==Some(&item.project_id);
                    println!("{} {} [{}] {}",
                        if selected{"*"}else{" "},item.project_id,
                        item.target.label(),item.name);
                }
            }
            "workspace-open"=>{
                let id=field(args,3,"project UUID")?;
                store.activate(id)?;
                println!("Selected workspace project {id}");
            }
            "workspace-import"=>{
                let name=field(args,3,"project name")?;
                let game=target(field(args,4,"target GBA")?)?;
                let file=Path::new(field(args,5,"source project")?);
                if std::fs::metadata(file).map_err(|e|e.to_string())?.len()
                    >map_studio::artwork::MAX_PACKAGE_BYTES as u64{
                    return Err("Source project is too large".into());
                }
                let bytes=std::fs::read(file).map_err(|e|e.to_string())?;
                let world=storage::decode_package(&bytes)?;
                let project=store.create(name,game,&world)?;
                println!("Imported {} maps as {}",
                    world.maps.len(),project.project_id);
            }
            "workspace-validate"=>{
                let id=field(args,3,"project UUID")?;
                let manifest=store.load_manifest(id)?;
                let world=store.load_world(id)?;
                if manifest.target.can_export(){export::validate(&world)?;}
                println!("Valid {} project {} ({} maps)",
                    manifest.target.label(),id,world.maps.len());
            }
            "workspace-publish"=>{
                let id=field(args,3,"project UUID")?;
                let path=compiled::publish(&store,id)?;
                println!("Published GBA map source to {}",path.display());
                println!("Native binaries generated; full ROM integration is not yet verified");
            }
            _=>return Err(format!("Unknown workspace command: {action}\n{HELP}")),
        }
        return Ok(());
    }
    let project=Path::new(field(args,2,"project path")?);
    if action=="new" {
        if project.exists(){return Err("Project already exists; refusing to overwrite".into());}
        storage::save(project,&World::new())?;
        println!("Created {}",project.display());
        return Ok(());
    }
    if action=="import" || action=="restore" {
        let input=Path::new(field(args,3,"source backup JSON or ZIP")?);
        if input==project {return Err("Source and target project must be different".into());}
        if action=="import" && project.exists() {
            return Err("Project already exists; use restore to replace it safely".into());
        }
        let size=std::fs::metadata(input).map_err(|e|format!("Read backup: {e}"))?.len();
        if size>storage::MAX_PROJECT_BYTES as u64 {
            return Err("Import exceeds the 20 MiB limit".into());
        }
        let bytes=std::fs::read(input).map_err(|e|e.to_string())?;
        let incoming=storage::decode_package(&bytes)?;
        if action=="restore" {
            let old=storage::load(project)?;
            let destination=project.with_file_name(format!("MapStudio-recovery-{}.json",
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default().as_millis()));
            storage::save(&destination,&old)?;
            println!("Recovery snapshot: {}",destination.display());
        }
        storage::save(project,&incoming)?;
        println!("Imported {} maps to {}",incoming.maps.len(),project.display());
        return Ok(());
    }
    // Project-scoped native source operations must respect the declared GBA
    // target. A FireRed project must never produce Emerald-layout source.
    if matches!(action,"validate"|"export")
        && project.file_name().is_some_and(|name|name=="project.json") {
        let manifest_path=project.with_file_name("manifest.json");
        if manifest_path.exists(){
            let data=std::fs::read(&manifest_path).map_err(|e|e.to_string())?;
            let manifest:ProjectManifest=serde_json::from_slice(&data)
                .map_err(|e|format!("Invalid project manifest: {e}"))?;
            manifest.validate()?;
            let folder=project.parent().and_then(|p|p.file_name())
                .and_then(|n|n.to_str()).ok_or("Project directory is invalid")?;
            if manifest.project_id!=folder{
                return Err("Project UUID and manifest identity differ".into());
            }
            if !manifest.target.can_export(){
                return Err(format!("{} game-source export is not implemented",
                    manifest.target.label()));
            }
        }
    }
    if matches!(action,"set-tileset"|"assets"|"assets-json"|"asset-json"|"place-asset"
        |"preview-transaction"|"apply-transaction"|"undo-transaction"
        |"place-prefab"|"objects"
        |"move-prefab"|"remove-prefab"|"walkway"){
        if project.file_name().is_some_and(|name|name=="project.json"){
            let manifest_path=project.with_file_name("manifest.json");
            if manifest_path.exists(){
                let manifest:ProjectManifest=serde_json::from_slice(
                    &std::fs::read(&manifest_path).map_err(|e|e.to_string())?)
                    .map_err(|e|format!("Invalid project target manifest: {e}"))?;
                manifest.validate()?;
                let folder=project.parent().and_then(|p|p.file_name())
                    .and_then(|n|n.to_str()).ok_or("Invalid project directory")?;
                if folder!=manifest.project_id || !manifest.target.can_export(){
                    return Err("Emerald source tools cannot edit a different GBA target".into());
                }
            }
        }
    }
    // Structured machine-readable protocol for AI agents. These commands
    // never bypass the native World model: only prepare() may execute edits,
    // and it works on a cloned draft until every step succeeds.
    if matches!(action,"revision"|"inspect-region"|"render-region"|"preview-transaction"
                       |"apply-transaction"|"undo-transaction") {
        let _lock=if matches!(action,"apply-transaction"|"undo-transaction"){
            Some(lock_transaction(project)?)
        }else{None};
        let original=storage::load(project)?;
        match action {
            "revision"=>{
                println!("{}",serde_json::json!({
                    "revision":transactions::revision(&original)?,
                    "maps":original.maps.len(),
                    "protocol_version":transactions::PROTOCOL_VERSION
                }));
            }
            "inspect-region"=>{
                let map_id:u32=int(args,3,"map-id")?;
                let x:u32=int(args,4,"local-x")?;
                let y:u32=int(args,5,"local-y")?;
                let w:u32=int(args,6,"width")?;
                let h:u32=int(args,7,"height")?;
                let inspected=transactions::inspect_region(&original,map_id,x,y,w,h)?;
                println!("{}",serde_json::to_string_pretty(&inspected)
                    .map_err(|e|e.to_string())?);
            }
            "render-region"=>{
                let map_id:u32=int(args,3,"map-id")?;
                let x:u32=int(args,4,"local-x")?;
                let y:u32=int(args,5,"local-y")?;
                let w:u32=int(args,6,"width")?;
                let h:u32=int(args,7,"height")?;
                let out=Path::new(field(args,8,"output PNG path")?);
                let image=transactions::render_region_png(&original,map_id,x,y,w,h)?;
                fs::write(out,&image).map_err(|e|format!("Write native map PNG: {e}"))?;
                println!("{}",serde_json::json!({
                    "status":"rendered","revision":transactions::revision(&original)?,
                    "path":out.display().to_string(),"bytes":image.len(),
                    "map_id":map_id,"origin":[x,y],"size":[w,h]
                }));
            }
            "preview-transaction"|"apply-transaction"=>{
                let request=read_request(Path::new(field(args,3,"transaction JSON file")?))?;
                if action=="apply-transaction" && args.len()!=4 {
                    return Err("Commit takes exactly one transaction file; render separately".into());
                }
                if action=="preview-transaction" && args.len()!=4 && args.len()!=10 {
                    return Err("Preview optional PNG: <map-id> <x> <y> <width> <height> <out.png>".into());
                }
                let prepared=transactions::prepare(&original,&request)?;
                if action=="apply-transaction"{
                    if prepared.report.result_revision==prepared.report.base_revision{
                        return Err("Transaction made no changes; not writing an empty commit".into());
                    }
                    let undo=LastTransaction {
                        transaction_id:request.transaction_id.clone(),
                        before_revision:prepared.report.base_revision.clone(),
                        after_revision:prepared.report.result_revision.clone(),
                        before_world:original,
                    };
                    // Preserve the prior World before writing the new project.
                    write_receipt(project,&undo)?;
                    storage::save(project,&prepared.world)?;
                }
                let mut response=serde_json::to_value(&prepared.report)
                    .map_err(|e|e.to_string())?;
                response["status"]=serde_json::json!(
                    if action=="apply-transaction"{"committed"}else{"preview_only"});
                if action=="preview-transaction" && args.len()==10 {
                    let map_id:u32=int(args,4,"map-id")?;
                    let x:u32=int(args,5,"local-x")?;
                    let y:u32=int(args,6,"local-y")?;
                    let w:u32=int(args,7,"width")?;
                    let h:u32=int(args,8,"height")?;
                    let out=Path::new(field(args,9,"output PNG path")?);
                    let image=transactions::render_region_png(
                        &prepared.world,map_id,x,y,w,h)?;
                    fs::write(out,&image)
                        .map_err(|e|format!("Write uncommitted preview PNG: {e}"))?;
                    response["rendered_preview"]=serde_json::json!({
                        "path":out.display().to_string(),"bytes":image.len(),
                        "map_id":map_id,"origin":[x,y],"size":[w,h]
                    });
                }
                println!("{}",serde_json::to_string_pretty(&response)
                    .map_err(|e|e.to_string())?);
            }
            "undo-transaction"=>{
                let receipt=receipt_path(project)?;
                let meta=fs::metadata(&receipt)
                    .map_err(|_|"No saved local transaction Undo exists")?;
                if meta.len()>storage::MAX_PROJECT_BYTES as u64 {
                    return Err("Undo snapshot exceeds safe size".into());
                }
                let data=fs::read(&receipt).map_err(|e|format!("Read Undo: {e}"))?;
                let undo:LastTransaction=serde_json::from_slice(&data)
                    .map_err(|e|format!("Parse Undo: {e}"))?;
                if transactions::revision(&original)?!=undo.after_revision {
                    return Err("STALE_REVISION: project changed since that transaction; Undo refused".into());
                }
                if transactions::revision(&undo.before_world)?!=undo.before_revision {
                    return Err("Undo snapshot failed integrity/revision verification".into());
                }
                storage::save(project,&undo.before_world)?;
                fs::remove_file(&receipt).map_err(|e|format!("Remove used Undo: {e}"))?;
                println!("{}",serde_json::json!({
                    "status":"undone",
                    "transaction_id":undo.transaction_id,
                    "revision":undo.before_revision
                }));
            }
            _=>unreachable!(),
        }
        return Ok(());
    }
    let mut world=storage::load(project)?;
    let mut changed=false;
    match action {
        "inspect" => {
            println!("Maps: {} | Connections: {}",world.maps.len(),world.connections().len());
            for map in &world.maps {
                println!("  ID={} {} at ({},{}) {}x{} painted={}/{} events={}",
                    map.id,map.name,map.origin.x,map.origin.y,map.width,map.height,
                    map.cells.iter().filter(|c|c.is_some()).count(),map.cells.len(),map.events.len());
                let count=world.prefab_instances.iter().filter(|o|o.map_id==map.id).count();
                println!("    Tracked source prefab objects: {count}");
            }
        }
        "add-map" => {
            let name=field(args,3,"name")?;
            let p=GridPoint::new(coord(args,4,"x")?,coord(args,5,"y")?);
            let width=int(args,6,"width")?;
            let height=int(args,7,"height")?;
            let id=world.add_map(name,p,width,height).map_err(|e|e.to_owned())?;
            changed=true;
            println!("Created map ID {id}");
        }
        "place-map" => {
            let name=field(args,3,"name")?;
            let p=GridPoint::new(coord(args,4,"fixed-x")?,coord(args,5,"fixed-y")?);
            let width=int(args,6,"width")?;
            let height=int(args,7,"height")?;
            let dir=direction(field(args,8,"direction")?)?;
            let origin=dir.origin_at(p,width,height)?;
            let id=world.add_map(name,origin,width,height).map_err(|e|e.to_owned())?;
            changed=true;
            println!("Created map ID {id}, growing {} from fixed point",dir.label());
        }
        "paint" => {
            let p=GridPoint::new(coord(args,3,"x")?,coord(args,4,"y")?);
            let tile=int(args,5,"metatile-id")?;
            if !world.paint_metatile(p,tile){return Err("No change: outside map or invalid metatile ID".into());}
            changed=true;
        }
        "tilesets" => {
            for pair in tiles::pairs().iter().filter(|p|p.collection=="Emerald"){
                println!("{} | {} | {} | {}",pair.key,
                    pair.label,pair.primary_symbol,pair.secondary_symbol);
            }
        }
        "set-tileset" => {
            let id:u32=int(args,3,"map-id")?;
            let pair=field(args,4,"native tileset pair")?;
            let target=tiles::pair(pair).ok_or("Unknown source pair")?;
            if target.collection!="Emerald"{
                return Err("Only source-verified Emerald tilesets are editable".into());
            }
            world.set_map_tileset(id,pair)?;
            changed=true;
            println!("Map #{id} now uses source tileset {pair}");
        }
        "assets" => {
            let id:u32=int(args,3,"map-id")?;
            let map=world.maps.iter().find(|m|m.id==id).ok_or("Map ID not found")?;
            let pair=tiles::pair_for(&map.primary_tileset,&map.secondary_tileset)
                .ok_or("Unrecognized native tileset pair")?;
            if pair.collection!="Emerald"{
                return Err("Emerald prefab tool does not modify other GBA collections".into());
            }
            println!("Compatible native prefab assets for map {} ({})",id,pair.key);
            for (index,prefab) in prefabs::emerald().iter().enumerate(){
                if prefabs::compatible(prefab,pair){
                    println!("  id={} | {} | {}x{} | primary={} secondary={}",
                        index,prefab.name,prefab.width,prefab.height,
                        prefab.primary_tileset,prefab.secondary_tileset);
                }
            }
        }
        "assets-json" => {
            let id:u32=int(args,3,"map-id")?;
            let map=world.maps.iter().find(|m|m.id==id).ok_or("Map ID not found")?;
            let pair=tiles::pair_for(&map.primary_tileset,&map.secondary_tileset)
                .ok_or("Unrecognized native tileset pair")?;
            let entries=emerald_library::list(pair,emerald_library::Group::All);
            let assets:Vec<_>=entries.into_iter().filter(|item|item.compatible)
                .map(|item|{
                    let asset=&prefabs::emerald()[item.index];
                    serde_json::json!({
                        "asset_id":prefabs::asset_id(item.index).unwrap(),
                        "name":asset.name,
                        "category":item.category.name(),
                        "width":asset.width,
                        "height":asset.height,
                        "source_pair":pair.key,
                        "source_map":asset.source_map,
                        "metatile_count":asset.metatiles.len(),
                    })
                }).collect();
            println!("{}",serde_json::json!({
                "map_id":id,"native_source":pair.key,"assets":assets
            }));
        }
        "asset-json" => {
            let id=field(args,3,"stable asset ID")?;
            let (index,asset)=prefabs::by_asset_id(id)
                .ok_or("Unknown Emerald source-backed asset ID")?;
            println!("{}",serde_json::json!({
                "asset_id":id,"legacy_index":index,
                "source_map":asset.source_map,"semantic_class":
                    emerald_library::group_for(asset).name(),
                "name":asset.name,"width":asset.width,"height":asset.height,
                "primary_tileset":asset.primary_tileset,
                "secondary_tileset":asset.secondary_tileset,
                "matrix":asset.metatiles,
            }));
        }
        "place-asset" => {
            let id:u32=int(args,3,"map-id")?;
            let asset_id=field(args,4,"stable asset ID")?;
            let (index,asset)=prefabs::by_asset_id(asset_id)
                .ok_or("Unknown Emerald source-backed asset ID")?;
            let x:i32=coord(args,5,"local-x")?;
            let y:i32=coord(args,6,"local-y")?;
            let map=world.maps.iter().find(|m|m.id==id)
                .ok_or("Map ID not found")?;
            let pair=tiles::pair_for(&map.primary_tileset,&map.secondary_tileset)
                .ok_or("Unknown map source pair")?;
            if pair.collection!="Emerald" || !prefabs::compatible(asset,pair){
                return Err(format!("Asset {asset_id} is incompatible with {}",
                    pair.key));
            }
            let pos=GridPoint::new(map.origin.x.checked_add(x)
                .ok_or("Asset X coordinate overflow")?,
                map.origin.y.checked_add(y).ok_or("Asset Y coordinate overflow")?);
            world.active_space_id=map.space_id;
            let n=world.try_paint_prefab(pos,asset)?;
            changed=true;
            println!("Placed {asset_id} (source index {index}) at map {id} ({x},{y}), {n} native cells");
        }
        "objects" => {
            let id:u32=int(args,3,"map-id")?;
            if !world.maps.iter().any(|m|m.id==id){
                return Err("Map ID not found".into());
            }
            for obj in world.prefab_instances.iter().filter(|o|o.map_id==id){
                let name=prefabs::emerald().get(obj.prefab_index)
                    .map(|p|p.name.as_str()).unwrap_or("Source asset missing");
                println!("{} | #{} {} | ({},{}) {}x{} | source={}/{}",
                    obj.instance_id,obj.prefab_index,name,obj.x,obj.y,
                    obj.width,obj.height,obj.primary_tileset,obj.secondary_tileset);
            }
        }
        "move-prefab" => {
            let id:u32=int(args,3,"map-id")?;
            let uuid=field(args,4,"source prefab instance UUID")?;
            let x:i32=int(args,5,"local-x")?;
            let y:i32=int(args,6,"local-y")?;
            let obj=world.prefab_instances.iter()
                .find(|p|p.instance_id==uuid && p.map_id==id)
                .ok_or("Tracked prefab instance not found in requested map")?;
            let map=world.maps.iter().find(|m|m.id==obj.map_id).ok_or("Map not found")?;
            let pos=GridPoint::new(map.origin.x.checked_add(x)
                .ok_or("X coordinate overflow")?,
                map.origin.y.checked_add(y).ok_or("Y coordinate overflow")?);
            let count=world.move_prefab_instance(uuid,pos)?;
            changed=true;
            println!("Moved prefab {uuid} to ({x},{y}) · {count} native tiles");
        }
        "remove-prefab" => {
            let id:u32=int(args,3,"map-id")?;
            let uuid=field(args,4,"source prefab instance UUID")?;
            if !world.prefab_instances.iter().any(|p|p.instance_id==uuid&&p.map_id==id){
                return Err("Prefab instance not found in requested map".into());
            }
            let count=world.remove_prefab_instance(uuid)?;
            changed=true;
            println!("Removed prefab {uuid}, restored {count} original native tiles");
        }
        "place-prefab" => {
            let id:u32=int(args,3,"map-id")?;
            let asset_index:usize=int(args,4,"source-prefab-index")?;
            let x:u32=int(args,5,"local-x")?;
            let y:u32=int(args,6,"local-y")?;
            let asset=prefabs::emerald().get(asset_index)
                .ok_or("Emerald source prefab index not found")?;
            let map=world.maps.iter().find(|m|m.id==id)
                .ok_or("Map ID not found")?;
            let pair=tiles::pair_for(&map.primary_tileset,&map.secondary_tileset)
                .ok_or("Unknown native map tileset")?;
            if pair.collection!="Emerald"{
                return Err("Cannot use Emerald prefab on another GBA source".into());
            }
            let x:i32=x.try_into().map_err(|_|"Local X is too large")?;
            let y:i32=y.try_into().map_err(|_|"Local Y is too large")?;
            let pos=GridPoint::new(map.origin.x.checked_add(x)
                .ok_or("Prefab coordinate overflow")?,
                map.origin.y.checked_add(y).ok_or("Prefab coordinate overflow")?);
            world.active_space_id=map.space_id;
            let changed_count=world.try_paint_prefab(pos,asset)?;
            if changed_count==0{return Err("Prefab already exists here".into());}
            changed=true;
            println!("Placed Emerald asset #{asset_index} {} at map #{id} local ({x},{y}), {} cells",
                asset.name,changed_count);
        }
        "walkway" => {
            let id:u32=int(args,3,"map-id")?;
            let local_x:i32=int(args,4,"local-x1")?;
            let local_y:i32=int(args,5,"local-y1")?;
            let local_x2:i32=int(args,6,"local-x2")?;
            let local_y2:i32=int(args,7,"local-y2")?;
            let map=world.maps.iter().find(|m|m.id==id).ok_or("Map ID not found")?;
            let origin=map.origin;
            let from=GridPoint::new(origin.x.checked_add(local_x)
                .ok_or("Pavement X coordinate overflow")?,
                origin.y.checked_add(local_y).ok_or("Pavement Y coordinate overflow")?);
            let to=GridPoint::new(origin.x.checked_add(local_x2)
                .ok_or("Pavement X coordinate overflow")?,
                origin.y.checked_add(local_y2).ok_or("Pavement Y coordinate overflow")?);
            let modified=emerald_terrain::draw_lilycove_walkway(&mut world,id,from,to)?;
            if modified==0{return Err("Native walkway already present".into());}
            changed=true;
            println!("Painted {modified} native Lilycove paved walkway cells");
        }
        "erase" => {
            let p=GridPoint::new(coord(args,3,"x")?,coord(args,4,"y")?);
            if !world.erase(p){return Err("No cell was erased".into());}
            changed=true;
        }
        "fill" => {
            let id:u32=int(args,3,"map-id")?;
            let tile:u16=int(args,4,"metatile-id")?;
            if tile>=1024 {return Err("Metatile ID must be in 0..1023".into());}
            let map=world.maps.iter_mut().find(|m|m.id==id).ok_or("Map ID not found")?;
            let cell=map_studio::model::Cell {terrain:TileKind::Grass,metatile_id:tile,
                collision:0,elevation:3};
            for slot in &mut map.cells {if slot.is_none(){*slot=Some(cell);changed=true;}}
        }
        "move" => {
            let id=int(args,3,"map-id")?;
            let p=GridPoint::new(coord(args,4,"x")?,coord(args,5,"y")?);
            if !world.move_frame(id,p){return Err("Frame move rejected (overlap or missing ID)".into());}
            changed=true;
        }
        "resize" => {
            let id=int(args,3,"map-id")?;
            let width=int(args,4,"width")?;
            let height=int(args,5,"height")?;
            let dir=match args.get(6) {
                Some(s)=>direction(s)?,
                None=>ExpandDirection::LowerRight,
            };
            world.resize_frame_towards(id,width,height,dir)?;
            changed=true;
        }
        "event" => {
            let kind=match field(args,3,"kind")? {
                "npc"=>EventKind::Npc,"sign"=>EventKind::Sign,
                "warp"=>EventKind::Warp,"trigger"=>EventKind::Trigger,
                _=>return Err("Event kind must be npc, sign, warp or trigger".into())
            };
            let p=GridPoint::new(coord(args,4,"x")?,coord(args,5,"y")?);
            let map=world.map_at(p).ok_or("No map at event position")?;
            let x=(p.x-map.origin.x) as u32;
            let y=(p.y-map.origin.y) as u32;
            if map.events.iter().any(|e|e.x==x && e.y==y){
                return Err("This cell already has an event; edit the project rather than replacing silently".into());
            }
            let info=world.add_event(p,kind).ok_or("Failed to create event")?;
            changed=true;
            println!("Created event in map {} at index {}",info.0,info.1);
        }
        "validate" => {export::validate(&world)?;println!("Ready to export.");}
        "export" => {
            let output=Path::new(field(args,3,"output zip")?);
            export::export_zip(&world,output)?;
            println!("Exported game source {}",output.display());
        }
        "export-project" => {
            let output=Path::new(field(args,3,"output zip")?);
            storage::export_project_zip(output,&world)?;
            println!("Exported editable project {}",output.display());
        }
        _=>return Err(format!("Unknown command: {action}\n{HELP}")),
    }
    if changed {storage::save(project,&world)?;}
    Ok(())
}

fn main() {
    let args:Vec<String>=env::args().collect();
    if let Err(err)=run(&args){
        eprintln!("Map Studio: {err}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transaction_preview_apply_undo_and_stale_revision_are_safe(){
        let work=std::env::temp_dir().join(format!(
            "mapstudio-transactions-test-{}",uuid::Uuid::new_v4()));
        fs::create_dir_all(&work).unwrap();
        let project=work.join("project.json");
        let request_file=work.join("transaction.json");
        let preview_png=work.join("preview.png");
        let mut initial=World::new();
        let id=initial.add_map("Town",GridPoint::new(0,0),20,20).unwrap();
        initial.set_map_tileset(id,"general/lilycove").unwrap();
        storage::save(&project,&initial).unwrap();
        let rev=transactions::revision(&initial).unwrap();
        let request=transactions::TransactionRequest{
            protocol_version:transactions::PROTOCOL_VERSION,
            transaction_id:uuid::Uuid::new_v4().to_string(),
            base_revision:rev,
            operations:vec![transactions::MapOperation::FillRect{
                map_id:id,x:1,y:1,width:3,height:4,metatile_id:1
            }],
        };
        fs::write(&request_file,serde_json::to_vec(&request).unwrap()).unwrap();
        let p=project.to_string_lossy().to_string();
        let t=request_file.to_string_lossy().to_string();
        run(&["bin".into(),"preview-transaction".into(),p.clone(),t.clone()]).unwrap();
        assert_eq!(storage::load(&project).unwrap(),initial);
        run(&["bin".into(),"preview-transaction".into(),p.clone(),t.clone(),
            id.to_string(),"0".into(),"0".into(),"8".into(),"8".into(),
            preview_png.to_string_lossy().to_string()]).unwrap();
        assert!(fs::read(&preview_png).unwrap().starts_with(b"\x89PNG"));
        assert_eq!(storage::load(&project).unwrap(),initial);
        run(&["bin".into(),"apply-transaction".into(),p.clone(),t.clone()]).unwrap();
        let after=storage::load(&project).unwrap();
        assert_ne!(after,initial);
        assert!(run(&["bin".into(),"apply-transaction".into(),p.clone(),t]).is_err());
        assert_eq!(storage::load(&project).unwrap(),after);
        run(&["bin".into(),"undo-transaction".into(),p]).unwrap();
        assert_eq!(storage::load(&project).unwrap(),initial);
        let _=fs::remove_dir_all(&work);
    }

    #[test]
    fn number_decoder_supports_hex_and_signed_positions() {
        let args=vec!["bin".into(),"paint".into(),"p".into(),"-7".into(),"12".into(),"0x170".into()];
        assert_eq!(coord(&args,3,"x").unwrap(),-7);
        assert_eq!(int::<u16>(&args,5,"tile").unwrap(),0x170);
    }
    #[test]
    fn set_tileset_and_native_paving_are_safe_ai_operations(){
        let path=std::env::temp_dir().join(format!(
            "mapstudio-cli-native-road-{}.json",uuid::Uuid::new_v4()));
        let mut world=World::new();
        let id=world.add_map("New Town",GridPoint::new(0,0),40,40).unwrap();
        storage::save(&path,&world).unwrap();
        let p=path.to_string_lossy().into_owned();
        run(&["bin".into(),"set-tileset".into(),p.clone(),id.to_string(),
            "general/lilycove".into()]).unwrap();
        run(&["bin".into(),"walkway".into(),p.clone(),id.to_string(),
            "8".into(),"12".into(),"24".into(),"12".into()]).unwrap();
        let w=storage::load(&path).unwrap();
        assert_eq!(w.maps[0].cell(8,12).unwrap().metatile_id,0x233);
        assert_eq!(w.maps[0].cell(7,11).unwrap().metatile_id,0x22A);
        assert!(run(&["bin".into(),"set-tileset".into(),p.clone(),id.to_string(),
            "general/petalburg".into()]).is_err());
        assert_eq!(storage::load(&path).unwrap(),w);
        let _=std::fs::remove_file(path);
    }
    #[test]
    fn prefab_cli_move_and_remove_preserve_original_map_data(){
        let path=std::env::temp_dir().join(format!(
            "mapstudio-cli-objects-{}.json",uuid::Uuid::new_v4()));
        let mut world=World::new();
        let id=world.add_map("Estate",GridPoint::new(0,0),40,40).unwrap();
        world.set_map_tileset(id,"general/lilycove").unwrap();
        storage::save(&path,&world).unwrap();
        let original=world.clone();
        let p=path.to_string_lossy().into_owned();
        let pair=tiles::pair("general/lilycove").unwrap();
        let asset_index=prefabs::compatible_indices(pair)[0];
        let place=vec!["bin".into(),"place-prefab".into(),p.clone(),
            id.to_string(),asset_index.to_string(),"4".into(),"4".into()];
        run(&place).unwrap();
        let placed=storage::load(&path).unwrap();
        assert_eq!(placed.prefab_instances.len(),1);
        let uuid=placed.prefab_instances[0].instance_id.clone();
        run(&["bin".into(),"move-prefab".into(),p.clone(),id.to_string(),
            uuid.clone(),"20".into(),"20".into()]).unwrap();
        let moved=storage::load(&path).unwrap();
        assert_eq!(moved.prefab_instances.len(),1);
        assert_eq!(moved.prefab_instances[0].x,20);
        run(&["bin".into(),"remove-prefab".into(),p.clone(),id.to_string(),
            uuid]).unwrap();
        assert_eq!(storage::load(&path).unwrap(),original);
        let _=std::fs::remove_file(path);
    }
    #[test]
    fn ai_can_place_source_verified_environment_by_stable_id(){
        let path=std::env::temp_dir().join(format!(
            "mapstudio-assetid-{}.json",uuid::Uuid::new_v4()));
        let mut world=World::new();
        let id=world.add_map("Route",GridPoint::new(0,0),36,36).unwrap();
        world.set_map_tileset(id,"general/rustboro").unwrap();
        storage::save(&path,&world).unwrap();
        let source=path.to_string_lossy().to_string();
        let asset_id="emerald:environment:pond-bank-route104";
        let args=vec!["bin".into(),"place-asset".into(),source.clone(),
            id.to_string(),asset_id.into(),"5".into(),"6".into()];
        run(&args).unwrap();
        let after=storage::load(&path).unwrap();
        assert_eq!(after.prefab_instances.len(),1);
        assert_eq!(after.prefab_instances[0].prefab_index,73);
        let original=after.clone();
        assert!(run(&["bin".into(),"place-asset".into(),source.clone(),
            id.to_string(),"unknown:asset".into(),"9".into(),"9".into()]).is_err());
        assert_eq!(storage::load(&path).unwrap(),original);
        let _=std::fs::remove_file(path);
    }
    #[test]
    fn source_prefab_cli_uses_native_full_footprint_transaction(){
        let path=std::env::temp_dir().join(format!(
            "mapstudio-cli-prefab-{}.json",uuid::Uuid::new_v4()));
        let mut world=World::new();
        let map_id=world.add_map("Town",GridPoint::new(0,0),18,18).unwrap();
        world.set_map_tileset(map_id,"general/petalburg").unwrap();
        storage::save(&path,&world).unwrap();
        let p=path.to_string_lossy().to_string();
        let args=|x:u32,y:u32|{
            vec!["bin".into(),"place-prefab".into(),p.clone(),
                map_id.to_string(),"1".into(),x.to_string(),y.to_string()]
        };
        assert!(run(&args(3,3)).is_ok());
        let before=storage::load(&path).unwrap();
        assert!(run(&args(17,17)).is_err());
        assert_eq!(storage::load(&path).unwrap(),before);
        let _=std::fs::remove_file(path);
    }
    #[test]
    fn script_must_never_overwrite_project_without_request() {
        let path=std::env::temp_dir().join(format!("mapstudio-cli-{}.json",std::process::id()));
        let _=std::fs::remove_file(&path);
        let args=vec!["bin".into(),"new".into(),path.to_string_lossy().into_owned()];
        run(&args).unwrap();
        assert!(run(&args).unwrap_err().contains("already exists"));
        std::fs::remove_file(path).unwrap();
    }
}

#[cfg(test)]
mod direction_cli_tests {
    use super::*;
    #[test]
    fn parser_accepts_four_directions_and_rejects_unknown() {
        for name in ["ul","ur","ll","lr","upper-left","upper-right","lower-left","lower-right"] {
            assert!(direction(name).is_ok(), "{name}");
        }
        assert!(direction("downward").is_err());
    }
    #[test]
    fn create_and_resize_anchored_frames_uses_same_engine_as_apk() {
        let path=std::env::temp_dir()
            .join(format!("mapstudio-cli-corners-{}.json",std::process::id()));
        let _=std::fs::remove_file(&path);
        let p=path.to_string_lossy().to_string();
        for cmd in [
            vec!["bin","new",p.as_str()],
            vec!["bin","place-map",p.as_str(),"A","20","30","3","4","upper-left"],
            vec!["bin","resize",p.as_str(),"1","5","6","upper-right"],
        ] {
            let args:Vec<String>=cmd.iter().map(|s|s.to_string()).collect();
            run(&args).unwrap();
        }
        let world=storage::load(&path).unwrap();
        assert_eq!(world.maps[0].origin,GridPoint::new(18,25));
        assert_eq!(world.maps[0].width,5);
        assert_eq!(world.maps[0].height,6);
        std::fs::remove_file(path).unwrap();
    }
}

#[cfg(test)]
mod import_restore_cli_tests {
    use super::*;
    #[test]
    fn rejects_replacement_without_explicit_restore() {
        let root=std::env::temp_dir();
        let target=root.join(format!("ms-import-existing-{}.json",std::process::id()));
        let source=root.join(format!("ms-import-source-{}.json",std::process::id()));
        let original=World::sample();
        storage::save(&target,&original).unwrap();
        storage::save(&source,&World::new()).unwrap();
        let command=vec!["bin","import",target.to_str().unwrap(),source.to_str().unwrap()]
            .into_iter().map(str::to_string).collect::<Vec<_>>();
        assert!(run(&command).unwrap_err().contains("already exists"));
        assert_eq!(storage::load(&target).unwrap(),original);
        let _=std::fs::remove_file(target);
        let _=std::fs::remove_file(source);
    }
    #[test]
    fn imports_valid_new_project() {
        let root=std::env::temp_dir();
        let target=root.join(format!("ms-import-new-{}.json",std::process::id()));
        let source=root.join(format!("ms-import-existing-source-{}.json",std::process::id()));
        let _=std::fs::remove_file(&target);
        storage::save(&source,&World::sample()).unwrap();
        let command=vec!["bin","import",target.to_str().unwrap(),source.to_str().unwrap()]
            .into_iter().map(str::to_string).collect::<Vec<_>>();
        run(&command).unwrap();
        assert_eq!(storage::load(&target).unwrap(),World::sample());
        let _=std::fs::remove_file(target);
        let _=std::fs::remove_file(source);
    }
}

#[cfg(test)]
mod portable_zip_cli_tests{
    use super::*;
    #[test]
    fn unfinished_project_can_be_exported_and_reimported(){
        let temp=std::env::temp_dir();
        let tag=std::process::id();
        let source=temp.join(format!("ms-editable-{tag}.json"));
        let archive=temp.join(format!("ms-editable-{tag}.zip"));
        let target=temp.join(format!("ms-editable-copy-{tag}.json"));
        let _=std::fs::remove_file(&target);
        let _=std::fs::remove_file(&archive);
        let mut world=World::new();
        world.add_map("Unfinished",GridPoint::new(0,0),7,7).unwrap();
        storage::save(&source,&world).unwrap();
        run(&["bin".into(),"export-project".into(),source.display().to_string(),
            archive.display().to_string()]).unwrap();
        run(&["bin".into(),"import".into(),target.display().to_string(),
            archive.display().to_string()]).unwrap();
        assert_eq!(storage::load(&target).unwrap(),world);
        for p in [source,archive,target] {let _=std::fs::remove_file(p);}
    }
}
