//! Engine-independent map geometry and provisional editor cells.
//! The visual tile kinds are placeholders until the real Emerald tilesets are decoded.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GridPoint {
    pub x: i32,
    pub y: i32,
}

impl GridPoint {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub terrain: TileKind,
    pub collision: u8,
    pub elevation: u8,
}

impl Cell {
    pub const fn new(terrain: TileKind) -> Self {
        Self {
            terrain,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Marker {
    pub x: u32,
    pub y: u32,
}

#[derive(Clone, Debug)]
pub struct MapFrame {
    pub id: u32,
    pub name: String,
    pub origin: GridPoint,
    pub width: u32,
    pub height: u32,
    pub cells: Vec<Option<Cell>>,
    pub markers: Vec<Marker>,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction { Up, Down, Left, Right }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Connection {
    pub from: u32,
    pub to: u32,
    pub direction: Direction,
    /// Other frame's origin minus source origin along the connected edge.
    pub offset: i32,
    pub common_tiles: u32,
}

#[derive(Debug)]
pub struct World {
    pub maps: Vec<MapFrame>,
    next_id: u32,
}

impl World {
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
        let c=Cell { terrain:TileKind::Grass, collision:1, elevation:3 };
        assert_eq!(c.pack(0x155),0x3555);
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
