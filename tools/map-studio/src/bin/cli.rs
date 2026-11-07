//! Headless entry point for AI agents, CI, and deterministic map compilation.
//! Each mutating command updates the same versioned JSON project used by Android.
use std::{env, path::Path};
use map_studio::{model::{EventKind,GridPoint,TileKind,World,ExpandDirection}, storage, export};

const HELP:&str = "Map Studio CLI
  new <project.json>
  inspect <project.json>
  add-map <project.json> <name> <x> <y> <width> <height>
  place-map <project.json> <name> <fixed-x> <fixed-y> <width> <height> <direction>
  paint <project.json> <x> <y> <metatile-id>
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

fn run(args:&[String])->Result<(),String> {
    let action=field(args,1,"command")?;
    if action=="help" || action=="--help" {println!("{HELP}");return Ok(());}
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
    let mut world=storage::load(project)?;
    let mut changed=false;
    match action {
        "inspect" => {
            println!("Maps: {} | Connections: {}",world.maps.len(),world.connections().len());
            for map in &world.maps {
                println!("  ID={} {} at ({},{}) {}x{} painted={}/{} events={}",
                    map.id,map.name,map.origin.x,map.origin.y,map.width,map.height,
                    map.cells.iter().filter(|c|c.is_some()).count(),map.cells.len(),map.events.len());
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
    fn number_decoder_supports_hex_and_signed_positions() {
        let args=vec!["bin".into(),"paint".into(),"p".into(),"-7".into(),"12".into(),"0x170".into()];
        assert_eq!(coord(&args,3,"x").unwrap(),-7);
        assert_eq!(int::<u16>(&args,5,"tile").unwrap(),0x170);
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
