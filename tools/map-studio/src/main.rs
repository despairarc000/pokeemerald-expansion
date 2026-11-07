fn main() -> eframe::Result {
    eframe::run_native(
        "Map Studio",
        eframe::NativeOptions::default(),
        Box::new(|cc| Ok(Box::new(map_studio::MapStudioApp::new(&cc.egui_ctx)))),
    )
}
