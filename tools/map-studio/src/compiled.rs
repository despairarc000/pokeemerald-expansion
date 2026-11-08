//! Deterministic, project-scoped native source publication.
//! Compiled/projects/<uuid> contains generated GBA map source files only.
//! Never rewrites the target game checkout or assumes a successful ROM build.
use crate::{export,model::World,projects::{GameTarget,ProjectManifest,Workspace}};
use serde_json::{json,Value};
use std::{fs,path::{Path,PathBuf}};

fn write(path:&Path,bytes:&[u8])->Result<(),String>{
    if let Some(parent)=path.parent(){fs::create_dir_all(parent).map_err(|e|e.to_string())?;}
    fs::write(path,bytes).map_err(|e|format!("Write {}: {e}",path.display()))
}
fn json_file(path:&Path,value:&Value)->Result<(),String>{
    let content=serde_json::to_vec_pretty(value).map_err(|e|e.to_string())?;
    write(path,&content)
}
fn sanitized_world(world:&World,id:&str)->World {
    let mut result=world.clone();
    // A stable per-project namespace prevents two projects with a "Town"
    // map from generating the same MAP_MEADOWMERE_TOWN game symbol.
    let namespace=id.replace('-',"");
    for map in &mut result.maps{
        map.name=format!("P{namespace}{}",map.name);
    }
    result
}

pub fn publish(workspace:&Workspace,id:&str)->Result<PathBuf,String>{
    let manifest=workspace.load_manifest(id)?;
    if manifest.target!=GameTarget::EmeraldExpansion || !manifest.target.can_export(){
        return Err(format!("{} game-source compilation is not supported by the current Emerald exporter",
            manifest.target.label()));
    }
    let world=workspace.load_world(id)?;
    let game_world=sanitized_world(&world,id);
    export::validate(&game_world)?;
    let root=workspace.compiled_root();
    fs::create_dir_all(&root).map_err(|e|e.to_string())?;
    let stage=root.join(format!(".{id}.staging-{}",uuid::Uuid::new_v4()));
    fs::create_dir(&stage).map_err(|e|e.to_string())?;
    let built=(||->Result<(),String>{
        let mut published=Vec::new();
        for map in &game_world.maps {
            let name=export::map_name(map);
            let layout=export::layout_json(map);
            let rows=layout["cells"].as_array().ok_or("Invalid exported cell grid")?;
            let mut packed=Vec::new();
            for row in rows{
                for cell in row.as_array().ok_or("Invalid exported cell row")?{
                    let n=cell.as_u64().ok_or("Invalid packed metatile")?;
                    let value=u16::try_from(n).map_err(|_|"Packed metatile exceeds u16")?;
                    packed.extend_from_slice(&value.to_le_bytes());
                }
            }
            if packed.len()!=map.cells.len()*2 {
                return Err("Native map binary dimensions differ from editor project".into());
            }
            let mut border=Vec::new();
            for tile in layout["border"].as_array().ok_or("Invalid border")? {
                let n=tile.as_u64().ok_or("Invalid native border tile")?;
                let value=u16::try_from(n).map_err(|_|"Border metatile exceeds u16")?;
                border.extend_from_slice(&value.to_le_bytes());
            }
            if border.len()!=8{return Err("Native border must be 2x2 tiles".into());}
            let map_dir=stage.join("data").join("maps").join(&name);
            let layout_dir=stage.join("data").join("layouts").join(&name);
            let custom_dir=stage.join("custom_maps").join(&name);
            write(&layout_dir.join("map.bin"),&packed)?;
            write(&layout_dir.join("border.bin"),&border)?;
            json_file(&map_dir.join("map.json"),&export::map_json(map,&game_world))?;
            write(&map_dir.join("scripts.inc"),export::script_text(map,&game_world).as_bytes())?;
            // Keep the exact numeric input so the repository's existing
            // textmap generator can perform native map registrations.
            json_file(&custom_dir.join("layout.mapstudio.json"),&layout)?;
            json_file(&custom_dir.join("map.json"),&export::map_json(map,&game_world))?;
            write(&custom_dir.join("scripts.inc"),export::script_text(map,&game_world).as_bytes())?;
            published.push(json!({
                "editor_map_id":world.maps.iter().find(|m|m.id==map.id).map(|m|m.id),
                "game_map":format!("MAP_{}",name.to_uppercase()),
                "game_layout":format!("LAYOUT_{}",name.to_uppercase()),
                "source_directory":format!("custom_maps/{name}"),
                "native_layout_directory":format!("data/layouts/{name}"),
                "native_map_directory":format!("data/maps/{name}")
            }));
        }
        json_file(&stage.join("source-manifest.json"),&json!({
            "format":"mapstudio-compiled-source-v1",
            "project_id":manifest.project_id,
            "project_name":manifest.name,
            "target":"emerald_expansion",
            "target_repository":manifest.target_repository,
            "target_commit":manifest.target_commit,
            "validation":"editor-and-native-source-only",
            "rom_build_verified":false,
            "note":"Use the target game's textmap builder to register the maps; complete ROM compilation requires the target checkout.",
            "maps":published
        }))?;
        Ok(())
    })();
    if let Err(err)=built{
        let _=fs::remove_dir_all(&stage);
        return Err(err);
    }
    let destination=root.join(id);
    let previous=root.join(format!(".{id}.previous-{}",uuid::Uuid::new_v4()));
    let had_previous=destination.exists();
    if had_previous{
        fs::rename(&destination,&previous).map_err(|e|{
            let _=fs::remove_dir_all(&stage);
            format!("Could not safeguard existing compiled output: {e}")
        })?;
    }
    if let Err(err)=fs::rename(&stage,&destination) {
        if had_previous{let _=fs::rename(&previous,&destination);}
        let _=fs::remove_dir_all(&stage);
        return Err(format!("Could not publish validated source: {err}"));
    }
    if had_previous{let _=fs::remove_dir_all(previous);}
    Ok(destination)
}

#[cfg(test)]
mod tests{
    use super::*;
    use crate::model::{Cell,GridPoint,TileKind};
    #[test]
    fn compiles_only_the_selected_project_with_stable_source_symbols(){
        let dir=std::env::temp_dir().join(format!("mapstudio-publish-{}",uuid::Uuid::new_v4()));
        let workspace=Workspace{root:dir.clone()};
        let mut world=World::new();
        world.add_map("Town",GridPoint::new(0,0),8,8).unwrap();
        for cell in &mut world.maps[0].cells{
            *cell=Some(Cell{terrain:TileKind::Grass,metatile_id:0x170,collision:0,elevation:3});
        }
        let emerald=workspace.create("First GBA",GameTarget::EmeraldExpansion,&world).unwrap();
        let fire=workspace.create("Second GBA",GameTarget::FireRed,&world).unwrap();
        let dest=publish(&workspace,&emerald.project_id).unwrap();
        let names:Vec<_>=fs::read_dir(dest.join("data/layouts")).unwrap()
            .flatten().map(|e|e.file_name().to_string_lossy().to_string()).collect();
        assert_eq!(names.len(),1);
        assert!(names[0].contains(&emerald.project_id.replace('-',"")));
        assert_eq!(fs::read(dest.join("data/layouts").join(&names[0]).join("map.bin")).unwrap().len(),128);
        assert_eq!(fs::read(dest.join("data/layouts").join(&names[0]).join("border.bin")).unwrap().len(),8);
        assert!(publish(&workspace,&fire.project_id).unwrap_err().contains("not supported"));
        assert!(!workspace.compiled_root().join(&fire.project_id).exists());
        // Repeated publish replaces output only, preserving source project.
        assert_eq!(publish(&workspace,&emerald.project_id).unwrap(),dest);
        assert_eq!(workspace.load_world(&emerald.project_id).unwrap().maps.len(),1);
        let _=fs::remove_dir_all(dir);
    }
    #[test]
    fn incomplete_map_never_changes_previous_compilation(){
        let dir=std::env::temp_dir().join(format!("mapstudio-invalid-publish-{}",uuid::Uuid::new_v4()));
        let workspace=Workspace{root:dir.clone()};
        let mut world=World::new();
        world.add_map("Empty",crate::model::GridPoint::new(0,0),8,8).unwrap();
        let p=workspace.create("Unfinished",GameTarget::EmeraldExpansion,&world).unwrap();
        assert!(publish(&workspace,&p.project_id).is_err());
        assert!(!workspace.compiled_root().join(&p.project_id).exists());
        let _=fs::remove_dir_all(dir);
    }
}
