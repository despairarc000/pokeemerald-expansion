//! Human-friendly NPC sprite choices. Every engine symbol here is present in
//! include/constants/event_objects.h. Do not allow free-form engine IDs in UI.
pub const NPC_SPRITES:&[(&str,&str)]=&[
    ("Youngster", "OBJ_EVENT_GFX_YOUNGSTER"),
    ("Little boy", "OBJ_EVENT_GFX_LITTLE_BOY"),
    ("Little girl", "OBJ_EVENT_GFX_LITTLE_GIRL"),
    ("Schoolboy", "OBJ_EVENT_GFX_SCHOOL_KID_M"),
    ("Lass", "OBJ_EVENT_GFX_LASS"),
    ("Bug catcher", "OBJ_EVENT_GFX_BUG_CATCHER"),
    ("Camper", "OBJ_EVENT_GFX_CAMPER"),
    ("Picnicker", "OBJ_EVENT_GFX_PICNICKER"),
    ("Older man", "OBJ_EVENT_GFX_OLD_MAN"),
    ("Older woman", "OBJ_EVENT_GFX_OLD_WOMAN"),
    ("Adult man", "OBJ_EVENT_GFX_MAN_1"),
    ("Adult woman", "OBJ_EVENT_GFX_WOMAN_1"),
    ("Nurse", "OBJ_EVENT_GFX_NURSE"),
    ("Scientist", "OBJ_EVENT_GFX_SCIENTIST_1"),
    ("Hiker", "OBJ_EVENT_GFX_HIKER"),
    ("Fisherman", "OBJ_EVENT_GFX_FISHERMAN"),
    ("Sailor", "OBJ_EVENT_GFX_SAILOR"),
    ("Shop assistant", "OBJ_EVENT_GFX_MART_EMPLOYEE"),
    ("Professor Birch", "OBJ_EVENT_GFX_PROF_BIRCH"),
];
pub fn npc_friendly(symbol:&str)->Option<&'static str>{
    NPC_SPRITES.iter().find(|(_,id)|*id==symbol).map(|(name,_)|*name)
}
#[cfg(test)]
mod tests {
    #[test]
    fn npc_choices_are_real_source_symbols() {
        let src=include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../include/constants/event_objects.h"));
        for (_,id) in super::NPC_SPRITES {
            assert!(src.contains(id),"Missing Emerald sprite ID {id}");
        }
    }
}

/// Check the real source enum rather than trusting arbitrary JSON string input.
pub fn is_verified_sprite_id(name:&str)->bool {
    if !name.starts_with("OBJ_EVENT_GFX_") {return false;}
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"),
        "/../../include/constants/event_objects.h"))
        .lines()
        .map(str::trim)
        .any(|line|line.split(',').next().unwrap_or("").trim()==name)
}
#[cfg(test)]
mod sprite_validation_tests {
    #[test]
    fn arbitrary_code_is_not_a_valid_sprite() {
        assert!(!super::is_verified_sprite_id("OBJ_EVENT_GFX_FAKE"));
        assert!(!super::is_verified_sprite_id("SCRIPT_SUPERUSER"));
        for (_,id) in super::NPC_SPRITES {
            assert!(super::is_verified_sprite_id(id));
        }
    }
}


/// Actual inventory items from this fork's constants/items.h.
/// A hidden item does not default to an item: the player must choose one.
pub const HIDDEN_ITEMS:&[(&str,&str)]=&[
    ("Poké Ball","ITEM_POKE_BALL"),
    ("Potion","ITEM_POTION"),
    ("Super Potion","ITEM_SUPER_POTION"),
    ("Antidote","ITEM_ANTIDOTE"),
    ("Full Heal","ITEM_FULL_HEAL"),
    ("Revive","ITEM_REVIVE"),
    ("Escape Rope","ITEM_ESCAPE_ROPE"),
    ("Ether","ITEM_ETHER"),
    ("Rare Candy","ITEM_RARE_CANDY"),
    ("Nugget","ITEM_NUGGET"),
    ("Tiny Mushroom","ITEM_TINY_MUSHROOM"),
];

/// These 16 persistent slots are marked unused in the checked-in Emerald flags
/// and must be treated as reserved to Map Studio projects. Never accept arbitrary
/// user-supplied flag identifiers. Conflicts with external mods still require review.
pub const PICKUP_SLOTS:[&str;16]=[
    "FLAG_UNUSED_0x020","FLAG_UNUSED_0x021","FLAG_UNUSED_0x022","FLAG_UNUSED_0x023",
    "FLAG_UNUSED_0x024","FLAG_UNUSED_0x025","FLAG_UNUSED_0x026","FLAG_UNUSED_0x027",
    "FLAG_UNUSED_0x028","FLAG_UNUSED_0x029","FLAG_UNUSED_0x02A","FLAG_UNUSED_0x02B",
    "FLAG_UNUSED_0x02C","FLAG_UNUSED_0x02D","FLAG_UNUSED_0x02E","FLAG_UNUSED_0x02F",
];
include!(concat!(env!("OUT_DIR"),"/items/item_icon_catalog.rs"));
pub const HIDDEN_ITEM_ATLAS:&[u8]=include_bytes!(concat!(
    env!("OUT_DIR"),"/items/item-previews.png"));
pub fn hidden_item_preview(id:&str)->Option<usize>{
    HIDDEN_ICON_REFS.iter().find(|(symbol,_)|*symbol==id).map(|(_,index)|*index)
}
pub fn valid_hidden_item(value:&str)->bool {
    HIDDEN_ITEMS.iter().any(|(_,id)|*id==value)
}
pub fn valid_pickup_slot(value:&str)->bool {
    PICKUP_SLOTS.contains(&value)
}
pub fn free_pickup_slot<'a>(in_use:impl Iterator<Item=&'a str>)->Option<&'static str>{
    let used:std::collections::HashSet<&str>=in_use.collect();
    PICKUP_SLOTS.iter().copied().find(|slot|!used.contains(slot))
}
#[cfg(test)]
mod item_picker_tests {
    #[test]
    fn item_and_flag_choices_exist_in_this_fork(){
        let items=include_str!(concat!(env!("CARGO_MANIFEST_DIR"),
            "/../../include/constants/items.h"));
        let flags=include_str!(concat!(env!("CARGO_MANIFEST_DIR"),
            "/../../include/constants/flags.h"));
        for (_,id) in super::HIDDEN_ITEMS{
            assert!(items.contains(id),"Missing real item {id}");
        }
        for flag in super::PICKUP_SLOTS {
            assert!(flags.contains(flag),"Missing reserved unused flag {flag}");
        }
    }
}


// Generated from the same source object-event picture and palette tables as
// the game's actual sprite renderer. No invented NPC art or unverified IDs.
include!(concat!(env!("OUT_DIR"),"/npc_sprite_catalog.rs"));
pub fn npc_preview_index(id:&str)->Option<usize>{
    NPC_PREVIEW_REFS.iter().find(|(name,_)|*name==id).map(|(_,n)|*n)
}
#[cfg(test)]
mod npc_preview_tests {
    #[test]
    fn editor_character_previews_use_only_supported_engine_ids(){
        assert!(super::NPC_PREVIEW_COUNT>=15);
        for (symbol,_) in super::NPC_PREVIEW_REFS {
            assert!(super::is_verified_sprite_id(symbol));
            assert!(super::NPC_SPRITES.iter().any(|(_,id)|id==symbol));
        }
        assert!(super::NPC_ATLAS_PNG.starts_with(&[137,80,78,71]));
    }
}

/// A deliberately limited set of real constants from
/// include/constants/event_object_movement.h. No script or trainer
/// movements are fabricated by the editor.
pub const NPC_MOVEMENTS:&[(&str,&str)]=&[
    ("Face down","MOVEMENT_TYPE_FACE_DOWN"),
    ("Face up","MOVEMENT_TYPE_FACE_UP"),
    ("Face left","MOVEMENT_TYPE_FACE_LEFT"),
    ("Face right","MOVEMENT_TYPE_FACE_RIGHT"),
    ("Look around","MOVEMENT_TYPE_LOOK_AROUND"),
    ("Wander around","MOVEMENT_TYPE_WANDER_AROUND"),
    ("Wander up/down","MOVEMENT_TYPE_WANDER_UP_AND_DOWN"),
    ("Wander left/right","MOVEMENT_TYPE_WANDER_LEFT_AND_RIGHT"),
    ("Walk up/down","MOVEMENT_TYPE_WALK_UP_AND_DOWN"),
    ("Walk left/right","MOVEMENT_TYPE_WALK_LEFT_AND_RIGHT"),
    ("Rotate clockwise","MOVEMENT_TYPE_ROTATE_CLOCKWISE"),
    ("Rotate counterclockwise","MOVEMENT_TYPE_ROTATE_COUNTERCLOCKWISE"),
];
pub fn valid_npc_movement(value:&str)->bool {
    NPC_MOVEMENTS.iter().any(|(_,id)|*id==value)
}
#[cfg(test)]
mod movement_choice_tests {
    use super::*;
    #[test]
    fn options_are_engine_constants_and_include_backward_compatible_default(){
        assert!(valid_npc_movement("MOVEMENT_TYPE_FACE_DOWN"));
        assert!(valid_npc_movement("MOVEMENT_TYPE_WANDER_AROUND"));
        assert!(!valid_npc_movement("MOVEMENT_TYPE_MAGIC_TRAINER"));
    }
}
