//! Safe, deterministic expansion source ZIP export.
//! The Python custom-map compiler consumes layout.mapstudio.json in each folder.
use crate::model::{MapFrame, World, EventKind, WarpMode};
use std::{collections::HashMap, fs, io::Write, path::Path};
use serde_json::{json, Value};

const GROUP: &str = "gMapGroup_Meadowmere";

pub fn map_name(map: &MapFrame) -> String {
    let base:String = map.name.chars().filter(|ch|ch.is_ascii_alphanumeric()).collect();
    format!("Meadowmere_{}", if base.is_empty() {"Map"} else {&base})
}

pub fn validate(world:&World) -> Result<(), String> {
    if world.maps.is_empty(){return Err("Add a map before exporting".into());}
    let mut names=std::collections::HashSet::new();
    let mut pickup_slots=std::collections::HashSet::new();
    for m in &world.maps {
        // Game map constants are uppercased; "Town" and "town" would
        // otherwise generate the same MAP_* symbol in the GBA source.
        let symbol=map_name(m).to_ascii_uppercase();
        if !names.insert(symbol.clone()) {
            return Err(format!("Duplicate native game map symbol: MAP_{symbol}"));
        }
        let tileset=crate::tiles::pair_for(&m.primary_tileset,&m.secondary_tileset)
            .ok_or_else(||format!("{} uses an unsupported source tileset",m.name))?;
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
            if !crate::tiles::valid_in(tileset,c.metatile_id) || c.collision>3 || c.elevation>15 {
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
                    if e.kind==EventKind::Npc {
                        if !crate::choices::is_verified_sprite_id(&e.graphics_id) {
                            return Err(format!("{} has an invalid graphics ID",m.name));
                        }
                        if !crate::choices::valid_npc_movement(&e.movement_type)
                            || e.movement_range_x>15 || e.movement_range_y>15 {
                            return Err(format!("{} has unsupported NPC movement settings",m.name));
                        }
                    }
                }
                EventKind::Warp => {
                    let target=world.maps.iter().find(|m|m.id==e.dest_map_id)
                        .ok_or_else(||format!("{} warp has no destination",m.name))?;
                    if e.arrival_only {
                        let p=crate::model::GridPoint::new(
                            i32::try_from(i64::from(m.origin.x)+i64::from(e.x)).map_err(|_|"Overflow")?,
                            i32::try_from(i64::from(m.origin.y)+i64::from(e.y)).map_err(|_|"Overflow")?,
                        );
                        if !Some(m).and_then(|frame|{
                            let tiles=crate::tiles::pair_for(&frame.primary_tileset,&frame.secondary_tileset)?;
                            let cell=frame.cell(e.x,e.y)?;
                            Some(cell.collision==0 && crate::tiles::valid_in(tiles,cell.metatile_id) &&
                                !crate::tiles::is_warp_in(tiles,cell.metatile_id))
                        }).unwrap_or(false) {
                            return Err(format!("{} arrival marker needs safe, non-warp terrain",m.name));
                        }
                    }
                    if e.warp_mode==WarpMode::Linked {
                        let native_count=target.events.iter().filter(|other|
                            other.kind==EventKind::Warp && other.warp_mode!=WarpMode::Direct).count();
                        if usize::from(e.dest_warp_id)>=native_count{
                            return Err(format!("{} references missing destination warp",m.name));
                        }
                    }else{
                        let cell=target.cell(e.dest_x,e.dest_y).ok_or_else(||
                            format!("{} direct arrival needs a painted destination tile",m.name))?;
                        let target_tiles=crate::tiles::pair_for(
                            &target.primary_tileset,&target.secondary_tileset)
                            .ok_or("Warp destination uses unsupported tileset")?;
                        if cell.collision!=0 || crate::tiles::is_warp_in(target_tiles,cell.metatile_id)
                            || !crate::tiles::valid_in(target_tiles,cell.metatile_id) {
                            return Err(format!("{} direct arrival needs walkable, non-warp terrain",m.name));
                        }
                    }
                }
                EventKind::Trigger => {
                    if !safe_dialogue(&e.dialogue){
                        return Err(format!("{} step trigger needs a valid message",m.name));
                    }
                }
                EventKind::HiddenItem => {
                    if !crate::choices::valid_hidden_item(&e.hidden_item) {
                        return Err(format!("{} hidden item needs a chosen item",m.name));
                    }
                    if !crate::choices::valid_pickup_slot(&e.hidden_flag)
                        || !pickup_slots.insert(e.hidden_flag.as_str()){
                        return Err(format!("{} hidden item requires a unique pickup slot",m.name));
                    }
                }
            }
        }
    }
    Ok(())
}

fn safe_dialogue(s:&str)->bool {
    !s.is_empty() && s.len()<=100 && s.chars().all(|c|
        (c.is_ascii_graphic() || c==' ') && c!='"' && c!='$' && (c as u32)!=92)
}

pub fn layout_json(map:&MapFrame) -> Value {
    let cells:Vec<Vec<u16>> = map.cells.chunks(map.width as usize).map(|row|row.iter()
        .map(|c|{let v=c.expect("validated before export");v.pack(v.metatile_id)}).collect()).collect();
    let tileset=crate::tiles::pair_for(&map.primary_tileset,&map.secondary_tileset)
        .expect("validated tileset pair");
    let w=map.width as usize;
    let h=map.height as usize;
    // Emerald border entries are *metatile IDs*, not packed map cells.
    let border=[
        map.cell(0,0).unwrap().metatile_id,
        map.cell((w-1) as u32,0).unwrap().metatile_id,
        map.cell(0,(h-1) as u32).unwrap().metatile_id,
        map.cell((w-1) as u32,(h-1) as u32).unwrap().metatile_id,
    ];
    json!({
        "format": "mapstudio-numeric-v1",
        "header": {
            "id": format!("LAYOUT_{}",map_name(map).to_uppercase()),
            "name": format!("{}_Layout",map_name(map)),
            "primary":tileset.primary_symbol,
            "secondary":tileset.secondary_symbol,
            "group":GROUP
        },
        "border":border,
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
                "movement_type":e.movement_type,
                "movement_range_x":e.movement_range_x,
                "movement_range_y":e.movement_range_y,
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
            EventKind::Warp => {
                if e.warp_mode==WarpMode::Direct {
                    coords.push(json!({
                        "type":"trigger","x":e.x,"y":e.y,"elevation":e.elevation,
                        "var":"TRIGGER_RUN_IMMEDIATELY","var_value":"0",
                        "script":format!("{}_EventScript_DirectWarp{}",map_name(map),number)
                    }));
                }else{
                    warps.push(json!({
                        "x":e.x,"y":e.y,"elevation":e.elevation,
                        "dest_map":format!("MAP_{}",by_id[&e.dest_map_id].to_uppercase()),
                        "dest_warp_id":e.dest_warp_id.to_string()
                    }));
                }
            }
            EventKind::Trigger => coords.push(json!({
                "type":"trigger","x":e.x,"y":e.y,"elevation":e.elevation,
                "var":"TRIGGER_RUN_IMMEDIATELY","var_value":"0",
                "script":format!("{}_EventScript_Trigger{}",map_name(map),number)
            })),
            EventKind::HiddenItem => signs.push(json!({
                "type":"hidden_item","x":e.x,"y":e.y,"elevation":e.elevation,
                "item":e.hidden_item,"flag":e.hidden_flag
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


pub(crate) fn script_text(map:&MapFrame,world:&World) -> String {
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
            EventKind::Warp if e.warp_mode==WarpMode::Direct => {
                if let Some(target)=world.maps.iter().find(|m|m.id==e.dest_map_id){
                    out.push_str(&format!(
                        "\n{name}_EventScript_DirectWarp{number}::\n\twarp MAP_{}, {}, {}\n\tend\n",
                        map_name(target).to_uppercase(),e.dest_x,e.dest_y));
                }
            }
            EventKind::Trigger => out.push_str(&format!(
                "\n{name}_EventScript_Trigger{number}::\n\tmsgbox {name}_Text_Trigger{number}, MSGBOX_DEFAULT\n\tend\n\n{name}_Text_Trigger{number}:\n\t.string \"{}$\"\n",
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
            ("scripts.inc",script_text(map,world).into_bytes())
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
    fn rejects_case_insensitive_native_map_symbol_collisions(){
        let mut world=World::new();
        world.add_map("Town",GridPoint::new(0,0),8,8).unwrap();
        world.add_map("town",GridPoint::new(20,0),8,8).unwrap();
        // A case-sensitive folder name is not a distinct game map constant.
        let err=validate(&world).unwrap_err();
        assert!(err.contains("Duplicate native game map symbol"),"{err}");
    }
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

#[cfg(test)]
mod map_event_export_tests {
    use super::*;
    use crate::model::{EventKind,GridPoint,TileKind,Cell};
    fn ready_world()->World {
        let mut world=World::new();
        world.add_map("Village",GridPoint::new(0,0),10,10).unwrap();
        world.add_map("House",GridPoint::new(10,0),10,10).unwrap();
        for map in &mut world.maps {
            map.cells.fill(Some(Cell::new(TileKind::Grass)));
        }
        world
    }
    #[test]
    fn dialogues_compile_to_object_and_sign_events() {
        let mut world=ready_world();
        let (npc_map,npc)=world.add_event(GridPoint::new(1,1),EventKind::Npc).unwrap();
        let (sign_map,sign)=world.add_event(GridPoint::new(2,2),EventKind::Sign).unwrap();
        world.maps.iter_mut().find(|m|m.id==npc_map).unwrap().events[npc].dialogue=
            "Hello there!".into();
        world.maps.iter_mut().find(|m|m.id==sign_map).unwrap().events[sign].dialogue=
            "Town Hall".into();
        validate(&world).unwrap();
        let m=&world.maps[0];
        let json=map_json(m,&world);
        assert_eq!(json["object_events"].as_array().unwrap().len(),1);
        assert_eq!(json["bg_events"].as_array().unwrap().len(),1);
        let scripts=script_text(m,&world);
        assert!(scripts.contains("MSGBOX_NPC"));
        assert!(scripts.contains("MSGBOX_SIGN"));
    }
    #[test]
    fn only_existing_destination_warps_are_exportable() {
        let mut world=ready_world();
        let from=world.add_event(GridPoint::new(1,1),EventKind::Warp).unwrap();
        assert!(validate(&world).unwrap_err().contains("no destination"));
        let to=world.add_event(GridPoint::new(11,1),EventKind::Warp).unwrap();
        world.link_warps(from,to,true).unwrap();
        validate(&world).unwrap();
        assert_eq!(map_json(&world.maps[0],&world)["warp_events"][0]["dest_map"],"MAP_MEADOWMERE_HOUSE");
    }
}

#[cfg(test)]
mod empty_project_tests {
    use super::*;
    #[test]
    fn empty_editor_project_is_backup_eligible_but_not_map_export_eligible() {
        assert!(validate(&World::new()).unwrap_err().contains("Add a map"));
    }
}

#[cfg(test)]
mod travel_exports {
    use super::*;
    use crate::model::{GridPoint,TileKind};
    fn prepared()->World{
        let mut w=World::new();
        w.add_map("Outside",GridPoint::new(0,0),5,5).unwrap();
        w.add_map("Home",GridPoint::new(10,0),5,5).unwrap();
        for m in &mut w.maps{m.cells.fill(Some(crate::model::Cell::new(TileKind::Grass)));}
        w
    }
    #[test]
    fn linked_and_arrival_only_destinations_are_native_warps() {
        let mut w=prepared();
        let from=w.add_event(GridPoint::new(1,1),EventKind::Warp).unwrap();
        let target=w.create_arrival_marker(from,GridPoint::new(11,1),false).unwrap();
        validate(&w).unwrap();
        assert!(w.event(target.0,target.1).unwrap().arrival_only);
        assert_eq!(map_json(&w.maps[0],&w)["warp_events"][0]["dest_map"],"MAP_MEADOWMERE_HOME");
        w.link_warps(from,target,true).unwrap();
        validate(&w).unwrap();
    }
    #[test]
    fn script_direct_arrival_does_not_require_a_destination_warp() {
        let mut w=prepared();
        let from=w.add_event(GridPoint::new(1,1),EventKind::Warp).unwrap();
        w.set_direct_arrival(from,GridPoint::new(12,2)).unwrap();
        validate(&w).unwrap();
        let j=map_json(&w.maps[0],&w);
        assert_eq!(j["coord_events"][0]["var"],"TRIGGER_RUN_IMMEDIATELY");
        assert!(j["warp_events"].as_array().unwrap().is_empty());
        assert!(script_text(&w.maps[0],&w).contains("warp MAP_MEADOWMERE_HOME, 2, 2"));
    }
    #[test]
    fn hidden_item_and_step_message_are_real_events() {
        let mut w=prepared();
        let item=w.add_event(GridPoint::new(1,1),EventKind::HiddenItem).unwrap();
        w.assign_hidden_slot(item.0,item.1).unwrap();
        w.event_mut(item.0,item.1).unwrap().hidden_item="ITEM_POTION".into();
        let step=w.add_event(GridPoint::new(2,2),EventKind::Trigger).unwrap();
        w.event_mut(step.0,step.1).unwrap().dialogue="Watch your step".into();
        validate(&w).unwrap();
        let j=map_json(&w.maps[0],&w);
        assert_eq!(j["bg_events"][0]["type"],"hidden_item");
        assert_eq!(j["coord_events"][0]["type"],"trigger");
    }
}

#[cfg(test)]
mod genuine_border_tests {
    use super::*;
    use crate::model::{Cell,GridPoint,TileKind};
    #[test]
    fn border_entries_use_raw_metatile_ids_not_packed_elevations(){
        let mut world=World::new();
        let id=world.add_map("Test",GridPoint::new(0,0),4,4).unwrap();
        let m=world.maps.iter_mut().find(|m|m.id==id).unwrap();
        m.cells.fill(Some(Cell{terrain:TileKind::Grass,metatile_id:0x170,collision:0,elevation:3}));
        let json=layout_json(m);
        assert_eq!(json["border"],serde_json::json!([0x170,0x170,0x170,0x170]));
        assert_eq!(json["cells"][0][0],serde_json::json!(0x3170));
    }
}

#[cfg(test)]
mod movement_export_tests{
    use super::*;
    use crate::model::{GridPoint,EventKind};
    #[test]
    fn source_movement_ranges_are_exported_not_hardcoded(){
        let mut world=World::new();
        let id=world.add_map("Walk",GridPoint::new(0,0),7,7).unwrap();
        let idx=world.add_event(GridPoint::new(2,3),EventKind::Npc).unwrap();
        let e=world.event_mut(idx.0,idx.1).unwrap();
        e.movement_type="MOVEMENT_TYPE_WANDER_AROUND".into();
        e.movement_range_x=2;
        e.movement_range_y=3;
        e.dialogue="Hello".into();
        let map=world.maps.iter().find(|m|m.id==id).unwrap();
        let out=map_json(map,&world);
        assert_eq!(out["object_events"][0]["movement_type"],
            "MOVEMENT_TYPE_WANDER_AROUND");
        assert_eq!(out["object_events"][0]["movement_range_x"],2);
        assert_eq!(out["object_events"][0]["movement_range_y"],3);
    }
}
