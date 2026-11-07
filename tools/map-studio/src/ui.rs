//! Native egui map editor with an atlas of real Emerald metatiles.
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2, pos2, vec2};
use crate::model::{Cell, EventKind, MapEvent, GridPoint, ExpandDirection, World};
use crate::{storage, export};
use std::{path::PathBuf, time::{Duration, Instant}};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode { Terrain, Properties, Events }

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
    move_tool: bool,
    moving: Option<(u32, GridPoint, GridPoint)>,
    resizing: Option<(u32, GridPoint, u32, u32)>,
    collision_brush: u8,
    pan_tool: bool,
    frame_tool: bool,
    blank_pan: bool,
    map_width: u32,
    map_height: u32,
    expand_direction: ExpandDirection,
    size_panel_open: bool,
    zoom: f32,
    offset: Vec2,
    canvas_size: Vec2,
    last_painted: Option<GridPoint>,
    multitouch_in_progress: bool,
    selected_map: Option<u32>,
    selected_cell: Option<GridPoint>,
    confirm_delete: Option<u32>,
    message: String,
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
            "Drag empty canvas to pan. Tap + Map to create a frame.".to_string()
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
            move_tool: false,
            moving: None,
            resizing: None,
            collision_brush: 1,
            pan_tool: false,
            frame_tool: false,
            blank_pan: false,
            map_width: 12,
            map_height: 10,
            expand_direction: ExpandDirection::LowerRight,
            size_panel_open: false,
            zoom: 14.0,
            offset: vec2(28.0, 52.0),
            canvas_size: vec2(360.0,540.0),
            last_painted: None,
            multitouch_in_progress: false,
            selected_map: None,
            selected_cell: None,
            confirm_delete: None,
            message,
        }
    }

    fn header(&mut self, ui: &mut egui::Ui) {
        ui.add_space(5.0);
        ui.horizontal(|ui| {
            ui.heading("Map Studio");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("Export").clicked() { self.try_export(); }
                if ui.small_button("Backup").clicked() { self.try_backup(); }
                if ui.small_button("Redo").clicked() { self.redo(); }
                if ui.small_button("Undo").clicked() { self.undo(); }
            });
        });
        ui.horizontal(|ui| {
            if ui.selectable_label(self.mode == Mode::Terrain, "Terrain").clicked() {
                self.mode = Mode::Terrain; self.pan_tool = false; self.move_tool = false;
            }
            if ui.selectable_label(self.mode == Mode::Properties, "Properties").clicked() {
                self.mode = Mode::Properties; self.pan_tool = false; self.move_tool = false;
            }
            if ui.selectable_label(self.mode == Mode::Events, "Events").clicked() {
                self.mode = Mode::Events; self.pan_tool = false; self.move_tool = false;
            }
        });
        ui.horizontal(|ui| {
            if ui.selectable_label(self.frame_tool, "+ Map").clicked() {
                self.frame_tool = !self.frame_tool;
                self.pan_tool = false;
                self.move_tool = false;
                if self.frame_tool { self.size_panel_open=true; }
            }
            if ui.small_button("Size").clicked() {
                if !self.size_panel_open {
                    if let Some(map)=self.world.maps.iter().find(|m|Some(m.id)==self.selected_map) {
                        self.map_width=map.width;
                        self.map_height=map.height;
                    }
                }
                self.size_panel_open = !self.size_panel_open;
            }
            if ui.selectable_label(self.pan_tool, "Pan").clicked() {
                self.pan_tool = !self.pan_tool;
                self.frame_tool = false;
                self.move_tool = false;
            }
            if ui.selectable_label(self.move_tool, "Move").clicked() {
                self.move_tool = !self.move_tool;
                self.pan_tool = false;
                self.frame_tool = false;
            }
            if ui.small_button("Fit").clicked() { self.fit_world(); }
            ui.separator();
            ui.label(egui::RichText::new(format!("{}%", (self.zoom / 14.0 * 100.0).round() as u32))
                .small().color(Color32::DARK_GRAY));
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
        egui::Window::new("Map dimensions")
            .id(egui::Id::new("map_size_dialog"))
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .anchor(Align2::CENTER_TOP,[0.0,150.0])
            .show(ctx,|ui|{
                ui.set_max_width(310.0);
                ui.horizontal(|ui|{
                    ui.label("Width");
                    ui.add(egui::DragValue::new(&mut self.map_width).range(1..=100).speed(1));
                    ui.label("Height");
                    ui.add(egui::DragValue::new(&mut self.map_height).range(1..=100).speed(1));
                });
                ui.label("Expand toward");
                egui::Grid::new("expansion_directions").num_columns(2).spacing([12.0,8.0])
                    .show(ui,|ui|{
                        ui.selectable_value(&mut self.expand_direction,ExpandDirection::UpperLeft,"↖ Upper left");
                        ui.selectable_value(&mut self.expand_direction,ExpandDirection::UpperRight,"↗ Upper right");
                        ui.end_row();
                        ui.selectable_value(&mut self.expand_direction,ExpandDirection::LowerLeft,"↙ Lower left");
                        ui.selectable_value(&mut self.expand_direction,ExpandDirection::LowerRight,"↘ Lower right");
                        ui.end_row();
                    });
                ui.label(egui::RichText::new("The opposite corner stays fixed.").small()
                    .color(Color32::DARK_GRAY));
                if (u64::from(self.map_width)+15)*(u64::from(self.map_height)+14)>10240 {
                    ui.colored_label(Color32::from_rgb(160,72,34),
                        "Large frame: export may exceed the Emerald buffer limit.");
                }
                if self.frame_tool {
                    ui.label("Tap empty canvas to place the new map.");
                    if ui.button("Done").clicked(){
                        self.size_panel_open=false;
                    }
                } else if let Some(id)=self.selected_map {
                    if ui.button("Apply to selected map").clicked(){
                        self.begin_edit();
                        match self.world.resize_frame_towards(id,self.map_width,self.map_height,self.expand_direction) {
                            Ok(())=>{
                                self.message=format!("Map resized to {}×{} toward {}",
                                    self.map_width,self.map_height,self.expand_direction.label());
                                self.size_panel_open=false;
                            }
                            Err(err)=>self.message=err,
                        }
                        self.commit_edit();
                    }
                } else {
                    ui.label("Select a map, or use + Map to create one.");
                }
            });
        self.size_panel_open &= open;
    }

    fn palette(&mut self, ui: &mut egui::Ui) {
        ui.add_space(3.0);
        match self.mode {
            Mode::Terrain => {
                ui.horizontal_wrapped(|ui| {
                    ui.strong("Metatiles");
                    ui.toggle_value(&mut self.erase_tool,"Erase");
                    if ui.small_button("Fill empty").clicked(){ self.fill_selected(); }
                    if ui.small_button("◀").clicked() { self.palette_page=self.palette_page.saturating_sub(1); }
                    if ui.small_button("▶").clicked() { self.palette_page=(self.palette_page+1).min(20); }
                });
                ui.horizontal(|ui| {
                    ui.label("ID");
                    if ui.add(egui::DragValue::new(&mut self.brush_id).range(0..=655).speed(1)).changed() {
                        self.erase_tool=false;
                    }
                    ui.label("Find");
                    ui.add(egui::TextEdit::singleline(&mut self.palette_search).desired_width(75.0).hint_text("number"));
                });
                // 32 entries per page: texture thumbnails remain fast on budget phones.
                let range:Vec<u16>= if let Ok(id)=self.palette_search.trim().parse::<u16>() {
                    if id<656 { vec![id] } else { Vec::new() }
                } else {
                    (self.palette_page*32..((self.palette_page+1)*32).min(656))
                        .map(|i|i as u16).collect()
                };
                egui::ScrollArea::horizontal().id_salt("real_tile_palette").show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for id in range {
                            ui.vertical(|ui| {
                                let (rect,response)=ui.allocate_exact_size(vec2(50.0,50.0),Sense::click());
                                ui.painter().rect_filled(rect,4.0,if id==self.brush_id&&!self.erase_tool
                                    {Color32::from_rgb(139,208,246)} else {Color32::from_gray(236)});
                                ui.painter().image(self.atlas.id(),rect.shrink(5.0),Self::tile_uv(id),Color32::WHITE);
                                if response.clicked(){
                                    self.brush_id=id;
                                    self.erase_tool=false;
                                    self.pan_tool=false;
                                    self.move_tool=false;
                                    self.frame_tool=false;
                                }
                                ui.label(egui::RichText::new(format!("0x{id:03X}")).small());
                            });
                        }
                    });
                });
            }
            Mode::Properties => {
                if let Some(id)=self.selected_map {
                    let before=self.world.clone();
                    let mut name_changed=false;
                    let mut request_delete=false;
                    if let Some(map)=self.world.maps.iter_mut().find(|m|m.id==id){
                        ui.horizontal(|ui|{
                            ui.label("Map");
                            name_changed=ui.add(egui::TextEdit::singleline(&mut map.name)
                                .desired_width(115.0)).changed();
                            ui.label(format!("{}x{}",map.width,map.height));
                            if ui.small_button("Delete").clicked(){request_delete=true;}
                        });
                    }
                    if name_changed {
                        self.before_gesture=Some(before);
                        self.commit_edit();
                    }
                    if request_delete {self.confirm_delete=Some(id);}
                    if self.confirm_delete==Some(id){
                        ui.horizontal(|ui|{
                            ui.label("Delete selected map?");
                            if ui.small_button("Confirm").clicked(){
                                self.begin_edit();
                                match self.world.remove_frame(id){
                                    Ok(())=>{
                                        self.selected_map=None;
                                        self.selected_event=None;
                                        self.message="Map deleted".into();
                                    }
                                    Err(err)=>self.message=err
                                }
                                self.commit_edit();
                                self.confirm_delete=None;
                            }
                            if ui.small_button("Cancel").clicked(){
                                self.confirm_delete=None;
                            }
                        });
                    }
                }
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.elevation_tool,false,"Collision");
                    ui.selectable_value(&mut self.elevation_tool,true,"Elevation");
                });
                if let Some(p)=self.selected_cell {
                    if let Some(map)=self.world.map_at(p) {
                        let x=(p.x-map.origin.x) as u32;
                        let y=(p.y-map.origin.y) as u32;
                        if let Some(cell)=map.cell(x,y) {
                            ui.label(egui::RichText::new(format!(
                                "{} · ({},{})  C{} / E{}",
                                map.name,x,y,cell.collision,cell.elevation
                            )).small().color(Color32::DARK_GRAY));
                        } else {
                            ui.label(egui::RichText::new(
                                "Unpainted cell — choose a metatile first"
                            ).small().color(Color32::DARK_GRAY));
                        }
                    }
                }
                if self.elevation_tool {
                    ui.horizontal(|ui| {
                        ui.label("Elevation");
                        ui.add(egui::Slider::new(&mut self.elevation_brush,0..=15).show_value(true));
                    });
                    ui.label(egui::RichText::new(
                        "Canvas shows E0–E15 with a height tint. This changes game elevation bits, not visual tile height."
                    ).small().color(Color32::DARK_GRAY));
                } else {
                    ui.horizontal_wrapped(|ui| {
                        for value in 0..4 {
                            if ui.selectable_label(self.collision_brush==value,
                                format!("C{value}")).clicked() {
                                self.collision_brush=value;
                            }
                        }
                    });
                    ui.label(egui::RichText::new(
                        "C0 clear · C1 blocking · C2/C3 other native collision values. Paint across tiles to apply."
                    ).small().color(Color32::DARK_GRAY));
                }
            }
            Mode::Events => {
                ui.horizontal_wrapped(|ui| {
                    ui.strong("Place");
                    for (kind,label) in [
                        (EventKind::Npc,"NPC"),(EventKind::Sign,"Sign"),
                        (EventKind::Warp,"Warp"),(EventKind::Trigger,"Trigger")
                    ] {
                        ui.selectable_value(&mut self.event_brush,kind,label);
                    }
                });
                if let Some((map_id,index))=self.selected_event {
                    let destinations:Vec<(u32,String)>=self.world.maps.iter()
                        .map(|m|(m.id,m.name.clone())).collect();
                    let before=self.world.clone();
                    let mut changed=false;
                    let mut remove=false;
                    let mut found=false;
                    if let Some(event)=self.world.maps.iter_mut()
                        .find(|m|m.id==map_id).and_then(|m|m.events.get_mut(index)) {
                        found=true;
                        ui.horizontal(|ui| {
                            ui.strong(format!("{:?} ({},{})",event.kind,event.x,event.y));
                            let text=if self.event_editor_open {"Close details"}else{"Edit details"};
                            if ui.small_button(text).clicked(){
                                self.event_editor_open=!self.event_editor_open;
                            }
                            if ui.small_button("Remove").clicked(){remove=true;}
                        });
                        ui.horizontal_wrapped(|ui| {
                            ui.label("Change");
                            for (kind,label) in [
                                (EventKind::Npc,"NPC"),(EventKind::Sign,"Sign"),
                                (EventKind::Warp,"Warp"),(EventKind::Trigger,"Trigger")
                            ] {
                                if ui.selectable_label(event.kind==kind,label).clicked() && event.kind!=kind {
                                    let mut new_event=MapEvent::new(kind,event.x,event.y);
                                    new_event.elevation=event.elevation;
                                    *event=new_event;
                                    changed=true;
                                }
                            }
                        });
                        if self.event_editor_open {
                            egui::ScrollArea::vertical().id_salt("event-editor")
                                .max_height(125.0).show(ui,|ui| {
                                    match event.kind {
                                        EventKind::Npc => {
                                            ui.horizontal(|ui|{
                                                ui.label("Dialogue");
                                                changed|=ui.text_edit_singleline(&mut event.dialogue).changed();
                                            });
                                            ui.horizontal(|ui|{
                                                ui.label("Graphics");
                                                changed|=ui.text_edit_singleline(&mut event.graphics_id).changed();
                                            });
                                        }
                                        EventKind::Sign => {
                                            ui.horizontal(|ui|{
                                                ui.label("Sign text");
                                                changed|=ui.text_edit_singleline(&mut event.dialogue).changed();
                                            });
                                        }
                                        EventKind::Warp => {
                                            ui.horizontal(|ui|{
                                                ui.label("To");
                                                egui::ComboBox::from_id_salt("warp-destination")
                                                    .selected_text(destinations.iter()
                                                        .find(|(id,_)|*id==event.dest_map_id)
                                                        .map(|(_,name)|name.as_str())
                                                        .unwrap_or("Choose map"))
                                                    .show_ui(ui,|ui|{
                                                        for (id,name) in &destinations {
                                                            changed|=ui.selectable_value(
                                                                &mut event.dest_map_id,*id,name
                                                            ).changed();
                                                        }
                                                    });
                                                ui.label("Warp ID");
                                                changed|=ui.add(egui::DragValue::new(&mut event.dest_warp_id)
                                                    .range(0..=30)).changed();
                                            });
                                            ui.small("Destination must have a matching warp.");
                                        }
                                        EventKind::Trigger => {
                                            ui.horizontal(|ui|{
                                                ui.label("Var");
                                                changed|=ui.text_edit_singleline(&mut event.var).changed();
                                                ui.label("Value");
                                                changed|=ui.add(egui::DragValue::new(&mut event.var_value)).changed();
                                            });
                                            ui.horizontal(|ui|{
                                                ui.label("Script");
                                                changed|=ui.text_edit_singleline(&mut event.script).changed();
                                            });
                                            ui.small("Use an existing script reference.");
                                        }
                                    }
                                    ui.horizontal(|ui|{
                                        ui.label("Elevation");
                                        changed|=ui.add(egui::DragValue::new(&mut event.elevation)
                                            .range(0..=15)).changed();
                                    });
                                });
                        } else {
                            ui.label(egui::RichText::new(
                                "Tap type to change · double-tap icon for full details"
                            ).small().color(Color32::DARK_GRAY));
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
                    } else if !found {
                        self.selected_event=None;
                        self.event_editor_open=false;
                    }
                } else {
                    ui.label(egui::RichText::new(
                        "Tap map to place · tap event to select · double-tap to edit"
                    ).small().color(Color32::DARK_GRAY));
                }
            }
        }
        ui.label(egui::RichText::new(&self.message).small().color(Color32::DARK_GRAY));
        ui.add_space(4.0);
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
            .unwrap_or_default().as_secs();
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

    fn handle_canvas_input(&mut self, ui: &egui::Ui, rect: Rect, response: &egui::Response) {
        let (multi,pressed,down,released,pointer)=ui.input(|input|(
            input.multi_touch(),
            input.pointer.button_pressed(egui::PointerButton::Primary),
            input.pointer.button_down(egui::PointerButton::Primary),
            input.pointer.button_released(egui::PointerButton::Primary),
            input.pointer.interact_pos(),
        ));

        if let Some(touch)=multi.filter(|touch|rect.contains(touch.center_pos)){
            self.multitouch_in_progress=true;
            self.blank_pan=false;
            self.last_painted=None;
            self.commit_edit();
            self.change_zoom(rect,touch.center_pos,touch.zoom_delta,touch.translation_delta);
            ui.ctx().request_repaint();
            return;
        }
        // Never paint on the way out of a two-finger gesture.
        if self.multitouch_in_progress {
            if !down {self.multitouch_in_progress=false;}
            return;
        }
        if self.pan_tool {
            if response.dragged() {
                self.offset+=response.drag_delta();
                ui.ctx().request_repaint();
            }
            if released {self.last_painted=None;self.blank_pan=false;self.commit_edit();}
            return;
        }

        if let Some(screen)=pointer {
            let p=self.grid_at(rect,screen);
            if pressed && rect.contains(screen) && response.is_pointer_button_down_on() {
                self.last_painted=None;
                self.blank_pan=false;

                if self.frame_tool {
                    let origin=self.expand_direction.origin_at(p,self.map_width,self.map_height);
                    match origin.and_then(|origin|{
                        let next=self.world.maps.iter().map(|m|m.id).max().unwrap_or(0)+1;
                        self.begin_edit();
                        self.world.add_map(format!("Map {next}"),origin,self.map_width,self.map_height)
                            .map_err(|e|e.to_owned())
                    }) {
                        Ok(id)=>{
                            self.selected_map=Some(id);
                            self.frame_tool=false;
                            self.size_panel_open=false;
                            self.message=format!("Created {}×{} map toward {}",
                                self.map_width,self.map_height,self.expand_direction.label());
                        }
                        Err(err)=>self.message=err,
                    }
                    self.commit_edit();
                    return;
                }

                if let Some((id,origin,width,height))=self.world.map_at(p)
                    .map(|map|(map.id,map.origin,map.width,map.height)) {
                    self.selected_map=Some(id);
                    self.selected_cell=Some(p);
                    if self.size_panel_open {
                        // Sizing mode selects a frame without painting or moving it.
                        self.map_width=width;
                        self.map_height=height;
                        self.message="Adjust dimensions and choose Apply to selected map".into();
                        return;
                    }
                    self.begin_edit();
                    if self.move_tool {
                        if p.x>=origin.x+(width as i32)-2 && p.y>=origin.y+(height as i32)-2 {
                            self.resizing=Some((id,p,width,height));
                        } else {
                            self.moving=Some((id,p,origin));
                        }
                    }
                } else {
                    // Drag on any empty canvas = pan, regardless of selected mode.
                    // New frames require explicit + Map, so gestures never fight.
                    self.blank_pan=true;
                    self.event_editor_open=false;
                }
            }

            if self.blank_pan {
                if down && response.dragged() {
                    self.offset+=response.drag_delta();
                    ui.ctx().request_repaint();
                }
            } else if down && rect.contains(screen) {
                if let Some((id,start,origin))=self.moving {
                    if self.world.move_frame(id,GridPoint::new(
                        origin.x+p.x-start.x,origin.y+p.y-start.y
                    )) {
                        ui.ctx().request_repaint();
                    }
                } else if let Some((id,start,width,height))=self.resizing {
                    let nw=(i64::from(width)+i64::from(p.x)-i64::from(start.x))
                        .clamp(1,1000) as u32;
                    let nh=(i64::from(height)+i64::from(p.y)-i64::from(start.y))
                        .clamp(1,1000) as u32;
                    self.world.resize_frame(id,nw,nh);
                    ui.ctx().request_repaint();
                } else if response.is_pointer_button_down_on() {
                    match self.mode {
                        Mode::Events => {
                            if pressed {
                                self.selected_event=self.world.add_event(p,self.event_brush);
                                self.event_editor_open=false;
                                if let Some((map_id,_))=self.selected_event{
                                    self.selected_map=Some(map_id);
                                    self.message="Event selected · choose type or double-tap to edit".into();
                                }
                            }
                        }
                        Mode::Properties => {
                            if self.world.map_at(p).and_then(|map|{
                                let x=(p.x-map.origin.x) as u32;
                                let y=(p.y-map.origin.y) as u32;
                                map.cell(x,y)
                            }).is_none(){
                                if pressed {
                                    self.message="Paint a metatile before editing collision or elevation".into();
                                }
                            } else {
                                self.paint_line(self.last_painted.unwrap_or(p),p);
                                self.last_painted=Some(p);
                                ui.ctx().request_repaint();
                            }
                        }
                        Mode::Terrain => {
                            self.paint_line(self.last_painted.unwrap_or(p),p);
                            self.last_painted=Some(p);
                            ui.ctx().request_repaint();
                        }
                    }
                }
            }
        }
        // egui signals double-click on release, not on pointer press.
        if self.mode==Mode::Events && response.double_clicked() && self.selected_event.is_some() {
            self.event_editor_open=true;
        }
        if released || !down {
            self.blank_pan=false;
            self.moving=None;
            self.resizing=None;
            self.last_painted=None;
            self.commit_edit();
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
                            if s >= 10.0 {
                                painter.text(tile_rect.center(),Align2::CENTER_CENTER,symbol,
                                    FontId::proportional((s*0.53).clamp(8.0,16.0)),
                                    Color32::from_black_alpha(220));
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
        let editing = response.is_pointer_button_down_on();
        self.draw_canvas(ui,rect,editing);
    }
}

impl eframe::App for MapStudioApp {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Android status/navigation-bar insets will be revisited after real-device testing.
        #[cfg(target_os = "android")]
        egui::Panel::top("status_inset").show(root,|ui| { ui.add_space(28.0); });

        egui::Panel::top("header").show(root,|ui|self.header(ui));
        egui::Panel::bottom("palette").show(root,|ui|self.palette(ui));
        egui::CentralPanel::default().show(root,|ui|self.canvas(ui));
        self.size_dialog(root.ctx());
        self.maybe_autosave(root.ctx());
    }
}

fn property_visual(cell: Cell, elevation_mode:bool) -> (Color32, &'static str) {
    if elevation_mode {
        const LABELS:[&str;16]=[
            "E0","E1","E2","E3","E4","E5","E6","E7",
            "E8","E9","E10","E11","E12","E13","E14","E15",
        ];
        let elevation=cell.elevation.min(15);
        // Gold for low heights through cool indigo at high heights.
        let tint=Color32::from_rgba_unmultiplied(
            220u8.saturating_sub(elevation.saturating_mul(9)),
            192u8.saturating_sub(elevation.saturating_mul(6)),
            75u8.saturating_add(elevation.saturating_mul(9)),
            106,
        );
        (tint,LABELS[elevation as usize])
    } else {
        match cell.collision {
            0=>(Color32::from_rgba_unmultiplied(55,196,96,94),"C0"),
            1=>(Color32::from_rgba_unmultiplied(230,67,67,112),"C1"),
            2=>(Color32::from_rgba_unmultiplied(152,99,207,110),"C2"),
            _=>(Color32::from_rgba_unmultiplied(241,169,53,110),"C3"),
        }
    }
}

#[cfg(test)]
mod properties_visual_tests {
    use super::*;
    use crate::model::TileKind;
    #[test]
    fn collision_overlay_displays_all_native_values() {
        let mut cell=Cell::new(TileKind::Grass);
        let mut seen=std::collections::HashSet::new();
        for collision in 0..4 {
            cell.collision=collision;
            let (color,label)=property_visual(cell,false);
            assert_eq!(label,format!("C{collision}"));
            seen.insert(color);
        }
        assert_eq!(seen.len(),4);
    }
    #[test]
    fn elevation_overlay_has_sixteen_distinct_labels() {
        let mut cell=Cell::new(TileKind::Grass);
        for height in 0..16 {
            cell.elevation=height;
            let (_,label)=property_visual(cell,true);
            assert_eq!(label,format!("E{height}"));
        }
    }
}
