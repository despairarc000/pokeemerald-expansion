use std::{env,path::PathBuf,process::Command};
fn main() {
    let root=PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    let output=PathBuf::from(env::var("OUT_DIR").unwrap()).join("metatiles.png");
    let script=root.join("tools/map-studio/scripts/build_atlas.py");
    println!("cargo:rerun-if-changed={}",script.display());
    for p in [
        "data/tilesets/primary/general/metatiles.bin",
        "data/tilesets/primary/general/tiles.png",
        "data/tilesets/secondary/petalburg/metatiles.bin",
        "data/tilesets/secondary/petalburg/tiles.png",
    ] { println!("cargo:rerun-if-changed={}",root.join(p).display());}
    let header=root.join("include/constants/metatile_behaviors.h");
    println!("cargo:rerun-if-changed={}",header.display());
    let source=std::fs::read_to_string(&header)
        .expect("Emerald behavior definitions must exist");
    let mut names=Vec::new();
    let mut within=false;
    for line in source.lines() {
        let t=line.trim();
        if t=="enum {" {within=true;continue;}
        if !within {continue;}
        if t.starts_with("NUM_METATILE_BEHAVIORS"){break;}
        if let Some(symbol)=t.split(',').next().map(str::trim).filter(|p|p.starts_with("MB_")) {
            names.push(format!("{symbol:?}"));
        }
    }
    assert!(names.len()>150,"Unexpected Emerald behavior table");
    let generated=PathBuf::from(env::var("OUT_DIR").unwrap()).join("metatile_behavior_names.rs");
    std::fs::write(generated,format!("pub const BEHAVIOR_NAMES:&[&str]=&[{}];",names.join(",")))
        .expect("Generate metatile behavior name table");
    for p in [
        "data/tilesets/primary/general/metatile_attributes.bin",
        "data/tilesets/secondary/petalburg/metatile_attributes.bin",
        "data/tilesets/primary/general/metatiles.bin",
        "data/tilesets/secondary/petalburg/metatiles.bin",
    ]{println!("cargo:rerun-if-changed={}",root.join(p).display());}
    let status=Command::new("python3").arg(&script).arg(&root).arg(&output)
        .status().expect("Python3 is required to generate real Emerald tileset atlas");
    assert!(status.success(),"Failed to generate actual Emerald metatile atlas");
    let catalog=root.join("tools/map-studio/scripts/build_catalog.py");
    let layouts=root.join("data/layouts/layouts.json");
    println!("cargo:rerun-if-changed={}",catalog.display());
    println!("cargo:rerun-if-changed={}",layouts.display());
    // All source tilesets affect the generated real sprite catalog.
    for group in ["primary","secondary"] {
        let parent=root.join("data/tilesets").join(group);
        for item in std::fs::read_dir(parent).expect("tileset directories") {
            let dir=item.expect("tileset entry").path();
            if !dir.is_dir(){continue;}
            for file in ["tiles.png","metatiles.bin","metatile_attributes.bin"]{
                println!("cargo:rerun-if-changed={}",dir.join(file).display());
            }
            println!("cargo:rerun-if-changed={}",dir.join("palettes").display());
        }
    }
    let atlas_folder=PathBuf::from(env::var("OUT_DIR").unwrap()).join("atlases");
    let result=Command::new("python3").arg(catalog).arg(&root).arg(&atlas_folder)
        .status().expect("Python atlas catalog builder unavailable");
    assert!(result.success(),"Cannot build real tileset atlas catalog");
    let sprite_script=root.join("tools/map-studio/scripts/build_sprites.py");
    println!("cargo:rerun-if-changed={}",sprite_script.display());
    for source in [
        "src/data/object_events/object_event_graphics_info.h",
        "src/data/object_events/object_event_graphics_info_pointers.h",
        "src/data/object_events/object_event_pic_tables.h",
        "src/data/object_events/object_event_graphics.h",
        "tools/map-studio/src/choices.rs",
    ] {
        println!("cargo:rerun-if-changed={}",root.join(source).display());
    }
    let sprite_output=PathBuf::from(env::var("OUT_DIR").unwrap()).join("sprites");
    let process=Command::new("python3").arg(sprite_script).arg(&root).arg(&sprite_output)
        .status().expect("Python sprite atlas builder unavailable");
    assert!(process.success(),"Failed to load source-verified NPC art");


    let item_script=root.join("tools/map-studio/scripts/build_items.py");
    println!("cargo:rerun-if-changed={}",item_script.display());
    for folder in ["graphics/items/icons","graphics/items/icon_palettes"] {
        println!("cargo:rerun-if-changed={}",root.join(folder).display());
    }
    let icon_out=PathBuf::from(env::var("OUT_DIR").unwrap()).join("items");
    let status=Command::new("python3").arg(item_script).arg(&root).arg(&icon_out)
        .status().expect("Cannot generate native item icons");
    assert!(status.success(),"Failed to decode source item icons");

}
