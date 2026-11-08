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

/// Original vanilla FireRed prefab definitions from huderlem/porymap, LGPL-3.0.
/// Their general tileset symbol uses the upstream FireRed name; this fork
/// names its native FRLG primary art gTileset_General_Frlg.
pub fn firered()->&'static [Prefab]{
    static PREFABS:OnceLock<Vec<Prefab>>=OnceLock::new();
    PREFABS.get_or_init(||{
        serde_json::from_str(include_str!("../assets/porymap-firered-prefabs.json"))
            .expect("Bundled Porymap FireRed prefabs must parse")
    })
}
pub fn by_collection(collection:&str)->&'static [Prefab]{
    match collection{
        "FireRed/LeafGreen"=>firered(),
        _=>emerald(),
    }
}

/// Source symbol pairing is part of the Porymap format; an empty symbol
/// represents a wildcard. Reject invalid or modified source IDs rather than
/// painting a lookalike with silently different graphics.
pub fn compatible(prefab:&Prefab,pair:&TilePair)->bool{
    let primary_matches=prefab.primary_tileset.is_empty()
        || prefab.primary_tileset==pair.primary_symbol
        || (pair.collection=="FireRed/LeafGreen" && matches!(
            (prefab.primary_tileset.as_str(),pair.primary_symbol),
            ("gTileset_General","gTileset_General_Frlg")
                | ("gTileset_Building","gTileset_BuildingFrlg")));
    primary_matches
        && (prefab.secondary_tileset.is_empty() || prefab.secondary_tileset==pair.secondary_symbol)
        && prefab.width>0 && prefab.height>0
        && prefab.width<=32 && prefab.height<=32
        && !prefab.metatiles.is_empty()
        && prefab.metatiles.iter().all(|p|p.x<prefab.width && p.y<prefab.height
            && p.collision<=3 && p.elevation<=15
            && tiles::valid_in(pair,p.metatile_id))
}
pub fn compatible_indices(pair:&TilePair)->Vec<usize>{
    by_collection(pair.collection).iter().enumerate()
        .filter_map(|(i,p)|compatible(p,pair).then_some(i)).collect()
}

impl World {
    /// Validate the ENTIRE source-authored footprint before changing a cell.
    /// A placed prefab may replace unpainted cells or ordinary source grass,
    /// but never silently overwrite a building, water, collision or event.
    pub fn preview_prefab_placement(&self,at:GridPoint,prefab:&Prefab)
        ->Result<(),String>{
        let map=self.maps.iter().find(|m|m.space_id==self.active_space_id
            && m.contains(at)).ok_or("Choose a point inside the target map")?;
        let pair=tiles::pair_for(&map.primary_tileset,&map.secondary_tileset)
            .ok_or("Unknown native map tileset")?;
        if !compatible(prefab,pair){
            return Err("Prefab requires another original source tileset".into());
        }
        let local_x=(i64::from(at.x)-i64::from(map.origin.x)) as u32;
        let local_y=(i64::from(at.y)-i64::from(map.origin.y)) as u32;
        if local_x.checked_add(prefab.width).is_none_or(|x|x>map.width)
            || local_y.checked_add(prefab.height).is_none_or(|y|y>map.height){
            return Err("The complete prefab must fit inside one map".into());
        }
        let mut seen=std::collections::HashSet::new();
        for tile in &prefab.metatiles {
            if !seen.insert((tile.x,tile.y)){
                return Err("Prefab source defines a duplicate footprint tile".into());
            }
            let idx=map.index(local_x+tile.x,local_y+tile.y)
                .ok_or("Prefab tile is outside the map")?;
            if let Some(existing)=map.cells[idx]{
                // The map's authentic General_Grass (id 1) is paintable
                // ground. Never classify other source IDs as expendable.
                if !(existing.metatile_id==1 && existing.collision==0)
                    && !(existing.metatile_id==tile.metatile_id
                        && existing.collision==tile.collision
                        && existing.elevation==tile.elevation){
                    return Err(format!(
                        "Placement blocked by painted tile at {},{}",
                        local_x+tile.x,local_y+tile.y));
                }
            }
            // Events cannot be overwritten accidentally. Existing events
            // remain authoritative native map objects.
            if map.events.iter().any(|e|e.x==local_x+tile.x && e.y==local_y+tile.y){
                return Err("Prefab overlaps an existing event".into());
            }
        }
        Ok(())
    }

    /// Transactional placement: complete stamp or no changes.
    pub fn try_paint_prefab(&mut self,at:GridPoint,prefab:&Prefab)
        ->Result<usize,String>{
        self.preview_prefab_placement(at,prefab)?;
        let map=self.maps.iter_mut().find(|m|
            m.space_id==self.active_space_id && m.contains(at))
            .ok_or("Target map disappeared")?;
        let local_x=(at.x-map.origin.x) as u32;
        let local_y=(at.y-map.origin.y) as u32;
        let mut changed=0;
        for tile in &prefab.metatiles {
            let index=map.index(local_x+tile.x,local_y+tile.y)
                .expect("preflight checked the full footprint");
            let previous=map.cells[index];
            let mut cell=previous.unwrap_or_else(||
                crate::model::Cell::new(crate::model::TileKind::Grass));
            cell.metatile_id=tile.metatile_id;
            cell.collision=tile.collision;
            cell.elevation=tile.elevation;
            if previous!=Some(cell) {
                map.cells[index]=Some(cell);
                changed+=1;
            }
        }
        Ok(changed)
    }

    /// Legacy caller convenience. New UI should use try_paint_prefab so
    /// failed placement can show an exact error.
    pub fn paint_prefab(&mut self,at:GridPoint,prefab:&Prefab)->usize{
        self.try_paint_prefab(at,prefab).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    use crate::model::{GridPoint,World};
    #[test]
    fn complete_prefab_placement_never_clips_at_map_edges(){
        let pair=tiles::pair(tiles::DEFAULT_PAIR).unwrap();
        let p=&emerald()[compatible_indices(pair)[0]];
        let mut world=World::new();
        let id=world.add_map("Edge",GridPoint::new(0,0),32,32).unwrap();
        world.set_map_tileset(id,pair.key).unwrap();
        let before=world.clone();
        assert!(world.try_paint_prefab(GridPoint::new(31,31),p)
            .unwrap_err().contains("complete prefab"));
        assert_eq!(world,before,"partial building was stamped");
    }
    #[test]
    fn complete_prefab_placement_refuses_to_overwrite_existing_structure(){
        let pair=tiles::pair(tiles::DEFAULT_PAIR).unwrap();
        let p=&emerald()[compatible_indices(pair)[0]];
        let mut world=World::new();
        let id=world.add_map("Protected",GridPoint::new(0,0),32,32).unwrap();
        world.set_map_tileset(id,pair.key).unwrap();
        assert!(world.try_paint_prefab(GridPoint::new(4,4),p).unwrap()>0);
        let before=world.clone();
        let overlapping=GridPoint::new(5,5);
        assert!(world.try_paint_prefab(overlapping,p).is_err());
        assert_eq!(world,before,"existing building was overwritten");
    }
    #[test]
    fn seventy_one_original_emerald_prefabs_are_parsed(){
        assert_eq!(emerald().len(),71);
        assert!(emerald().iter().any(|p|p.name=="Mart"));
        assert!(emerald().iter().any(|p|p.name=="Player House"));
    }
    #[test]
    fn firered_definitions_are_bundled_and_source_scoped(){
        assert_eq!(firered().len(),45);
        let pairs:Vec<_>=tiles::pairs().iter()
            .filter(|pair|pair.collection=="FireRed/LeafGreen").collect();
        assert!(!pairs.is_empty(),"The fork contains native FRLG source pairs");
        assert!(pairs.iter().any(|pair|!compatible_indices(pair).is_empty()),
            "No bundled FireRed prefab is source-compatible");
        for pair in pairs{
            for index in compatible_indices(pair){
                assert!(compatible(&firered()[index],pair));
            }
        }
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
