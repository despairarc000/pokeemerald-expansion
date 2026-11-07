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
pub enum EventKind { Npc, Sign, Warp, Trigger, HiddenItem }

/// Linked warps use a destination map's numbered warp event.
/// Direct travel is implemented as a script-triggered coordinate warp.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum WarpMode {
    #[default] Linked,
    Direct,
}

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
    pub warp_mode: WarpMode,
    #[serde(default)]
    pub arrival_only: bool,
    #[serde(default)]
    pub dest_x: u32,
    #[serde(default)]
    pub dest_y: u32,
    #[serde(default)]
    pub hidden_item: String,
    #[serde(default)]
    pub hidden_flag: String,
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
            warp_mode: WarpMode::Linked,
            arrival_only: false,
            dest_x: 0,
            dest_y: 0,
            hidden_item: String::new(),
            hidden_flag: String::new(),
            script: String::new(),
            graphics_id: default_graphics_id(),
            var: default_trigger_var(),
            var_value: 0,
        }
    }
}

/// Editor-only grouping; Emerald map coordinates always remain map-local.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpaceKind { Surface, Interior, Cave, Other }
impl SpaceKind {
    pub const ALL:[Self;4]=[Self::Surface,Self::Interior,Self::Cave,Self::Other];
    pub fn label(self)->&'static str {
        match self {Self::Surface=>"Surface",Self::Interior=>"Interior",
            Self::Cave=>"Cave floor",Self::Other=>"Other"}
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorSpace {
    pub id:u32,
    pub name:String,
    pub kind:SpaceKind,
    /// A visual association, not an Emerald game connection.
    pub parent_map_id:Option<u32>,
    pub anchor:GridPoint,
}
fn default_spaces()->Vec<EditorSpace>{
    vec![EditorSpace{id:0,name:"Surface".into(),kind:SpaceKind::Surface,
        parent_map_id:None,anchor:GridPoint::new(0,0)}]
}
fn default_next_space_id()->u32{1}
fn default_tileset_primary()->String{"general".into()}
fn default_tileset_secondary()->String{"petalburg".into()}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapFrame {
    #[serde(default)]
    pub space_id:u32,
    #[serde(default="default_tileset_primary")]
    pub primary_tileset:String,
    #[serde(default="default_tileset_secondary")]
    pub secondary_tileset:String,
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
    #[serde(default="default_spaces")]
    pub spaces:Vec<EditorSpace>,
    #[serde(default)]
    pub active_space_id:u32,
    #[serde(default="default_next_space_id")]
    next_space_id:u32,
}

impl World {
    /// Validate all invariants after deserializing an untrusted project.
    /// Editor projects permit empty cells; game export applies stricter checks.
    pub fn check_integrity(&self) -> Result<(),String> {
        if self.maps.len()>256 {return Err("Project has too many map frames".into());}
        if self.spaces.is_empty() || self.spaces[0].id!=0 {
            return Err("Project is missing the Surface editing space".into());
        }
        let mut space_ids=std::collections::HashSet::new();
        let mut largest_space_id=0;
        for sp in &self.spaces {
            if !space_ids.insert(sp.id) || sp.name.trim().is_empty() || sp.name.len()>90 {
                return Err("Invalid or duplicate editing space".into());
            }
            largest_space_id=largest_space_id.max(sp.id);
            if let Some(parent)=sp.parent_map_id {
                let map=self.maps.iter().find(|m|m.id==parent)
                    .ok_or("A Space refers to a missing parent map")?;
                if map.space_id==sp.id {return Err("A Space cannot parent itself".into());}
            }
        }
        if self.next_space_id<=largest_space_id || !space_ids.contains(&self.active_space_id) {
            return Err("Editing Space selector is inconsistent".into());
        }
        let mut ids=std::collections::HashSet::new();
        let mut max_id=0;
        for map in &self.maps {
            if map.id==0 || !ids.insert(map.id) {
                return Err("Project contains duplicate or zero map IDs".into());
            }
            max_id=max_id.max(map.id);
            if !space_ids.contains(&map.space_id) {
                return Err(format!("Map {} belongs to a missing Space",map.name));
            }
            let count=(map.width as u64)*(map.height as u64);
            if map.width==0 || map.height==0 || count>100_000
                || count!=(map.cells.len() as u64) {
                return Err(format!("Invalid dimensions for {}",map.name));
            }
            if map.name.len()>128 {
                return Err("Map name is too long".into());
            }
            // Legacy projects may contain unused metatile IDs from older
            // palettes. Keep their data loadable and reject them at export,
            // where the selected tileset is fully validated.
            for cell in map.cells.iter().flatten() {
                if cell.metatile_id>=1024 || cell.collision>3 || cell.elevation>15 {
                    return Err(format!("Invalid native cell value in {}",map.name));
                }
            }
            if map.markers.iter().any(|e|e.x>=map.width || e.y>=map.height)
                || map.events.iter().any(|e|e.x>=map.width || e.y>=map.height ||
                    e.elevation>15 || e.dest_warp_id>127) {
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
                if a.space_id!=b.space_id {continue;}
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

    pub fn new() -> Self {
        Self { maps:Vec::new(),next_id:1,spaces:default_spaces(),
            active_space_id:0,next_space_id:1 }
    }

    pub fn active_space(&self)->&EditorSpace{
        self.spaces.iter().find(|s|s.id==self.active_space_id)
            .expect("validated active Space")
    }
    pub fn set_active_space(&mut self,id:u32)->Result<(),String>{
        if !self.spaces.iter().any(|s|s.id==id){
            return Err("Editing Space not found".into());
        }
        self.active_space_id=id;
        Ok(())
    }
    pub fn create_space(&mut self,kind:SpaceKind,
        parent_map_id:Option<u32>,anchor:GridPoint)->Result<u32,String>{
        if self.spaces.len()>=128{return Err("Too many editing Spaces".into());}
        if let Some(id)=parent_map_id {
            if !self.maps.iter().any(|m|m.id==id) {
                return Err("Parent map no longer exists".into());
            }
        }
        let id=self.next_space_id;
        self.next_space_id=self.next_space_id.checked_add(1)
            .ok_or("Too many editing spaces")?;
        self.spaces.push(EditorSpace{
            id,
            name:format!("{} {id}",kind.label()),
            kind,parent_map_id,anchor,
        });
        self.active_space_id=id;
        Ok(id)
    }
    pub fn set_map_tileset(&mut self,id:u32,key:&str)->Result<(),String>{
        let tileset=crate::tiles::pair(key).ok_or("Tileset not available")?;
        let m=self.maps.iter_mut().find(|m|m.id==id).ok_or("Map not found")?;
        if m.cells.iter().flatten().any(|cell|!crate::tiles::valid_in(tileset,cell.metatile_id)){
            return Err("The new tileset lacks a tile already painted on this map".into());
        }
        m.primary_tileset=tileset.primary.into();
        m.secondary_tileset=tileset.secondary.into();
        Ok(())
    }

    pub fn active_maps(&self)->impl Iterator<Item=&MapFrame> {
        self.maps.iter().filter(|m|m.space_id==self.active_space_id)
    }
    pub fn space_for_map(&self,id:u32)->Option<u32>{
        self.maps.iter().find(|m|m.id==id).map(|m|m.space_id)
    }


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
        if self.maps.iter().any(|m| m.space_id==self.active_space_id &&
            left < m.right() && right > i64::from(m.origin.x)
            && top < m.bottom() && bottom > i64::from(m.origin.y)) {
            return Err("Map frames cannot overlap");
        }
        let id = self.next_id;
        self.next_id = self.next_id.checked_add(1).ok_or("Too many maps")?;
        let (primary_tileset,secondary_tileset)=match self.active_space().kind {
            SpaceKind::Interior=>("building".into(),"generic_building".into()),
            SpaceKind::Cave=>("general".into(),"cave".into()),
            _=>("general".into(),"petalburg".into()),
        };
        self.maps.push(MapFrame {
            id, name: name.into(), origin, width, height,
            space_id:self.active_space_id,primary_tileset,secondary_tileset,
            cells: vec![None; (width as usize) * (height as usize)],
            markers: Vec::new(),
            events: Vec::new(),
        });
        Ok(id)
    }

    pub fn map_at(&self, p: GridPoint) -> Option<&MapFrame> {
        self.maps.iter().find(|m| m.space_id==self.active_space_id && m.contains(p))
    }

    pub fn paint(&mut self, p: GridPoint, terrain: TileKind) -> bool {
        if let Some(map) = self.maps.iter_mut().find(|m| m.space_id==self.active_space_id && m.contains(p)) {
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
        if let Some(map) = self.maps.iter_mut().find(|m| m.space_id==self.active_space_id && m.contains(p)) {
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


    pub fn assign_hidden_slot(&mut self,map_id:u32,index:usize)->Result<(),String>{
        let used:Vec<String>=self.maps.iter().flat_map(|m|m.events.iter())
            .filter(|e|e.kind==EventKind::HiddenItem && !e.hidden_flag.is_empty())
            .map(|e|e.hidden_flag.clone()).collect();
        let slot=crate::choices::free_pickup_slot(used.iter().map(String::as_str))
            .ok_or("No free hidden-item pickup slots (16 available)")?;
        let e=self.event_mut(map_id,index).ok_or("Hidden item not found")?;
        if e.kind!=EventKind::HiddenItem {return Err("Not a hidden item".into());}
        if e.hidden_flag.is_empty(){e.hidden_flag=slot.into();}
        Ok(())
    }

    pub fn add_event(&mut self, p: GridPoint, kind: EventKind) -> Option<(u32,usize)> {
        if let Some(map)=self.maps.iter_mut().find(|m|
            m.space_id==self.active_space_id && m.contains(p)) {
            let x=(p.x-map.origin.x) as u32;
            let y=(p.y-map.origin.y) as u32;
            if let Some(index)=map.events.iter().position(|e|e.x==x&&e.y==y) {
                return Some((map.id,index));
            }
            let mut event=MapEvent::new(kind,x,y);
            if kind==EventKind::Warp { event.elevation=0; }
            map.events.push(event);
            Some((map.id,map.events.len()-1))
        } else { None }
    }

    pub fn event_at(&self,p:GridPoint)->Option<(u32,usize)> {
        let map=self.map_at(p)?;
        let x=(i64::from(p.x)-i64::from(map.origin.x)) as u32;
        let y=(i64::from(p.y)-i64::from(map.origin.y)) as u32;
        map.events.iter().position(|e|e.x==x && e.y==y).map(|i|(map.id,i))
    }

    pub fn event(&self,map_id:u32,index:usize)->Option<&MapEvent>{
        self.maps.iter().find(|m|m.id==map_id)?.events.get(index)
    }

    pub fn event_mut(&mut self,map_id:u32,index:usize)->Option<&mut MapEvent>{
        self.maps.iter_mut().find(|m|m.id==map_id)?.events.get_mut(index)
    }

    /// Engine IDs enumerate *native* warp events in map order. Direct/scripted
    /// travel triggers don't consume warp IDs.
    pub fn warp_id(&self,map_id:u32,event_index:usize)->Option<u8>{
        let m=self.maps.iter().find(|m|m.id==map_id)?;
        let e=m.events.get(event_index)?;
        if e.kind!=EventKind::Warp || e.warp_mode==WarpMode::Direct {return None;}
        let order=m.events[..event_index].iter().filter(|e|
            e.kind==EventKind::Warp && e.warp_mode!=WarpMode::Direct).count();
        u8::try_from(order).ok()
    }

    pub fn move_event(&mut self,map_id:u32,event_index:usize,p:GridPoint)->Result<(),String>{
        let m=self.maps.iter_mut().find(|m|m.id==map_id).ok_or("Map not found")?;
        if !m.contains(p){return Err("Keep this event inside its map".into());}
        if event_index>=m.events.len(){return Err("Event no longer exists".into());}
        let x=(i64::from(p.x)-i64::from(m.origin.x)) as u32;
        let y=(i64::from(p.y)-i64::from(m.origin.y)) as u32;
        if m.events.iter().enumerate().any(|(i,e)|i!=event_index && e.x==x && e.y==y){
            return Err("This tile already contains an event".into());
        }
        m.events[event_index].x=x;
        m.events[event_index].y=y;
        Ok(())
    }

    /// Point a source warp at an existing native destination marker.
    /// Optional reverse linking converts arrival-only markers into active return warps.
    pub fn link_warps(&mut self,source:(u32,usize),destination:(u32,usize),
        create_return:bool)->Result<(),String>{
        if source==destination {return Err("Choose a different destination warp".into());}
        let target=self.event(destination.0,destination.1).ok_or("Destination event missing")?;
        if target.kind!=EventKind::Warp || target.warp_mode==WarpMode::Direct {
            return Err("Select a numbered destination warp".into());
        }
        let source_event=self.event(source.0,source.1).ok_or("Source event missing")?;
        if source_event.kind!=EventKind::Warp || source_event.arrival_only {
            return Err("Choose an editable source warp".into());
        }
        let is_direct=source_event.warp_mode==WarpMode::Direct;
        let source_id=if is_direct {
            let map=self.maps.iter().find(|m|m.id==source.0).ok_or("Source map missing")?;
            u8::try_from(map.events[..source.1].iter().filter(|e|
                e.kind==EventKind::Warp && e.warp_mode!=WarpMode::Direct).count())
                .map_err(|_|"Too many warp IDs")?
        }else{self.warp_id(source.0,source.1).ok_or("Source warp has no ID")?};
        let mut target_id=self.warp_id(destination.0,destination.1)
            .ok_or("Destination has no native warp ID")?;
        if is_direct {
            // Inserting a new native warp shifts all following native IDs.
            for map in &mut self.maps {for e in &mut map.events {
                if e.kind==EventKind::Warp && e.warp_mode==WarpMode::Linked
                    && e.dest_map_id==source.0 && e.dest_warp_id>=source_id {
                    e.dest_warp_id=e.dest_warp_id.checked_add(1)
                        .ok_or("Warp index limit reached")?;
                }
            }}
            if destination.0==source.0 && target_id>=source_id {
                target_id=target_id.checked_add(1).ok_or("Warp index limit reached")?;
            }
        }
        let src=self.event_mut(source.0,source.1).ok_or("Source warp missing")?;
        src.warp_mode=WarpMode::Linked;
        src.arrival_only=false;
        src.dest_map_id=destination.0;
        src.dest_warp_id=target_id;
        if create_return {
            let dest=self.event_mut(destination.0,destination.1)
                .ok_or("Destination warp missing")?;
            dest.warp_mode=WarpMode::Linked;
            dest.arrival_only=false;
            dest.dest_map_id=source.0;
            dest.dest_warp_id=source_id;
        }
        Ok(())
    }

    /// Creates an arrival-only native warp at a specific painted tile, then
    /// links the source. The marker has no playable return unless configured.
    pub fn create_arrival_marker(&mut self,source:(u32,usize),p:GridPoint,
        create_return:bool)->Result<(u32,usize),String>{
        if self.event(source.0,source.1).map(|e|e.kind)!=Some(EventKind::Warp) {
            return Err("Select a warp source".into());
        }
        if self.event_at(p).is_some(){return Err("Choose an empty destination tile".into());}
        let map_id=self.map_at(p).ok_or("Choose a destination map")?.id;
        if map_id==source.0 {return Err("Choose another map for the arrival marker".into());}
        // The entry must not be on a tile whose behavior immediately warps again.
        if !self.safe_arrival_at(p) {
            return Err("Choose a painted, passable arrival tile without a warp behavior".into());
        }
        let (target_map,target_index)=self.add_event(p,EventKind::Warp)
            .ok_or("Cannot add destination marker")?;
        let target_id=self.warp_id(target_map,target_index).ok_or("Too many warps")?;
        let target=self.event_mut(target_map,target_index).ok_or("Marker missing")?;
        target.arrival_only=true;
        target.dest_map_id=target_map;
        target.dest_warp_id=target_id; // valid inert native warp; validated non-warp metatile
        self.link_warps(source,(target_map,target_index),create_return)?;
        Ok((target_map,target_index))
    }

    pub fn safe_arrival_at(&self,p:GridPoint)->bool {
        let Some(map)=self.map_at(p) else {return false;};
        let x=(i64::from(p.x)-i64::from(map.origin.x)) as u32;
        let y=(i64::from(p.y)-i64::from(map.origin.y)) as u32;
        let Some(cell)=map.cell(x,y) else {return false;};
        crate::tiles::pair_for(&map.primary_tileset,&map.secondary_tileset)
            .map(|tileset|cell.collision==0 &&
                crate::tiles::valid_in(tileset,cell.metatile_id) &&
                !crate::tiles::is_warp_in(tileset,cell.metatile_id))
            .unwrap_or(false)
    }

    pub fn set_direct_arrival(&mut self,source:(u32,usize),p:GridPoint)->Result<(),String>{
        if !self.safe_arrival_at(p){return Err("Pick a painted, passable arrival tile".into());}
        let target=self.map_at(p).ok_or("Arrival map missing")?;
        let map_id=target.id;
        let x=(i64::from(p.x)-i64::from(target.origin.x)) as u32;
        let y=(i64::from(p.y)-i64::from(target.origin.y)) as u32;
        let src=self.event(source.0,source.1).ok_or("Warp source missing")?;
        if src.kind!=EventKind::Warp || src.arrival_only {
            return Err("Choose a configurable warp source".into());
        }
        if src.warp_mode!=WarpMode::Direct {
            let native_id=self.warp_id(source.0,source.1)
                .ok_or("Source has no native warp ID")?;
            if self.maps.iter().any(|m|m.events.iter().enumerate().any(|(i,e)|
                (m.id!=source.0 || i!=source.1) &&
                e.kind==EventKind::Warp && e.warp_mode==WarpMode::Linked &&
                !e.arrival_only && e.dest_map_id==source.0 && e.dest_warp_id==native_id
            )) {
                return Err("Other warps arrive here; unlink them first".into());
            }
            for m in &mut self.maps {for e in &mut m.events {
                if e.kind==EventKind::Warp && e.warp_mode==WarpMode::Linked &&
                    e.dest_map_id==source.0 && e.dest_warp_id>native_id {
                    e.dest_warp_id-=1;
                }
            }}
        }
        let e=self.event_mut(source.0,source.1).ok_or("Source disappeared")?;
        e.warp_mode=WarpMode::Direct;
        e.arrival_only=false;
        e.dest_map_id=map_id;
        e.dest_x=x;
        e.dest_y=y;
        Ok(())
    }

    /// Remove without silently re-targeting other warps after indices shift.
    pub fn remove_event(&mut self,map_id:u32,index:usize)->bool {
        let Some(event)=self.event(map_id,index).cloned() else {return false;};
        let removed_warp_id=self.warp_id(map_id,index);
        if let Some(id)=removed_warp_id {
            if self.maps.iter().any(|m|m.events.iter().enumerate().any(|(i,e)|
                (m.id!=map_id || i!=index) && e.kind==EventKind::Warp &&
                e.warp_mode==WarpMode::Linked && e.dest_map_id==map_id &&
                e.dest_warp_id==id && !e.arrival_only)) {
                return false;
            }
        }
        if let Some(map)=self.maps.iter_mut().find(|m|m.id==map_id){
            map.events.remove(index);
        }
        if event.kind==EventKind::Warp && event.warp_mode!=WarpMode::Direct {
            if let Some(id)=removed_warp_id {
                for map in &mut self.maps {for e in &mut map.events {
                    if e.kind==EventKind::Warp && e.warp_mode==WarpMode::Linked &&
                        e.dest_map_id==map_id && e.dest_warp_id>id {
                        e.dest_warp_id-=1;
                    }
                }}
            }
        }
        true
    }

    pub fn add_marker(&mut self, p: GridPoint) -> bool {
        if let Some(map) = self.maps.iter_mut().find(|m| m.space_id==self.active_space_id && m.contains(p)) {
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
        if let Some(map) = self.maps.iter_mut().find(|m| m.space_id==self.active_space_id && m.contains(p)) {
            let Some(pair)=crate::tiles::pair_for(&map.primary_tileset,&map.secondary_tileset) else{
                return false;
            };
            if !crate::tiles::valid_in(pair,id){return false;}
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
        if let Some(map) = self.maps.iter_mut().find(|m| m.space_id==self.active_space_id && m.contains(p)) {
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
        if let Some(map) = self.maps.iter_mut().find(|m| m.space_id==self.active_space_id && m.contains(p)) {
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
        if self.spaces.iter().any(|s|s.parent_map_id==Some(id)){
            return Err("A nested editing Space is attached to this map".into());
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
            other.space_id==self.active_space_id &&
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
        let old=self.maps[index].origin;
        self.maps[index].origin = origin;
        let dx=i64::from(origin.x)-i64::from(old.x);
        let dy=i64::from(origin.y)-i64::from(old.y);
        for space in &mut self.spaces {
            if space.parent_map_id==Some(id){
                if let (Ok(x),Ok(y))=(i32::try_from(i64::from(space.anchor.x)+dx),
                    i32::try_from(i64::from(space.anchor.y)+dy)){
                    space.anchor=GridPoint::new(x,y);
                }
            }
        }
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
        let space_id=match self.maps.iter().find(|m|m.id==id){
            Some(m)=>m.space_id,None=>return false
        };
        self.maps.iter().filter(|m| m.id != id && m.space_id==space_id).all(|m|
            l >= m.right() || r <= i64::from(m.origin.x)
                || t >= m.bottom() || b <= i64::from(m.origin.y))
    }

    pub fn connections(&self) -> Vec<Connection> {
        let mut edges = Vec::new();
        for a in &self.maps {
            for b in &self.maps {
                if a.id == b.id || a.space_id!=b.space_id { continue; }
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
        assert_eq!(w.maps[1].events[index].dest_map_id,0);
        assert!(w.remove_event(id,index));
    }
    #[test]
    fn event_survives_world_serialization_and_move() {
        let mut w=World::sample();
        w.add_event(GridPoint::new(1,1),EventKind::Npc).unwrap();
        assert!(w.move_frame(1,GridPoint::new(-12,0)));
        let restored:World=serde_json::from_str(&serde_json::to_string(&w).unwrap()).unwrap();
        assert_eq!(restored.maps[0].events[0].x,1);
        assert_eq!(restored.maps[0].events[0].dialogue,"");
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
        let source=world.add_event(GridPoint::new(13,1),EventKind::Warp).unwrap();
        let target=world.add_event(GridPoint::new(1,1),EventKind::Warp).unwrap();
        world.link_warps(source,target,false).unwrap();
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

#[cfg(test)]
mod warp_link_tests {
    use super::*;
    fn painted_world()->World{
        let mut w=World::new();
        w.add_map("Outside",GridPoint::new(0,0),5,5).unwrap();
        w.add_map("Inside",GridPoint::new(10,0),5,5).unwrap();
        for map in &mut w.maps {for cell in &mut map.cells {
            *cell=Some(Cell{terrain:TileKind::Grass,metatile_id:1,collision:0,elevation:3});
        }}
        w
    }
    #[test]
    fn one_way_link_does_not_invent_reverse() {
        let mut w=painted_world();
        let a=w.add_event(GridPoint::new(1,1),EventKind::Warp).unwrap();
        let b=w.add_event(GridPoint::new(11,1),EventKind::Warp).unwrap();
        w.link_warps(a,b,false).unwrap();
        assert_eq!(w.event(a.0,a.1).unwrap().dest_map_id,b.0);
        assert_eq!(w.event(a.0,a.1).unwrap().dest_warp_id,0);
        assert_eq!(w.event(b.0,b.1).unwrap().dest_map_id,0);
    }
    #[test]
    fn optional_return_link_is_explicit() {
        let mut w=painted_world();
        let a=w.add_event(GridPoint::new(1,1),EventKind::Warp).unwrap();
        let b=w.add_event(GridPoint::new(11,1),EventKind::Warp).unwrap();
        w.link_warps(a,b,true).unwrap();
        assert_eq!(w.event(b.0,b.1).unwrap().dest_map_id,a.0);
        assert_eq!(w.event(b.0,b.1).unwrap().dest_warp_id,0);
    }
    #[test]
    fn arrival_marker_uses_real_inert_tile_and_no_reverse() {
        let mut w=painted_world();
        let a=w.add_event(GridPoint::new(1,1),EventKind::Warp).unwrap();
        let b=w.create_arrival_marker(a,GridPoint::new(11,1),false).unwrap();
        assert!(w.event(b.0,b.1).unwrap().arrival_only);
        assert_eq!(w.event(b.0,b.1).unwrap().dest_map_id,b.0);
        assert_eq!(w.event(a.0,a.1).unwrap().dest_map_id,b.0);
    }
    #[test]
    fn direct_arrival_requires_no_destination_warp() {
        let mut w=painted_world();
        let a=w.add_event(GridPoint::new(1,1),EventKind::Warp).unwrap();
        w.set_direct_arrival(a,GridPoint::new(12,2)).unwrap();
        assert_eq!(w.event(a.0,a.1).unwrap().warp_mode,WarpMode::Direct);
        assert_eq!(w.event(a.0,a.1).unwrap().dest_x,2);
        assert_eq!(w.event(a.0,a.1).unwrap().dest_y,2);
        assert!(w.warp_id(a.0,a.1).is_none());
    }
    #[test]
    fn moving_event_rejects_occupied_tile_or_different_map() {
        let mut w=painted_world();
        let a=w.add_event(GridPoint::new(1,1),EventKind::Npc).unwrap();
        w.add_event(GridPoint::new(2,2),EventKind::Sign);
        assert!(w.move_event(a.0,a.1,GridPoint::new(2,2)).is_err());
        assert!(w.move_event(a.0,a.1,GridPoint::new(11,1)).is_err());
        w.move_event(a.0,a.1,GridPoint::new(3,3)).unwrap();
        assert_eq!(w.event(a.0,a.1).unwrap().x,3);
    }
}

#[cfg(test)]
mod warp_conversion_tests {
    use super::*;
    #[test]
    fn direct_warp_can_be_relinked_with_return(){
        let mut w=World::new();
        w.add_map("A",GridPoint::new(0,0),5,5).unwrap();
        w.add_map("B",GridPoint::new(10,0),5,5).unwrap();
        for map in &mut w.maps {map.cells.fill(Some(Cell::new(TileKind::Grass)));}
        let source=w.add_event(GridPoint::new(1,1),EventKind::Warp).unwrap();
        let target=w.add_event(GridPoint::new(11,1),EventKind::Warp).unwrap();
        w.set_direct_arrival(source,GridPoint::new(12,2)).unwrap();
        assert_eq!(w.warp_id(source.0,source.1),None);
        w.link_warps(source,target,true).unwrap();
        assert_eq!(w.warp_id(source.0,source.1),Some(0));
        assert_eq!(w.event(target.0,target.1).unwrap().dest_map_id,source.0);
    }
    #[test]
    fn inbound_warps_prevent_removal_of_target(){
        let mut w=World::new();
        w.add_map("A",GridPoint::new(0,0),5,5).unwrap();
        w.add_map("B",GridPoint::new(10,0),5,5).unwrap();
        let source=w.add_event(GridPoint::new(1,1),EventKind::Warp).unwrap();
        let target=w.add_event(GridPoint::new(11,1),EventKind::Warp).unwrap();
        w.link_warps(source,target,false).unwrap();
        assert!(!w.remove_event(target.0,target.1));
        assert!(w.remove_event(source.0,source.1));
        assert!(w.remove_event(target.0,target.1));
    }
}

#[cfg(test)]
mod editing_space_tests {
    use super::*;
    #[test]
    fn maps_in_independent_spaces_overlap_without_game_connections(){
        let mut world=World::new();
        let outdoors=world.add_map("Town",GridPoint::new(0,0),10,10).unwrap();
        let interior=world.create_space(SpaceKind::Interior,Some(outdoors),
            GridPoint::new(4,5)).unwrap();
        let inside=world.add_map("House",GridPoint::new(0,0),10,10).unwrap();
        assert_ne!(interior,0);
        assert!(world.connections().is_empty());
        world.check_integrity().unwrap();
        assert_eq!(world.map_at(GridPoint::new(1,1)).unwrap().id,inside);
        world.set_active_space(0).unwrap();
        assert_eq!(world.map_at(GridPoint::new(1,1)).unwrap().id,outdoors);
        assert_eq!(world.space_for_map(inside),Some(interior));
    }
    #[test]
    fn moving_parent_preserves_nested_space_anchor(){
        let mut world=World::new();
        let parent=world.add_map("A",GridPoint::new(0,0),5,5).unwrap();
        world.create_space(SpaceKind::Cave,Some(parent),GridPoint::new(2,3)).unwrap();
        world.set_active_space(0).unwrap();
        assert!(world.move_frame(parent,GridPoint::new(10,20)));
        assert_eq!(world.spaces[1].anchor,GridPoint::new(12,23));
        assert!(world.remove_frame(parent).is_err());
    }
    #[test]
    fn old_world_json_migrates_to_single_surface_space(){
        let mut w=World::sample();
        let mut j=serde_json::to_value(&w).unwrap();
        let obj=j.as_object_mut().unwrap();
        obj.remove("spaces");obj.remove("active_space_id");obj.remove("next_space_id");
        for map in obj.get_mut("maps").unwrap().as_array_mut().unwrap(){
            let data=map.as_object_mut().unwrap();
            data.remove("space_id");data.remove("primary_tileset");data.remove("secondary_tileset");
        }
        let restored:World=serde_json::from_value(j).unwrap();
        restored.check_integrity().unwrap();
        assert_eq!(restored.spaces.len(),1);
        assert!(restored.maps.iter().all(|map|map.space_id==0));
    }
}

#[cfg(test)]
mod space_tileset_tests {
    use super::*;
    #[test]
    fn tileset_is_per_map_not_per_world(){
        let mut w=World::new();
        let outside=w.add_map("Town",GridPoint::new(0,0),4,4).unwrap();
        let sid=w.create_space(SpaceKind::Interior,Some(outside),GridPoint::new(2,2)).unwrap();
        let inside=w.add_map("House",GridPoint::new(0,0),4,4).unwrap();
        assert_ne!(sid,0);
        assert_eq!(w.maps[1].primary_tileset,"building");
        assert_eq!(w.maps[1].secondary_tileset,"generic_building");
        w.check_integrity().unwrap();
        assert!(w.set_map_tileset(inside,"building/pokemon_center").is_ok());
        w.check_integrity().unwrap();
    }
}

#[cfg(test)]
mod cross_space_event_tests {
    use super::*;
    #[test]
    fn events_are_placed_only_in_the_active_space_even_at_same_coordinates(){
        let mut w=World::new();
        let surface=w.add_map("Outside",GridPoint::new(0,0),6,6).unwrap();
        let inside_space=w.create_space(SpaceKind::Interior,Some(surface),GridPoint::new(3,3)).unwrap();
        let inside=w.add_map("Inside",GridPoint::new(0,0),6,6).unwrap();
        let (new_id,index)=w.add_event(GridPoint::new(2,2),EventKind::Npc).unwrap();
        assert_eq!(new_id,inside);
        assert_eq!(index,0);
        assert_eq!(w.maps.iter().find(|m|m.id==surface).unwrap().events.len(),0);
        w.set_active_space(0).unwrap();
        let (id,_)=w.add_event(GridPoint::new(2,2),EventKind::Sign).unwrap();
        assert_eq!(id,surface);
        assert_eq!(w.maps.iter().find(|m|m.id==inside).unwrap().events.len(),1);
        w.set_active_space(inside_space).unwrap();
        assert_eq!(w.event_at(GridPoint::new(2,2)),Some((inside,0)));
        w.check_integrity().unwrap();
    }
}
