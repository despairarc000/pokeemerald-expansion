//! Source-verified Emerald General/Lilycove path compositor.
//! Original LilycoveCity map.bin and its matching native tileset contain
//! a contiguous 3x3 pavement family (0x22A..0x23C). These are not guesses:
//! the exact top/side/bottom shapes were inspected in the source atlas.
//! Unverified pond/shoreline tile rules are intentionally not supplied.
use crate::{model::{GridPoint,World,Cell,TileKind,MapFrame},tiles};
use std::collections::VecDeque;
pub const LILYCOVE_PAIR:&str="general/lilycove";
pub const LILYCOVE_PAVED_METATILE:u16=0x233;
const PAVEMENT:[[u16;3];3]=[
    [0x22A,0x22B,0x22C], // top left, top, top right
    [0x232,0x233,0x234], // west, center, east
    [0x23A,0x23B,0x23C], // bottom left, bottom, bottom right
];

fn is_pavement(id:u16)->bool{PAVEMENT.iter().flatten().any(|v|*v==id)}
fn at(map:&MapFrame,x:i32,y:i32)->Result<usize,String>{
    let local_x=i64::from(x)-i64::from(map.origin.x);
    let local_y=i64::from(y)-i64::from(map.origin.y);
    if local_x<0 || local_y<0 || local_x>=i64::from(map.width)
        || local_y>=i64::from(map.height){
        return Err("Full three-tile-wide walkway must fit inside its map".into());
    }
    map.index(local_x as u32,local_y as u32).ok_or("Map index invalid".into())
}
fn can_replace(cell:Option<Cell>)->bool{
    match cell{
        None=>true,
        Some(c) if c.collision==0 && c.metatile_id==1=>true,
        Some(c) if c.collision==0 && is_pavement(c.metatile_id)=>true,
        _=>false
    }
}
fn native_tile(north:bool,south:bool,west:bool,east:bool)
    ->Result<u16,String>{
    if (!north&&!south)||(!west&&!east){
        return Err("A path segment is too narrow for complete native corner artwork".into());
    }
    let row=if !north{0}else if !south{2}else{1};
    let col=if !west{0}else if !east{2}else{1};
    Ok(PAVEMENT[row][col])
}

/// Source-backed 3-tile-wide walkway with automatic corner/edge/junction
/// recomputation. An interrupted stroke or blocked terrain leaves the entire
/// World unmodified. Fully atomic: all preflight checks happen before writes.
pub fn draw_lilycove_walkway(world:&mut World,map_id:u32,from:GridPoint,to:GridPoint)
    ->Result<usize,String>{
    let map=world.maps.iter().find(|m|m.id==map_id)
        .ok_or("Target map not found")?;
    if format!("{}/{}",map.primary_tileset,map.secondary_tileset)!=LILYCOVE_PAIR{
        return Err("Lilycove paving requires original Emerald General/Lilycove tiles".into());
    }
    let pair=tiles::pair(LILYCOVE_PAIR).ok_or("Lilycove source not present")?;
    if PAVEMENT.iter().flatten().any(|t|!tiles::valid_in(pair,*t)){
        return Err("Verified native paving artwork is missing".into());
    }
    let width=map.width as usize;
    let height=map.height as usize;
    let count=width.checked_mul(height).ok_or("Map dimensions overflow")?;
    let mut existing=vec![false;count];
    for (idx,cell) in map.cells.iter().enumerate(){
        existing[idx]=cell.is_some_and(|c|is_pavement(c.metatile_id));
    }
    let mut new_mask=vec![false;count];
    let mut x=from.x;
    let mut y=from.y;
    let dx=(i64::from(to.x)-i64::from(x)).abs();
    let dy=-(i64::from(to.y)-i64::from(y)).abs();
    if dx+(-dy)>20_000{return Err("Paving stroke is too long".into());}
    let sx=if x<to.x{1}else{-1};
    let sy=if y<to.y{1}else{-1};
    let mut err=dx+dy;
    loop{
        for yy in -1i32..=1{
            for xx in -1i32..=1{
                let px=x.checked_add(xx).ok_or("X coordinate overflow")?;
                let py=y.checked_add(yy).ok_or("Y coordinate overflow")?;
                let idx=at(map,px,py)?;
                new_mask[idx]=true;
            }
        }
        if x==to.x&&y==to.y{break;}
        let twice=2*err;
        if twice>=dy{err+=dy;x=x.checked_add(sx).ok_or("X coordinate overflow")?;}
        if twice<=dx{err+=dx;y=y.checked_add(sy).ok_or("Y coordinate overflow")?;}
    }
    // All requested tiles must be replaceable before even calculating
    // transitions. This preserves unrelated prefab and event sprites.
    for (idx,flag) in new_mask.iter().enumerate(){
        if !flag{continue;}
        if !can_replace(map.cells[idx]){
            return Err(format!("Walkway would overwrite protected tile at {},{}",
                idx%width,idx/width));
        }
        if map.events.iter().any(|event|
            event.x as usize==idx%width && event.y as usize==idx/width){
            return Err("Walkway intersects an existing event".into());
        }
    }
    let union:Vec<bool>=existing.iter().zip(&new_mask).map(|(a,b)|*a||*b).collect();
    // Connected component touched by this stroke. Unrelated old v0.18
    // single-tile paths elsewhere are never silently reinterpreted.
    let mut affected=vec![false;count];
    let mut queue=VecDeque::new();
    for (idx,set) in new_mask.iter().enumerate(){
        if *set && !affected[idx]{affected[idx]=true;queue.push_back(idx);}
    }
    while let Some(idx)=queue.pop_front(){
        let x=idx%width;let y=idx/width;
        let near=[
            (x>0).then_some(idx.saturating_sub(1)),
            (x+1<width).then_some(idx+1),
            (y>0).then_some(idx.saturating_sub(width)),
            (y+1<height).then_some(idx+width),
        ];
        for candidate in near.iter().flatten(){
            if union[*candidate]&&!affected[*candidate]{
                affected[*candidate]=true;
                queue.push_back(*candidate);
            }
        }
    }
    let mut proposed=Vec::new();
    for idx in 0..count{
        if !affected[idx]{continue;}
        let x=idx%width;let y=idx/width;
        let n=y>0&&union[idx-width];
        let s=y+1<height&&union[idx+width];
        let w=x>0&&union[idx-1];
        let e=x+1<width&&union[idx+1];
        let id=native_tile(n,s,w,e)?;
        let prior=map.cells[idx];
        if !can_replace(prior){
            return Err(format!("Paving touches protected tile at {x},{y}"));
        }
        if map.events.iter().any(|event|
            event.x as usize==x && event.y as usize==y)
            && prior.is_none_or(|c|c.metatile_id!=id){
            return Err("Recomputed pavement would alter an event tile".into());
        }
        let target=Some(Cell{terrain:TileKind::Path,metatile_id:id,
            collision:0,elevation:prior.map_or(3,|v|v.elevation)});
        if target!=prior{proposed.push((idx,target));}
    }
    let map=world.maps.iter_mut().find(|m|m.id==map_id)
        .ok_or("Map disappeared during paving")?;
    for (idx,value) in &proposed{map.cells[*idx]=*value;}
    Ok(proposed.len())
}

#[cfg(test)]
mod tests{
    use super::*;
    fn test_world()->(World,u32){
        let mut world=World::new();
        let id=world.add_map("Lilycove",GridPoint::new(10,20),32,32).unwrap();
        world.set_map_tileset(id,LILYCOVE_PAIR).unwrap();
        (world,id)
    }
    #[test]
    fn exact_nine_native_source_metatiles_for_single_three_by_three_stamp(){
        let (mut world,id)=test_world();
        let count=draw_lilycove_walkway(&mut world,id,
            GridPoint::new(16,26),GridPoint::new(16,26)).unwrap();
        assert_eq!(count,9);
        let m=&world.maps[0];
        for (r,row) in PAVEMENT.iter().enumerate(){
            for (col,expected) in row.iter().enumerate(){
                assert_eq!(m.cell(5+col as u32,5+r as u32).unwrap().metatile_id,*expected);
            }
        }
        world.check_integrity().unwrap();
    }
    #[test]
    fn longer_stroke_automatically_joins_source_edges(){
        let (mut world,id)=test_world();
        draw_lilycove_walkway(&mut world,id,GridPoint::new(16,27),GridPoint::new(20,27)).unwrap();
        let m=&world.maps[0];
        for x in 6..=10{
            let top=m.cell(x,6).unwrap().metatile_id;
            let mid=m.cell(x,7).unwrap().metatile_id;
            let bottom=m.cell(x,8).unwrap().metatile_id;
            assert_eq!(top,if x==6{0x22A}else if x==10{0x22C}else{0x22B});
            assert_eq!(mid,if x==6{0x232}else if x==10{0x234}else{0x233});
            assert_eq!(bottom,if x==6{0x23A}else if x==10{0x23C}else{0x23B});
        }
        let changed=draw_lilycove_walkway(&mut world,id,
            GridPoint::new(19,24),GridPoint::new(19,29)).unwrap();
        assert!(changed>0);
        assert_eq!(world.maps[0].cell(9,7).unwrap().metatile_id,0x233);
    }
    #[test]
    fn obstruction_or_frame_edge_preserves_original_world(){
        let (mut world,id)=test_world();
        let before=world.clone();
        assert!(draw_lilycove_walkway(&mut world,id,
            GridPoint::new(10,20),GridPoint::new(12,20)).is_err());
        assert_eq!(world,before);
        world.paint_metatile(GridPoint::new(18,27),0x170);
        let before=world.clone();
        assert!(draw_lilycove_walkway(&mut world,id,
            GridPoint::new(16,27),GridPoint::new(22,27)).is_err());
        assert_eq!(world,before);
    }
    #[test]
    fn different_tileset_never_uses_lilycove_native_ids(){
        let mut w=World::new();
        let id=w.add_map("Other",GridPoint::new(0,0),20,20).unwrap();
        let before=w.clone();
        assert!(draw_lilycove_walkway(&mut w,id,
            GridPoint::new(3,3),GridPoint::new(9,3)).is_err());
        assert_eq!(w,before);
    }
}
