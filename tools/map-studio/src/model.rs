//! Engine-independent map geometry and actual numeric Emerald map cells.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridPoint {
    pub x: i32,
    pub y: i32,
}

impl GridPoint {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TileKind {
    Grass,
    Path,
    Trees,
    Water,
    Ice,
    Flowers,
    Roof,
    Cliff,
}

impl TileKind {
    pub const ALL: [TileKind; 8] = [
        Self::Grass, Self::Path, Self::Trees, Self::Water, Self::Ice,
        Self::Flowers, Self::Roof, Self::Cliff,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Grass => "Grass",
            Self::Path => "Path",
            Self::Trees => "Trees",
            Self::Water => "Water",
            Self::Ice => "Ice",
            Self::Flowers => "Flowers",
            Self::Roof => "Roof",
            Self::Cliff => "Cliff",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    pub terrain: TileKind,
    #[serde(default = "default_metatile_id")]
    pub metatile_id: u16,
    pub collision: u8,
    pub elevation: u8,
}

fn default_metatile_id() -> u16 { 1 }

impl Cell {
    pub const fn new(terrain: TileKind) -> Self {
        Self {
            terrain,
            metatile_id: match terrain { TileKind::Water => 0x170, TileKind::Trees => 0x00D, TileKind::Path => 0x015, _ => 0x001 },
            collision: match terrain {
                TileKind::Trees | TileKind::Roof | TileKind::Cliff => 1,
                _ => 0,
            },
            elevation: match terrain {
                TileKind::Water => 1,
                _ => 3,
            },
        }
    }

    /// Emerald binary packing; NOT a proof that a provisional tile kind has a valid metatile ID.
    pub fn pack(self, real_metatile_id: u16) -> u16 {
        (real_metatile_id & 0x03ff)
            | (((self.collision as u16) & 3) << 10)
            | (((self.elevation as u16) & 15) << 12)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Marker {
    pub x: u32,
    pub y: u32,
}


#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventKind { Npc, Sign, Warp, Trigger }

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapEvent {
    pub kind: EventKind,
    pub x: u32,
    pub y: u32,
    pub elevation: u8,
    #[serde(default)]
    pub dialogue: String,
    #[serde(default)]
    pub dest_map_id: u32,
    #[serde(default)]
    pub dest_warp_id: u8,
    #[serde(default)]
    pub script: String,
    #[serde(default = "default_graphics_id")]
    pub graphics_id: String,
    #[serde(default = "default_trigger_var")]
    pub var: String,
    #[serde(default)]
    pub var_value: u16,
}

fn default_graphics_id() -> String { "OBJ_EVENT_GFX_YOUNGSTER".into() }
fn default_trigger_var() -> String { "VAR_TEMP_0".into() }

impl MapEvent {
    pub fn new(kind: EventKind, x: u32, y: u32) -> Self {
        Self {
            kind, x, y,
            elevation: 3,
            // Never invent a dialogue line for a new event.
            dialogue: String::new(),
            dest_map_id: 0,
            dest_warp_id: 0,
            script: String::new(),
            graphics_id: default_graphics_id(),
            var: default_trigger_var(),
            var_value: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapFrame {
    pub id: u32,
    pub name: String,
    pub origin: GridPoint,
    pub width: u32,
    pub height: u32,
    pub cells: Vec<Option<Cell>>,
    pub markers: Vec<Marker>,
    #[serde(default)]
    pub events: Vec<MapEvent>,
}

impl MapFrame {
    fn right(&self) -> i64 { i64::from(self.origin.x) + i64::from(self.width) }
    fn bottom(&self) -> i64 { i64::from(self.origin.y) + i64::from(self.height) }

    pub fn contains(&self, p: GridPoint) -> bool {
        i64::from(p.x) >= i64::from(self.origin.x)
            && i64::from(p.y) >= i64::from(self.origin.y)
            && i64::from(p.x) < self.right()
            && i64::from(p.y) < self.bottom()
    }

    pub fn index(&self, local_x: u32, local_y: u32) -> Option<usize> {
        if local_x >= self.width || local_y >= self.height { return None; }
        Some((local_y as usize) * (self.width as usize) + local_x as usize)
    }

    pub fn cell(&self, x: u32, y: u32) -> Option<Cell> {
        self.index(x, y).and_then(|i| self.cells[i])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction { Up, Down, Left, Right }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Connection {
    pub from: u32,
    pub to: u32,
    pub direction: Direction,
    /// Other frame's origin minus source origin along the connected edge.
    pub offset: i32,
    pub common_tiles: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct World {
    pub maps: Vec<MapFrame>,
    next_id: u32,
}

impl World {
    /// Validate all invariants after deserializing an untrusted project.
    /// Editor projects permit empty cells; game export applies stricter checks.
    pub fn check_integrity(&self) -> Result<(),String> {
        if self.maps.len()>256 {return Err("Project has too many map frames".into());}
        let mut ids=std::collections::HashSet::new();
        let mut max_id=0;
        for map in &self.maps {
            if map.id==0 || !ids.insert(map.id) {
                return Err("Project contains duplicate or zero map IDs".into());
            }
            max_id=max_id.max(map.id);
            let count=(map.width as u64)*(map.height as u64);
            if map.width==0 || map.height==0 || count>100_000
                || count!=(map.cells.len() as u64) {
                return Err(format!("Invalid dimensions for {}",map.name));
            }
            if map.name.len()>128 {
                return Err("Map name is too long".into());
            }
            for cell in map.cells.iter().flatten() {
                if cell.metatile_id>=1024 || cell.collision>3 || cell.elevation>15 {
                    return Err(format!("Invalid native cell value in {}",map.name));
                }
            }
            if map.markers.iter().any(|e|e.x>=map.width || e.y>=map.height)
                || map.events.iter().any(|e|e.x>=map.width || e.y>=map.height ||
                    e.elevation>15) {
                return Err(format!("Invalid event position in {}",map.name));
            }
        }
        if self.next_id <= max_id {
            return Err("Project map ID counter is inconsistent".into());
        }
        for (i,a) in self.maps.iter().enumerate() {
            let al=i64::from(a.origin.x);
            let at=i64::from(a.origin.y);
            let ar=al+i64::from(a.width);
            let ab=at+i64::from(a.height);
            for b in self.maps.iter().skip(i+1) {
                let bl=i64::from(b.origin.x);
                let bt=i64::from(b.origin.y);
                let br=bl+i64::from(b.width);
                let bb=bt+i64::from(b.height);
                if al<br && ar>bl && at<bb && ab>bt {
                    return Err(format!("Map frames overlap: {} and {}",a.name,b.name));
                }
            }
        }
        Ok(())
    }

    pub fn new() -> Self { Self { maps: Vec::new(), next_id: 1 } }

    pub fn sample() -> Self {
        let mut w = Self::new();
        w.add_map("Map A", GridPoint::new(0, 0), 12, 10).unwrap();
        w.add_map("Map B", GridPoint::new(12, 0), 9, 5).unwrap();
        w.add_map("Map C", GridPoint::new(12, 5), 9, 5).unwrap();
        w
    }

    pub fn add_map(
        &mut self, name: impl Into<String>, origin: GridPoint, width: u32, height: u32,
    ) -> Result<u32, &'static str> {
        if width == 0 || height == 0 { return Err("Maps must have positive dimensions"); }
        // Editor allocation guard, not an Emerald engine limit.
        if (width as u64) * (height as u64) > 100_000 { return Err("Map is too large to edit in this prototype"); }
        let left = i64::from(origin.x);
        let top = i64::from(origin.y);
        let right = left + i64::from(width);
        let bottom = top + i64::from(height);
        if self.maps.iter().any(|m| left < m.right() && right > i64::from(m.origin.x)
            && top < m.bottom() && bottom > i64::from(m.origin.y)) {
            return Err("Map frames cannot overlap");
        }
        let id = self.next_id;
        self.next_id = self.next_id.checked_add(1).ok_or("Too many maps")?;
        self.maps.push(MapFrame {
            id, name: name.into(), origin, width, height,
            cells: vec![None; (width as usize) * (height as usize)],
            markers: Vec::new(),
            events: Vec::new(),
        });
        Ok(id)
    }

    pub fn map_at(&self, p: GridPoint) -> Option<&MapFrame> {
        self.maps.iter().find(|m| m.contains(p))
    }

    pub fn paint(&mut self, p: GridPoint, terrain: TileKind) -> bool {
        if let Some(map) = self.maps.iter_mut().find(|m| m.contains(p)) {
            let x = (p.x - map.origin.x) as u32;
            let y = (p.y - map.origin.y) as u32;
            if let Some(i) = map.index(x, y) {
                let value = Some(Cell::new(terrain));
                if map.cells[i] != value { map.cells[i] = value; return true; }
            }
        }
        false
    }

    pub fn set_collision(&mut self, p: GridPoint, value: u8) -> bool {
        if value > 3 { return false; }
        if let Some(map) = self.maps.iter_mut().find(|m| m.contains(p)) {
            let x = (p.x - map.origin.x) as u32;
            let y = (p.y - map.origin.y) as u32;
            if let Some(i) = map.index(x, y) {
                if let Some(mut cell) = map.cells[i] {
                    if cell.collision != value {
                        cell.collision = value;
                        map.cells[i] = Some(cell);
                        return true;
                    }
                }
            }
        }
        false
    }


    pub fn add_event(&mut self, p: GridPoint, kind: EventKind) -> Option<(u32,usize)> {
        let suggested_target=self.maps.iter().find(|m|!m.contains(p)).map(|m|m.id).unwrap_or(0);
        if let Some(map)=self.maps.iter_mut().find(|m|m.contains(p)) {
            let x=(p.x-map.origin.x) as u32;
            let y=(p.y-map.origin.y) as u32;
            if let Some(index)=map.events.iter().position(|e|e.x==x&&e.y==y) {
                return Some((map.id,index));
            }
            let mut event=MapEvent::new(kind,x,y);
            if kind==EventKind::Warp {
                event.elevation=0;
                event.dest_map_id=suggested_target;
            }
            map.events.push(event);
            Some((map.id,map.events.len()-1))
        } else { None }
    }

    pub fn remove_event(&mut self,map_id:u32,index:usize) -> bool {
        if let Some(map)=self.maps.iter_mut().find(|m|m.id==map_id) {
            if index<map.events.len() { map.events.remove(index); return true; }
        }
        false
    }

    pub fn add_marker(&mut self, p: GridPoint) -> bool {
        if let Some(map) = self.maps.iter_mut().find(|m| m.contains(p)) {
            let x = (p.x - map.origin.x) as u32;
            let y = (p.y - map.origin.y) as u32;
            if !map.markers.iter().any(|e| e.x == x && e.y == y) {
                map.markers.push(Marker { x, y });
                return true;
            }
        }
        false
    }


    pub fn paint_metatile(&mut self, p: GridPoint, id: u16) -> bool {
        if id >= 1024 { return false; }
        if let Some(map) = self.maps.iter_mut().find(|m| m.contains(p)) {
            let x = (p.x - map.origin.x) as u32;
            let y = (p.y - map.origin.y) as u32;
            if let Some(i) = map.index(x, y) {
                let old = map.cells[i];
                let value = Some(match old {
                    Some(mut cell) => {
                        cell.metatile_id = id;
                        cell
                    }
                    None => Cell { terrain: TileKind::Grass, metatile_id: id,
                        collision: 0, elevation: if id == 0x170 { 1 } else { 3 } },
                });
                if map.cells[i] != value { map.cells[i] = value; return true; }
            }
        }
        false
    }

    pub fn erase(&mut self, p: GridPoint) -> bool {
        if let Some(map) = self.maps.iter_mut().find(|m| m.contains(p)) {
            let x = (p.x - map.origin.x) as u32;
            let y = (p.y - map.origin.y) as u32;
            if let Some(i) = map.index(x, y) {
                return map.cells[i].take().is_some();
            }
        }
        false
    }

    pub fn set_elevation(&mut self, p: GridPoint, value: u8) -> bool {
        if value > 15 { return false; }
        if let Some(map) = self.maps.iter_mut().find(|m| m.contains(p)) {
            let x = (p.x - map.origin.x) as u32;
            let y = (p.y - map.origin.y) as u32;
            if let Some(i) = map.index(x, y) {
                if let Some(mut cell) = map.cells[i] {
                    if cell.elevation != value {
                        cell.elevation = value;
                        map.cells[i] = Some(cell);
                        return true;
                    }
                }
            }
        }
        false
    }

    pub fn remove_frame(&mut self, id:u32) -> Result<(),String> {
        if !self.maps.iter().any(|m|m.id==id) {
            return Err("Map not found".into());
        }
        if self.maps.iter().any(|m|m.id!=id && m.events.iter().any(
            |e|e.kind==EventKind::Warp && e.dest_map_id==id
        )) {
            return Err("Map is a warp destination; remove incoming warps first".into());
        }
        self.maps.retain(|m|m.id!=id);
        Ok(())
    }

    /// Non-mutating overlap probe used by the touchscreen previews.
    /// Touching the edge of a neighboring frame is permitted.
    pub fn can_place_frame(&self, origin:GridPoint,width:u32,height:u32)->Result<(),String>{
        if width==0 || height==0 || (width as u64)*(height as u64)>100_000 {
            return Err("Choose a valid map size".into());
        }
        if self.maps.len()>=256 {return Err("Map limit reached".into());}
        let (x,y)=(i64::from(origin.x),i64::from(origin.y));
        let (right,bottom)=(x+i64::from(width),y+i64::from(height));
        if self.maps.iter().any(|other| {
            x<other.right() && right>i64::from(other.origin.x) &&
            y<other.bottom() && bottom>i64::from(other.origin.y)
        }) {return Err("This map would overlap another map".into());}
        Ok(())
    }

    pub fn preview_resize(&self,id:u32,width:u32,height:u32,
        direction:ExpandDirection)->Result<(GridPoint,u32,u32),String>{
        let Some(m)=self.maps.iter().find(|map|map.id==id) else {return Err("No map selected".into())};
        let (left,top)=(i64::from(m.origin.x),i64::from(m.origin.y));
        let x=left+if matches!(direction,ExpandDirection::UpperLeft|ExpandDirection::LowerLeft) {
            i64::from(m.width)-i64::from(width)
        } else {0};
        let y=top+if matches!(direction,ExpandDirection::UpperLeft|ExpandDirection::UpperRight) {
            i64::from(m.height)-i64::from(height)
        } else {0};
        let origin=GridPoint::new(
            i32::try_from(x).map_err(|_|"Map would extend beyond canvas coordinates")?,
            i32::try_from(y).map_err(|_|"Map would extend beyond canvas coordinates")?,
        );
        if !self.fits_rect(id,origin,width,height) {
            return Err("That size would overlap a map or exceed editor limits".into());
        }
        // Prevent a shrinking preview from destroying painted tiles or events.
        for yy in 0..m.height {for xx in 0..m.width {
            let nx=left+i64::from(xx)-x;
            let ny=top+i64::from(yy)-y;
            if (nx<0||ny<0||nx>=i64::from(width)||ny>=i64::from(height)) &&
                m.cell(xx,yy).is_some() {
                return Err("This size would cut off painted tiles".into());
            }
        }}
        for (xx,yy) in m.events.iter().map(|event|(event.x,event.y))
            .chain(m.markers.iter().map(|marker|(marker.x,marker.y))) {
            let nx=left+i64::from(xx)-x;
            let ny=top+i64::from(yy)-y;
            if nx<0||ny<0||nx>=i64::from(width)||ny>=i64::from(height) {
                return Err("This size would cut off an event".into());
            }
        }
        Ok((origin,width,height))
    }

    pub fn move_frame(&mut self, id: u32, origin: GridPoint) -> bool {
        let Some(index) = self.maps.iter().position(|m| m.id == id) else { return false; };
        let (width, height) = (self.maps[index].width, self.maps[index].height);
        if !self.fits_rect(id, origin, width, height) { return false; }
        self.maps[index].origin = origin;
        true
    }

    pub fn resize_frame(&mut self, id: u32, width: u32, height: u32) -> bool {
        self.resize_frame_towards(id,width,height,ExpandDirection::LowerRight).is_ok()
    }

    /// Resize towards a chosen corner while retaining every occupied cell/event in
    /// world coordinates. Reject truncation and overlap rather than erasing work.
    pub fn resize_frame_towards(
        &mut self,id:u32,width:u32,height:u32,direction:ExpandDirection,
    )->Result<(),String> {
        let index=self.maps.iter().position(|m|m.id==id).ok_or("Map not found")?;
        let m=&self.maps[index];
        let (old_x,old_y)=(i64::from(m.origin.x),i64::from(m.origin.y));
        let new_x=old_x+if matches!(direction,ExpandDirection::UpperLeft|ExpandDirection::LowerLeft){
            i64::from(m.width)-i64::from(width)
        } else {0};
        let new_y=old_y+if matches!(direction,ExpandDirection::UpperLeft|ExpandDirection::UpperRight){
            i64::from(m.height)-i64::from(height)
        } else {0};
        let origin=GridPoint::new(
            i32::try_from(new_x).map_err(|_|"Map extends beyond world coordinates")?,
            i32::try_from(new_y).map_err(|_|"Map extends beyond world coordinates")?,
        );
        if !self.fits_rect(id,origin,width,height){
            return Err("Map size overlaps a neighbor or exceeds editor limits".into());
        }
        let old=&self.maps[index];
        let mut cells=vec![None;(width as usize)*(height as usize)];
        for y in 0..old.height {
            for x in 0..old.width {
                if let Some(cell)=old.cell(x,y) {
                    let new_lx=old_x+i64::from(x)-new_x;
                    let new_ly=old_y+i64::from(y)-new_y;
                    if new_lx<0 || new_ly<0 || new_lx>=i64::from(width) || new_ly>=i64::from(height) {
                        return Err("Resize would discard painted tiles; erase them first".into());
                    }
                    cells[(new_ly as usize)*(width as usize)+(new_lx as usize)]=Some(cell);
                }
            }
        }
        let transform=|x:u32,y:u32|->Result<(u32,u32),String>{
            let nx=old_x+i64::from(x)-new_x;
            let ny=old_y+i64::from(y)-new_y;
            if nx<0 || ny<0 || nx>=i64::from(width) || ny>=i64::from(height) {
                Err("Resize would discard an event; move it first".into())
            } else {Ok((nx as u32,ny as u32))}
        };
        let mut markers=old.markers.clone();
        for marker in &mut markers {
            (marker.x,marker.y)=transform(marker.x,marker.y)?;
        }
        let mut events=old.events.clone();
        for event in &mut events {
            (event.x,event.y)=transform(event.x,event.y)?;
        }
        let updated=&mut self.maps[index];
        updated.origin=origin;
        updated.width=width;
        updated.height=height;
        updated.cells=cells;
        updated.markers=markers;
        updated.events=events;
        Ok(())
    }

    fn fits_rect(&self, id: u32, origin: GridPoint, width: u32, height: u32) -> bool {
        if width == 0 || height == 0 || (width as u64) * (height as u64) > 100_000 { return false; }
        let (l, t) = (i64::from(origin.x), i64::from(origin.y));
        let (r, b) = (l + i64::from(width), t + i64::from(height));
        self.maps.iter().filter(|m| m.id != id).all(|m|
            l >= m.right() || r <= i64::from(m.origin.x)
                || t >= m.bottom() || b <= i64::from(m.origin.y))
    }

    pub fn connections(&self) -> Vec<Connection> {
        let mut edges = Vec::new();
        for a in &self.maps {
            for b in &self.maps {
                if a.id == b.id { continue; }
                let vspan = (a.bottom().min(b.bottom())
                    - i64::from(a.origin.y).max(i64::from(b.origin.y))).max(0) as u32;
                let hspan = (a.right().min(b.right())
                    - i64::from(a.origin.x).max(i64::from(b.origin.x))).max(0) as u32;
                if vspan > 0 && a.right() == i64::from(b.origin.x) {
                    edges.push(Connection { from:a.id, to:b.id, direction:Direction::Right,
                        offset:b.origin.y - a.origin.y, common_tiles:vspan });
                } else if vspan > 0 && i64::from(a.origin.x) == b.right() {
                    edges.push(Connection { from:a.id, to:b.id, direction:Direction::Left,
                        offset:b.origin.y - a.origin.y, common_tiles:vspan });
                } else if hspan > 0 && a.bottom() == i64::from(b.origin.y) {
                    edges.push(Connection { from:a.id, to:b.id, direction:Direction::Down,
                        offset:b.origin.x - a.origin.x, common_tiles:hspan });
                } else if hspan > 0 && i64::from(a.origin.y) == b.bottom() {
                    edges.push(Connection { from:a.id, to:b.id, direction:Direction::Up,
                        offset:b.origin.x - a.origin.x, common_tiles:hspan });
                }
            }
        }
        edges
    }
}

/// Direction new terrain grows from the opposite fixed corner.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExpandDirection { UpperRight, LowerRight, UpperLeft, LowerLeft }

impl ExpandDirection {
    pub const ALL:[Self;4]=[
        Self::UpperLeft, Self::UpperRight, Self::LowerLeft, Self::LowerRight
    ];
    pub const fn label(self)->&'static str {
        match self {
            Self::UpperRight=>"Upper right", Self::LowerRight=>"Lower right",
            Self::UpperLeft=>"Upper left", Self::LowerLeft=>"Lower left",
        }
    }
    /// The tapped cell is the fixed corner; includes it in the new rectangle.
    pub fn origin_at(self, fixed:GridPoint, width:u32, height:u32)->Result<GridPoint,String> {
        if width==0 || height==0 {return Err("Map dimensions must be positive".into());}
        let dx=i64::from(width)-1;
        let dy=i64::from(height)-1;
        let x=i64::from(fixed.x)-if matches!(self,Self::UpperLeft|Self::LowerLeft){dx}else{0};
        let y=i64::from(fixed.y)-if matches!(self,Self::UpperLeft|Self::UpperRight){dy}else{0};
        Ok(GridPoint::new(
            i32::try_from(x).map_err(|_|"Map origin would overflow")?,
            i32::try_from(y).map_err(|_|"Map origin would overflow")?,
        ))
    }
}

impl Default for World { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn touching_is_allowed_but_overlap_is_not() {
        let mut w=World::new();
        w.add_map("A",GridPoint::new(0,0),10,10).unwrap();
        w.add_map("B",GridPoint::new(10,0),5,5).unwrap();
        assert!(w.add_map("bad",GridPoint::new(9,4),4,4).is_err());
        assert!(w.add_map("corner",GridPoint::new(15,5),4,4).is_ok());
    }

    #[test]
    fn adjacent_half_maps_have_distinct_connections_and_reverses() {
        let mut w=World::new();
        let a=w.add_map("A",GridPoint::new(0,0),10,10).unwrap();
        let b=w.add_map("B",GridPoint::new(10,0),5,5).unwrap();
        let c=w.add_map("C",GridPoint::new(10,5),5,5).unwrap();
        let v=w.connections();
        assert_eq!(v.len(),6);
        assert!(v.contains(&Connection { from:a,to:b,direction:Direction::Right,offset:0,common_tiles:5 }));
        assert!(v.contains(&Connection { from:a,to:c,direction:Direction::Right,offset:5,common_tiles:5 }));
        assert!(v.contains(&Connection { from:c,to:a,direction:Direction::Left,offset:-5,common_tiles:5 }));
        assert!(v.contains(&Connection { from:b,to:c,direction:Direction::Down,offset:0,common_tiles:5 }));
    }

    #[test]
    fn packed_emerald_cell_has_the_expected_bit_fields() {
        let c=Cell { terrain:TileKind::Grass, metatile_id:0x155, collision:1, elevation:3 };
        assert_eq!(c.pack(c.metatile_id),0x3555);
    }

    #[test]
    fn painting_changes_only_targeted_map_cells() {
        let mut w=World::sample();
        assert!(w.paint(GridPoint::new(12,5),TileKind::Water));
        assert_eq!(w.maps[2].cell(0,0).unwrap().terrain,TileKind::Water);
        assert!(w.maps[0].cells.iter().all(Option::is_none));
        assert!(!w.paint(GridPoint::new(-1,10),TileKind::Grass));
    }
}

#[cfg(test)]
mod more_tests {
    use super::*;
    #[test]
    fn move_and_resize_preserve_cells_without_overlaps() {
        let mut w=World::sample();
        assert!(w.paint_metatile(GridPoint::new(1,1),0x155));
        assert!(!w.move_frame(2, GridPoint::new(10,0)));
        assert!(w.move_frame(2, GridPoint::new(25,0)));
        assert!(w.resize_frame(1, 11, 9));
        assert_eq!(w.maps[0].cell(1,1).unwrap().metatile_id,0x155);
        assert!(w.maps[0].cell(11,1).is_none());
    }
    #[test]
    fn serialize_is_lossless_for_numeric_tiles() {
        let mut w=World::sample();
        w.paint_metatile(GridPoint::new(2,1),0x170);
        let b=serde_json::to_vec(&w).unwrap();
        let restored:World=serde_json::from_slice(&b).unwrap();
        assert_eq!(w,restored);
    }
    #[test]
    fn rejects_overflow_metatile_and_outside_paint() {
        let mut w=World::sample();
        assert!(!w.paint_metatile(GridPoint::new(2,2),1024));
        assert!(!w.paint_metatile(GridPoint::new(-200,0),1));
        assert!(w.paint_metatile(GridPoint::new(2,2),1));
    }
}

#[cfg(test)]
mod event_tests {
    use super::*;
    #[test]
    fn event_position_remains_map_local() {
        let mut w=World::sample();
        let (id,index)=w.add_event(GridPoint::new(13,1),EventKind::Warp).unwrap();
        assert_eq!(id,2);
        assert_eq!(w.maps[1].events[index].x,1);
        assert_eq!(w.maps[1].events[index].y,1);
        assert_eq!(w.maps[1].events[index].dest_map_id,1);
        assert!(w.remove_event(id,index));
    }
    #[test]
    fn event_survives_world_serialization_and_move() {
        let mut w=World::sample();
        w.add_event(GridPoint::new(1,1),EventKind::Npc).unwrap();
        assert!(w.move_frame(1,GridPoint::new(-12,0)));
        let restored:World=serde_json::from_str(&serde_json::to_string(&w).unwrap()).unwrap();
        assert_eq!(restored.maps[0].events[0].x,1);
        assert_eq!(restored.maps[0].events[0].dialogue,"Hello, traveler!");
    }
}

#[cfg(test)]
mod integrity_tests {
    use super::*;
    #[test]
    fn detects_corrupt_map_lengths() {
        let mut w=World::sample();
        w.maps[0].cells.pop();
        assert!(w.check_integrity().unwrap_err().contains("dimensions"));
    }
    #[test]
    fn detects_overlaps_and_duplicate_ids() {
        let mut w=World::sample();
        w.maps[1].origin=GridPoint::new(1,1);
        assert!(w.check_integrity().unwrap_err().contains("overlap"));
        w=World::sample();
        w.maps[1].id=w.maps[0].id;
        assert!(w.check_integrity().unwrap_err().contains("duplicate"));
    }
    #[test]
    fn normal_editable_sample_is_valid() {
        World::sample().check_integrity().unwrap();
    }
}

#[cfg(test)]
mod remove_map_tests {
    use super::*;
    #[test]
    fn deletion_refuses_to_break_warp_destinations() {
        let mut world=World::sample();
        world.add_event(GridPoint::new(13,1),EventKind::Warp);
        assert!(world.remove_frame(1).unwrap_err().contains("warp destination"));
        assert_eq!(world.maps.len(),3);
        world.remove_frame(2).unwrap();
        assert_eq!(world.maps.len(),2);
        world.check_integrity().unwrap();
    }
}

#[cfg(test)]
mod anchored_resize_tests {
    use super::*;
    #[test]
    fn four_corner_expansion_preserves_absolute_tile_positions() {
        for (dir,x,y) in [
            (ExpandDirection::UpperRight,10,8),
            (ExpandDirection::LowerRight,10,10),
            (ExpandDirection::UpperLeft,8,8),
            (ExpandDirection::LowerLeft,8,10),
        ] {
            let mut w=World::new();
            let id=w.add_map("A",GridPoint::new(10,10),4,4).unwrap();
            w.paint_metatile(GridPoint::new(11,11),0x170);
            let (mid,index)=w.add_event(GridPoint::new(12,12),EventKind::Npc).unwrap();
            assert_eq!(mid,id);
            w.resize_frame_towards(id,6,6,dir).unwrap();
            assert_eq!(w.maps[0].origin,GridPoint::new(x,y));
            let map=&w.maps[0];
            assert_eq!(map.cell((11-x) as u32,(11-y) as u32).unwrap().metatile_id,0x170);
            assert_eq!((map.events[index].x,map.events[index].y),
                ((12-x) as u32,(12-y) as u32));
            w.check_integrity().unwrap();
        }
    }
    #[test]
    fn reject_overlap_and_destructive_shrink() {
        let mut w=World::new();
        let a=w.add_map("A",GridPoint::new(0,0),5,5).unwrap();
        w.add_map("B",GridPoint::new(5,0),5,5).unwrap();
        assert!(w.resize_frame_towards(a,6,5,ExpandDirection::LowerRight).is_err());
        w.paint_metatile(GridPoint::new(0,0),1);
        assert!(w.resize_frame_towards(a,4,4,ExpandDirection::UpperLeft).is_err());
        assert_eq!(w.maps[0].width,5);
        assert_eq!(w.maps[0].origin,GridPoint::new(0,0));
    }
    #[test]
    fn fixed_tap_corner_origin_is_inclusive() {
        let p=GridPoint::new(20,30);
        assert_eq!(ExpandDirection::UpperLeft.origin_at(p,3,4).unwrap(),GridPoint::new(18,27));
        assert_eq!(ExpandDirection::LowerRight.origin_at(p,3,4).unwrap(),p);
        assert_eq!(ExpandDirection::UpperRight.origin_at(p,3,4).unwrap(),GridPoint::new(20,27));
        assert_eq!(ExpandDirection::LowerLeft.origin_at(p,3,4).unwrap(),GridPoint::new(18,30));
    }
}

#[cfg(test)]
mod property_bit_tests {
    use super::*;
    #[test]
    fn collision_and_elevation_edit_real_packed_cell_bits() {
        let mut world=World::new();
        world.add_map("A",GridPoint::new(0,0),5,5).unwrap();
        let p=GridPoint::new(2,2);
        assert!(!world.set_collision(p,1)); // Blank cells are not native map cells.
        world.paint_metatile(p,0x0170);
        for c in 0..4 {
            assert!(world.set_collision(p,c) || c==0);
            for e in [0u8,1u8,3u8,15u8] {
                world.set_elevation(p,e);
                let packed=world.maps[0].cell(2,2).unwrap().pack(0x0170);
                assert_eq!((packed>>10)&3,u16::from(c));
                assert_eq!((packed>>12)&15,u16::from(e));
            }
        }
        world.paint_metatile(p,0x0001);
        assert_eq!(world.maps[0].cell(2,2).unwrap().collision,3);
        assert_eq!(world.maps[0].cell(2,2).unwrap().elevation,15);
    }
}

#[cfg(test)]
mod touch_geometry_tests {
    use super::*;
    #[test]
    fn overlap_probe_warns_without_modifying_world() {
        let mut w=World::new();
        w.add_map("A",GridPoint::new(0,0),10,10).unwrap();
        let before=w.clone();
        assert!(w.can_place_frame(GridPoint::new(5,5),6,6).is_err());
        assert!(w.can_place_frame(GridPoint::new(10,0),6,6).is_ok());
        assert_eq!(before,w);
    }
    #[test]
    fn preview_resize_is_nondestructive_and_warns_about_neighbors() {
        let mut w=World::new();
        let id=w.add_map("A",GridPoint::new(0,0),10,10).unwrap();
        w.add_map("B",GridPoint::new(10,0),6,5).unwrap();
        assert!(w.preview_resize(id,11,10,ExpandDirection::LowerRight).is_err());
        let before=w.clone();
        assert_eq!(w.preview_resize(id,12,10,ExpandDirection::LowerLeft).unwrap().0,
            GridPoint::new(-2,0));
        assert_eq!(before,w);
    }
}
