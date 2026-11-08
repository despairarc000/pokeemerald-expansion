//! Source-verified Emerald/Lilycove outdoor terrain tools.
//! The source's Lilycove City native map uses 0x22B (555) repeatedly for
//! the beige/green paved pedestrian walkway. Never invent transition IDs.
//! This is a safe, single-tile pavement-family tool. Autotiling/shorelines
//! require separate source-verified neighbor rules and are NOT implemented.
use crate::{model::{GridPoint,World,Cell,TileKind},tiles};
pub const LILYCOVE_PAVED_METATILE:u16=0x22B;
pub const LILYCOVE_PAIR:&str="general/lilycove";

/// Paints a continuous paved route. The source map's true metatile ID,
/// compatible tileset, existing buildings, events and occupancy all matter.
/// Either the whole requested route is applied or none of it is.
pub fn draw_lilycove_walkway(world:&mut World,map_id:u32,from:GridPoint,to:GridPoint)
    ->Result<usize,String>{
    let map=world.maps.iter().find(|m|m.id==map_id)
        .ok_or("Target map not found")?;
    if format!("{}/{}",map.primary_tileset,map.secondary_tileset)!=LILYCOVE_PAIR{
        return Err("This walkway requires native Emerald General/Lilycove source".into());
    }
    let pair=tiles::pair(LILYCOVE_PAIR)
        .ok_or("Native Lilycove art unavailable")?;
    if !tiles::valid_in(pair,LILYCOVE_PAVED_METATILE){
        return Err("Native Lilycove walkway tile unavailable".into());
    }
    let (mut x,mut y)=(from.x,from.y);
    let dx=(i64::from(to.x)-i64::from(x)).abs();
    let dy=-(i64::from(to.y)-i64::from(y)).abs();
    let sx=if x<to.x{1}else{-1};
    let sy=if y<to.y{1}else{-1};
    let mut err=dx+dy;
    let mut positions=Vec::new();
    loop {
        let at=GridPoint::new(x,y);
        if !map.contains(at){
            return Err("Walkway must remain entirely inside the selected map".into());
        }
        let local_x=(x-map.origin.x) as u32;
        let local_y=(y-map.origin.y) as u32;
        if map.events.iter().any(|event|event.x==local_x&&event.y==local_y){
            return Err("Walkway intersects an existing event".into());
        }
        let idx=map.index(local_x,local_y).ok_or("Outside map")?;
        if let Some(cell)=map.cells[idx]{
            let grass=cell.metatile_id==1 && cell.collision==0;
            let already_paved=cell.metatile_id==LILYCOVE_PAVED_METATILE
                && cell.collision==0;
            if !grass && !already_paved{
                return Err(format!("Walkway would overwrite a protected source tile at {local_x},{local_y}"));
            }
        }
        positions.push(idx);
        if x==to.x&&y==to.y{break;}
        let twice=2*err;
        if twice>=dy {
            err+=dy;
            x=x.checked_add(sx).ok_or("Walkway coordinate overflow")?;
        }
        if twice<=dx {
            err+=dx;
            y=y.checked_add(sy).ok_or("Walkway coordinate overflow")?;
        }
        if positions.len()>10000{return Err("Walkway is too long".into());}
    }
    let map=world.maps.iter_mut().find(|m|m.id==map_id)
        .ok_or("Target map changed")?;
    let mut changed=0;
    for idx in positions{
        let prior=map.cells[idx];
        let next=Some(Cell{terrain:TileKind::Path,metatile_id:LILYCOVE_PAVED_METATILE,
            collision:0,elevation:prior.map_or(3,|c|c.elevation)});
        if prior!=next{
            map.cells[idx]=next;
            changed+=1;
        }
    }
    Ok(changed)
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn native_walkway_is_source_valid_and_preserves_obstacles(){
        let mut world=World::new();
        let map=world.add_map("Lilycove",GridPoint::new(10,20),25,25).unwrap();
        world.set_map_tileset(map,LILYCOVE_PAIR).unwrap();
        for x in 10..35{
            world.paint_metatile(GridPoint::new(x,24),1);
        }
        assert!(tiles::valid_in(tiles::pair(LILYCOVE_PAIR).unwrap(),LILYCOVE_PAVED_METATILE));
        let count=draw_lilycove_walkway(&mut world,map,
            GridPoint::new(12,24),GridPoint::new(26,24)).unwrap();
        assert_eq!(count,15);
        let before=world.clone();
        // A conflicting native metatile is not unconditionally replaced.
        world.paint_metatile(GridPoint::new(20,24),0x170);
        let protected=world.clone();
        assert!(draw_lilycove_walkway(&mut world,map,
            GridPoint::new(11,24),GridPoint::new(27,24)).is_err());
        assert_eq!(world,protected);
        assert_ne!(world,before);
    }
    #[test]
    fn cannot_walk_between_unrelated_gba_source_pairs(){
        let mut world=World::new();
        let map=world.add_map("Default",GridPoint::new(0,0),20,20).unwrap();
        let initial=world.clone();
        assert!(draw_lilycove_walkway(&mut world,map,
            GridPoint::new(1,1),GridPoint::new(2,2)).is_err());
        assert_eq!(world,initial);
    }
    #[test]
    fn path_rejects_outside_frame_without_partial_write(){
        let mut world=World::new();
        let map=world.add_map("Boundary",GridPoint::new(0,0),10,10).unwrap();
        world.set_map_tileset(map,LILYCOVE_PAIR).unwrap();
        let before=world.clone();
        assert!(draw_lilycove_walkway(&mut world,map,
            GridPoint::new(5,5),GridPoint::new(20,5)).is_err());
        assert_eq!(world,before);
    }
}
