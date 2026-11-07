//! Safe, deterministic expansion source ZIP export.
//! The Python custom-map compiler consumes layout.mapstudio.json in each folder.
use crate::model::{MapFrame, World, EventKind};
use std::{collections::HashMap, fs, io::Write, path::Path};
use serde_json::{json, Value};

const PRIMARY: &str = "gTileset_General";
const SECONDARY: &str = "gTileset_Petalburg";
const GROUP: &str = "gMapGroup_Meadowmere";

pub fn map_name(map: &MapFrame) -> String {
    let base:String = map.name.chars().filter(|ch|ch.is_ascii_alphanumeric()).collect();
    format!("Meadowmere_{}", if base.is_empty() {"Map"} else {&base})
}

pub fn validate(world:&World) -> Result<(), String> {
    let mut names=std::collections::HashSet::new();
    for m in &world.maps {
        if !names.insert(map_name(m)) {return Err(format!("Duplicate Emerald map name: {}",map_name(m))); }
        if m.width == 0 || m.height == 0 ||
            (u64::from(m.width)+15)*(u64::from(m.height)+14)>10240 {
            return Err(format!("{} exceeds current Emerald map buffer capacity",m.name));
        }
        if m.cells.len() != (m.width as usize)*(m.height as usize) {
            return Err(format!("{} has invalid map dimensions",m.name));
        }
        if let Some(index)=m.cells.iter().position(|c|c.is_none()) {
            return Err(format!("{} has unpainted cell ({},{}); use Fill first",
                m.name,index%(m.width as usize),index/(m.width as usize)));
        }
        for c in m.cells.iter().flatten() {
            if c.metatile_id>=1024 || c.collision>3 || c.elevation>15 {
                return Err(format!("{} contains invalid native tile values",m.name));
            }
        }
        if !m.markers.is_empty(){
            return Err(format!("{} contains {} unscripted event marker(s); remove or implement scripts before exporting",
                m.name,m.markers.len()));
        }
        for e in &m.events {
            if e.x>=m.width || e.y>=m.height || e.elevation>15 {
                return Err(format!("{} has an out-of-bounds event",m.name));
            }
            match e.kind {
                EventKind::Npc | EventKind::Sign => {
                    let chars=e.dialogue.chars();
                    if e.dialogue.is_empty() || e.dialogue.len()>100
                       || chars.clone().any(|ch| !ch.is_ascii_graphic() && ch!=' ')
                       || chars.clone().any(|ch| ch=='"' || ch as u32==36 || ch as u32==92) {
                        return Err(format!("{} has invalid dialogue",m.name));
                    }
                    if e.kind==EventKind::Npc && !valid_symbol(&e.graphics_id) {
                        return Err(format!("{} has an invalid graphics ID",m.name));
                    }
                }
                EventKind::Warp => {
                    let Some(target)=world.maps.iter().find(|target|target.id==e.dest_map_id)
                        else {return Err(format!("{} warp destination missing",m.name));};
                    let count=target.events.iter().filter(|item|item.kind==EventKind::Warp).count();
                    if usize::from(e.dest_warp_id)>=count {
                        return Err(format!("{} references missing destination warp {}",m.name,e.dest_warp_id));
                    }
                }
                EventKind::Trigger => {
                    if !valid_symbol(&e.script) || !valid_symbol(&e.var) {
                        return Err(format!("{} trigger requires valid script and variable references",m.name));
                    }
                }
            }
        }
    }
    Ok(())
}

fn valid_symbol(s:&str)->bool {
    !s.is_empty() && s.bytes().all(|b|b.is_ascii_alphanumeric() || b==b'_')
}

pub fn layout_json(map:&MapFrame) -> Value {
    let cells:Vec<Vec<u16>> = map.cells.chunks(map.width as usize).map(|row|row.iter()
        .map(|c|{let v=c.expect("validated before export");v.pack(v.metatile_id)}).collect()).collect();
    json!({
        "format": "mapstudio-numeric-v1",
        "header": {
            "id": format!("LAYOUT_{}",map_name(map).to_uppercase()),
            "name": format!("{}_Layout",map_name(map)),
            "primary":PRIMARY,"secondary":SECONDARY,"group":GROUP
        },
        "border":[0x170,0x170,0x170,0x170],
        "cells": cells
    })
}

pub fn map_json(map:&MapFrame, world:&World) -> Value {
    let by_id:HashMap<u32,String>=world.maps.iter().map(|m|(m.id,map_name(m))).collect();
    let connections:Vec<Value>=world.connections().into_iter().filter(|c|c.from==map.id)
        .map(|c|{
            let direction=match c.direction {
                crate::model::Direction::Up=>"up",
                crate::model::Direction::Down=>"down",
                crate::model::Direction::Left=>"left",
                crate::model::Direction::Right=>"right",
            };
            json!({"map":format!("MAP_{}",by_id[&c.to].to_uppercase()),
                "offset":c.offset,"direction":direction})
        }).collect();
    let mut objects=Vec::new();
    let mut warps=Vec::new();
    let mut coords=Vec::new();
    let mut signs=Vec::new();
    for (index,e) in map.events.iter().enumerate() {
        let number=index+1;
        match e.kind {
            EventKind::Npc => objects.push(json!({
                "graphics_id":e.graphics_id,
                "x":e.x,"y":e.y,"elevation":e.elevation,
                "movement_type":"MOVEMENT_TYPE_FACE_DOWN",
                "movement_range_x":0,"movement_range_y":0,
                "trainer_type":"TRAINER_TYPE_NONE",
                "trainer_sight_or_berry_tree_id":"0",
                "script":format!("{}_EventScript_Npc{}",map_name(map),number),
                "flag":"0"
            })),
            EventKind::Sign => signs.push(json!({
                "type":"sign","x":e.x,"y":e.y,"elevation":e.elevation,
                "player_facing_dir":"BG_EVENT_PLAYER_FACING_ANY",
                "script":format!("{}_EventScript_Sign{}",map_name(map),number)
            })),
            EventKind::Warp => warps.push(json!({
                "x":e.x,"y":e.y,"elevation":e.elevation,
                "dest_map":format!("MAP_{}",by_id[&e.dest_map_id].to_uppercase()),
                "dest_warp_id":e.dest_warp_id.to_string()
            })),
            EventKind::Trigger => coords.push(json!({
                "type":"trigger","x":e.x,"y":e.y,"elevation":e.elevation,
                "var":e.var,"var_value":e.var_value.to_string(),"script":e.script
            })),
        }
    }
    json!({
        "id":format!("MAP_{}",map_name(map).to_uppercase()),
        "name":map_name(map),
        "layout":format!("LAYOUT_{}",map_name(map).to_uppercase()),
        "music":"MUS_LITTLEROOT",
        "region":"REGION_HOENN",
        "region_map_section":"MAPSEC_LITTLEROOT_TOWN",
        "requires_flash":false,
        "weather":"WEATHER_SUNNY",
        "map_type":"MAP_TYPE_TOWN",
        "allow_cycling":true,
        "allow_escaping":false,
        "allow_running":true,
        "show_map_name":true,
        "battle_scene":"MAP_BATTLE_SCENE_NORMAL",
        "connections":connections,
        "object_events":objects,"warp_events":warps,"coord_events":coords,"bg_events":signs
    })
}


fn script_text(map:&MapFrame) -> String {
    let name=map_name(map);
    let mut out=format!("{name}_MapScripts::\n\t.byte 0\n");
    for (index,e) in map.events.iter().enumerate() {
        let number=index+1;
        match e.kind {
            EventKind::Npc => out.push_str(&format!(
                "\n{name}_EventScript_Npc{number}::\n\tmsgbox {name}_Text_Npc{number}, MSGBOX_NPC\n\tend\n\n{name}_Text_Npc{number}:\n\t.string \"{}$\"\n",
                e.dialogue)),
            EventKind::Sign => out.push_str(&format!(
                "\n{name}_EventScript_Sign{number}::\n\tmsgbox {name}_Text_Sign{number}, MSGBOX_SIGN\n\tend\n\n{name}_Text_Sign{number}:\n\t.string \"{}$\"\n",
                e.dialogue)),
            _ => {}
        }
    }
    out
}

pub fn export_zip(world:&World, destination:&Path) -> Result<(),String> {
    validate(world)?;
    if let Some(parent)=destination.parent(){fs::create_dir_all(parent).map_err(|e|e.to_string())?;}
    let output=fs::File::create(destination).map_err(|e|format!("Create ZIP: {e}"))?;
    let mut zip=zip::ZipWriter::new(output);
    let options=zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for map in &world.maps {
        let name=map_name(map);
        let folder=format!("custom_maps/{name}/");
        for (filename,contents) in [
            ("layout.mapstudio.json",serde_json::to_vec_pretty(&layout_json(map)).map_err(|e|e.to_string())?),
            ("map.json",serde_json::to_vec_pretty(&map_json(map,world)).map_err(|e|e.to_string())?),
            ("scripts.inc",script_text(map).into_bytes())
        ] {
            zip.start_file(format!("{folder}{filename}"),options)
                .map_err(|e|format!("ZIP entry: {e}"))?;
            zip.write_all(&contents).map_err(|e|format!("ZIP data: {e}"))?;
        }
    }
    zip.start_file("mapstudio-project.json",options).map_err(|e|e.to_string())?;
    zip.write_all(&serde_json::to_vec_pretty(&crate::storage::Project::new(world.clone()))
        .map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    zip.finish().map_err(|e|format!("Finish ZIP: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{GridPoint,TileKind};
    #[test]
    fn rejects_unpainted_and_unscripted_events() {
        let mut w=World::sample();
        assert!(validate(&w).unwrap_err().contains("unpainted"));
        for map in &mut w.maps {
            for c in &mut map.cells { *c=Some(crate::model::Cell::new(TileKind::Grass)); }
        }
        assert!(validate(&w).is_ok());
        w.add_marker(GridPoint::new(0,0));
        assert!(validate(&w).unwrap_err().contains("unscripted"));
    }
    #[test]
    fn native_connections_export_to_real_json() {
        let mut w=World::new();
        w.add_map("A",GridPoint::new(0,0),10,10).unwrap();
        w.add_map("B",GridPoint::new(10,0),5,5).unwrap();
        w.add_map("C",GridPoint::new(10,5),5,5).unwrap();
        let j=map_json(&w.maps[0],&w);
        let v=j["connections"].as_array().unwrap();
        assert_eq!(v.len(),2);
        assert!(v.iter().any(|c|c["offset"]==5 && c["direction"]=="right"));
    }
    #[test]
    fn zip_contains_numeric_layouts_and_project_snapshot() {
        let mut w=World::new();
        w.add_map("Lake",GridPoint::new(0,0),3,2).unwrap();
        for y in 0..2 { for x in 0..3 {w.paint_metatile(GridPoint::new(x,y),0x170);}}
        let path=std::env::temp_dir().join(format!("mapstudio-{}-export.zip",std::process::id()));
        export_zip(&w,&path).unwrap();
        let f=fs::File::open(&path).unwrap();
        let mut archive=zip::ZipArchive::new(f).unwrap();
        assert!(archive.by_name("custom_maps/Meadowmere_Lake/layout.mapstudio.json").is_ok());
        assert!(archive.by_name("mapstudio-project.json").is_ok());
        let _=fs::remove_file(&path);
    }
}
