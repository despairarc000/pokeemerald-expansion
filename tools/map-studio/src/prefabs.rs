//! Source-backed prefab library. Default Emerald definitions from Porymap.
//! Prefabs are NOT new tilesets. IDs are validated against the active pair.
use crate::{model::{GridPoint,World},tiles::{self,TilePair}};
use serde::{Deserialize,Serialize};
use std::sync::OnceLock;

#[derive(Clone,Debug,PartialEq,Eq,Deserialize,Serialize)]
pub struct PrefabTile {
    pub x:u32,
    pub y:u32,
    pub metatile_id:u16,
    pub collision:u8,
    pub elevation:u8,
}

#[derive(Clone,Debug,PartialEq,Eq,Deserialize,Serialize)]
pub struct Prefab {
    pub name:String,
    pub width:u32,
    pub height:u32,
    #[serde(default)]
    pub primary_tileset:String,
    #[serde(default)]
    pub secondary_tileset:String,
    pub metatiles:Vec<PrefabTile>,
}

/// Original vanilla Emerald prefabs from huderlem/porymap, LGPL-3.0.
pub fn emerald()->&'static [Prefab]{
    static PREFABS:OnceLock<Vec<Prefab>>=OnceLock::new();
    PREFABS.get_or_init(||{
        serde_json::from_str(include_str!("../assets/porymap-emerald-prefabs.json"))
            .expect("Bundled Porymap prefabs must parse")
    })
}

/// Source symbol pairing is part of the Porymap format; an empty symbol
/// represents a wildcard. Reject invalid or modified source IDs rather than
/// painting a lookalike with silently different graphics.
pub fn compatible(prefab:&Prefab,pair:&TilePair)->bool{
    (prefab.primary_tileset.is_empty() || prefab.primary_tileset==pair.primary_symbol)
        && (prefab.secondary_tileset.is_empty() || prefab.secondary_tileset==pair.secondary_symbol)
        && prefab.width>0 && prefab.height>0
        && prefab.width<=32 && prefab.height<=32
        && !prefab.metatiles.is_empty()
        && prefab.metatiles.iter().all(|p|p.x<prefab.width && p.y<prefab.height
            && p.collision<=3 && p.elevation<=15
            && tiles::valid_in(pair,p.metatile_id))
}
pub fn compatible_indices(pair:&TilePair)->Vec<usize>{
    emerald().iter().enumerate().filter_map(|(i,p)|compatible(p,pair).then_some(i)).collect()
}

impl World {
    /// Paint an entire verified prefab without altering any neighboring map.
    /// Unlike a loose rectangular selection, source-authored collision and
    /// elevation values are applied intentionally.
    pub fn paint_prefab(&mut self,at:GridPoint,prefab:&Prefab)->usize{
        let Some(map)=self.maps.iter().find(|m|m.space_id==self.active_space_id && m.contains(at)) else{return 0;};
        let Some(pair)=tiles::pair_for(&map.primary_tileset,&map.secondary_tileset) else{return 0;};
        if !compatible(prefab,pair){return 0;}
        let map_id=map.id;
        let mut changed=0;
        for tile in &prefab.metatiles{
            let (Some(x),Some(y))=(at.x.checked_add(tile.x as i32),at.y.checked_add(tile.y as i32))
                else{continue;};
            let pos=GridPoint::new(x,y);
            let Some(map)=self.maps.iter_mut().find(|m|m.id==map_id && m.contains(pos))
                else{continue;};
            let local_x=(pos.x-map.origin.x) as u32;
            let local_y=(pos.y-map.origin.y) as u32;
            if let Some(index)=map.index(local_x,local_y){
                let previous=map.cells[index];
                let mut cell=previous.unwrap_or_else(||
                    crate::model::Cell::new(crate::model::TileKind::Grass));
                cell.metatile_id=tile.metatile_id;
                cell.collision=tile.collision;
                cell.elevation=tile.elevation;
                if previous!=Some(cell){
                    map.cells[index]=Some(cell);
                    changed+=1;
                }
            }
        }
        changed
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    use crate::model::{GridPoint,World};
    #[test]
    fn seventy_one_original_emerald_prefabs_are_parsed(){
        assert_eq!(emerald().len(),71);
        assert!(emerald().iter().any(|p|p.name=="Mart"));
        assert!(emerald().iter().any(|p|p.name=="Player House"));
    }
    #[test]
    fn built_in_prefabs_are_only_exposed_when_matching_real_pair(){
        let pair=tiles::pair(tiles::DEFAULT_PAIR).unwrap();
        let chosen=compatible_indices(pair);
        assert!(!chosen.is_empty(),"Emerald must have usable prefab selections");
        for index in chosen{
            assert!(compatible(&emerald()[index],pair));
        }
        let mut invalid=emerald()[0].clone();
        invalid.metatiles[0].metatile_id=1024;
        assert!(!compatible(&invalid,pair));
    }
    #[test]
    fn prefab_paint_keeps_native_properties_and_other_maps_untouched(){
        let pair=tiles::pair(tiles::DEFAULT_PAIR).unwrap();
        let index=compatible_indices(pair)[0];
        let p=&emerald()[index];
        let mut world=World::new();
        let id=world.add_map("Test",GridPoint::new(0,0),32,32).unwrap();
        let id2=world.add_map("Neighbor",GridPoint::new(32,0),32,32).unwrap();
        world.set_map_tileset(id,pair.key).unwrap();
        world.set_map_tileset(id2,pair.key).unwrap();
        assert!(world.paint_prefab(GridPoint::new(5,5),p)>0);
        let map=world.maps.iter().find(|m|m.id==id).unwrap();
        for tile in &p.metatiles{
            let cell=map.cell(5+tile.x,5+tile.y).unwrap();
            assert_eq!(cell.metatile_id,tile.metatile_id);
            assert_eq!(cell.collision,tile.collision);
            assert_eq!(cell.elevation,tile.elevation);
        }
        let neighbor=world.maps.iter().find(|m|m.id==id2).unwrap();
        assert!(neighbor.cells.iter().all(|x|x.is_none()));
    }
}
