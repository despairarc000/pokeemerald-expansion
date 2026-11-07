//! Native egui map editor with an atlas of real Emerald metatiles.
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2, pos2, vec2};
use crate::model::{Cell, EventKind, MapEvent, GridPoint, ExpandDirection, World};
use crate::{storage, export, tiles, choices};
use std::{path::PathBuf, time::{Duration, Instant}};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode { Terrain, Properties, Events }

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
}

pub struct MapStudioApp {
    world: World,
    mode: Mode,
    event_brush: EventKind,
    selected_event: Option<(u32,usize)>,
    event_editor_open: bool,
    brush_id: u16,
    erase_tool: bool,
    elevation_brush: u8,
    elevation_tool: bool,
    atlas: egui::TextureHandle,
    palette_page: usize,
    palette_search: String,
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
    size_panel_open: bool,
    zoom: f32,
    offset: Vec2,
    canvas_size: Vec2,
    multitouch_in_progress: bool,
    selected_map: Option<u32>,
    selected_cell: Option<GridPoint>,
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
            brush_id: 1,
            erase_tool: false,
            elevation_brush: 3,
            elevation_tool: false,
            atlas: texture,
            palette_page: 0,
            palette_search: String::new(),
            save_path,
            export_dir,
            dirty_since: None,
            history: Vec::new(),
            redo: Vec::new(),
            before_gesture: None,
            gesture: CanvasGesture::Idle,
            preview_error: None,
            collision_brush: 0,
            panel_height: 205.0,
            map_width: 12,
            map_height: 10,
            expand_direction: ExpandDirection::LowerRight,
            size_panel_open: false,
            zoom: 14.0,
            offset: vec2(28.0, 52.0),
            canvas_size: vec2(360.0,540.0),
            multitouch_in_progress: false,
            selected_map: None,
            selected_cell: None,
            confirm_delete: None,
            message,
            project_restore_open: false,
            available_backups: Vec::new(),
            selected_backup: None,
            new_project_confirm: false,
        }
    }

    fn header(&mut self, ui: &mut egui::Ui) {
        ui.add_space(5.0);
        ui.horizontal(|ui| {
            ui.heading("Map Studio");
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
                if ui.small_button("Redo").clicked() { self.redo(); }
                if ui.small_button("Undo").clicked() { self.undo(); }
            });
        });
        ui.horizontal(|ui| {
            if ui.selectable_label(self.mode == Mode::Terrain, "Terrain").clicked() {
                self.mode = Mode::Terrain; self.gesture = CanvasGesture::Idle;
            }
            if ui.selectable_label(self.mode == Mode::Properties, "Properties").clicked() {
                self.mode = Mode::Properties; self.gesture = CanvasGesture::Idle;
            }
            if ui.selectable_label(self.mode == Mode::Events, "Events").clicked() {
                self.mode = Mode::Events; self.gesture = CanvasGesture::Idle;
            }
        });
        ui.horizontal(|ui| {
            if ui.add_enabled(self.selected_map.is_some(),egui::Button::new("Size")).clicked(){
                if let Some(map)=self.world.maps.iter().find(|m|Some(m.id)==self.selected_map){
                    self.map_width=map.width;
                    self.map_height=map.height;
                }
                self.size_panel_open=true;
            }
            if ui.small_button("Fit").clicked() { self.fit_world(); }
            ui.separator();
            ui.label(egui::RichText::new(format!("{} maps", self.world.maps.len()))
                .small().color(Color32::GRAY));
        });
        ui.add_space(4.0);
    }

    fn fit_world(&mut self){
        if self.world.maps.is_empty() || self.canvas_size.x<40.0 || self.canvas_size.y<40.0 {
            return;
        }
        let min_x=self.world.maps.iter().map(|m|i64::from(m.origin.x)).min().unwrap();
        let min_y=self.world.maps.iter().map(|m|i64::from(m.origin.y)).min().unwrap();
        let max_x=self.world.maps.iter().map(|m|i64::from(m.origin.x)+i64::from(m.width)).max().unwrap();
        let max_y=self.world.maps.iter().map(|m|i64::from(m.origin.y)+i64::from(m.height)).max().unwrap();
        let width=(max_x-min_x).max(1) as f32;
        let height=(max_y-min_y).max(1) as f32;
        let padding=24.0;
        self.zoom=((self.canvas_size.x-2.0*padding)/width)
            .min((self.canvas_size.y-2.0*padding)/height).clamp(5.0,54.0);
        self.offset=vec2(
            (self.canvas_size.x-width*self.zoom)*0.5 - min_x as f32*self.zoom,
            (self.canvas_size.y-height*self.zoom)*0.5 - min_y as f32*self.zoom,
        );
    }

    fn size_dialog(&mut self,ctx:&egui::Context) {
        if !self.size_panel_open {return;}
        let mut open=self.size_panel_open;
        egui::Window::new("Adjust map size")
            .id(egui::Id::new("map_size_dialog"))
            .open(&mut open)
            .resizable(false).collapsible(false)
            .anchor(Align2::CENTER_TOP,[0.0,145.0])
            .show(ctx,|ui|{
                ui.horizontal(|ui|{
                    ui.label("Wide");
                    ui.add(egui::DragValue::new(&mut self.map_width).range(1..=100).speed(1));
                    ui.label("Tall");
                    ui.add(egui::DragValue::new(&mut self.map_height).range(1..=100).speed(1));
                });
                ui.label("Grow toward");
                egui::Grid::new("expansion_directions").num_columns(2).spacing([8.0,8.0])
                    .show(ui,|ui|{
                        ui.selectable_value(&mut self.expand_direction,ExpandDirection::UpperLeft,"↖ Upper left");
                        ui.selectable_value(&mut self.expand_direction,ExpandDirection::UpperRight,"↗ Upper right");
                        ui.end_row();
                        ui.selectable_value(&mut self.expand_direction,ExpandDirection::LowerLeft,"↙ Lower left");
                        ui.selectable_value(&mut self.expand_direction,ExpandDirection::LowerRight,"↘ Lower right");
                        ui.end_row();
                    });
                if let Some(id)=self.selected_map {
                    // Recalculate for every keystroke: warn *before* Apply is pressed.
                    let test=self.world.preview_resize(id,self.map_width,self.map_height,self.expand_direction);
                    match &test {
                        Ok(_)=>{ui.small("Preview valid — existing painted tiles will stay in place.");}
                        Err(err)=>{ui.colored_label(Color32::from_rgb(195,52,52),format!("⚠ {err}"));}
                    }
                    if ui.add_enabled(test.is_ok(),egui::Button::new("Apply size")).clicked(){
                        self.begin_edit();
                        match self.world.resize_frame_towards(id,self.map_width,self.map_height,self.expand_direction) {
                            Ok(())=>{
                                self.message="Map resized.".into();
                                self.size_panel_open=false;
                            }
                            Err(err)=>self.message=err,
                        }
                        self.commit_edit();
                    }
                }
            });
        self.size_panel_open &= open;
    }

    fn palette(&mut self, ui: &mut egui::Ui) {
        // Large touch target: dragging the *top edge* resizes this dock, not the canvas.
        let (handle,response)=ui.allocate_exact_size(vec2(ui.available_width(),38.0),
            Sense::click_and_drag());
        let center=handle.center();
        ui.painter().rect_filled(handle,0.0,Color32::from_gray(243));
        ui.painter().rect_filled(Rect::from_center_size(
            pos2(center.x,center.y-9.0),vec2(58.0,4.0)),3.0,Color32::from_gray(150));
        ui.painter().text(pos2(center.x,center.y+5.0),Align2::CENTER_CENTER,
            "Drag to resize",FontId::proportional(10.0),Color32::DARK_GRAY);
        if response.dragged() {
            self.panel_height=(self.panel_height-response.drag_delta().y)
                .clamp(126.0,ui.ctx().content_rect().height()*0.72);
            ui.ctx().request_repaint();
        }
        if response.clicked() {
            self.panel_height=if self.panel_height<260.0 {390.0}else{185.0};
        }

        egui::ScrollArea::vertical().id_salt("dock_content")
            .auto_shrink([false,false]).show(ui,|ui|{
                self.palette_content(ui);
            });
    }

    fn palette_content(&mut self, ui: &mut egui::Ui) {
        match self.mode {
            Mode::Terrain => {
                ui.horizontal_wrapped(|ui|{
                    ui.strong("Tiles");
                    ui.toggle_value(&mut self.erase_tool,"Erase");
                    if ui.small_button("Fill empty").clicked(){ self.fill_selected(); }
                    if ui.small_button("◀").clicked(){self.palette_page=self.palette_page.saturating_sub(1);}
                    let count=tiles::all_metatiles().len();
                    if ui.small_button("▶").clicked(){
                        self.palette_page=(self.palette_page+1).min(count.saturating_sub(1)/32);
                    }
                });
                ui.horizontal(|ui|{
                    ui.label("Find tile ID");
                    ui.add(egui::TextEdit::singleline(&mut self.palette_search)
                        .desired_width(75.0).hint_text("e.g. 368"));
                });
                let available=tiles::all_metatiles();
                let displayed:Vec<u16>=if let Ok(id)=self.palette_search.trim().parse::<u16>() {
                    available.into_iter().filter(|candidate|*candidate==id).collect()
                }else{
                    let start=(self.palette_page*32).min(available.len());
                    available.into_iter().skip(start).take(32).collect()
                };
                egui::ScrollArea::horizontal().id_salt("metatile_palette").show(ui,|ui|{
                    ui.horizontal(|ui|{
                        for id in displayed {
                            ui.vertical(|ui|{
                                let (tile,response)=ui.allocate_exact_size(vec2(52.0,52.0),Sense::click());
                                ui.painter().rect_filled(tile,5.0,if id==self.brush_id&&!self.erase_tool
                                    {Color32::from_rgb(153,208,241)}else{Color32::from_gray(231)});
                                ui.painter().image(self.atlas.id(),tile.shrink(5.0),
                                    Self::tile_uv(id),Color32::WHITE);
                                if response.clicked(){self.brush_id=id;self.erase_tool=false;}
                                ui.label(egui::RichText::new(format!("{id}")).small());
                            });
                        }
                    });
                });
            }
            Mode::Properties => {
                if let Some(id)=self.selected_map {
                    let before=self.world.clone();
                    let mut changed=false;
                    let mut delete=false;
                    if let Some(map)=self.world.maps.iter_mut().find(|m|m.id==id){
                        ui.horizontal(|ui|{
                            ui.label("Map name");
                            changed=ui.add(egui::TextEdit::singleline(&mut map.name)
                                .desired_width(130.0)).changed();
                            ui.label(format!("{} × {}",map.width,map.height));
                            if ui.small_button("Delete").clicked(){delete=true;}
                        });
                    }
                    if changed{self.before_gesture=Some(before);self.commit_edit();}
                    if delete{self.confirm_delete=Some(id);}
                    if self.confirm_delete==Some(id){
                        ui.horizontal(|ui|{
                            ui.strong("Delete this map?");
                            if ui.button("Delete map").clicked(){
                                self.begin_edit();
                                match self.world.remove_frame(id){
                                    Ok(())=>{
                                        self.selected_map=None;
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
                }
                ui.horizontal(|ui|{
                    ui.selectable_value(&mut self.elevation_tool,false,"Collision");
                    ui.selectable_value(&mut self.elevation_tool,true,"Elevation");
                });
                if let Some(p)=self.selected_cell {
                    if let Some(map)=self.world.map_at(p) {
                        let (x,y)=((p.x-map.origin.x) as u32,(p.y-map.origin.y) as u32);
                        if let Some(cell)=map.cell(x,y) {
                            let behavior=tiles::friendly_behavior(cell.metatile_id).unwrap_or("Normal");
                            ui.label(egui::RichText::new(format!(
                                "{} ({x},{y}) · Collision {} · Height {} · {behavior}",
                                map.name,cell.collision,cell.elevation,
                            )).small().color(Color32::DARK_GRAY));
                        }
                    }
                }
                if self.elevation_tool {
                    ui.strong("Choose height");
                    egui::Grid::new("height_choices").num_columns(4).spacing([9.0,6.0])
                        .show(ui,|ui|{
                            for elevation in 0..16u8 {
                                let label=match elevation {
                                    0=>"0 · Steps".to_string(),
                                    1=>"1 · Water".to_string(),
                                    3=>"3 · Land".to_string(),
                                    15=>"15 · Bridge".to_string(),
                                    _=>elevation.to_string(),
                                };
                                if ui.selectable_label(self.elevation_brush==elevation,label).clicked(){
                                    self.elevation_brush=elevation;
                                }
                                if elevation%4==3{ui.end_row();}
                            }
                        });
                    ui.small("Height controls game-level traversal. Colors show the selected levels.");
                }else{
                    ui.horizontal_wrapped(|ui|{
                        for (code,label) in [(0,"0 · Walk"),(1,"1 · Block"),(2,"2"),(3,"3")]{
                            ui.selectable_value(&mut self.collision_brush,code,label);
                        }
                    });
                    ui.small("~ water · ❄ ice · arrows mark ledges — read from the actual tile.");
                }
            }
            Mode::Events => {
                ui.horizontal(|ui|{
                    ui.strong("Add event");
                    ui.selectable_value(&mut self.event_brush,EventKind::Npc,"Person");
                    ui.selectable_value(&mut self.event_brush,EventKind::Sign,"Sign");
                });
                if let Some((map_id,index))=self.selected_event {
                    let before=self.world.clone();
                    let mut changed=false;
                    let mut remove=false;
                    let mut found=false;
                    if let Some(event)=self.world.maps.iter_mut().find(|m|m.id==map_id)
                        .and_then(|m|m.events.get_mut(index)){
                        found=true;
                        ui.horizontal_wrapped(|ui|{
                            ui.strong(match event.kind{
                                EventKind::Npc=>"Person", EventKind::Sign=>"Sign",
                                EventKind::Warp=>"Warp", EventKind::Trigger=>"Trigger"
                            });
                            if ui.small_button(if self.event_editor_open{"Less"}else{"Edit"}).clicked(){
                                self.event_editor_open=!self.event_editor_open;
                            }
                            if ui.small_button("Remove").clicked(){remove=true;}
                        });
                        if matches!(event.kind,EventKind::Npc|EventKind::Sign){
                            ui.horizontal(|ui|{
                                ui.label("Type");
                                changed|=ui.selectable_value(&mut event.kind,EventKind::Npc,"Person").changed();
                                changed|=ui.selectable_value(&mut event.kind,EventKind::Sign,"Sign").changed();
                            });
                        }
                        if self.event_editor_open {
                            match event.kind {
                                EventKind::Npc => {
                                    ui.horizontal(|ui|{
                                        ui.label("Character");
                                        let shown=choices::npc_friendly(&event.graphics_id)
                                            .unwrap_or("Choose a character");
                                        egui::ComboBox::from_id_salt("npc_character")
                                            .selected_text(shown).show_ui(ui,|ui|{
                                                for (label,symbol) in choices::NPC_SPRITES {
                                                    changed|=ui.selectable_value(
                                                        &mut event.graphics_id,(*symbol).to_string(),*label
                                                    ).changed();
                                                }
                                            });
                                    });
                                    ui.horizontal(|ui|{
                                        ui.label("Says");
                                        changed|=ui.add(egui::TextEdit::singleline(&mut event.dialogue)
                                            .desired_width(200.0).hint_text("Enter dialogue")).changed();
                                    });
                                }
                                EventKind::Sign => {
                                    ui.horizontal(|ui|{
                                        ui.label("Sign text");
                                        changed|=ui.add(egui::TextEdit::singleline(&mut event.dialogue)
                                            .desired_width(220.0).hint_text("Enter sign text")).changed();
                                    });
                                }
                                EventKind::Warp|EventKind::Trigger=>{
                                    ui.label("Advanced event details will be discussed separately.");
                                }
                            }
                            if matches!(event.kind,EventKind::Npc|EventKind::Sign){
                                ui.horizontal(|ui|{
                                    ui.label("Event height");
                                    egui::ComboBox::from_id_salt("event_elevation")
                                        .selected_text(event.elevation.to_string())
                                        .show_ui(ui,|ui|{
                                            for h in 0..=15u8 {
                                                changed|=ui.selectable_value(
                                                    &mut event.elevation,h,h.to_string()
                                                ).changed();
                                            }
                                        });
                                });
                            }
                        }else{
                            ui.small("Tap event to select · double-tap to edit.");
                        }
                    }
                    if changed {
                        self.before_gesture=Some(before);
                        self.commit_edit();
                    }
                    if remove {
                        self.begin_edit();
                        self.world.remove_event(map_id,index);
                        self.commit_edit();
                        self.selected_event=None;
                        self.event_editor_open=false;
                    }else if !found{
                        self.selected_event=None;
                        self.event_editor_open=false;
                    }
                }else{
                    ui.small("Tap on a map to add or select. Warps are postponed.");
                }
            }
        }
        ui.label(egui::RichText::new(&self.message).small().color(Color32::DARK_GRAY));
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
        let new_zoom = (old_zoom * factor).clamp(5.0, 54.0);
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
                Mode::Terrain => { if self.erase_tool { self.world.erase(at); } else {
                    self.world.paint_metatile(at, self.brush_id); } }
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
                if let Some(m)=self.world.map_at(at) {
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
                    CanvasGesture::EmptyPress{at:start,screen:origin,since}=>{
                        let delta=(screen-origin).length();
                        if Self::long_hold_complete(since,now,delta) {
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
                        self.preview_error=self.world.can_place_frame(origin,w,h).err();
                        ui.ctx().request_repaint();
                    }
                    CanvasGesture::MapPress{id,at:start,screen:origin,since,origin:map_origin}=>{
                        let delta=(screen-origin).length();
                        if Self::long_hold_complete(since,now,delta) {
                            self.gesture=CanvasGesture::MoveFrame{id,from:start,origin:map_origin};
                            self.begin_edit();
                            self.message="Slide the map to move it".into();
                            ui.ctx().request_repaint();
                        }else if delta>DRAG_PX {
                            if self.mode==Mode::Events{
                                // Events aren't painted by dragging a finger.
                                self.gesture=CanvasGesture::Painting{last:start};
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
                CanvasGesture::MapPress{at:start,..}=>{
                    if self.mode==Mode::Events {
                        if let Some(point)=at {
                            if self.world.map_at(point).is_some() {
                                self.begin_edit();
                                let was_existing=self.world.map_at(point)
                                    .and_then(|map|map.events.iter()
                                        .position(|ev|
                                            ev.x==(point.x-map.origin.x) as u32 &&
                                            ev.y==(point.y-map.origin.y) as u32)).is_some();
                                self.selected_event=self.world.add_event(point,self.event_brush);
                                self.event_editor_open=response.double_clicked() && was_existing;
                                self.commit_edit();
                            }
                        }
                    }else{
                        self.begin_edit();
                        self.paint_line(start,start);
                        self.commit_edit();
                    }
                }
                CanvasGesture::Painting{..} | CanvasGesture::MoveFrame{..}=>{
                    self.commit_edit();
                }
                _=>{}
            }
            self.preview_error=None;
            if let CanvasGesture::MoveFrame{..}=gesture {
                self.message="Map position updated".into();
            }
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

        // First: gray frames, painted metatiles, property tint, then labels and events.
        for map in &self.world.maps {
            let top_left = self.screen_at(rect,map.origin.x as f32,map.origin.y as f32);
            let mrect = Rect::from_min_size(top_left,vec2(map.width as f32*s,map.height as f32*s));
            if !rect.intersects(mrect) { continue; }
            painter.rect_filled(mrect,0.0,Color32::from_gray(225));

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
                        painter.image(self.atlas.id(),tile_rect,Self::tile_uv(cell.metatile_id),Color32::WHITE);
                        if s >= 11.0 {
                            painter.rect_stroke(tile_rect,0.0,
                                Stroke::new(0.5,Color32::from_white_alpha(48)),StrokeKind::Inside);
                        }
                        if self.mode == Mode::Properties {
                            let (tint,symbol) = property_visual(cell,self.elevation_tool);
                            painter.rect_filled(tile_rect,0.0,tint);
                            // Water, ice and ledges are prominent at the *default* zoom;
                            // collision bits remain visible in the corner of those tiles.
                            let behavior=if self.elevation_tool{None}
                                else {tiles::behavior_overlay(cell.metatile_id)};
                            if s>=10.0 {
                                painter.text(tile_rect.center(),Align2::CENTER_CENTER,
                                    behavior.unwrap_or(symbol),
                                    FontId::proportional((s*0.58).clamp(8.0,17.0)),
                                    Color32::from_black_alpha(230));
                                if behavior.is_some() && s>=13.0 {
                                    painter.text(tile_rect.right_bottom()+vec2(-1.5,-1.5),
                                        Align2::RIGHT_BOTTOM,symbol,
                                        FontId::proportional((s*0.35).clamp(7.0,11.0)),
                                        Color32::from_rgb(19,49,91));
                                }
                            }
                        }
                    }
                }
            }
            if s >= 9.0 {
                for event in &map.events {
                    let center=self.screen_at(rect,
                        map.origin.x as f32+event.x as f32+0.5,
                        map.origin.y as f32+event.y as f32+0.5);
                    let (symbol,color)=match event.kind {
                        EventKind::Npc=>("N",Color32::from_rgb(121,82,193)),
                        EventKind::Sign=>("S",Color32::from_rgb(179,124,48)),
                        EventKind::Warp=>("W",Color32::from_rgb(55,128,203)),
                        EventKind::Trigger=>("T",Color32::from_rgb(187,65,103)),
                    };
                    painter.circle_filled(center,(s*0.39).clamp(4.0,10.0),color);
                    painter.text(center,Align2::CENTER_CENTER,symbol,
                        FontId::proportional(10.0),Color32::WHITE);
                }
                for marker in &map.markers {
                    let center = self.screen_at(rect,
                        (map.origin.x as f32 + marker.x as f32 + 0.5),
                        (map.origin.y as f32 + marker.y as f32 + 0.5));
                    painter.circle_filled(center,(s*0.36).clamp(4.0,10.0),Color32::from_rgb(117,66,179));
                    painter.text(center,Align2::CENTER_CENTER,"!",FontId::proportional(10.0),Color32::WHITE);
                }
            }
            painter.text(top_left + vec2(5.0,5.0),Align2::LEFT_TOP,&map.name,
                FontId::proportional(11.0),Color32::from_black_alpha(if is_editing {48} else {160}));
        }

        if self.mode==Mode::Properties {
            if let Some(p)=self.selected_cell {
                if self.world.map_at(p).is_some() {
                    let at=self.screen_at(rect,p.x as f32,p.y as f32);
                    let selected=Rect::from_min_size(at,Vec2::splat(s));
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
            let blocked=self.world.can_place_frame(origin,w,h).is_err();
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
        // Outlines drawn LAST, above terrain, properties, labels, and markers.
        let colors = [
            Color32::from_rgb(166,64,79),
            Color32::from_rgb(190,145,32),
            Color32::from_rgb(49,150,95),
            Color32::from_rgb(58,112,190),
        ];
        for map in &self.world.maps {
            let min = self.screen_at(rect,map.origin.x as f32,map.origin.y as f32);
            let frame_rect = Rect::from_min_size(min,vec2(map.width as f32*s,map.height as f32*s));
            if rect.intersects(frame_rect) {
                let col = colors[((map.id-1) as usize)%colors.len()];
                let stroke = Stroke::new(if self.selected_map == Some(map.id){3.3}else{2.4},col);
                painter.rect_stroke(frame_rect,0.0,stroke,StrokeKind::Inside);
            }
        }
    }

    fn canvas(&mut self, ui: &mut egui::Ui) {
        let size = ui.available_size();
        self.canvas_size=size;
        let (rect,response) = ui.allocate_exact_size(size,Sense::click_and_drag());
        self.handle_canvas_input(ui,rect,&response);
        let editing = !matches!(self.gesture,CanvasGesture::Idle);
        self.draw_canvas(ui,rect,editing);
    }
}

impl eframe::App for MapStudioApp {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Android status/navigation-bar insets will be revisited after real-device testing.
        #[cfg(target_os = "android")]
        egui::Panel::top("status_inset").show(root,|ui| { ui.add_space(28.0); });

        egui::Panel::top("header").show(root,|ui|self.header(ui));
        let maximum=(root.available_height()*0.76).max(145.0);
        self.panel_height=self.panel_height.clamp(126.0,maximum);
        egui::Panel::bottom("palette")
            .resizable(false).exact_size(self.panel_height)
            .show(root,|ui|self.palette(ui));
        egui::CentralPanel::default().show(root,|ui|self.canvas(ui));
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
        (Color32::from_rgba_unmultiplied(
            218u8.saturating_sub(e.saturating_mul(9)),
            191u8.saturating_sub(e.saturating_mul(6)),
            76u8.saturating_add(e.saturating_mul(10)),110),VALUES[e as usize])
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
