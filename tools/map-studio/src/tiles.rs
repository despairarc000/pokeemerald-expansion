//! Metatile metadata from the same checked-in Emerald tileset binaries as the ROM.
//! Behavior is *not* an arbitrary editable per-cell property.
const PRIMARY_ATTR:&[u8]=include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),
    "/../../data/tilesets/primary/general/metatile_attributes.bin"));
const SECONDARY_ATTR:&[u8]=include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),
    "/../../data/tilesets/secondary/petalburg/metatile_attributes.bin"));
const PRIMARY_TILES:&[u8]=include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),
    "/../../data/tilesets/primary/general/metatiles.bin"));
const SECONDARY_TILES:&[u8]=include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),
    "/../../data/tilesets/secondary/petalburg/metatiles.bin"));
include!(concat!(env!("OUT_DIR"),"/metatile_behavior_names.rs"));

pub fn valid_metatile(id:u16)->bool {
    let (i,attributes,graphics)=if id<512 {
        (id as usize,PRIMARY_ATTR,PRIMARY_TILES)
    } else {
        ((id-512) as usize,SECONDARY_ATTR,SECONDARY_TILES)
    };
    (i+1)*2<=attributes.len() && (i+1)*16<=graphics.len()
}
pub fn all_metatiles()->Vec<u16> {
    (0..1024u16).filter(|&id|valid_metatile(id)).collect()
}
pub fn behavior_name(id:u16)->Option<&'static str>{
    let (i,attributes)=if id<512 {(id as usize,PRIMARY_ATTR)} else {
        ((id-512) as usize,SECONDARY_ATTR)
    };
    if !valid_metatile(id) {return None;}
    let offset=i*2;
    BEHAVIOR_NAMES.get(attributes[offset] as usize).copied()
}
/// Verified from metatile attribute behavior values, never guessed from graphics.
pub fn behavior_overlay(id:u16)->Option<&'static str> {
    match behavior_name(id)? {
        "MB_ICE"|"MB_THIN_ICE"|"MB_CRACKED_ICE"=>Some("❄"),
        "MB_POND_WATER"|"MB_INTERIOR_DEEP_WATER"|"MB_DEEP_WATER"
          |"MB_WATERFALL"|"MB_SOOTOPOLIS_DEEP_WATER"|"MB_OCEAN_WATER"
          |"MB_SHALLOW_WATER"|"MB_FAST_WATER"|"MB_CYCLING_ROAD_WATER"
          |"MB_PUDDLE"|"MB_HOT_SPRINGS" => Some("~"),
        "MB_JUMP_NORTH"=>Some("↑"),
        "MB_JUMP_SOUTH"=>Some("↓"),
        "MB_JUMP_EAST"=>Some("→"),
        "MB_JUMP_WEST"=>Some("←"),
        _=>None
    }
}
/// Disallow arrival-only markers on automatic transition tiles to avoid
/// unintentional self-warping in the unmodified GBA engine.
pub fn is_warp_behavior(id:u16)->bool{
    let Some(name)=behavior_name(id) else {return true;};
    ["WARP","DOOR","LADDER","ESCALATOR","HOLE","STAIR","TELEPORT"]
        .iter().any(|part|name.contains(part))
}

pub fn friendly_behavior(id:u16)->Option<&'static str> {
    match behavior_overlay(id) {
        Some("❄")=>Some("Ice"),
        Some("~")=>Some("Water"),
        Some("↑"|"↓"|"←"|"→")=>Some("Ledge"),
        _=>None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ids_are_backed_by_real_binaries() {
        assert!(valid_metatile(0));
        assert!(!valid_metatile(1024));
        assert!(all_metatiles().len()>512);
        for id in all_metatiles(){
            assert!(behavior_name(id).is_some(),"Missing real behavior for metatile {id}");
        }
    }
    #[test]
    fn behavior_categories_use_actual_emerald_constants() {
        let required=["MB_POND_WATER","MB_ICE","MB_JUMP_SOUTH"];
        for name in required {assert!(BEHAVIOR_NAMES.contains(&name));}
    }
}
