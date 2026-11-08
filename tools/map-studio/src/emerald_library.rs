//! New Emerald-first source library. Old source-pair records are retained
//! for reading legacy projects, but the picker no longer exposes a multi-game
//! collection maze. Every prefab maps to its ORIGINAL Porymap definition.
use crate::{prefabs,tiles::{self,TilePair}};

pub const THUMBNAILS:&[u8]=include_bytes!(concat!(env!("OUT_DIR"),"/emerald_prefabs.png"));
pub const THUMB_SIZE:usize=96;
pub const THUMB_COLS:usize=8;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Group{All,Homes,Services,Landmarks,Nature,Other}
impl Group{
    pub const ALL:[Self;6]=[
        Self::All,Self::Homes,Self::Services,
        Self::Landmarks,Self::Nature,Self::Other
    ];
    pub const fn name(self)->&'static str{
        match self{
            Self::All=>"All",Self::Homes=>"Homes",Self::Services=>"Services",
            Self::Landmarks=>"Landmarks",Self::Nature=>"Nature",Self::Other=>"Other"
        }
    }
}
pub fn group_for(name:&str)->Group{
    let value=name.to_ascii_lowercase();
    if ["house","home","cottage","residence","villa"].iter().any(|s|value.contains(s)){
        return Group::Homes;
    }
    if ["mart","center","shop","store","harbor","shipyard","hotel","lab","market",
        "school","fan club","battle tent"].iter().any(|s|value.contains(s)){
        return Group::Services;
    }
    if ["gym","contest","museum","corp","tower","facility","frontier","safari",
        "hideout","hall"].iter().any(|s|value.contains(s)){
        return Group::Landmarks;
    }
    if ["tree","rock","bush","pond","water","garden","flower","sand","grass",
        "bridge"].iter().any(|s|value.contains(s)){
        return Group::Nature;
    }
    Group::Other
}
pub struct Entry{
    pub index:usize,
    pub source_pair:Option<&'static TilePair>,
    pub compatible:bool,
    pub category:Group,
}

/// Exact native Porymap source compatibility; no guessed metatile remapping.
/// A secondary-only prefab is resolved against General first if supported.
pub fn source_pair(index:usize)->Option<&'static TilePair>{
    let prefab=prefabs::emerald().get(index)?;
    let compatible:Vec<_>=tiles::pairs().iter()
        .filter(|pair|pair.collection=="Emerald" && prefabs::compatible(prefab,pair))
        .collect();
    compatible.iter().copied().find(|pair|pair.primary=="general")
        .or_else(||compatible.first().copied())
}
pub fn list(current:&TilePair,group:Group)->Vec<Entry>{
    let mut entries:Vec<_>=prefabs::emerald().iter().enumerate()
        .filter(|(_,prefab)|group==Group::All || group_for(&prefab.name)==group)
        .map(|(index,prefab)|Entry{
            index,source_pair:source_pair(index),
            compatible:current.collection=="Emerald" && prefabs::compatible(prefab,current),
            category:group_for(&prefab.name),
        })
        .collect();
    // Source-ready first; all others stay in the same gallery, without a
    // second expandable list or an intermediate "Load" action.
    entries.sort_by_key(|e|(!e.compatible,e.index));
    entries
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn all_emerald_prefabs_remain_discoverable(){
        let map=tiles::pair("general/lilycove").unwrap();
        let items=list(map,Group::All);
        assert_eq!(items.len(),71);
        assert_eq!(items.iter().filter(|x|x.compatible).count(),
            prefabs::compatible_indices(map).len());
        assert!(items.iter().take_while(|x|x.compatible).count()>0);
        assert!(items.iter().any(|x|!x.compatible));
        assert!(items.iter().all(|x|x.source_pair.is_some() || !x.compatible));
    }
    #[test]
    fn predefined_assets_are_stable_and_source_scoped(){
        let pair=tiles::pair("general/lilycove").unwrap();
        for item in list(pair,Group::All){
            assert!(item.index<prefabs::emerald().len());
            if item.compatible{
                assert!(prefabs::compatible(&prefabs::emerald()[item.index],pair));
            }
            if let Some(source)=item.source_pair{
                assert!(prefabs::compatible(&prefabs::emerald()[item.index],source));
            }
        }
    }
    #[test]
    fn groups_are_presentation_only_and_never_change_source_tiles(){
        assert_eq!(group_for("House Large"),Group::Homes);
        assert_eq!(group_for("Mart"),Group::Services);
        assert_eq!(group_for("Gym"),Group::Landmarks);
    }
}
