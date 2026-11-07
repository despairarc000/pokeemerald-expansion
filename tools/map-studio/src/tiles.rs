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
    pub primary_symbol:&'static str,
    pub secondary_symbol:&'static str,
    pub primary:&'static str,
    pub secondary:&'static str,
    pub primary_count:usize,
    pub secondary_count:usize,
    pub valid_mask:&'static [u8;128],
    pub atlas:&'static [u8],
    pub labels:&'static [(u16,&'static str)],
    pub attr_primary:&'static [u8],
    pub attr_secondary:&'static [u8],
}
include!(concat!(env!("OUT_DIR"),"/tileset_catalog.rs"));
pub const DEFAULT_PAIR:&str="general/petalburg";
pub const SOURCE_CATEGORIES:[&str;5]=[
    "Towns","Houses","Interiors","Caves","Outdoor & Special"
];
pub fn pairs()->&'static [TilePair]{TILESET_PAIRS}
/// Deterministic source-catalog order, so a category always has an
/// immediate first option and never creates a fabricated tileset.
pub fn first_in_category(category:&str)->Option<&'static TilePair>{
    pairs().iter().find(|pair|pair.category==category)
}
pub fn pair(key:&str)->Option<&'static TilePair>{
    TILESET_PAIRS.iter().find(|p|p.key==key)
}
pub fn pair_for(primary:&str,secondary:&str)->Option<&'static TilePair>{
    TILESET_PAIRS.iter().find(|p|p.primary==primary&&p.secondary==secondary)
}
pub fn valid_in(pair:&TilePair,id:u16)->bool{
    let num=id as usize;
    if num>=1024 || pair.valid_mask[num/8]&(1<<(num%8))==0 {return false;}
    if id<512{
        num<pair.primary_count && (num+1)*2<=pair.attr_primary.len()
    } else {
        let local=num-512;
        local<pair.secondary_count && (local+1)*2<=pair.attr_secondary.len()
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
/// Metatile names from the matching primary/secondary source definitions.
pub fn source_label_in(pair:&TilePair,id:u16)->Option<&'static str>{
    pair.labels.iter().find(|(candidate,_)|*candidate==id)
        .map(|(_,name)|*name)
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct SourceMetatileAttributes {
    pub metatile_id:u16,
    pub behavior:&'static str,
    pub layer_type:&'static str,
    pub raw:u16,
}
/// Only fields actually defined for Emerald (behavior and layer type) are
/// exposed; collision/elevation are independent per-cell block properties.
pub fn attributes_in(pair:&TilePair,id:u16)->Option<SourceMetatileAttributes>{
    if !valid_in(pair,id){return None;}
    let (attrs,index)=if id<512{(pair.attr_primary,id as usize)}
        else{(pair.attr_secondary,(id-512) as usize)};
    let raw=u16::from_le_bytes([attrs[index*2],attrs[index*2+1]]);
    let layer_type=match (raw>>12)&15 {
        0=>"Normal — middle/top",1=>"Covered — bottom/middle",
        2=>"Split — bottom/top",_=>"Unknown source layer"
    };
    Some(SourceMetatileAttributes{
        metatile_id:id,
        behavior:behavior_name_in(pair,id).unwrap_or("Unknown behavior"),
        layer_type,raw,
    })
}
pub fn is_warp_in(pair:&TilePair,id:u16)->bool {
    let Some(name)=behavior_name_in(pair,id) else{return true;};
    ["WARP","DOOR","LADDER","ESCALATOR","HOLE","STAIR","TELEPORT"]
        .iter().any(|needle|name.contains(needle))
}
/// Filters are backed by engine behavior names, never unverified image labels.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum TileFilter{All,Water,Ice,Doors,Grass,Paths,Buildings,Trees,Cliffs}
impl TileFilter{
    pub const ALL:[Self;9]=[Self::All,Self::Grass,Self::Water,Self::Paths,
        Self::Buildings,Self::Trees,Self::Cliffs,Self::Doors,Self::Ice];
    pub fn label(self)->&'static str{
        match self{
            Self::All=>"All art",Self::Water=>"Water",Self::Ice=>"Ice",
            Self::Doors=>"Doors / stairs",Self::Grass=>"Grass",
            Self::Paths=>"Paths / roads",Self::Buildings=>"Buildings*",
            Self::Trees=>"Trees / plants*",Self::Cliffs=>"Rocks / cliffs*"
        }
    }
}
/// Filters highlight candidate cells in place, never remove or shift them.
/// Asterisked object categories use *named* source metatiles, not invented
/// computer-vision guesses about unlabelled roof/tree/rock artwork.
pub fn matches_filter(pair:&TilePair,id:u16,filter:TileFilter)->bool{
    if filter==TileFilter::All{return true;}
    let behavior=behavior_name_in(pair,id).unwrap_or("");
    let label=source_label_in(pair,id).unwrap_or("").to_ascii_uppercase();
    let named=|words:&[&str]|words.iter().any(|word|label.contains(word));
    match filter{
        TileFilter::All=>true,
        TileFilter::Water=>behavior.contains("WATER")||behavior.contains("PUDDLE")
            ||named(&["WATER","POND","OCEAN","SEA","SHORE"]),
        TileFilter::Ice=>behavior.contains("ICE")||named(&["ICE","SNOW"]),
        TileFilter::Doors=>is_warp_in(pair,id)||named(&["DOOR","STAIR","LADDER"]),
        TileFilter::Grass=>behavior.contains("GRASS")||named(&["GRASS"]),
        TileFilter::Paths=>behavior.contains("ROAD")||behavior.contains("SAND")
            ||behavior.contains("ASH")||behavior.contains("BRIDGE")
            ||named(&["PATH","ROAD","BRIDGE","SAND","STAIR"]),
        TileFilter::Buildings=>named(&["ROOF","HOUSE","BUILDING","WALL",
            "WINDOW","POKEMART","POKECENTER","GYM","SHOP"]),
        TileFilter::Trees=>named(&["TREE","BUSH","FLOWER","SHRUB","LEAF","VINE"]),
        TileFilter::Cliffs=>named(&["ROCK","CLIFF","CAVE","BOULDER","MOUNTAIN"]),
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
    behavior_overlay_in(pair(DEFAULT_PAIR)?,id)
}
pub fn behavior_overlay_in(pair:&TilePair,id:u16)->Option<&'static str> {
    match behavior_name_in(pair,id)? {
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


/// Keep metatiles in native ID order, eight columns per source sheet. A
/// rectangle represents a real multi-metatile paint stamp, not a new tileset.
pub const SHEET_COLUMNS:usize=8;
pub const MAX_STAMP_SIDE:usize=12;
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct MetatileStamp {
    pub pair_key:String,
    pub base:u16,
    pub origin:u16,
    pub width:usize,
    pub height:usize,
    pub cells:Vec<Option<u16>>,
}
impl MetatileStamp {
    pub fn is_single(&self)->bool {self.width==1 && self.height==1}
}

/// Source sheets keep vacant metatile slots in place. Selecting across a gap
/// skips that cell when painting rather than closing the gap or inventing art.
/// Reject cross-primary/secondary selections and unusually large gestures.
pub fn stamp_from_sheet(pair:&TilePair,base:u16,from:u16,to:u16)->Option<MetatileStamp>{
    if base!=0 && base!=512 {return None;}
    let last=base+512;
    if from<base || to<base || from>=last || to>=last {return None;}
    let a=(from-base) as usize;
    let b=(to-base) as usize;
    let (left,right)=((a%SHEET_COLUMNS).min(b%SHEET_COLUMNS),
                      (a%SHEET_COLUMNS).max(b%SHEET_COLUMNS));
    let (top,bottom)=((a/SHEET_COLUMNS).min(b/SHEET_COLUMNS),
                      (a/SHEET_COLUMNS).max(b/SHEET_COLUMNS));
    let width=right-left+1;
    let height=bottom-top+1;
    if width>MAX_STAMP_SIDE || height>MAX_STAMP_SIDE {return None;}
    let mut cells=Vec::with_capacity(width*height);
    for y in top..=bottom {
        for x in left..=right {
            let tile=base+(y*SHEET_COLUMNS+x) as u16;
            cells.push(valid_in(pair,tile).then_some(tile));
        }
    }
    if cells.iter().all(Option::is_none){return None;}
    Some(MetatileStamp{pair_key:pair.key.to_string(),base,
        origin:base+(top*SHEET_COLUMNS+left) as u16,width,height,cells})
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
            assert!(available.len()>=40,"Too few real tiles in {}",chosen.key);
            for id in available.into_iter().take(15){
                assert!(behavior_name_in(chosen,id).is_some());
            }
        }
    }
}

#[cfg(test)]
mod sheet_stamp_tests {
    use super::*;
    #[test]
    fn source_grid_selection_keeps_native_adjacency() {
        let p=pair(DEFAULT_PAIR).unwrap();
        let stamp=stamp_from_sheet(p,0,16,25).unwrap();
        assert_eq!((stamp.width,stamp.height),(2,2));
        assert_eq!(stamp.cells.len(),4);
        assert_eq!(stamp.cells[0],Some(16));
        assert_eq!(stamp.cells[1],Some(17));
        assert_eq!(stamp.cells[2],Some(24));
        assert_eq!(stamp.cells[3],Some(25));
    }
    #[test]
    fn selection_never_spills_between_source_sheets() {
        let p=pair(DEFAULT_PAIR).unwrap();
        assert!(stamp_from_sheet(p,0,511,512).is_none());
        assert!(stamp_from_sheet(p,512,512,512).is_some());
        assert!(stamp_from_sheet(p,0,0,511).is_none());
    }
}

#[cfg(test)]
mod source_filter_tests {
    use super::*;
    #[test]
    fn native_names_are_scoped_to_selected_pair() {
        let p=pair(DEFAULT_PAIR).unwrap();
        assert!(source_label_in(p,1).is_some());
        assert_eq!(source_label_in(p,1),Some("METATILE_General_Grass"));
        assert!(matches_filter(p,1,TileFilter::Grass));
        assert!(matches_filter(p,1,TileFilter::All));
        assert!(!matches_filter(p,1,TileFilter::Buildings));
    }
}

#[cfg(test)]
mod source_properties_tests{
    use super::*;
    #[test]
    fn source_metadata_uses_actual_pair_and_binary(){
        let pair=pair(DEFAULT_PAIR).unwrap();
        let prop=attributes_in(pair,1).unwrap();
        assert_eq!(prop.metatile_id,1);
        assert_eq!(prop.behavior,"MB_NORMAL");
        assert!(prop.layer_type.starts_with("Normal")||
            prop.layer_type.starts_with("Covered")||
            prop.layer_type.starts_with("Split"));
        assert!(attributes_in(pair,1024).is_none());
    }
}

#[cfg(test)]
mod artwork_category_tests{
    use super::*;
    #[test]
    fn every_category_has_real_art_and_its_first_pair_belongs_to_it(){
        for category in SOURCE_CATEGORIES {
            let first=first_in_category(category)
                .unwrap_or_else(||panic!("No source tilesets for {category}"));
            assert_eq!(first.category,category);
            assert!(pair(first.key).is_some());
            assert!(pairs().iter().any(|p|p.key==first.key));
        }
    }
    #[test]
    fn unknown_category_cannot_create_a_fake_tileset(){
        assert!(first_in_category("Not a real tileset family").is_none());
    }
}
