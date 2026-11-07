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
