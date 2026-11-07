//! Native egui map editor with an atlas of real Emerald metatiles.
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2, pos2, vec2};
use crate::model::{Cell, EventKind, MapEvent, WarpMode, GridPoint, ExpandDirection, SpaceKind, World};
use crate::{storage, export, tiles, choices};
use std::{path::PathBuf, collections::HashMap, time::{Duration, Instant}};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode { Terrain, Properties, Events }

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
enum SizeAxis { Width, Height }
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
enum ResizeEdge { Left, Right, Top, Bottom }

#[derive(Clone,Copy,Debug)]
struct ResizeGuidance {
    map_id:u32,
    width:u32,
    height:u32,
    direction:ExpandDirection,
    max_width:u32,
    max_height:u32,
}


/// Only a long hold creates/moves a frame. Immediate dragging still paints/pans.
#[derive(Clone,Copy,Debug)]
enum CanvasGesture {
    Idle,
    EmptyPress { at: GridPoint, screen: Pos2, since: f64 },
    PanBlank,
    DrawFrame { from: GridPoint, to: GridPoint },
    MapPress { id:u32, at: GridPoint, screen: Pos2, since:f64, origin:GridPoint },
    Painting { last: GridPoint },
    MoveFrame { id:u32, from:GridPoint, origin:GridPoint },
    ResizeFrame {
        id:u32,edge:ResizeEdge,from:GridPoint,
        base_width:u32,base_height:u32,width:u32,height:u32,
        direction:ExpandDirection,
    },
    EventPress { map_id:u32, index:usize, at:GridPoint, screen:Pos2, since:f64 },
    MoveEvent { map_id:u32, index:usize },
    PickWarp { at:GridPoint },
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
enum WarpPickMethod { Existing, CreateMarker, Direct }

#[derive(Clone,Copy,Debug)]
struct WarpPick {
    source:(u32,usize),
    method:WarpPickMethod,
    return_link:bool,
}

pub struct MapStudioApp {
    world: World,
    mode: Mode,
    event_brush: EventKind,
    selected_event: Option<(u32,usize)>,
    event_editor_open: bool,
    warp_pick: Option<WarpPick>,
    warp_return_link: bool,
    brush_id: u16,
    selected_stamp:Option<tiles::MetatileStamp>,
    sheet_anchor:Option<(u16,u16)>,
    sheet_hold:Option<(u16,u16,f64,Pos2)>,
    sheet_inertia:f32,
    bucket_tool:bool,
    erase_tool: bool,
    tool_active:bool,
    find_index:usize,
    find_target:Option<u16>,
    elevation_brush: u8,
    elevation_tool: bool,
    atlas: egui::TextureHandle,
    npc_atlas:egui::TextureHandle,
    item_atlas:egui::TextureHandle,
    atlas_cache: HashMap<String,egui::TextureHandle>,
    preferred_pair: String,
    tile_filter: tiles::TileFilter,
    tileset_group:String,
    pending_tileset:Option<String>,
    tile_preview: Option<u16>,
    save_path: Option<PathBuf>,
    export_dir: Option<PathBuf>,
    dirty_since: Option<Instant>,
    history: Vec<World>,
    redo: Vec<World>,
    before_gesture: Option<World>,
    gesture: CanvasGesture,
    preview_error: Option<String>,
    collision_brush: u8,
    panel_height: f32,
    map_width: u32,
    map_height: u32,
    expand_direction: ExpandDirection,
    number_axis:Option<SizeAxis>,
    replace_number:bool,
    resize_guidance:Option<ResizeGuidance>,
    size_panel_open: bool,
    show_parent:bool,
    overlays_open:bool,
    overlays_button:Option<Rect>,
    show_events:bool,
    show_properties:bool,
    zoom: f32,
    offset: Vec2,
    canvas_size: Vec2,
    last_window_size:Vec2,
    multitouch_in_progress: bool,
    selected_map: Option<u32>,
    selected_cell: Option<GridPoint>,
    warp_focus: Option<GridPoint>,
    confirm_delete: Option<u32>,
    message: String,
    project_restore_open: bool,
    available_backups: Vec<(String,String)>,
    selected_backup: Option<String>,
    new_project_confirm: bool,
}

impl MapStudioApp {
    pub fn new(ctx: &egui::Context) -> Self {
        Self::with_paths(ctx,Some(PathBuf::from("map-studio-project.json")),
            Some(PathBuf::from("map-studio-exports")))
    }

    pub fn with_paths(
        ctx: &egui::Context, save_path: Option<PathBuf>, export_dir: Option<PathBuf>
    ) -> Self {
        ctx.set_visuals(egui::Visuals::light());
        // Read actual game metatiles, rendered from the repository's graphics + palettes + bin data.
        let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/metatiles.png"));
        let image = image::load_from_memory(bytes).expect("compiled Emerald atlas").to_rgba8();
        let dimensions = [image.width() as usize, image.height() as usize];
        let texture = ctx.load_texture("Emerald metatiles",egui::ColorImage::from_rgba_unmultiplied(
            dimensions,image.as_raw()),egui::TextureOptions::NEAREST);
        let npc_image=image::load_from_memory(choices::NPC_ATLAS_PNG)
            .expect("source-verified NPC preview sprite atlas").to_rgba8();
        let npc_dimensions=[npc_image.width() as usize,npc_image.height() as usize];
        let npc_atlas=ctx.load_texture("Emerald NPC sprites",
            egui::ColorImage::from_rgba_unmultiplied(npc_dimensions,npc_image.as_raw()),
            egui::TextureOptions::NEAREST);

        let item_image=image::load_from_memory(choices::HIDDEN_ITEM_ATLAS)
            .expect("verified source item icon atlas").to_rgba8();
        let item_size=[item_image.width() as usize,item_image.height() as usize];
        let item_atlas=ctx.load_texture("Emerald item icons",
            egui::ColorImage::from_rgba_unmultiplied(item_size,item_image.as_raw()),
            egui::TextureOptions::NEAREST);

        // Never silently overwrite an incompatible/corrupt project with demo maps.
        let (world, load_error)=match save_path.as_deref() {
            Some(path) if path.exists() => match storage::load(path) {
                Ok(world)=>(world,None),
                Err(err)=>(World::new(),Some(err)),
            },
            _=>(World::new(),None),
        };
        let message=if let Some(err)=&load_error {
            format!("Saved project cannot be loaded; autosave disabled: {err}")
        } else {
            "Hold empty grid to draw a map · hold a map to move".to_string()
        };
        let save_path=if load_error.is_some(){None}else{save_path};
        Self {
            world,
            mode: Mode::Terrain,
            event_brush: EventKind::Npc,
            selected_event: None,
            event_editor_open: false,
            warp_pick: None,
            warp_return_link: false,
            brush_id: 1,
            selected_stamp:None,
            sheet_anchor:None,
            sheet_hold:None,
            sheet_inertia:0.0,
            bucket_tool:false,
            erase_tool: false,
            tool_active:true,
            find_index:0,
            find_target:None,
            elevation_brush: 3,
            elevation_tool: false,
            atlas: texture.clone(),
            npc_atlas,
            item_atlas,
            atlas_cache: HashMap::from([(tiles::DEFAULT_PAIR.to_string(),texture)]),
            preferred_pair:tiles::DEFAULT_PAIR.into(),
            tile_filter:tiles::TileFilter::All,
            tileset_group:"Towns".into(),
            pending_tileset:None,
            tile_preview: None,
            save_path,
            export_dir,
            dirty_since: None,
            history: Vec::new(),
            redo: Vec::new(),
            before_gesture: None,
            gesture: CanvasGesture::Idle,
            preview_error: None,
            collision_brush: 0,
            panel_height: 280.0,
            map_width: 12,
            map_height: 10,
            expand_direction: ExpandDirection::LowerRight,
            number_axis:Some(SizeAxis::Width),
            replace_number:true,
            resize_guidance:None,
            size_panel_open: false,
            show_parent:true,
            overlays_open:false,
            overlays_button:None,
            show_events:true,
            show_properties:false,
            zoom: 14.0,
            offset: vec2(28.0, 52.0),
            canvas_size: vec2(360.0,540.0),
            last_window_size:Vec2::ZERO,
            multitouch_in_progress: false,
            selected_map: None,
            selected_cell: None,
            warp_focus: None,
            confirm_delete: None,
            message,
            project_restore_open: false,
            available_backups: Vec::new(),
            selected_backup: None,
            new_project_confirm: false,
        }
    }

    fn header(&mut self, ui: &mut egui::Ui) {
        let compact=ui.ctx().content_rect().height()<530.0
            || ui.available_width()<300.0;
        ui.add_space(if compact{1.0}else{5.0});
        ui.horizontal(|ui| {
            if compact{ui.strong("Map Studio");}else{ui.heading("Map Studio");}
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.menu_button("Project",|ui| {
                    if ui.button("New blank project").clicked() {
                        self.new_project_confirm=true;
                        ui.close();
                    }
                    if ui.button("Backup to Downloads").clicked() {
                        self.try_backup();
                        ui.close();
                    }
                    if ui.button("Restore backup").clicked() {
                        self.open_backup_list();
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Export game maps").clicked() {
                        self.try_export();
                        ui.close();
                    }
                });
                if ui.small_button(if compact{"↷"}else{"Redo"}).clicked() { self.redo(); }
                if ui.small_button(if compact{"↶"}else{"Undo"}).clicked() { self.undo(); }
            });
        });
        ui.horizontal_wrapped(|ui| {
            if ui.selectable_label(self.mode == Mode::Terrain, "Terrain").clicked() {
                self.mode = Mode::Terrain; self.gesture = CanvasGesture::Idle;
                self.selected_event=None;self.event_editor_open=false;
            }
            if ui.selectable_label(self.mode == Mode::Properties, "Properties").clicked() {
                self.mode = Mode::Properties; self.gesture = CanvasGesture::Idle;
                self.selected_event=None;self.event_editor_open=false;
            }
            if ui.selectable_label(self.mode == Mode::Events, "Events").clicked() {
                self.mode = Mode::Events; self.gesture = CanvasGesture::Idle;
            }
            ui.separator();
            let overlays=ui.button("Overlays");
            self.overlays_button=Some(overlays.rect);
            if overlays.clicked(){self.overlays_open=!self.overlays_open;}
        });
        if compact{
            ui.horizontal_wrapped(|ui|{
                let current=self.world.active_space().name.clone();
                ui.menu_button(format!("Space: {current}"),|ui|{
                    let entries:Vec<_>=self.world.spaces.iter()
                        .map(|sp|(sp.id,sp.name.clone())).collect();
                    for (space_id,name) in entries{
                        if ui.button(name).clicked(){
                            self.switch_space(space_id);ui.close();
                        }
                    }
                    ui.separator();
                    for kind in [SpaceKind::Interior,SpaceKind::Cave,SpaceKind::Other]{
                        if ui.button(format!("+ {}",kind.label())).clicked(){
                            self.create_editor_space(kind);ui.close();
                        }
                    }
                });
                if ui.small_button("Fit").clicked(){self.fit_world();}
                let selected=self.selected_map.and_then(|id|
                    self.world.active_maps().find(|m|m.id==id).map(|m|m.name.clone()));
                ui.small(format!("Selected: {}",selected.unwrap_or_else(||"None".into())));
                if ui.add_enabled(self.selected_map.is_some(),
                    egui::Button::new("Size")).clicked(){
                    if let Some(map)=self.selected_map.and_then(|id|
                        self.world.maps.iter().find(|m|m.id==id)){
                        self.map_width=map.width;self.map_height=map.height;
                    }
                    self.number_axis=Some(SizeAxis::Width);
                    self.replace_number=true;
                    self.resize_guidance=None;
                    self.size_panel_open=true;
                }
            });
            return;
        }
        // Spaces are editor-only; maps retain native map-local coordinates.
        let current=self.world.active_space();
        let space_label=current.name.clone();
        let parent_space=current.parent_map_id
            .and_then(|map_id|self.world.space_for_map(map_id));
        ui.horizontal_wrapped(|ui|{
            ui.menu_button(format!("◫ {space_label}"),|ui|{
                let entries:Vec<_>=self.world.spaces.iter()
                    .map(|sp|(sp.id,sp.name.clone(),sp.kind)).collect();
                for (id,name,kind) in entries {
                    if ui.selectable_label(id==self.world.active_space_id,
                        format!("{} · {}",kind.label(),name)).clicked(){
                        self.switch_space(id);
                        ui.close();
                    }
                }
            });
            ui.menu_button("+ Space",|ui|{
                for kind in [SpaceKind::Interior,SpaceKind::Cave,SpaceKind::Other]{
                    if ui.button(format!("New {}",kind.label())).clicked(){
                        self.create_editor_space(kind);
                        ui.close();
                    }
                }
            });
            if let Some(parent)=parent_space {
                if ui.small_button("↑ Parent").clicked(){self.switch_space(parent);}
            }
            if self.world.active_space_id!=0 {
                ui.toggle_value(&mut self.show_parent,"◉ Overlay");
            }
        });
        ui.horizontal(|ui| {
            if ui.add_enabled(self.selected_map.is_some(),egui::Button::new("Size")).clicked(){
                if let Some(map)=self.world.maps.iter().find(|m|Some(m.id)==self.selected_map){
                    self.map_width=map.width;
                    self.map_height=map.height;
                }
                self.number_axis=Some(SizeAxis::Width);
                self.replace_number=true;
                self.resize_guidance=None;
                self.size_panel_open=true;
            }
            if ui.small_button("Fit").clicked() { self.fit_world(); }
            ui.separator();
            let selected=self.selected_map.and_then(|id|
                self.world.active_maps().find(|m|m.id==id).map(|m|m.name.clone()));
            ui.strong(format!("Selected: {}",
                selected.unwrap_or_else(||"None".into())));
            ui.label(egui::RichText::new(format!("{} maps", self.world.active_maps().count()))
                .small().color(Color32::GRAY));
        });
        ui.add_space(4.0);
    }

    fn current_pair(&self)->&'static tiles::TilePair {
        let map=self.selected_map.and_then(|id|self.world.active_maps()
            .find(|m|m.id==id));
        let selected=map.map(|m|format!("{}/{}",m.primary_tileset,m.secondary_tileset))
            .unwrap_or_else(||self.preferred_pair.clone());
        tiles::pair(&selected).or_else(||tiles::pair(tiles::DEFAULT_PAIR))
            .expect("default source tileset catalog must exist")
    }

    fn switch_preferred_to_space(&mut self){
        let key=match self.world.active_space().kind {
            SpaceKind::Interior=>"building/generic_building",
            SpaceKind::Cave=>"general/cave",
            _=>tiles::DEFAULT_PAIR,
        };
        if let Some(pair)=tiles::pair(key){
            self.preferred_pair=pair.key.to_string();
            self.tileset_group=pair.category.into();
        }
        self.pending_tileset=None;
        self.tile_filter=tiles::TileFilter::All;
        self.selected_stamp=None;
        self.sheet_anchor=None;
    }

    fn ensure_atlas_cache(&mut self,ctx:&egui::Context) {
        let mut keys=vec![self.current_pair().key.to_string()];
        // Decode only currently visible source atlases. Hundreds of maps may
        // exist, but a phone should never upload every tileset to its GPU.
        let left=(-self.offset.x/self.zoom).floor() as i64-2;
        let top=(-self.offset.y/self.zoom).floor() as i64-2;
        let right=((self.canvas_size.x-self.offset.x)/self.zoom).ceil() as i64+2;
        let bottom=((self.canvas_size.y-self.offset.y)/self.zoom).ceil() as i64+2;
        for map in self.world.active_maps(){
            let mx=i64::from(map.origin.x);
            let my=i64::from(map.origin.y);
            if mx<right && mx+i64::from(map.width)>left &&
                my<bottom && my+i64::from(map.height)>top {
                keys.push(format!("{}/{}",map.primary_tileset,map.secondary_tileset));
            }
        }
        if self.show_parent {
            if let Some(p)=self.world.active_space().parent_map_id
                .and_then(|id|self.world.maps.iter().find(|m|m.id==id)) {
                keys.push(format!("{}/{}",p.primary_tileset,p.secondary_tileset));
            }
        }
        if let Some(key)=self.pending_tileset.as_ref(){keys.push(key.clone());}
        keys.sort();keys.dedup();
        self.atlas_cache.retain(|key,_|
            key==tiles::DEFAULT_PAIR || keys.contains(key));
        for key in keys {
            if self.atlas_cache.contains_key(&key){continue;}
            let Some(pair)=tiles::pair(&key) else{continue;};
            if let Ok(decoded)=image::load_from_memory(pair.atlas) {
                let rgba=decoded.to_rgba8();
                let dimensions=[rgba.width() as usize,rgba.height() as usize];
                let handle=ctx.load_texture(format!("Emerald {key}"),
                    egui::ColorImage::from_rgba_unmultiplied(dimensions,rgba.as_raw()),
                    egui::TextureOptions::NEAREST);
                self.atlas_cache.insert(key,handle);
            }
        }
    }

    fn apply_pair(&mut self,key:&str){
        if !tiles::pair(key).is_some(){self.message="Tileset unavailable".into();return;}
        if let Some(map_id)=self.selected_map.filter(|id|
            self.world.space_for_map(*id)==Some(self.world.active_space_id)){
            self.begin_edit();
            match self.world.set_map_tileset(map_id,key){
                Ok(())=>{
                    self.message="Map now uses the selected source tileset".into();
                    self.preferred_pair=key.into();
                    self.pending_tileset=None;
                    self.brush_id=1;
                    self.selected_stamp=None;
                    self.commit_edit();
                }
                Err(err)=>{self.message=err;self.commit_edit();}
            }
        }else{
            self.preferred_pair=key.into();
            self.pending_tileset=None;
            self.brush_id=1;
            self.selected_stamp=None;
        }
    }

    fn switch_space(&mut self,id:u32){
        if id==self.world.active_space_id{return;}
        self.commit_edit();
        self.begin_edit();
        if let Err(err)=self.world.set_active_space(id){
            self.message=err;
            return;
        }
        self.commit_edit();
        self.selected_map=None;
        self.selected_cell=None;
        self.warp_focus=None;
        if self.warp_pick.is_none(){self.selected_event=None;}
        self.gesture=CanvasGesture::Idle;
        self.tile_preview=None;
        self.selected_stamp=None;
        self.sheet_anchor=None;
        self.sheet_hold=None;
        self.size_panel_open=false;
        self.switch_preferred_to_space();
        self.fit_world();
        self.message=format!("Editing {} · overlays do not affect game coordinates",
            self.world.active_space().name);
    }

    fn create_editor_space(&mut self,kind:SpaceKind){
        let parent_id=self.selected_map.filter(|id|
            self.world.space_for_map(*id)==Some(self.world.active_space_id));
        let anchor=parent_id.and_then(|id|{
            let m=self.world.maps.iter().find(|m|m.id==id)?;
            let event=self.selected_event
                .filter(|(mid,_)|*mid==id)
                .and_then(|(_,index)|m.events.get(index));
            Some(event.map(|e|GridPoint::new(
                m.origin.x.saturating_add(e.x as i32),
                m.origin.y.saturating_add(e.y as i32)))
                .unwrap_or(GridPoint::new(
                    m.origin.x.saturating_add((m.width/2) as i32),
                    m.origin.y.saturating_add((m.height/2) as i32))))
        }).unwrap_or_default();
        self.begin_edit();
        match self.world.create_space(kind,parent_id,anchor) {
            Ok(id)=>{
                self.message=format!("{} created · draw its first map here",kind.label());
                self.selected_map=None;
                self.selected_event=None;
                self.gesture=CanvasGesture::Idle;
                self.switch_preferred_to_space();
                self.fit_world();
                self.commit_edit();
                debug_assert_eq!(self.world.active_space_id,id);
            }
            Err(err)=>{self.message=err;self.commit_edit();}
        }
    }

    fn fit_world(&mut self){
        if self.world.active_maps().next().is_none() ||
            self.canvas_size.x<40.0 || self.canvas_size.y<40.0 {
            self.offset=vec2(28.0,40.0);
            return;
        }
        let min_x=self.world.active_maps().map(|m|i64::from(m.origin.x)).min().unwrap();
        let min_y=self.world.active_maps().map(|m|i64::from(m.origin.y)).min().unwrap();
        let max_x=self.world.active_maps().map(|m|i64::from(m.origin.x)+i64::from(m.width)).max().unwrap();
        let max_y=self.world.active_maps().map(|m|i64::from(m.origin.y)+i64::from(m.height)).max().unwrap();
        let width=(max_x-min_x).max(1) as f32;
        let height=(max_y-min_y).max(1) as f32;
        let padding=24.0;
        self.zoom=((self.canvas_size.x-2.0*padding)/width)
            .min((self.canvas_size.y-2.0*padding)/height).clamp(2.0,54.0);
        self.offset=vec2(
            (self.canvas_size.x-width*self.zoom)*0.5 - min_x as f32*self.zoom,
            (self.canvas_size.y-height*self.zoom)*0.5 - min_y as f32*self.zoom,
        );
    }

    fn configure_map_from_double_tap(&mut self,id:u32) {
        if let Some(m)=self.world.maps.iter().find(|m|m.id==id) {
            self.selected_map=Some(id);
            self.map_width=m.width;
            self.map_height=m.height;
            self.number_axis=Some(SizeAxis::Width);
            self.replace_number=true;
            self.resize_guidance=None;
            self.size_panel_open=true;
            self.message="Map settings: name, size and expansion direction".into();
        }
    }

    /// Enter a map using its ID without typing an engine identifier.
    fn handle_warp_destination(&mut self,p:GridPoint)->Result<(),String>{
        let pick=self.warp_pick.ok_or("No warp selected")?;
        match pick.method {
            WarpPickMethod::Existing=>{
                let dest=self.world.event_at(p).ok_or("Tap an existing warp marker")?;
                if self.world.event(dest.0,dest.1).map(|e|e.kind)!=Some(EventKind::Warp){
                    return Err("Tap a warp marker, not another kind of event".into());
                }
                self.world.link_warps(pick.source,dest,pick.return_link)?;
            }
            WarpPickMethod::CreateMarker=>{
                let target=self.world.create_arrival_marker(pick.source,p,pick.return_link)?;
                self.message=format!("Arrival marker created on map {}.",target.0);
            }
            WarpPickMethod::Direct=>{
                self.world.set_direct_arrival(pick.source,p)?;
            }
        }
        self.warp_pick=None;
        self.selected_event=Some(pick.source);
        self.event_editor_open=true;
        self.message=match pick.method {
            WarpPickMethod::Existing=>"Linked to destination warp".into(),
            WarpPickMethod::CreateMarker=>"Arrival marker linked; return is optional".into(),
            WarpPickMethod::Direct=>"Direct arrival set; no marker required".into(),
        };
        Ok(())
    }

    fn warp_summary(&self,event:&MapEvent)->String {
        if event.arrival_only {return "Arrival-only marker".into();}
        let Some(map)=self.world.maps.iter().find(|m|m.id==event.dest_map_id)
            else{return "Destination not set".into();};
        match event.warp_mode {
            WarpMode::Linked=>format!("{} · warp {}",map.name,event.dest_warp_id),
            WarpMode::Direct=>format!("{} · tile ({}, {})",map.name,event.dest_x,event.dest_y)
        }
    }

    fn resize_thumbnail(&self,ui:&mut egui::Ui,id:u32){
        let Some(m)=self.world.maps.iter().find(|m|m.id==id) else{return;};
        let Some((origin,w,h))=self.world.resize_ghost(id,self.map_width,
            self.map_height,self.expand_direction) else{return;};
        let (area,_) = ui.allocate_exact_size(
            vec2(ui.available_width().min(320.0),116.0),Sense::hover());
        let painter=ui.painter_at(area);
        painter.rect_filled(area,5.0,Color32::from_gray(245));
        // Preview shows existing footprint, candidate and nearby neighbors.
        let l=i64::from(m.origin.x).min(i64::from(origin.x));
        let t=i64::from(m.origin.y).min(i64::from(origin.y));
        let r=(i64::from(m.origin.x)+i64::from(m.width)).max(i64::from(origin.x)+i64::from(w));
        let b=(i64::from(m.origin.y)+i64::from(m.height)).max(i64::from(origin.y)+i64::from(h));
        let mut min_x=l-1;let mut min_y=t-1;
        let mut max_x=r+1;let mut max_y=b+1;
        let close=self.world.maps.iter().filter(|other|{
            other.id!=id && other.space_id==m.space_id
                && i64::from(other.origin.x)<r+8 && (i64::from(other.origin.x)+i64::from(other.width))>l-8
                && i64::from(other.origin.y)<b+8 && (i64::from(other.origin.y)+i64::from(other.height))>t-8
        }).collect::<Vec<_>>();
        for other in &close {
            min_x=min_x.min(i64::from(other.origin.x));
            min_y=min_y.min(i64::from(other.origin.y));
            max_x=max_x.max((i64::from(other.origin.x)+i64::from(other.width)));
            max_y=max_y.max((i64::from(other.origin.y)+i64::from(other.height)));
        }
        let factor=((area.width()-16.0)/(max_x-min_x).max(1) as f32)
            .min((area.height()-16.0)/(max_y-min_y).max(1) as f32);
        let at=|x:i64,y:i64| pos2(
            area.left()+8.0+(x-min_x) as f32*factor,
            area.top()+8.0+(y-min_y) as f32*factor);
        let bounds=|x:i64,y:i64,w:u32,h:u32|
            Rect::from_min_size(at(x,y),vec2(w as f32*factor,h as f32*factor));
        for other in close {
            let box_rect=bounds(i64::from(other.origin.x),
                i64::from(other.origin.y),other.width,other.height);
            painter.rect_filled(box_rect,2.0,Color32::from_gray(202));
            painter.rect_stroke(box_rect,2.0,
                Stroke::new(1.3,Color32::from_gray(105)),StrokeKind::Inside);
        }
        let original=bounds(i64::from(m.origin.x),i64::from(m.origin.y),
            m.width,m.height);
        painter.rect_filled(original,2.0,Color32::from_rgba_unmultiplied(85,130,202,75));
        painter.rect_stroke(original,2.0,
            Stroke::new(1.4,Color32::from_rgb(64,107,182)),StrokeKind::Inside);
        let next=bounds(i64::from(origin.x),i64::from(origin.y),w,h);
        let valid=w<=100 && h<=100
            && self.world.preview_resize(id,w,h,self.expand_direction).is_ok();
        let color=if valid{Color32::from_rgb(36,147,104)}
            else{Color32::from_rgb(195,65,65)};
        painter.rect_filled(next,2.0,
            if valid{Color32::from_rgba_unmultiplied(46,171,103,65)}
            else{Color32::from_rgba_unmultiplied(213,76,76,85)});
        painter.rect_stroke(next,2.0,Stroke::new(2.0,color),StrokeKind::Inside);
        painter.text(next.center(),Align2::CENTER_CENTER,
            format!("{} x {}",w,h),
            FontId::proportional(11.0),color);
    }

    fn size_dialog(&mut self,ctx:&egui::Context) {
        if !self.size_panel_open{return;}
        let mut open=self.size_panel_open;
        egui::Window::new("Map settings")
            .id(egui::Id::new("map_size_dialog"))
            .open(&mut open)
            .resizable(false).collapsible(false)
            .anchor(Align2::CENTER_TOP,[0.0,52.0])
            .show(ctx,|ui|{
                egui::ScrollArea::vertical()
                    .id_salt("size_editor_scroll")
                    .max_height((ctx.content_rect().height()-104.0).max(175.0))
                    .show(ui,|ui|{
                let Some(id)=self.selected_map else{
                    ui.small("Select a map to resize.");return;
                };
                let original=self.world.maps.iter().find(|m|m.id==id)
                    .map(|m|(m.width,m.height));
                let Some((old_width,old_height))=original else{
                    ui.small("Map no longer available.");return;
                };
                let mut changed=false;
                let before=self.world.clone();
                if let Some(map)=self.world.maps.iter_mut().find(|m|m.id==id){
                    ui.horizontal(|ui|{
                        ui.label("Map name");
                        changed=ui.add(egui::TextEdit::singleline(&mut map.name)
                            .desired_width(160.0)).changed();
                    });
                }
                if changed{
                    self.before_gesture=Some(before);
                    self.commit_edit();
                }
                ui.horizontal(|ui|{
                    let width=ui.add(egui::Button::new(
                        format!("Width: {}",self.map_width))
                        .fill(if self.number_axis==Some(SizeAxis::Width){
                            Color32::from_rgb(176,217,248)
                        }else{Color32::from_gray(234)}));
                    if width.clicked(){
                        self.number_axis=Some(SizeAxis::Width);
                        self.replace_number=true;
                    }
                    let height=ui.add(egui::Button::new(
                        format!("Height: {}",self.map_height))
                        .fill(if self.number_axis==Some(SizeAxis::Height){
                            Color32::from_rgb(176,217,248)
                        }else{Color32::from_gray(234)}));
                    if height.clicked(){
                        self.number_axis=Some(SizeAxis::Height);
                        self.replace_number=true;
                    }
                });
                if let Some(axis)=self.number_axis{
                    ui.small("Tap digits to replace the selected number. Delete removes one digit.");
                    egui::Grid::new("size_numeric_pad")
                        .num_columns(3).spacing([5.0,4.0]).show(ui,|ui|{
                        for row in [
                            ["1","2","3"],["4","5","6"],
                            ["7","8","9"],["Clear","0","Delete"]
                        ]{
                            for label in row{
                                if ui.add_sized(vec2(64.0,33.0),
                                    egui::Button::new(label)).clicked(){
                                    let value=match axis{
                                        SizeAxis::Width=>&mut self.map_width,
                                        SizeAxis::Height=>&mut self.map_height,
                                    };
                                    match label{
                                        "Clear"=>{*value=0;self.replace_number=false;}
                                        "Delete"=>{*value/=10;self.replace_number=false;}
                                        digit=>{
                                            let d=digit.parse::<u32>().unwrap_or(0);
                                            *value=if self.replace_number {d}
                                                else{value.saturating_mul(10).saturating_add(d)
                                                    .min(999)};
                                            self.replace_number=false;
                                        }
                                    }
                                    self.resize_guidance=None;
                                }
                            }
                            ui.end_row();
                        }
                    });
                }
                ui.label("Grow toward");
                let dirs=[
                    (ExpandDirection::UpperLeft,"Upper left"),
                    (ExpandDirection::UpperRight,"Upper right"),
                    (ExpandDirection::LowerLeft,"Lower left"),
                    (ExpandDirection::LowerRight,"Lower right"),
                ];
                egui::Grid::new("expansion_directions")
                    .num_columns(2).spacing([8.0,8.0])
                    .show(ui,|ui|{
                        for (index,(dir,label)) in dirs.iter().enumerate(){
                            let requested_grow=self.map_width>old_width
                                ||self.map_height>old_height;
                            let requested_shrink=self.map_width<old_width
                                ||self.map_height<old_height;
                            let enabled=if requested_grow || requested_shrink {
                                self.map_width<=100 && self.map_height<=100
                                    && self.world.preview_resize(id,self.map_width,
                                        self.map_height,*dir).is_ok()
                            }else{
                                self.world.preview_resize(id,old_width+1,
                                    old_height,*dir).is_ok()
                                    ||self.world.preview_resize(id,old_width,
                                        old_height+1,*dir).is_ok()
                            };
                            let chosen=*dir==self.expand_direction;
                            let button=egui::Button::new(*label)
                                .fill(if chosen {Color32::from_rgb(177,217,246)}
                                      else{Color32::from_gray(238)});
                            if ui.add_enabled(enabled,button).clicked(){
                                self.expand_direction=*dir;
                                self.resize_guidance=None;
                            }
                            if index%2==1 {ui.end_row();}
                        }
                    });
                let key=(id,self.map_width,self.map_height,self.expand_direction);
                let refresh=self.resize_guidance.as_ref().is_none_or(|cache|
                    (cache.map_id,cache.width,cache.height,cache.direction)!=key);
                if refresh{
                    self.resize_guidance=Some(ResizeGuidance{
                        map_id:id,width:self.map_width,height:self.map_height,
                        direction:self.expand_direction,
                        max_width:self.world.resize_axis_limit(id,self.map_height,
                            self.expand_direction,true),
                        max_height:self.world.resize_axis_limit(id,self.map_width,
                            self.expand_direction,false),
                    });
                }
                if let Some(limits)=self.resize_guidance{
                    ui.small(format!(
                        "Max without overlap or lost content: width {} at height {}; height {} at width {}",
                        limits.max_width,self.map_height,limits.max_height,self.map_width));
                }
                self.resize_thumbnail(ui,id);
                let valid=self.map_width>0 && self.map_height>0
                    && self.map_width<=100 && self.map_height<=100;
                let result=if valid{
                    self.world.preview_resize(id,self.map_width,self.map_height,
                        self.expand_direction)
                }else{Err("Width and height must be between 1 and 100".into())};
                match &result{
                    Ok(_)=>{ui.small("Green preview: painted cells and events are preserved.");}
                    Err(err)=>{ui.colored_label(Color32::from_rgb(185,54,54),
                        format!("Cannot apply: {err}"));}
                }
                if ui.add_enabled(result.is_ok(),
                    egui::Button::new("Apply size")).clicked(){
                    self.begin_edit();
                    match self.world.resize_frame_towards(id,self.map_width,
                        self.map_height,self.expand_direction){
                        Ok(())=>{
                            self.message="Map resized".into();
                            self.size_panel_open=false;
                            self.resize_guidance=None;
                        }
                        Err(err)=>self.message=err,
                    }
                    self.commit_edit();
                }
                });
            });
        self.size_panel_open&=open;
    }

    fn request_tileset(&mut self,key:&str) {
        let Some(pair)=tiles::pair(key) else{return;};
        self.tileset_group=pair.category.to_string();
        if self.current_pair().key==key {
            self.pending_tileset=None;
            return;
        }
        let painted=self.selected_map.and_then(|id|
            self.world.active_maps().find(|m|m.id==id))
            .is_some_and(|m|m.cells.iter().any(Option::is_some));
        if painted{
            self.pending_tileset=Some(key.into());
            self.message="Confirm the new tileset before changing painted art".into();
        }else{
            self.apply_pair(key);
        }
    }

    fn palette(&mut self, ui: &mut egui::Ui) {
        // Only the long source-art sheet is scrollable. The palette handle,
        // source-pair selector, painting tools and Find navigation stay pinned.
        let (handle,response)=ui.allocate_exact_size(
            vec2(ui.available_width(),21.0),Sense::click_and_drag());
        let center=handle.center();
        ui.painter().rect_filled(handle,0.0,Color32::from_gray(243));
        ui.painter().rect_filled(Rect::from_center_size(
            center,vec2(46.0,4.0)),3.0,Color32::from_gray(144));
        if response.dragged() {
            let max=(ui.ctx().content_rect().height()*0.72).max(44.0);
            self.panel_height=(self.panel_height-response.drag_delta().y).clamp(44.0,max);
            ui.ctx().request_repaint();
        }
        if response.double_clicked() {
            self.panel_height=if self.panel_height<230.0 {370.0}else{165.0};
        }
        if self.mode==Mode::Terrain {
            let chosen=self.current_pair();
            ui.horizontal(|ui|{
                ui.menu_button(format!("Tileset · {} ▾",chosen.label),|ui|{
                    egui::ScrollArea::vertical().max_height(280.0).show(ui,|ui|{
                        for group in ["Towns","Houses","Interiors","Caves",
                                      "Outdoor & Special"]{
                            ui.collapsing(group,|ui|{
                                let entries=tiles::pairs().iter()
                                    .filter(|p|p.category==group).collect::<Vec<_>>();
                                if let Some(first)=entries.first(){
                                    if ui.small_button("Use first in category").clicked(){
                                        self.request_tileset(first.key);
                                        ui.close();
                                    }
                                }
                                for item in entries {
                                    if ui.selectable_label(
                                        chosen.key==item.key,item.label).clicked(){
                                        self.request_tileset(item.key);
                                        ui.close();
                                    }
                                }
                            });
                        }
                    });
                });
            });
        }
        if self.mode==Mode::Terrain {
            self.terrain_toolbar(ui);
        }
        ui.separator();
        egui::ScrollArea::vertical().id_salt("dock_content")
            .auto_shrink([false,false]).show(ui,|ui|{
                if self.mode==Mode::Terrain {
                    // Inertia advances the source sheet after a quick swipe.
                    if self.sheet_inertia.abs()>=0.45 {
                        ui.scroll_with_delta(vec2(0.0,self.sheet_inertia));
                        self.sheet_inertia*=0.86;
                        ui.ctx().request_repaint_after(Duration::from_millis(16));
                    }else{
                        self.sheet_inertia=0.0;
                    }
                    self.terrain_sheet_content(ui);
                }else{
                    self.palette_content(ui);
                }
            });
    }

    fn sanitize_dialogue(input:&mut String) {
        input.retain(|ch| (ch.is_ascii_graphic() || ch == ' ')
            && (ch as u32) != 34 && (ch as u32) != 36 && (ch as u32) != 92);
        input.truncate(100);
    }

    fn find_arrow(ui:&mut egui::Ui,up:bool,enabled:bool)->bool {
        // Render the arrow as geometry: the default Android egui font does
        // not always contain Unicode up/down glyphs.
        let (area,response)=ui.allocate_exact_size(vec2(33.0,29.0),Sense::click());
        ui.painter().rect_filled(area,5.0,
            Color32::from_gray(if enabled{230}else{246}));
        let c=area.center();
        let col=if enabled{Color32::from_rgb(58,76,96)}
            else{Color32::from_gray(186)};
        let points=if up {
            vec![c+vec2(0.0,-6.0),c+vec2(-6.0,4.0),c+vec2(6.0,4.0)]
        }else{
            vec![c+vec2(-6.0,-4.0),c+vec2(6.0,-4.0),c+vec2(0.0,6.0)]
        };
        ui.painter().add(egui::Shape::convex_polygon(
            points,col,Stroke::NONE));
        enabled && response.clicked()
    }


    fn terrain_toolbar(&mut self, ui:&mut egui::Ui) {
                ui.horizontal_wrapped(|ui|{
                    ui.strong("Tool");
                    let brush=self.tool_active && !self.erase_tool && !self.bucket_tool;
                    if ui.selectable_label(brush,"Brush").clicked(){
                        self.tool_active=!brush;
                        self.erase_tool=false;self.bucket_tool=false;
                    }
                    let bucket=self.tool_active && self.bucket_tool;
                    if ui.selectable_label(bucket,"Bucket").clicked(){
                        self.tool_active=!bucket;
                        self.bucket_tool=self.tool_active;
                        self.erase_tool=false;
                    }
                    let erase=self.tool_active && self.erase_tool;
                    if ui.selectable_label(erase,"Eraser").clicked(){
                        self.tool_active=!erase;
                        self.erase_tool=self.tool_active;
                        self.bucket_tool=false;
                    }
                    if !self.tool_active {ui.small("Pan mode");}
                });
                if let Some(pending)=self.pending_tileset.clone(){
                    if let Some(p)=tiles::pair(&pending){
                        ui.horizontal_wrapped(|ui|{
                            ui.colored_label(Color32::from_rgb(157,83,33),
                                format!("Change existing map to {}?",p.label));
                            if ui.button("Apply").clicked(){self.apply_pair(&pending);}
                            if ui.small_button("Cancel").clicked(){self.pending_tileset=None;}
                        });
                    }
                }
                let chosen=self.current_pair();
                ui.horizontal_wrapped(|ui|{
                    ui.label("Find");
                    let mut filter_changed=false;
                    egui::ComboBox::from_id_salt("metatile-highlight")
                        .selected_text(self.tile_filter.label())
                        .width(122.0)
                        .show_ui(ui,|ui|{
                            for filter in tiles::TileFilter::ALL {
                                filter_changed |= ui.selectable_value(
                                    &mut self.tile_filter,filter,filter.label()).changed();
                            }
                        });
                    let matches=if self.tile_filter==tiles::TileFilter::All {
                        Vec::new()
                    }else{
                        tiles::tile_ids(chosen).into_iter()
                            .filter(|id|tiles::matches_filter(chosen,*id,self.tile_filter))
                            .collect::<Vec<_>>()
                    };
                    if filter_changed{
                        self.find_index=0;
                        self.find_target=matches.first().copied();
                    }
                    if !matches.is_empty(){
                        self.find_index=self.find_index.min(matches.len()-1);
                    }
                    if self.tile_filter!=tiles::TileFilter::All {
                        if Self::find_arrow(ui,true,!matches.is_empty()){
                            self.find_index=(self.find_index+matches.len()-1)%matches.len();
                            self.find_target=Some(matches[self.find_index]);
                        }
                        if Self::find_arrow(ui,false,!matches.is_empty()){
                            self.find_index=(self.find_index+1)%matches.len();
                            self.find_target=Some(matches[self.find_index]);
                        }
                        ui.small(format!("{}/{}",
                            if matches.is_empty(){0}else{self.find_index+1},matches.len()));
                    }
                });
    }

    fn terrain_sheet_content(&mut self, ui:&mut egui::Ui) {
        let chosen=self.current_pair();
                ui.small("Tap a tile to paint. Hold then slide to select a rectangle.");
                self.metatile_sheet(ui,chosen,0,chosen.primary_count,"Shared terrain");
                self.metatile_sheet(ui,chosen,512,chosen.secondary_count,"Local tiles");
                if let Some(stamp)=self.selected_stamp.as_ref() {
                    ui.label(format!("Selected {} × {} metatiles · source {}",
                        stamp.width,stamp.height,chosen.label));
                }else if let Some(tile)=self.tile_preview {
                    ui.label(format!("Selected · {}",tiles::source_label_in(chosen,tile)
                        .unwrap_or("unnamed source metatile")));
                }

        ui.small(&self.message);
    }

    fn palette_content(&mut self, ui: &mut egui::Ui) {
        match self.mode {
            Mode::Terrain => {}
            Mode::Properties => {
                ui.strong("Paint map properties");
                ui.horizontal_wrapped(|ui|{
                    ui.label("Paint layer");
                    ui.selectable_value(&mut self.elevation_tool,false,"Collision");
                    ui.selectable_value(&mut self.elevation_tool,true,"Elevation");
                });
                // Focus on the editable map-block properties only.
                // Detailed source metatile metadata was reference information,
                // not a control, and does not belong in this painting toolbar.
                ui.group(|ui|{
                    if self.elevation_tool {
                        ui.strong("Elevation · game layer");
                        egui::Grid::new("elevation_palette").num_columns(4).spacing([5.0,6.0])
                            .show(ui,|ui|{
                                for level in 0..16u8 {
                                    let label=level.to_string();
                                    let selected=self.elevation_brush==level;
                                    let color=if selected{Color32::from_rgb(132,191,241)}
                                        else{Color32::from_rgb(231,236,245)};
                                    if ui.add_sized(vec2(72.0,34.0),
                                        egui::Button::new(label).fill(color)).clicked(){
                                        self.elevation_brush=level;
                                    }
                                    if level%4==3{ui.end_row();}
                                }
                            });
                        ui.small("Select an elevation, then touch or drag over painted tiles.");
                    }else{
                        ui.strong("Collision values");
                        ui.horizontal_wrapped(|ui|{
                            for (value,label,color) in [
                                (0u8,"0 · Walk",Color32::from_rgb(180,234,193)),
                                (1,"1 · Block",Color32::from_rgb(245,190,190)),
                                (2,"2",Color32::from_rgb(215,194,244)),
                                (3,"3",Color32::from_rgb(248,219,172)),
                            ]{
                                let fill=if self.collision_brush==value{color}
                                    else{Color32::from_gray(235)};
                                if ui.add_sized(vec2(104.0,36.0),
                                    egui::Button::new(label).fill(fill)).clicked(){
                                    self.collision_brush=value;
                                }
                            }
                        });
                        ui.small("Paint collision data. Terrain behavior (water, ledges, doors, ice) belongs to the chosen metatile.");
                    }
                });
                if let Some(id)=self.selected_map {
                    ui.collapsing("Map settings",|ui|{
                        let before=self.world.clone();
                        let mut changed=false;
                        let mut delete=false;
                        if let Some(map)=self.world.maps.iter_mut().find(|m|m.id==id){
                            ui.horizontal_wrapped(|ui|{
                                ui.label("Name");
                                changed=ui.add(egui::TextEdit::singleline(&mut map.name)
                                    .desired_width(155.0)).changed();
                                ui.label(format!("{} × {}",map.width,map.height));
                                if ui.small_button("Delete map").clicked(){delete=true;}
                            });
                        }
                        if changed{self.before_gesture=Some(before);self.commit_edit();}
                        if delete{self.confirm_delete=Some(id);}
                        if self.confirm_delete==Some(id) {
                            ui.horizontal_wrapped(|ui|{
                                ui.colored_label(Color32::DARK_RED,"Remove this map?");
                                if ui.button("Confirm delete").clicked() {
                                    self.begin_edit();
                                    match self.world.remove_frame(id) {
                                        Ok(())=>{
                                            self.selected_map=None;
                                            self.selected_cell=None;
                                            self.selected_event=None;
                                            self.message="Map deleted.".into();
                                        }
                                        Err(err)=>self.message=err,
                                    }
                                    self.commit_edit();
                                    self.confirm_delete=None;
                                }
                                if ui.small_button("Cancel").clicked(){self.confirm_delete=None;}
                            });
                        }
                    });
                }
            }
            Mode::Events => {
                ui.horizontal_wrapped(|ui|{
                    ui.strong("Add event");
                    for (kind,title) in [
                        (EventKind::Npc,"Person"),
                        (EventKind::Sign,"Sign"),
                        (EventKind::Warp,"Warp"),
                        (EventKind::Trigger,"Step message"),
                        (EventKind::HiddenItem,"Hidden item"),
                    ]{
                        ui.selectable_value(&mut self.event_brush,kind,title);
                    }
                });
                if let Some(pick)=self.warp_pick {
                    let instruction=match pick.method {
                        WarpPickMethod::Existing=>"Tap an existing warp in the destination map",
                        WarpPickMethod::CreateMarker=>"Tap a painted, walkable destination tile",
                        WarpPickMethod::Direct=>"Tap any painted, walkable arrival tile",
                    };
                    ui.colored_label(Color32::from_rgb(31,91,153),instruction);
                    if ui.button("Cancel destination selection").clicked(){
                        self.warp_pick=None;
                        self.event_editor_open=true;
                    }
                }
                if let Some((map_id,index))=self.selected_event {
                    let summary=self.world.event(map_id,index).cloned();
                    if let Some(ev)=summary.as_ref() {
                        if ev.kind==EventKind::Warp {
                            ui.horizontal_wrapped(|ui|{
                                ui.strong("Destination");
                                ui.label(self.warp_summary(ev));
                            });
                            if let Some(native_id)=self.world.warp_id(map_id,index) {
                                ui.small(format!("This marker: warp {native_id} in its own map"));
                            }else if ev.warp_mode==WarpMode::Direct {
                                ui.small("Direct travel has no numbered source warp");
                            }
                            if let Some((destination_id,destination_space))=
                                self.world.maps.iter().find(|m|m.id==ev.dest_map_id)
                                    .map(|m|(m.id,m.space_id)) {
                                if ui.small_button("Open destination warp").clicked(){
                                    // Native linked warp IDs index only numbered
                                    // warp events; direct arrivals are coordinates.
                                    let target=self.world.maps.iter()
                                        .find(|m|m.id==destination_id)
                                        .and_then(|target|{
                                            if ev.warp_mode==WarpMode::Direct {
                                                Some((None,ev.dest_x,ev.dest_y,
                                                    target.origin))
                                            }else{
                                                target.events.iter().enumerate()
                                                    .filter(|(_,e)|e.kind==EventKind::Warp
                                                        && e.warp_mode!=WarpMode::Direct)
                                                    .nth(ev.dest_warp_id as usize)
                                                    .map(|(i,e)|(Some(i),e.x,e.y,target.origin))
                                            }
                                        });
                                    self.switch_space(destination_space);
                                    self.mode=Mode::Events;
                                    self.selected_map=Some(destination_id);
                                    if let Some((event_index,x,y,origin))=target {
                                        let at=GridPoint::new(
                                            origin.x.saturating_add(x as i32),
                                            origin.y.saturating_add(y as i32));
                                        self.selected_event=event_index.map(|i|(destination_id,i));
                                        self.selected_cell=Some(at);
                                        self.warp_focus=Some(at);
                                        self.zoom=self.zoom.max(16.0);
                                        self.offset=self.canvas_size*0.5
                                            -vec2(at.x as f32+0.5,at.y as f32+0.5)*self.zoom;
                                        self.message="Destination warp selected and centered".into();
                                    }else{
                                        self.selected_event=None;
                                        self.message="Destination map opened; warp target is unavailable".into();
                                    }
                                }
                            }
                        }
                    }
                    let before=self.world.clone();
                    let mut changed=false;
                    let mut remove=false;
                    let mut select_pick=None;
                    let mut create_space_from_warp=None;
                    let mut found=false;
                    if let Some(ev)=self.world.event_mut(map_id,index) {
                        found=true;
                        ui.horizontal(|ui|{
                            ui.strong(match ev.kind {
                                EventKind::Npc=>"Person",
                                EventKind::Sign=>"Sign",
                                EventKind::Warp=>"Warp",
                                EventKind::Trigger=>"Step message",
                                EventKind::HiddenItem=>"Hidden item",
                            });
                            if ui.button(if self.event_editor_open{"Close"}else{"Edit"}).clicked(){
                                self.event_editor_open=!self.event_editor_open;
                            }
                            if ui.small_button("Remove").clicked(){remove=true;}
                        });
                        if self.event_editor_open {
                            match ev.kind {
                                EventKind::Npc=>{
                                    ui.horizontal(|ui|{
                                        ui.label("Character");
                                        let shown=choices::npc_friendly(&ev.graphics_id)
                                            .unwrap_or("Choose a character");
                                        egui::ComboBox::from_id_salt("npc-character")
                                            .selected_text(shown).show_ui(ui,|ui|{
                                                for (name,symbol) in choices::NPC_SPRITES {
                                                    changed|=ui.selectable_value(&mut ev.graphics_id,
                                                        (*symbol).into(),*name).changed();
                                                }
                                            });
                                    });
                                    ui.horizontal(|ui|{
                                        ui.label("Dialogue");
                                        let editing=ui.add(egui::TextEdit::singleline(&mut ev.dialogue)
                                            .desired_width(210.0).hint_text("What do they say?")).changed();
                                        if editing {Self::sanitize_dialogue(&mut ev.dialogue);}
                                        changed|=editing;
                                    });
                                }
                                EventKind::Sign=>{
                                    ui.horizontal(|ui|{
                                        ui.label("Text");
                                        let editing=ui.add(egui::TextEdit::singleline(&mut ev.dialogue)
                                            .desired_width(235.0).hint_text("Sign message")).changed();
                                        if editing {Self::sanitize_dialogue(&mut ev.dialogue);}
                                        changed|=editing;
                                    });
                                }
                                EventKind::Trigger=>{
                                    ui.small("Shows a message when the player steps here");
                                    ui.horizontal(|ui|{
                                        ui.label("Message");
                                        let editing=ui.add(egui::TextEdit::singleline(&mut ev.dialogue)
                                            .desired_width(220.0).hint_text("Message on step")).changed();
                                        if editing {Self::sanitize_dialogue(&mut ev.dialogue);}
                                        changed|=editing;
                                    });
                                }
                                EventKind::HiddenItem=>{
                                    ui.horizontal(|ui|{
                                        ui.label("Item");
                                        let selected=choices::HIDDEN_ITEMS.iter()
                                            .find(|(_,id)|*id==ev.hidden_item)
                                            .map(|(name,_)|*name).unwrap_or("Choose item");
                                        egui::ComboBox::from_id_salt("hidden-item")
                                            .selected_text(selected).show_ui(ui,|ui|{
                                                for (name,id) in choices::HIDDEN_ITEMS {
                                                    changed|=ui.selectable_value(&mut ev.hidden_item,
                                                        (*id).into(),*name).changed();
                                                }
                                            });
                                    });
                                    ui.small(if ev.hidden_flag.is_empty(){
                                        "No pickup slot assigned"
                                    }else{"One-time pickup · collection slot assigned automatically"});
                                }
                                EventKind::Warp=>{
                                    ui.checkbox(&mut self.warp_return_link,"Also link a return warp");
                                    ui.small("For direct arrivals, create a separate return warp if needed.");
                                    ui.horizontal_wrapped(|ui|{
                                        if ui.small_button("New interior here").clicked(){
                                            create_space_from_warp=Some(SpaceKind::Interior);
                                        }
                                        if ui.small_button("New cave floor here").clicked(){
                                            create_space_from_warp=Some(SpaceKind::Cave);
                                        }
                                    });
                                    ui.horizontal_wrapped(|ui|{
                                        if ui.button("Link existing warp").clicked(){
                                            select_pick=Some(WarpPickMethod::Existing);
                                        }
                                        if ui.button("Create arrival marker").clicked(){
                                            select_pick=Some(WarpPickMethod::CreateMarker);
                                        }
                                        if ui.button("Direct arrival").clicked(){
                                            select_pick=Some(WarpPickMethod::Direct);
                                        }
                                    });
                                    if ev.arrival_only {
                                        ui.small("This marker is for arriving; it does not return unless linked.");
                                    }
                                }
                            }
                            if ev.kind!=EventKind::Warp {
                                ui.horizontal(|ui|{
                                    ui.label("Height");
                                    egui::ComboBox::from_id_salt("event-height")
                                        .selected_text(ev.elevation.to_string())
                                        .show_ui(ui,|ui|{
                                            for height in 0..=15u8 {
                                                changed|=ui.selectable_value(&mut ev.elevation,
                                                    height,height.to_string()).changed();
                                            }
                                        });
                                });
                            }
                        }else{
                            ui.small("Tap selects · double-tap opens details · hold moves");
                        }
                    }
                    if changed {
                        self.before_gesture=Some(before);
                        self.commit_edit();
                    }
                    if let Some(kind)=create_space_from_warp {
                        self.create_editor_space(kind);
                    }
                    if let Some(method)=select_pick {
                        self.warp_pick=Some(WarpPick{
                            source:(map_id,index),method,return_link:self.warp_return_link
                        });
                        self.event_editor_open=false;
                    }
                    if remove {
                        self.begin_edit();
                        if self.world.remove_event(map_id,index) {
                            self.selected_event=None;
                            self.warp_pick=None;
                            self.event_editor_open=false;
                        }else{
                            self.message="Cannot delete: another warp points here".into();
                        }
                        self.commit_edit();
                    }else if !found {
                        self.selected_event=None;
                    }
                }else{
                    ui.small("Tap a map tile to place an event; hold to move, double-tap to edit.");
                    ui.small("Available: NPC dialogue, signs, linked/direct warps, step messages and one-time hidden items. Advanced scripted trainers, conditional scenes and special events are not yet authored here.");
                }
            }
        }
        ui.label(egui::RichText::new(&self.message).small().color(Color32::DARK_GRAY));
    }

    /// Select single tiles by tap or a native-order rectangle by holding
    /// before sliding. Quick swipes remain scrolling gestures.
    fn choose_sheet_stamp(&mut self,pair:&tiles::TilePair,base:u16,from:u16,to:u16) {
        if let Some(stamp)=tiles::stamp_from_sheet(pair,base,from,to){
            if let Some(first)=stamp.cells.iter().find_map(|tile|*tile){
                self.brush_id=first;
                self.tile_preview=Some(first);
                self.erase_tool=false;
                self.bucket_tool=false;
                self.tool_active=true;
                self.selected_stamp=Some(stamp);
            }
        }
    }

    fn metatile_sheet(&mut self,ui:&mut egui::Ui,pair:&tiles::TilePair,
                       base:u16,count:usize,label:&str) {
        if count==0{return;}
        ui.label(egui::RichText::new(format!("{label} · {count} source slots"))
            .strong().small());
        let side=(ui.available_width()/tiles::SHEET_COLUMNS as f32)
            .floor().clamp(16.0,52.0);
        let rows=(count+tiles::SHEET_COLUMNS-1)/tiles::SHEET_COLUMNS;
        let (rect,response)=ui.allocate_exact_size(
            vec2(side*tiles::SHEET_COLUMNS as f32,side*rows as f32),
            Sense::click_and_drag());
        let locate=|point:Pos2| -> Option<u16> {
            if !rect.contains(point){return None;}
            let x=((point.x-rect.left())/side).floor() as usize;
            let y=((point.y-rect.top())/side).floor() as usize;
            let offset=y*tiles::SHEET_COLUMNS+x;
            (x<tiles::SHEET_COLUMNS && offset<count)
                .then_some(base+offset as u16)
        };
        let (pressed,down,released,now,pointer)=ui.input(|input|(
            input.pointer.primary_pressed(),
            input.pointer.primary_down(),
            input.pointer.primary_released(),
            input.time,
            input.pointer.interact_pos()
        ));
        if pressed && response.is_pointer_button_down_on(){
            self.sheet_hold=pointer.and_then(locate).map(|id|(base,id,now,pointer.unwrap()));
            self.sheet_anchor=None;
        }
        if down {
            if let Some((sheet,first,since,press_position))=self.sheet_hold {
                if sheet==base {
                    let travel=pointer.map(|p|(p-press_position).length()).unwrap_or(0.0);
                    if self.sheet_anchor.is_none(){
                        if now-since>=0.46 && travel<=10.0 {
                            self.sheet_anchor=Some((base,first));
                            self.sheet_inertia=0.0;
                            self.choose_sheet_stamp(pair,base,first,first);
                        }else if travel>10.0 {
                            // An immediate finger swipe scrolls the sheet.
                            self.sheet_hold=None;
                        }else{
                            ui.ctx().request_repaint_after(Duration::from_millis(25));
                        }
                    } else if let (Some((anchor_sheet,start)),Some(to))=
                        (self.sheet_anchor,pointer.and_then(locate)) {
                        if anchor_sheet==base {
                            self.choose_sheet_stamp(pair,base,start,to);
                        }
                    }
                }
            }
            if self.sheet_hold.is_none() && response.dragged()
                && self.sheet_anchor.is_none() {
                let delta=response.drag_delta();
                ui.scroll_with_delta(delta);
                self.sheet_inertia=delta.y.clamp(-65.0,65.0);
            }
        }
        if released {
            self.sheet_hold=None;
            self.sheet_anchor=None;
        }
        if response.clicked(){
            if let Some(id)=pointer.and_then(locate){
                self.choose_sheet_stamp(pair,base,id,id);
            }
        }
        if let Some(id)=self.find_target{
            if (base..base+512).contains(&id) {
                let index=(id-base) as usize;
                let row=index/tiles::SHEET_COLUMNS;
                let col=index%tiles::SHEET_COLUMNS;
                let destination=Rect::from_min_size(
                    pos2(rect.left()+col as f32*side,rect.top()+row as f32*side),
                    Vec2::splat(side));
                ui.scroll_to_rect(destination,Some(egui::Align::Center));
                self.find_target=None;
            }
        }
        let visible=rect.intersect(ui.clip_rect());
        if visible.width()<=0.0 || visible.height()<=0.0{return;}
        let start_row=((visible.top()-rect.top())/side).floor().max(0.0) as usize;
        let end_row=(((visible.bottom()-rect.top())/side).ceil() as usize)
            .min(rows);
        let texture=self.atlas_cache.get(pair.key).unwrap_or(&self.atlas).id();
        for row in start_row..end_row {
            for col in 0..tiles::SHEET_COLUMNS {
                let index=row*tiles::SHEET_COLUMNS+col;
                if index>=count{break;}
                let id=base+index as u16;
                let cell=Rect::from_min_size(pos2(rect.left()+col as f32*side,
                    rect.top()+row as f32*side),Vec2::splat(side));
                ui.painter().rect_filled(cell,0.0,Color32::from_gray(225));
                if tiles::valid_in(pair,id) {
                    ui.painter().image(texture,cell,Self::tile_uv(id),Color32::WHITE);
                    if self.tile_filter!=tiles::TileFilter::All
                        && !tiles::matches_filter(pair,id,self.tile_filter) {
                        ui.painter().rect_filled(cell,0.0,Color32::from_black_alpha(120));
                    }
                } else {
                    ui.painter().line_segment([cell.left_top(),cell.right_bottom()],
                        Stroke::new(0.6,Color32::from_gray(205)));
                }
                let selected=self.selected_stamp.as_ref().is_some_and(|stamp|{
                    if stamp.pair_key!=pair.key || stamp.base!=base {return false;}
                    let origin=(stamp.origin-base) as usize;
                    let left=origin%tiles::SHEET_COLUMNS;
                    let top=origin/tiles::SHEET_COLUMNS;
                    row>=top && row<top+stamp.height && col>=left && col<left+stamp.width
                });
                if selected {
                    ui.painter().rect_stroke(cell.shrink(0.5),0.0,
                        Stroke::new(2.0,Color32::from_rgb(16,120,213)),
                        StrokeKind::Inside);
                }
            }
        }
    }

    fn tile_uv(id:u16) -> Rect {
        let col=(id%16) as f32/16.0;
        let row=(id/16) as f32/64.0;
        Rect::from_min_max(pos2(col,row),pos2(col+1.0/16.0,row+1.0/64.0))
    }

    fn begin_edit(&mut self) {
        if self.before_gesture.is_none() { self.before_gesture=Some(self.world.clone()); }
    }

    fn commit_edit(&mut self) {
        if let Some(before)=self.before_gesture.take() {
            if before!=self.world {
                self.history.push(before);
                if self.history.len()>25 {self.history.remove(0);}
                self.redo.clear();
                self.dirty_since=Some(Instant::now());
            }
        }
    }

    fn undo(&mut self) {
        self.commit_edit();
        if let Some(previous)=self.history.pop() {
            self.redo.push(std::mem::replace(&mut self.world,previous));
            self.dirty_since=Some(Instant::now());
            self.message="Undo".into();
        }
    }

    fn redo(&mut self) {
        self.commit_edit();
        if let Some(next)=self.redo.pop() {
            self.history.push(std::mem::replace(&mut self.world,next));
            self.dirty_since=Some(Instant::now());
            self.message="Redo".into();
        }
    }

    fn fill_selected(&mut self) {
        if self.selected_stamp.as_ref().is_some_and(|stamp|!stamp.is_single()){
            self.message="Fill empty uses one tile. Tap one sheet cell first.".into();
            return;
        }
        if self.world.maps.iter().any(|m|Some(m.id)==self.selected_map) {
            let new=Cell {terrain:crate::model::TileKind::Grass,metatile_id:self.brush_id,
                collision:0,elevation:3};
            let before=self.world.clone();
            if let Some(map)=self.world.maps.iter_mut().find(|m|Some(m.id)==self.selected_map) {
                for cell in &mut map.cells {if cell.is_none(){*cell=Some(new);}}
            }
            self.before_gesture=Some(before);
            self.commit_edit();
            self.message="Filled unpainted cells in selected map.".into();
        } else {
            self.message="Select a map with Move before filling.".into();
        }
    }

    fn maybe_autosave(&mut self,ctx:&egui::Context) {
        if let Some(since)=self.dirty_since {
            if since.elapsed() >= Duration::from_millis(850) {
                if let Some(path)=&self.save_path {
                    match storage::save(path,&self.world) {
                        Ok(()) => {self.message="Saved locally".into();self.dirty_since=None;}
                        Err(err) => {self.message=err;self.dirty_since=None;}
                    }
                } else {
                    self.message="Save path unavailable on this device".into();
                    self.dirty_since=None;
                }
            } else {ctx.request_repaint_after(Duration::from_millis(250));}
        }
    }

    fn load_restore_choices(&self)->Result<Vec<(String,String)>,String>{
        let mut items=Vec::new();
        if let Ok(dir)=self.output_dir(){
            if dir.is_dir() {
                for entry in std::fs::read_dir(&dir).map_err(|e|e.to_string())? {
                    let entry=entry.map_err(|e|e.to_string())?;
                    let file=entry.file_name().to_string_lossy().to_string();
                    if ((file.starts_with("MapStudio-project-")
                        || file.starts_with("MapStudio-recovery-"))
                        && file.ends_with(".json")
                        || file.starts_with("MapStudio-maps-") && file.ends_with(".zip"))
                        && entry.path().is_file(){
                        items.push((format!("local:{file}"),format!("{file} (local recovery)")));
                    }
                }
            }
        }
        #[cfg(target_os="android")]
        {
            for entry in crate::android_export::list_backups()? {
                items.push((format!("android:{}",entry.id),entry.name));
            }
        }
        items.sort_by(|a,b|b.1.cmp(&a.1));
        items.truncate(80);
        Ok(items)
    }

    fn open_backup_list(&mut self) {
        match self.load_restore_choices() {
            Ok(items)=>{
                self.available_backups=items;
                self.selected_backup=None;
                self.project_restore_open=true;
                self.message="Select a backup. Your current work will be preserved first.".into();
            }
            Err(err)=>self.message=format!("Cannot browse backups: {err}"),
        }
    }

    fn read_restore_entry(&self,id:&str)->Result<Vec<u8>,String> {
        if let Some(filename)=id.strip_prefix("local:") {
            if !filename.starts_with("MapStudio-")
                || !(filename.ends_with(".json") || filename.ends_with(".zip"))
                || filename.contains('/') || filename.contains('\\') {
                return Err("Invalid backup file name".into());
            }
            let path=self.output_dir()?.join(filename);
            let metadata=std::fs::metadata(&path).map_err(|e|e.to_string())?;
            if metadata.len()>(storage::MAX_PROJECT_BYTES as u64) {
                return Err("Project exceeds the 20 MiB import safety limit".into());
            }
            return std::fs::read(path).map_err(|e|e.to_string());
        }
        #[cfg(target_os="android")]
        if let Some(number)=id.strip_prefix("android:") {
            let id=number.parse::<i64>().map_err(|_|"Invalid Android backup ID")?;
            return crate::android_export::read_backup(id);
        }
        Err("Unsupported backup source".into())
    }

    fn install_world(&mut self,incoming:World)->Result<(),String> {
        incoming.check_integrity()?;
        let path=self.save_path.clone().ok_or(
            "Project autosave unavailable: don't risk replacing the current work")?;
        let output_dir=self.output_dir()?;
        std::fs::create_dir_all(&output_dir).map_err(|e|e.to_string())?;
        // The recovery file is written first and is never deleted by the restore.
        let recovery_name=Self::filename("recovery","json");
        storage::save(&output_dir.join(&recovery_name),&self.world)?;
        storage::save(&path,&incoming)?;
        self.world=incoming;
        self.dirty_since=None;
        self.history.clear();
        self.redo.clear();
        self.before_gesture=None;
        self.selected_map=None;
        self.selected_event=None;
        self.selected_cell=None;
        self.event_editor_open=false;
        self.confirm_delete=None;
        self.size_panel_open=false;
        self.fit_world();
        self.message=format!("Project opened. Previous work saved as {recovery_name}");
        Ok(())
    }

    fn restore_selected_backup(&mut self) {
        let result=(||->Result<(),String>{
            let key=self.selected_backup.as_deref().ok_or("Select a backup first")?;
            let bytes=self.read_restore_entry(key)?;
            // Validate untrusted media before writing a recovery snapshot or active save.
            let world=storage::decode_package(&bytes)?;
            self.install_world(world)
        })();
        if let Err(err)=result{self.message=format!("Restore failed: {err}");}
        else {
            self.project_restore_open=false;
            self.selected_backup=None;
        }
    }

    fn start_new_project(&mut self){
        match self.install_world(World::new()){
            Ok(())=>self.message="New blank project created. Recovery copy saved.".into(),
            Err(err)=>self.message=format!("New project blocked: {err}"),
        }
        self.new_project_confirm=false;
    }

    fn project_dialogs(&mut self,ctx:&egui::Context) {
        if self.new_project_confirm {
            let mut open=true;
            egui::Window::new("New project")
                .collapsible(false).resizable(false).open(&mut open)
                .anchor(Align2::CENTER_CENTER,[0.0,0.0])
                .show(ctx,|ui|{
                    ui.label("Create a blank canvas? A recovery copy is saved first.");
                    ui.horizontal(|ui|{
                        if ui.button("Create new").clicked(){self.start_new_project();}
                        if ui.button("Cancel").clicked(){self.new_project_confirm=false;}
                    });
                });
            if !open {self.new_project_confirm=false;}
        }
        if self.project_restore_open {
            let mut open=true;
            egui::Window::new("Restore project backup")
                .collapsible(false).resizable(true)
                .default_width(310.0).open(&mut open)
                .anchor(Align2::CENTER_CENTER,[0.0,0.0])
                .show(ctx,|ui|{
                    ui.small("Only readable Map Studio backups are shown. Current work is backed up before replacement.");
                    if self.available_backups.is_empty(){
                        ui.label("No backups in this app's Downloads/MapStudio or local recovery folder.");
                    } else {
                        egui::ScrollArea::vertical().max_height(190.0).show(ui,|ui|{
                            for (id,name) in &self.available_backups {
                                ui.selectable_value(&mut self.selected_backup,Some(id.clone()),name);
                            }
                        });
                    }
                    ui.horizontal(|ui|{
                        if ui.add_enabled(self.selected_backup.is_some(),
                            egui::Button::new("Restore selected")).clicked() {
                            self.restore_selected_backup();
                        }
                        if ui.button("Cancel").clicked(){
                            self.project_restore_open=false;
                        }
                    });
                });
            if !open {self.project_restore_open=false;}
        }
    }

    fn output_dir(&self) -> Result<PathBuf,String> {
        self.export_dir.clone().or_else(||self.save_path.as_ref()
            .and_then(|p|p.parent().map(|p|p.to_path_buf())))
            .ok_or("No app storage directory available".into())
    }

    fn publish_output(&self, file:&std::path::Path, name:&str) -> Result<String,String> {
        #[cfg(target_os="android")]
        { crate::android_export::publish(file,name) }
        #[cfg(not(target_os="android"))]
        { let _=name; Ok(file.display().to_string()) }
    }

    fn filename(kind:&str, extension:&str) -> String {
        use std::time::{SystemTime,UNIX_EPOCH};
        let stamp=SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap_or_default().as_millis();
        format!("MapStudio-{kind}-{stamp}.{extension}")
    }

    fn try_backup(&mut self) {
        self.commit_edit();
        let result=(|| -> Result<String,String> {
            let directory=self.output_dir()?;
            std::fs::create_dir_all(&directory).map_err(|e|e.to_string())?;
            let name=Self::filename("project","json");
            let file=directory.join(&name);
            storage::save(&file,&self.world)?;
            self.publish_output(&file,&name)
        })();
        self.message=match result {
            Ok(path)=>format!("Project backup: {path}"),
            Err(err)=>format!("Backup failed: {err}"),
        };
    }

    fn try_export(&mut self) {
        self.commit_edit();
        let result=(|| -> Result<String,String> {
            let directory=self.output_dir()?;
            std::fs::create_dir_all(&directory).map_err(|e|e.to_string())?;
            let name=Self::filename("maps","zip");
            let file=directory.join(&name);
            export::export_zip(&self.world,&file)?;
            self.publish_output(&file,&name)
        })();
        self.message=match result {
            Ok(path)=>format!("Exported: {path}"),
            Err(err)=>format!("Export blocked: {err}"),
        };
    }

    fn screen_at(&self, rect: Rect, x: f32, y: f32) -> Pos2 {
        pos2(rect.min.x + self.offset.x + x * self.zoom,
             rect.min.y + self.offset.y + y * self.zoom)
    }

    /// Match a marker's visible screen-sized hit area, even at low world
    /// zoom where its owning metatile can be only a few physical pixels.
    fn edge_at_screen(&self,canvas:Rect,id:u32,point:Pos2)->Option<ResizeEdge>{
        let map=self.world.maps.iter().find(|map|map.id==id)?;
        let top_left=self.screen_at(canvas,
            map.origin.x as f32,map.origin.y as f32);
        let outline=Rect::from_min_size(top_left,
            vec2(map.width as f32*self.zoom,map.height as f32*self.zoom));
        if outline.width()<32.0 || outline.height()<32.0
            || !outline.contains(point){return None;}
        let options=[
            ((point.x-outline.left()).abs(),ResizeEdge::Left),
            ((point.x-outline.right()).abs(),ResizeEdge::Right),
            ((point.y-outline.top()).abs(),ResizeEdge::Top),
            ((point.y-outline.bottom()).abs(),ResizeEdge::Bottom),
        ];
        options.into_iter().filter(|(distance,_)|*distance<=12.0)
            .min_by(|a,b|a.0.total_cmp(&b.0))
            .map(|(_,edge)|edge)
    }

    fn event_at_screen(&self,rect:Rect,point:Pos2)->Option<(u32,usize)>{
        self.world.active_maps().flat_map(|map|{
            map.events.iter().enumerate().filter_map(move |(index,event)|{
                let center=self.screen_at(rect,
                    map.origin.x as f32+event.x as f32+0.5,
                    map.origin.y as f32+event.y as f32+0.5);
                let bounds=if event.kind==EventKind::Npc {
                    Rect::from_center_size(
                        center+vec2(0.0,-self.zoom*0.75),
                        vec2((self.zoom*1.5).max(26.0),
                             (self.zoom*2.5).max(42.0)))
                }else{
                    Rect::from_center_size(center,Vec2::splat(28.0))
                };
                if bounds.contains(point){
                    Some(((map.id,index),(point-center).length_sq()))
                }else{None}
            })
        }).min_by(|a,b|a.1.total_cmp(&b.1)).map(|(key,_)|key)
    }

    fn grid_at(&self, rect: Rect, screen: Pos2) -> GridPoint {
        GridPoint::new(
            ((screen.x - rect.min.x - self.offset.x) / self.zoom).floor() as i32,
            ((screen.y - rect.min.y - self.offset.y) / self.zoom).floor() as i32,
        )
    }

    fn long_hold_complete(since:f64,now:f64,travel:f32)->bool{
        now-since>=0.46 && travel<=9.0
    }

    fn change_zoom(&mut self, rect: Rect, focus: Pos2, factor: f32, translation: Vec2) {
        let old_zoom = self.zoom;
        let new_zoom = (old_zoom * factor).clamp(2.0, 54.0);
        let focus_local = focus - rect.min;
        let anchor = (focus_local - self.offset) / old_zoom;
        self.offset = focus_local - anchor * new_zoom + translation;
        self.zoom = new_zoom;
    }

    fn paint_line(&mut self, from: GridPoint, to: GridPoint) {
        // Bresenham avoids holes when the finger moves faster than render frames.
        let (mut x, mut y) = (from.x, from.y);
        let dx = (to.x - x).abs();
        let sx = if x < to.x { 1 } else { -1 };
        let dy = -(to.y - y).abs();
        let sy = if y < to.y { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            let at = GridPoint::new(x, y);
            match self.mode {
                Mode::Terrain => {
                    if self.tool_active && !self.bucket_tool {
                        if self.erase_tool {
                            self.world.erase(at);
                        } else if let Some(stamp)=self.selected_stamp.as_ref() {
                            self.world.paint_stamp(at,stamp);
                        } else {
                            self.world.paint_metatile(at,self.brush_id);
                        }
                    }
                }
                Mode::Properties => { if self.elevation_tool {
                    self.world.set_elevation(at, self.elevation_brush);
                } else { self.world.set_collision(at, self.collision_brush); } }
                Mode::Events => {}
            }
            if x == to.x && y == to.y { break; }
            let e2 = 2 * err;
            if e2 >= dy { err += dy; x += sx; }
            if e2 <= dx { err += dx; y += sy; }
        }
    }

    fn bounds(from:GridPoint,to:GridPoint)->(GridPoint,u32,u32) {
        let origin=GridPoint::new(from.x.min(to.x),from.y.min(to.y));
        let width=(i64::from(from.x.max(to.x))-i64::from(origin.x)+1) as u32;
        let height=(i64::from(from.y.max(to.y))-i64::from(origin.y)+1) as u32;
        (origin,width,height)
    }

    fn handle_canvas_input(&mut self, ui:&egui::Ui,rect:Rect,response:&egui::Response) {
        // A tap outside the overlay panel dismisses it without accidentally
        // painting, creating an event, or moving a map underneath.
        if self.overlays_open {
            self.gesture=CanvasGesture::Idle;
            return;
        }
        const DRAG_PX:f32=9.0;
        let (multi,pressed,down,released,pointer,now)=ui.input(|input|(
            input.multi_touch(),
            input.pointer.button_pressed(egui::PointerButton::Primary),
            input.pointer.button_down(egui::PointerButton::Primary),
            input.pointer.button_released(egui::PointerButton::Primary),
            input.pointer.interact_pos(),input.time
        ));

        if let Some(touch)=multi.filter(|touch|rect.contains(touch.center_pos)) {
            self.multitouch_in_progress=true;
            self.gesture=CanvasGesture::Idle;
            self.preview_error=None;
            self.commit_edit();
            self.change_zoom(rect,touch.center_pos,touch.zoom_delta,touch.translation_delta);
            ui.ctx().request_repaint();
            return;
        }
        if self.multitouch_in_progress {
            if !down {self.multitouch_in_progress=false;}
            return;
        }
        if pressed && response.is_pointer_button_down_on() {
            if let Some(screen)=pointer.filter(|pos|rect.contains(*pos)) {
                let at=self.grid_at(rect,screen);
                self.warp_focus=None;
                if self.warp_pick.is_some() {
                    self.gesture=CanvasGesture::PickWarp{at};
                }else if let Some((map_id,index))=if self.mode==Mode::Events
                    && self.show_events {
                    self.event_at_screen(rect,screen)
                }else{None} {
                    self.selected_map=Some(map_id);
                    self.selected_event=Some((map_id,index));
                    self.gesture=CanvasGesture::EventPress{
                        map_id,index,at,screen,since:now
                    };
                    ui.ctx().request_repaint_after(Duration::from_millis(24));
                }else if let Some(m)=self.world.map_at(at) {
                    self.selected_map=Some(m.id);
                    self.selected_cell=Some(at);
                    if self.size_panel_open {
                        self.gesture=CanvasGesture::Idle;
                    } else {
                        self.gesture=CanvasGesture::MapPress {
                            id:m.id,at,screen,since:now,origin:m.origin
                        };
                        ui.ctx().request_repaint_after(Duration::from_millis(24));
                    }
                } else {
                    self.gesture=CanvasGesture::EmptyPress{at,screen,since:now};
                    self.preview_error=None;
                    ui.ctx().request_repaint_after(Duration::from_millis(24));
                }
            }
        }

        if down {
            if let Some(screen)=pointer {
                let at=self.grid_at(rect,screen);
                match self.gesture {
                    CanvasGesture::Idle => {}
                    CanvasGesture::PickWarp{..} => {}
                    CanvasGesture::EventPress{map_id,index,screen:origin,since,..}=>{
                        let delta=(screen-origin).length();
                        if Self::long_hold_complete(since,now,delta){
                            self.gesture=CanvasGesture::MoveEvent{map_id,index};
                            self.begin_edit();
                            self.message="Move this marker within its map".into();
                            ui.ctx().request_repaint();
                        }else if delta>DRAG_PX {
                            self.gesture=CanvasGesture::PanBlank;
                            self.offset+=response.drag_delta();
                            ui.ctx().request_repaint();
                        }else{
                            ui.ctx().request_repaint_after(Duration::from_millis(24));
                        }
                    }
                    CanvasGesture::MoveEvent{map_id,index}=>{
                        if let Err(err)=self.world.move_event(map_id,index,at){
                            self.preview_error=Some(err);
                        }else{
                            self.preview_error=None;
                        }
                        ui.ctx().request_repaint();
                    }
                    CanvasGesture::EmptyPress{at:start,screen:origin,since}=>{
                        let delta=(screen-origin).length();
                        if self.mode==Mode::Terrain && !self.tool_active && delta>DRAG_PX {
                            self.gesture=CanvasGesture::PanBlank;
                            self.offset+=response.drag_delta();
                            ui.ctx().request_repaint();
                        }else if Self::long_hold_complete(since,now,delta)
                            && !(self.mode==Mode::Terrain && !self.tool_active) {
                            self.gesture=CanvasGesture::DrawFrame{from:start,to:at};
                            self.message="Drag to draw map boundary".into();
                            ui.ctx().request_repaint();
                        } else if delta>DRAG_PX {
                            self.gesture=CanvasGesture::PanBlank;
                            self.offset+=response.drag_delta();
                            ui.ctx().request_repaint();
                        } else {
                            ui.ctx().request_repaint_after(Duration::from_millis(24));
                        }
                    }
                    CanvasGesture::PanBlank=>{
                        if response.dragged() {
                            self.offset+=response.drag_delta();
                            ui.ctx().request_repaint();
                        }
                    }
                    CanvasGesture::DrawFrame{from,..}=>{
                        self.gesture=CanvasGesture::DrawFrame{from,to:at};
                        let (origin,w,h)=Self::bounds(from,at);
                        self.preview_error=if w<2 || h<2 {
                            Some("Draw at least 2 by 2 tiles".into())
                        }else{
                            self.world.can_place_frame(origin,w,h).err()
                        };
                        ui.ctx().request_repaint();
                    }
                    CanvasGesture::MapPress{id,at:start,screen:origin,since,origin:map_origin}=>{
                        let delta=(screen-origin).length();
                        if self.mode==Mode::Terrain && !self.tool_active && delta>DRAG_PX {
                            self.gesture=CanvasGesture::PanBlank;
                            self.offset+=response.drag_delta();
                            ui.ctx().request_repaint();
                        }else if Self::long_hold_complete(since,now,delta)
                            && !(self.mode==Mode::Terrain && !self.tool_active) {
                            if let Some(edge)=self.edge_at_screen(rect,id,origin){
                                if let Some(map)=self.world.maps.iter().find(|m|m.id==id){
                                    let (base_width,base_height)=(map.width,map.height);
                                    let direction=match edge{
                                        ResizeEdge::Left=>ExpandDirection::LowerLeft,
                                        ResizeEdge::Right=>ExpandDirection::LowerRight,
                                        ResizeEdge::Top=>ExpandDirection::UpperRight,
                                        ResizeEdge::Bottom=>ExpandDirection::LowerRight,
                                    };
                                    self.gesture=CanvasGesture::ResizeFrame{
                                        id,edge,from:start,base_width,base_height,
                                        width:base_width,height:base_height,direction
                                    };
                                    self.message="Drag edge to preview, then release to resize".into();
                                }
                            }else{
                                self.gesture=CanvasGesture::MoveFrame{
                                    id,from:start,origin:map_origin
                                };
                                self.begin_edit();
                                self.message="Slide the map to move it".into();
                            }
                            ui.ctx().request_repaint();
                        }else if delta>DRAG_PX {
                            if self.mode==Mode::Terrain && self.tool_active && self.bucket_tool {
                                self.gesture=CanvasGesture::Idle;
                            }else if self.mode==Mode::Events{
                                self.gesture=CanvasGesture::PanBlank;
                                self.offset+=response.drag_delta();
                                ui.ctx().request_repaint();
                            }else{
                                self.begin_edit();
                                self.paint_line(start,at);
                                self.gesture=CanvasGesture::Painting{last:at};
                                ui.ctx().request_repaint();
                            }
                        }else{
                            ui.ctx().request_repaint_after(Duration::from_millis(24));
                        }
                    }
                    CanvasGesture::Painting{last}=>{
                        if self.mode!=Mode::Events {
                            self.paint_line(last,at);
                        }
                        self.gesture=CanvasGesture::Painting{last:at};
                        ui.ctx().request_repaint();
                    }
                    CanvasGesture::ResizeFrame{
                        id,edge,from,base_width,base_height,direction,..
                    }=>{
                        let dx=i64::from(at.x)-i64::from(from.x);
                        let dy=i64::from(at.y)-i64::from(from.y);
                        let change=match edge{
                            ResizeEdge::Left=>-dx,ResizeEdge::Right=>dx,
                            ResizeEdge::Top=>-dy,ResizeEdge::Bottom=>dy,
                        };
                        let mut width=base_width;
                        let mut height=base_height;
                        if matches!(edge,ResizeEdge::Left|ResizeEdge::Right){
                            width=(i64::from(base_width)+change).clamp(1,100) as u32;
                        }else{
                            height=(i64::from(base_height)+change).clamp(1,100) as u32;
                        }
                        self.preview_error=self.world.preview_resize(id,width,height,direction).err();
                        self.gesture=CanvasGesture::ResizeFrame{
                            id,edge,from,base_width,base_height,width,height,direction
                        };
                        ui.ctx().request_repaint();
                    }
                    CanvasGesture::MoveFrame{id,from,origin}=>{
                        let (dx,dy)=(i64::from(at.x)-i64::from(from.x),
                            i64::from(at.y)-i64::from(from.y));
                        let candidate=match (i32::try_from(i64::from(origin.x)+dx),
                                             i32::try_from(i64::from(origin.y)+dy)) {
                            (Ok(x),Ok(y))=>Some(GridPoint::new(x,y)),
                            _=>None
                        };
                        if let Some(pos)=candidate {
                            if self.world.move_frame(id,pos) {
                                self.preview_error=None;
                            }else{
                                self.preview_error=Some("Maps can't overlap".into());
                            }
                        }else{
                            self.preview_error=Some("Map is beyond canvas limits".into());
                        }
                        ui.ctx().request_repaint();
                    }
                }
            }
        }
        if released || (!down && !matches!(self.gesture,CanvasGesture::Idle)) {
            let at=pointer.map(|screen|self.grid_at(rect,screen));
            let gesture=std::mem::replace(&mut self.gesture,CanvasGesture::Idle);
            match gesture {
                CanvasGesture::PickWarp{at:start}=>{
                    if response.clicked() {
                        if let Some(where_to)=at.or(Some(start)){
                            let original=self.world.clone();
                            self.begin_edit();
                            if let Err(err)=self.handle_warp_destination(where_to){
                                self.world=original;
                                self.message=format!("Cannot use destination: {err}");
                            }
                            self.commit_edit();
                        }
                    }
                }
                CanvasGesture::EventPress{map_id,index,..}=>{
                    self.selected_event=Some((map_id,index));
                    self.selected_map=Some(map_id);
                    self.mode=Mode::Events;
                    self.event_editor_open=response.double_clicked();
                    if self.event_editor_open {self.panel_height=self.panel_height.max(310.0);}
                    self.message=if self.event_editor_open {
                        "Event settings opened".into()
                    }else{
                        "Event selected · hold to move · double-tap to edit".into()
                    };
                }
                CanvasGesture::MoveEvent{..}=>{
                    self.message=if self.preview_error.is_some(){
                        "Marker kept at its last valid tile".into()
                    }else{"Event moved".into()};
                    self.commit_edit();
                }
                CanvasGesture::EmptyPress{..}=>{
                    if response.clicked() {
                        self.selected_map=None;
                        self.selected_cell=None;
                        self.selected_event=None;
                        self.event_editor_open=false;
                        self.confirm_delete=None;
                        self.message="Selection cleared".into();
                    }
                    if response.double_clicked(){
                        if let Some(center)=pointer {
                            self.change_zoom(rect,center,1.7,Vec2::ZERO);
                            self.message="Zoomed in".into();
                        }
                    }
                }
                CanvasGesture::DrawFrame{from,to}=>{
                    let final_to=at.unwrap_or(to);
                    let (origin,w,h)=Self::bounds(from,final_to);
                    if w<2 || h<2 {
                        self.message="Draw at least 2 tiles wide and 2 tall".into();
                    }else{
                        match self.world.can_place_frame(origin,w,h) {
                            Ok(())=>{
                                self.begin_edit();
                                let next=self.world.maps.iter().map(|m|m.id).max().unwrap_or(0)+1;
                                match self.world.add_map(format!("Map {next}"),origin,w,h){ 
                                    Ok(id)=>{
                                        if let Err(err)=self.world.set_map_tileset(id,&self.preferred_pair){
                                            self.message=format!("Tileset unavailable: {err}");
                                        }
                                        self.selected_map=Some(id);
                                        self.map_width=w;
                                        self.map_height=h;
                                        self.message=format!("Map created: {w} × {h}. Use Size to adjust.");
                                    }
                                    Err(err)=>self.message=err.into(),
                                }
                                self.commit_edit();
                            }
                            Err(err)=>self.message=format!("Map wasn't created: {err}"),
                        }
                    }
                }
                CanvasGesture::MapPress{id,at:start,..}=>{
                    if response.double_clicked(){
                        let painted=self.world.map_at(start).and_then(|m|{
                            let x=(i64::from(start.x)-i64::from(m.origin.x)) as u32;
                            let y=(i64::from(start.y)-i64::from(m.origin.y)) as u32;
                            m.cell(x,y)
                        }).is_some();
                        if painted {
                            if self.mode==Mode::Properties {
                                self.selected_cell=Some(start);
                                self.panel_height=self.panel_height.max(275.0);
                                self.message="Selected tile properties".into();
                            } else if self.mode==Mode::Terrain {
                                self.message="To edit collision or elevation, open Properties".into();
                            }
                        }else if self.mode!=Mode::Events {
                            self.configure_map_from_double_tap(id);
                        }
                    }else if self.mode==Mode::Events {
                        if let Some(point)=at {
                            if self.world.map_at(point).is_some(){
                                self.begin_edit();
                                self.selected_event=self.world.add_event(point,self.event_brush);
                                if let Some((map_id,index))=self.selected_event{
                                    if self.event_brush==EventKind::HiddenItem {
                                        if let Err(err)=self.world.assign_hidden_slot(map_id,index){
                                            self.message=err;
                                        }
                                    }
                                    self.message="Event created · double-tap to edit".into();
                                }
                                self.commit_edit();
                            }
                        }
                    }else if self.mode==Mode::Terrain && self.tool_active && self.bucket_tool {
                        if self.selected_stamp.as_ref().is_some_and(|stamp|!stamp.is_single()){
                            self.message="Bucket fills a single metatile. Tap one tile in the sheet.".into();
                        }else{
                            self.begin_edit();
                            let filled=self.world.bucket_fill(start,self.brush_id);
                            self.commit_edit();
                            self.message=format!("Bucket filled {filled} connected tiles");
                            // Bucket is a one-shot operation; default back to Brush.
                            self.bucket_tool=false;
                            self.erase_tool=false;
                            self.tool_active=true;
                        }
                    }else if self.mode==Mode::Properties
                        || (self.mode==Mode::Terrain && self.tool_active) {
                        self.begin_edit();
                        self.paint_line(start,start);
                        self.commit_edit();
                    }
                }
                CanvasGesture::ResizeFrame{id,width,height,direction,..}=>{
                    match self.world.preview_resize(id,width,height,direction){
                        Ok(_)=>{
                            self.begin_edit();
                            match self.world.resize_frame_towards(id,width,height,direction){
                                Ok(())=>{
                                    self.map_width=width;self.map_height=height;
                                    self.resize_guidance=None;
                                    self.message=format!("Map resized to {width} x {height}");
                                }
                                Err(err)=>self.message=err,
                            }
                            self.commit_edit();
                        }
                        Err(err)=>self.message=format!("Cannot resize: {err}"),
                    }
                }
                CanvasGesture::Painting{..} | CanvasGesture::MoveFrame{..}=>{
                    self.commit_edit();
                }
                _=>{}
            }
            if let CanvasGesture::MoveFrame{..}=gesture {
                self.message=if self.preview_error.is_some(){
                    "Map stopped at its last valid position".into()
                }else{"Map moved".into()};
            }
            self.preview_error=None;
        }
    }

    /// Choose a 1/2/5 decade step with legible spacing in screen points.
    /// Coordinates are UI-only world-grid indices: no map data is changed.
    fn coordinate_step(zoom:f32)->i32 {
        for step in [1,2,5,10,20,50,100,200,500,1_000,2_000,
                     5_000,10_000,20_000,50_000,100_000] {
            if zoom*step as f32>=48.0{return step;}
        }
        100_000
    }

    fn draw_coordinate_guides(&self,painter:&egui::Painter,rect:Rect){
        let step=Self::coordinate_step(self.zoom);
        let (left,right,top,bottom)=(
            (-self.offset.x/self.zoom).floor() as i32,
            ((rect.width()-self.offset.x)/self.zoom).ceil() as i32,
            (-self.offset.y/self.zoom).floor() as i32,
            ((rect.height()-self.offset.y)/self.zoom).ceil() as i32);
        let first_x=left.div_euclid(step)*step;
        let first_y=top.div_euclid(step)*step;
        let col=Color32::from_black_alpha(155);
        let font=FontId::proportional(10.0);
        for n in 0..=150 {
            let x=first_x.saturating_add(n*step);
            if x>right {break;}
            let px=self.screen_at(rect,x as f32,0.0).x;
            if px>rect.left()+22.0 && px<rect.right()-20.0 {
                painter.text(pos2(px+2.0,rect.top()+2.0),
                    Align2::LEFT_TOP,x.to_string(),font.clone(),col);
            }
        }
        for n in 0..=150 {
            let y=first_y.saturating_add(n*step);
            if y>bottom {break;}
            let py=self.screen_at(rect,0.0,y as f32).y;
            if py>rect.top()+18.0 && py<rect.bottom()-15.0 {
                painter.text(pos2(rect.left()+3.0,py+1.0),
                    Align2::LEFT_TOP,y.to_string(),font.clone(),col);
            }
        }
        // World (0,0) is the grid CELL, not a fixed-size circle floating
        // at the intersection. Its hollow border scales exactly with zoom.
        let first_cell=Rect::from_min_size(
            self.screen_at(rect,0.0,0.0),Vec2::splat(self.zoom));
        if rect.intersects(first_cell) {
            painter.rect_stroke(first_cell,0.0,
                Stroke::new((self.zoom*0.10).clamp(0.6,2.0),
                    Color32::from_rgb(30,110,195)),StrokeKind::Inside);
        }
    }

    fn draw_canvas(&self, ui: &egui::Ui, rect: Rect, is_editing: bool) {
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, Color32::WHITE);
        let s = self.zoom;
        let start_x = ((-self.offset.x / s).floor() as i32 - 1).max(-200_000);
        let end_x = (((rect.width() - self.offset.x) / s).ceil() as i32 + 1).min(200_000);
        let start_y = ((-self.offset.y / s).floor() as i32 - 1).max(-200_000);
        let end_y = (((rect.height() - self.offset.y) / s).ceil() as i32 + 1).min(200_000);
        let grid_stroke = Stroke::new(0.6, Color32::from_gray(224));
        for x in start_x..=end_x {
            let at = self.screen_at(rect, x as f32, 0.0).x;
            painter.line_segment([pos2(at,rect.top()),pos2(at,rect.bottom())],grid_stroke);
        }
        for y in start_y..=end_y {
            let at = self.screen_at(rect, 0.0,y as f32).y;
            painter.line_segment([pos2(rect.left(),at),pos2(rect.right(),at)],grid_stroke);
        }

        // A parent map may be ghosted behind the active Space near its
        // entry point. It is NEVER hit-testable or painted through.
        if self.show_parent && self.world.active_space_id!=0 {
            if let Some(parent)=self.world.active_space().parent_map_id
                .and_then(|id|self.world.maps.iter().find(|m|m.id==id)) {
                let offset=self.world.active_space().anchor;
                let name=format!("{}/{}",parent.primary_tileset,parent.secondary_tileset);
                let ghost_texture=self.atlas_cache.get(&name).unwrap_or(&self.atlas);
                let from_x=parent.origin.x.saturating_sub(offset.x);
                let from_y=parent.origin.y.saturating_sub(offset.y);
                let x0=((-self.offset.x/s).floor() as i64-i64::from(from_x))
                    .clamp(0,i64::from(parent.width)) as u32;
                let y0=((-self.offset.y/s).floor() as i64-i64::from(from_y))
                    .clamp(0,i64::from(parent.height)) as u32;
                let x1=(((rect.width()-self.offset.x)/s).ceil() as i64-i64::from(from_x))
                    .clamp(0,i64::from(parent.width)) as u32;
                let y1=(((rect.height()-self.offset.y)/s).ceil() as i64-i64::from(from_y))
                    .clamp(0,i64::from(parent.height)) as u32;
                for ly in y0..y1 {
                    for lx in x0..x1 {
                        if let Some(cell)=parent.cell(lx,ly){
                            let min=self.screen_at(rect,
                                from_x as f32+lx as f32,
                                from_y as f32+ly as f32);
                            let spot=Rect::from_min_size(min,Vec2::splat(s));
                            if rect.intersects(spot) {
                                painter.image(ghost_texture.id(),spot,
                                    Self::tile_uv(cell.metatile_id),
                                    Color32::from_white_alpha(73));
                            }
                        }
                    }
                }
                painter.text(self.screen_at(rect,from_x as f32,from_y as f32)+vec2(3.0,3.0),
                    Align2::LEFT_TOP,format!("Underlay: {}",parent.name),
                    FontId::proportional(10.0),Color32::from_gray(140));
            }
        }

        // First: gray frames, painted metatiles, property tint, then labels and events.
        for map in self.world.active_maps() {
            let top_left = self.screen_at(rect,map.origin.x as f32,map.origin.y as f32);
            let mrect = Rect::from_min_size(top_left,vec2(map.width as f32*s,map.height as f32*s));
            if !rect.intersects(mrect) { continue; }
            painter.rect_filled(mrect,0.0,Color32::from_gray(225));
            let pair_key=format!("{}/{}",map.primary_tileset,map.secondary_tileset);
            let map_texture=self.atlas_cache.get(&pair_key).unwrap_or(&self.atlas).id();

            let lx0 = ((-self.offset.x / s).floor() as i32 - map.origin.x).max(0);
            let ly0 = ((-self.offset.y / s).floor() as i32 - map.origin.y).max(0);
            let lx1 = (((rect.width() - self.offset.x) / s).ceil() as i32 - map.origin.x)
                .min(map.width as i32);
            let ly1 = (((rect.height() - self.offset.y) / s).ceil() as i32 - map.origin.y)
                .min(map.height as i32);

            for y in ly0..ly1 {
                for x in lx0..lx1 {
                    if let Some(cell) = map.cell(x as u32,y as u32) {
                        let min = self.screen_at(rect, (map.origin.x+x) as f32,
                            (map.origin.y+y) as f32);
                        let tile_rect = Rect::from_min_size(min,Vec2::splat(s));
                        painter.image(map_texture,tile_rect,Self::tile_uv(cell.metatile_id),Color32::WHITE);
                        if s >= 11.0 {
                            painter.rect_stroke(tile_rect,0.0,
                                Stroke::new(0.5,Color32::from_white_alpha(48)),StrokeKind::Inside);
                        }
                        if self.show_properties {
                            // Exact source cell bits rendered as translucent
                            // color only: no numeric text or invented behavior labels.
                            let (tint,_) = property_visual(cell,self.elevation_tool);
                            painter.rect_filled(tile_rect,0.0,tint);
                        }
                    }
                }
            }
            // Draw map-grid strokes AFTER terrain images, before event art.
            // Previously textures covered all grid lines within maps.
            let grid_color=Color32::from_black_alpha(if s<6.0{78}else{59});
            let grid_line=Stroke::new(if s<6.0{0.8}else{0.7},grid_color);
            if s>=2.0 {
                for gx in lx0.max(0)..=lx1.max(0) {
                    let x=self.screen_at(rect,(map.origin.x+gx) as f32,0.0).x;
                    painter.line_segment([pos2(x,mrect.top().max(rect.top())),
                        pos2(x,mrect.bottom().min(rect.bottom()))],grid_line);
                }
                for gy in ly0.max(0)..=ly1.max(0) {
                    let y=self.screen_at(rect,0.0,(map.origin.y+gy) as f32).y;
                    painter.line_segment([pos2(mrect.left().max(rect.left()),y),
                        pos2(mrect.right().min(rect.right()),y)],grid_line);
                }
            }
            // Visual overlays are independent from interaction mode.
            // Only the respective tab can hit-test or edit each layer.
            if self.show_events {
            for (event_index,event) in map.events.iter().enumerate() {
                let center=self.screen_at(rect,
                    map.origin.x as f32+event.x as f32+0.5,
                    map.origin.y as f32+event.y as f32+0.5);
                if !rect.expand(28.0).contains(center) {continue;}
                let selected=self.mode==Mode::Events
                    && self.selected_event==Some((map.id,event_index));
                let subdued=self.mode!=Mode::Events;
                let alpha=if subdued{178}else{255};
                if event.kind==EventKind::Npc {
                    if let Some(idx)=choices::npc_preview_index(&event.graphics_id){
                        let rows=(choices::NPC_PREVIEW_COUNT+7)/8;
                        let col=(idx%8) as f32;
                        let row=(idx/8) as f32;
                        let uv=Rect::from_min_max(
                            pos2(col/8.0,row/(rows as f32)),
                            pos2((col+1.0)/8.0,(row+1.0)/(rows as f32)));
                        // Pokémon sprite dimensions are proportional to a
                        // 16px metatile: approximately 24x40 source pixels.
                        let height=s*2.5;
                        let width=s*1.5;
                        let bounds=Rect::from_min_max(
                            pos2(center.x-width*0.5,center.y+s*0.5-height),
                            pos2(center.x+width*0.5,center.y+s*0.5));
                        painter.image(self.npc_atlas.id(),bounds,uv,
                            Color32::from_white_alpha(alpha));
                        if selected {
                            // Outline the NPC's map-footprint, not the tall
                            // sprite texture, which looked like an oversized
                            // selection rectangle at low zoom.
                            painter.circle_stroke(center+vec2(0.0,s*0.5),s*0.46,
                                Stroke::new(1.7,Color32::from_rgb(25,108,190)));
                        }
                        continue;
                    }
                }
                // Small event decals: remain inside their real map cell.
                // A sign/warp/step trigger must not hide the underlying art.
                let f=s/24.0;
                let size=s*0.78;
                let icon=Rect::from_center_size(center,Vec2::splat(size));
                match event.kind {
                    EventKind::HiddenItem=>{
                        if let Some(index)=choices::hidden_item_preview(&event.hidden_item){
                            let col=(index%8) as f32;
                            let row=(index/8) as f32;
                            let uv=Rect::from_min_max(pos2(col/8.0,row/2.0),
                                pos2((col+1.0)/8.0,(row+1.0)/2.0));
                            painter.image(self.item_atlas.id(),icon,uv,
                                Color32::from_white_alpha(alpha));
                        }else{
                            // Before a source item is chosen, show a parcel
                            // outline, never the old generic question circle.
                            painter.rect_filled(icon.shrink(3.0*f),2.0,
                                Color32::from_rgb(205,163,80));
                            painter.line_segment([center+vec2(-5.0*f,0.0*f),
                                center+vec2(5.0*f,0.0*f)],
                                Stroke::new(1.4,Color32::from_rgb(112,72,27)));
                            painter.line_segment([center+vec2(0.0*f,-6.0*f),
                                center+vec2(0.0*f,6.0*f)],
                                Stroke::new(1.4,Color32::from_rgb(112,72,27)));
                        }
                    }
                    EventKind::Sign=>{
                        let brown=Color32::from_rgb(127,82,41);
                        painter.rect_filled(Rect::from_center_size(
                            center+vec2(0.0*f,5.0*f),vec2(3.5*f,13.0*f)),1.0,brown);
                        painter.rect_filled(Rect::from_center_size(
                            center+vec2(0.0*f,-3.5*f),vec2(18.0*f,11.0*f)),2.0,brown);
                        painter.rect_filled(Rect::from_center_size(
                            center+vec2(0.0*f,-3.5*f),vec2(15.0*f,8.0*f)),1.0,
                            Color32::from_rgb(241,208,142));
                        painter.line_segment([center+vec2(-4.0*f,-5.0*f),
                            center+vec2(4.0*f,-5.0*f)],
                            Stroke::new(1.2,Color32::from_rgb(128,83,44)));
                    }
                    EventKind::Warp=>{
                        let blue=Color32::from_rgb(45,119,194);
                        let door=Rect::from_center_size(center,vec2(16.0*f,19.0*f));
                        painter.rect_filled(door,2.0,Color32::from_rgb(36,69,110));
                        painter.rect_filled(door.shrink(2.5),1.0,
                            Color32::from_rgb(158,214,250));
                        painter.text(center,Align2::CENTER_CENTER,
                            if event.arrival_only{"↓"}else if event.warp_mode==WarpMode::Direct{"↗"}else{"→"},
                            FontId::proportional(13.0*f),blue);
                    }
                    EventKind::Trigger=>{
                        let points=vec![center+vec2(0.0*f,-11.0*f),
                            center+vec2(11.0*f,0.0*f),center+vec2(0.0*f,11.0*f),
                            center+vec2(-11.0*f,0.0*f)];
                        painter.add(egui::Shape::convex_polygon(points,
                            Color32::from_rgb(195,69,106),Stroke::NONE));
                        painter.text(center,Align2::CENTER_CENTER,"!",
                            FontId::proportional(14.0*f),Color32::WHITE);
                    }
                    EventKind::Npc=>{
                        painter.circle_filled(center,8.0*f,Color32::from_rgb(122,88,168));
                        painter.text(center,Align2::CENTER_CENTER,"N",
                            FontId::proportional(12.0*f),Color32::WHITE);
                    }
                }
                if selected{
                    painter.rect_stroke(icon.expand(2.0*f),3.0*f,
                        Stroke::new(1.3,Color32::from_rgb(20,100,193)),
                        StrokeKind::Outside);
                }
            }
            for marker in &map.markers {
                let center=self.screen_at(rect,
                    map.origin.x as f32+marker.x as f32+0.5,
                    map.origin.y as f32+marker.y as f32+0.5);
                if !rect.contains(center){continue;}
                painter.circle_filled(center,(s*0.38).max(1.0),Color32::from_rgb(117,66,179));
                painter.text(center,Align2::CENTER_CENTER,"!",
                    FontId::proportional(s*0.55),Color32::WHITE);
            }
            } // end Events-only markers
            painter.text(top_left + vec2(5.0,5.0),Align2::LEFT_TOP,&map.name,
                FontId::proportional(11.0),Color32::from_black_alpha(if is_editing {48} else {160}));
        }

        // Highlight travel links only when event overlays are enabled.
        if self.show_events && self.mode==Mode::Events {
        if let Some((from_id,event_index))=self.selected_event {
            if let Some(map)=self.world.maps.iter().find(|m|m.id==from_id) {
                if let Some(warp)=map.events.get(event_index).filter(|e|e.kind==EventKind::Warp){
                    if let Some(target)=self.world.active_maps().find(|m|m.id==warp.dest_map_id) {
                        let to=if warp.warp_mode==WarpMode::Direct {
                            Some((warp.dest_x,warp.dest_y))
                        }else{
                            target.events.iter().filter(|e|e.kind==EventKind::Warp &&
                                e.warp_mode!=WarpMode::Direct)
                                .nth(usize::from(warp.dest_warp_id))
                                .map(|e|(e.x,e.y))
                        };
                        if let Some((tx,ty))=to {
                            let source=self.screen_at(rect,
                                map.origin.x as f32+warp.x as f32+0.5,
                                map.origin.y as f32+warp.y as f32+0.5);
                            let destination=self.screen_at(rect,
                                target.origin.x as f32+tx as f32+0.5,
                                target.origin.y as f32+ty as f32+0.5);
                            painter.line_segment([source,destination],
                                Stroke::new(1.7,Color32::from_rgb(39,105,195)));
                        }
                    }
                }
            }
        }

        }
        if self.show_events && self.mode==Mode::Events {
            if let Some(p)=self.warp_focus{
                let cell=Rect::from_min_size(self.screen_at(rect,p.x as f32,p.y as f32),
                    Vec2::splat(s));
                if rect.intersects(cell){
                    painter.rect_stroke(cell.expand(2.0),2.0,
                        Stroke::new(2.2,Color32::from_rgb(29,119,211)),
                        StrokeKind::Outside);
                }
            }
        }
        if self.show_properties && self.mode==Mode::Properties {
            if let Some(p)=self.selected_cell {
                if self.world.map_at(p).is_some() {
                    let at=self.screen_at(rect,p.x as f32,p.y as f32);
                    let cell_rect=Rect::from_min_size(at,Vec2::splat(s));
                    // The outline follows the real map cell at every zoom.
                    let selected=cell_rect;
                    if rect.intersects(selected) {
                        painter.rect_stroke(selected,0.0,
                            Stroke::new(2.0,Color32::from_rgb(37,62,121)),
                            StrokeKind::Inside);
                    }
                }
            }
        }
        // Visual feedback appears during the *gesture*, before anything is committed.
        if let CanvasGesture::DrawFrame{from,to}=self.gesture {
            let (origin,w,h)=Self::bounds(from,to);
            let preview=Rect::from_min_size(self.screen_at(rect,origin.x as f32,
                origin.y as f32),vec2(w as f32*s,h as f32*s));
            let blocked=w<2||h<2||self.world.can_place_frame(origin,w,h).is_err();
            let outline=if blocked {Color32::from_rgb(220,57,57)}
                else {Color32::from_rgb(81,137,224)};
            painter.rect_filled(preview,0.0,if blocked{
                Color32::from_rgba_unmultiplied(218,56,56,40)
            }else{
                Color32::from_rgba_unmultiplied(69,128,206,34)
            });
            painter.rect_stroke(preview,0.0,Stroke::new(3.0,outline),StrokeKind::Inside);
            painter.text(preview.left_top()+vec2(5.0,5.0),Align2::LEFT_TOP,
                format!("{w} × {h}"),FontId::proportional(13.0),outline);
        }
        if let Some(err)=&self.preview_error {
            let box_rect=Rect::from_min_size(rect.left_top()+vec2(8.0,8.0),
                vec2((err.chars().count() as f32*7.7+24.0).min(rect.width()-16.0),38.0));
            painter.rect_filled(box_rect,5.0,Color32::from_rgb(255,226,225));
            painter.text(box_rect.center(),Align2::CENTER_CENTER,err,
                FontId::proportional(12.0),Color32::from_rgb(154,29,29));
        }
        // Interactive edge preview is non-destructive until the finger
        // releases. Blocked geometry is red; valid expansion is green.
        if let CanvasGesture::ResizeFrame{id,width,height,direction,..}=self.gesture {
            if let Some((origin,w,h))=self.world.resize_ghost(id,width,height,direction){
                let from=self.screen_at(rect,origin.x as f32,origin.y as f32);
                let target=Rect::from_min_size(from,vec2(w as f32*s,h as f32*s));
                if rect.intersects(target){
                    let valid=self.world.preview_resize(id,w,h,direction).is_ok();
                    let col=if valid{Color32::from_rgb(42,152,94)}
                        else{Color32::from_rgb(210,62,62)};
                    painter.rect_filled(target,0.0,
                        if valid{Color32::from_rgba_unmultiplied(39,157,97,42)}
                        else{Color32::from_rgba_unmultiplied(206,53,53,60)});
                    painter.rect_stroke(target,0.0,Stroke::new(2.4,col),StrokeKind::Inside);
                    painter.text(target.center(),Align2::CENTER_CENTER,
                        format!("{w} x {h}"),FontId::proportional(12.0),col);
                }
            }
        }
        // Outlines drawn LAST, above terrain, properties, labels, and markers.
        let colors = [
            Color32::from_rgb(166,64,79),
            Color32::from_rgb(190,145,32),
            Color32::from_rgb(49,150,95),
            Color32::from_rgb(58,112,190),
        ];
        for map in self.world.active_maps() {
            let min = self.screen_at(rect,map.origin.x as f32,map.origin.y as f32);
            let frame_rect = Rect::from_min_size(min,vec2(map.width as f32*s,map.height as f32*s));
            if rect.intersects(frame_rect) {
                let col = colors[((map.id-1) as usize)%colors.len()];
                if self.selected_map==Some(map.id) {
                    painter.rect_stroke(frame_rect.expand(1.0),0.0,
                        Stroke::new(4.0,Color32::from_rgb(25,111,207)),
                        StrokeKind::Outside);
                    painter.rect_stroke(frame_rect,0.0,
                        Stroke::new(1.1,Color32::WHITE),StrokeKind::Inside);
                }else{
                    painter.rect_stroke(frame_rect,0.0,
                        Stroke::new(1.7,col),StrokeKind::Inside);
                }
            }
        }
        self.draw_coordinate_guides(&painter,rect);
    }

    fn canvas(&mut self, ui: &mut egui::Ui) {
        let size = ui.available_size().max(vec2(8.0,8.0));
        self.canvas_size=size;
        let (rect,response) = ui.allocate_exact_size(size,Sense::click_and_drag());
        self.handle_canvas_input(ui,rect,&response);
        let editing = !matches!(self.gesture,CanvasGesture::Idle);
        self.draw_canvas(ui,rect,editing);
    }
}

impl eframe::App for MapStudioApp {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let window=root.ctx().content_rect().size();
        if (window-self.last_window_size).length()>2.0 {
            // A resized native activity must never keep a half-completed
            // gesture from before Android split/freeform-window changes.
            self.gesture=CanvasGesture::Idle;
            self.multitouch_in_progress=false;
            self.last_window_size=window;
            root.ctx().request_repaint();
        }
        #[cfg(target_os = "android")]
        if window.y>580.0 {
            egui::Panel::top("status_inset")
                .show(root,|ui| { ui.add_space(12.0); });
        }
        egui::Panel::top("header").show(root,|ui|self.header(ui));
        self.ensure_atlas_cache(root.ctx());
        let viewport=root.available_height().max(48.0);
        // Leave meaningful canvas space in short split-screen windows.
        let minimum=(viewport*0.19).clamp(36.0,86.0);
        let maximum=(viewport*0.55).max(minimum);
        self.panel_height=self.panel_height.clamp(minimum,maximum);
        egui::Panel::bottom("palette")
            .resizable(false).exact_size(self.panel_height)
            .show(root,|ui|self.palette(ui));
        egui::CentralPanel::default().show(root,|ui|self.canvas(ui));
        // A small independent panel: checkbox clicks do NOT dismiss it.
        // Close using X, the Overlays button, or a tap outside its bounds.
        if self.overlays_open {
            let mut open=true;
            let popup=egui::Window::new("Overlay visibility")
                .id(egui::Id::new("overlay_visibility"))
                .open(&mut open).collapsible(false).resizable(false)
                .anchor(Align2::RIGHT_TOP,[-10.0,92.0])
                .show(root.ctx(),|ui|{
                    ui.checkbox(&mut self.show_events,"Show events");
                    ui.checkbox(&mut self.show_properties,"Show properties");
                    ui.small("The active tab controls editing.");
                });
            if !open {self.overlays_open=false;}
            if let Some(popup)=popup{
                let outside=root.ctx().input(|input|
                    input.pointer.any_pressed()
                        && input.pointer.interact_pos().is_some_and(|point|
                            !popup.response.rect.contains(point)
                            && !self.overlays_button.is_some_and(|r|r.contains(point))));
                if outside {self.overlays_open=false;}
            }
        }
        self.size_dialog(root.ctx());
        self.project_dialogs(root.ctx());
        self.maybe_autosave(root.ctx());
    }
}

fn property_visual(cell: Cell,elevation_mode:bool)->(Color32,&'static str){
    const VALUES:[&str;16]=[
        "0","1","2","3","4","5","6","7","8","9","10","11","12","13","14","15"
    ];
    if elevation_mode {
        let e=cell.elevation.min(15);
        // Sixteen source elevation values, rendered as an accessible
        // blue→teal→green→yellow→coral→violet gradient.
        // Colors are visualization only; stored elevation is unchanged.
        const RAINBOW:[(u8,u8,u8);16]=[
            (57,110,220),(51,138,221),(45,164,210),(41,183,188),
            (38,180,150),(58,185,113),(97,193,79),(147,200,65),
            (193,202,65),(223,185,66),(233,153,70),(230,124,84),
            (225,94,115),(205,85,158),(174,85,198),(133,100,220),
        ];
        let (r,g,b)=RAINBOW[e as usize];
        (Color32::from_rgba_unmultiplied(r,g,b,102),VALUES[e as usize])
    }else{
        let tint=match cell.collision {
            0=>Color32::from_rgba_unmultiplied(55,196,96,88),
            1=>Color32::from_rgba_unmultiplied(230,67,67,112),
            2=>Color32::from_rgba_unmultiplied(152,99,207,106),
            _=>Color32::from_rgba_unmultiplied(242,162,61,106),
        };
        (tint,VALUES[cell.collision.min(3) as usize])
    }
}

#[cfg(test)]
mod properties_visual_tests {
    use super::*;
    use crate::model::TileKind;
    #[test]
    fn collision_values_use_original_numbers() {
        let mut cell=Cell::new(TileKind::Grass);
        for collision in 0..4 {
            cell.collision=collision;
            let (_,label)=property_visual(cell,false);
            assert_eq!(label,collision.to_string());
        }
    }
    #[test]
    fn elevation_values_use_direct_selection_numbers() {
        let mut cell=Cell::new(TileKind::Grass);
        for height in 0..16 {
            cell.elevation=height;
            let (_,label)=property_visual(cell,true);
            assert_eq!(label,height.to_string());
        }
    }
}

#[cfg(test)]
mod gesture_intent_tests {
    use super::MapStudioApp;
    #[test]
    fn hold_vs_drag_is_unambiguous() {
        assert!(!MapStudioApp::long_hold_complete(10.0,10.25,0.0));
        assert!(MapStudioApp::long_hold_complete(10.0,10.47,1.0));
        assert!(!MapStudioApp::long_hold_complete(10.0,10.60,10.0));
        assert!(!MapStudioApp::long_hold_complete(10.0,10.20,20.0));
    }
}

#[cfg(test)]
mod coordinate_guide_tests {
    use super::MapStudioApp;
    #[test]
    fn labels_skip_by_a_1_2_5_interval_as_zoom_changes() {
        assert_eq!(MapStudioApp::coordinate_step(54.0),1);
        assert_eq!(MapStudioApp::coordinate_step(24.0),2);
        assert_eq!(MapStudioApp::coordinate_step(14.0),5);
        assert_eq!(MapStudioApp::coordinate_step(5.0),10);
        assert_eq!(MapStudioApp::coordinate_step(2.0),50);
    }
}
