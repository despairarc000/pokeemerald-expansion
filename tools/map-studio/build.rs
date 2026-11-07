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
    let status=Command::new("python3").arg(&script).arg(&root).arg(&output)
        .status().expect("Python3 is required to generate real Emerald tileset atlas");
    assert!(status.success(),"Failed to generate actual Emerald metatile atlas");
}
