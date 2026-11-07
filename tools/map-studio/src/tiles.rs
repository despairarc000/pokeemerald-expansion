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
/// One genuine layout-defined tileset pairing. Metatile IDs are local to the
/// primary/secondary pair and must never be treated as a global art atlas.
pub struct TilePair {
    pub key:&'static str,
    pub label:&'static str,
    pub category:&'static str,
    pub primary:&'static str,
    pub secondary:&'static str,
    pub primary_count:usize,
    pub secondary_count:usize,
    pub atlas:&'static [u8],
    pub attr_primary:&'static [u8],
    pub attr_secondary:&'static [u8],
}
include!(concat!(env!("OUT_DIR"),"/tileset_catalog.rs"));
pub const DEFAULT_PAIR:&str="general/petalburg";
pub fn pairs()->&'static [TilePair]{TILESET_PAIRS}
pub fn pair(key:&str)->Option<&'static TilePair>{
    TILESET_PAIRS.iter().find(|p|p.key==key)
}
pub fn pair_for(primary:&str,secondary:&str)->Option<&'static TilePair>{
    TILESET_PAIRS.iter().find(|p|p.primary==primary&&p.secondary==secondary)
}
pub fn valid_in(pair:&TilePair,id:u16)->bool{
    if id<512{(id as usize)<pair.primary_count &&
        (id as usize+1)*2<=pair.attr_primary.len()}
    else {
        ((id-512) as usize)<pair.secondary_count &&
        ((id-512) as usize+1)*2<=pair.attr_secondary.len()
    }
}
pub fn tile_ids(pair:&TilePair)->Vec<u16>{
    (0..1024u16).filter(|&id|valid_in(pair,id)).collect()
}
pub fn behavior_name_in(pair:&TilePair,id:u16)->Option<&'static str>{
    if !valid_in(pair,id){return None;}
    let (attrs,i)=if id<512{(pair.attr_primary,id as usize)}
        else{(pair.attr_secondary,(id-512) as usize)};
    BEHAVIOR_NAMES.get(attrs[i*2] as usize).copied()
}
pub fn is_warp_in(pair:&TilePair,id:u16)->bool {
    let Some(name)=behavior_name_in(pair,id) else{return true;};
    ["WARP","DOOR","LADDER","ESCALATOR","HOLE","STAIR","TELEPORT"]
        .iter().any(|needle|name.contains(needle))
}
/// Filters are backed by engine behavior names, never unverified image labels.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum TileFilter{All,Water,Ice,Doors,Grass,Paths}
impl TileFilter{
    pub const ALL:[Self;6]=[Self::All,Self::Water,Self::Ice,Self::Doors,Self::Grass,Self::Paths];
    pub fn label(self)->&'static str{
        match self{Self::All=>"All tiles",Self::Water=>"Water",Self::Ice=>"Ice",
            Self::Doors=>"Doors / stairs",Self::Grass=>"Grass",Self::Paths=>"Paths / roads"}
    }
}
pub fn matches_filter(pair:&TilePair,id:u16,filter:TileFilter)->bool{
    if filter==TileFilter::All{return true;}
    let Some(name)=behavior_name_in(pair,id) else{return false;};
    match filter{
        TileFilter::All=>true,
        TileFilter::Water=>name.contains("WATER")||name.contains("PUDDLE"),
        TileFilter::Ice=>name.contains("ICE"),
        TileFilter::Doors=>is_warp_in(pair,id),
        TileFilter::Grass=>name.contains("GRASS")||name.contains("TALL_GRASS"),
        TileFilter::Paths=>name.contains("ROAD")||name.contains("SAND")||name.contains("ASH")
            ||name.contains("BRIDGE")
    }
}


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


#[cfg(test)]
mod all_tileset_catalog_tests {
    use super::*;
    #[test]
    fn catalog_is_real_and_contains_buildings_caves_and_towns(){
        assert!(pairs().len()>60,"Expected source-defined tileset pair coverage");
        for key in ["general/petalburg","general/cave","building/generic_building"] {
            let entry=pair(key).unwrap();
            assert!(entry.atlas.starts_with(&[137,80,78,71]));
            assert!(tile_ids(entry).len()>400);
        }
    }
    #[test]
    fn behavior_and_tile_filter_use_selected_pair(){
        for chosen in pairs(){
            let available=tile_ids(chosen);
            assert!(available.len()>100);
            for id in available.into_iter().take(15){
                assert!(behavior_name_in(chosen,id).is_some());
            }
        }
    }
}
