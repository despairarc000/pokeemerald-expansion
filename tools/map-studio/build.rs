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
}
