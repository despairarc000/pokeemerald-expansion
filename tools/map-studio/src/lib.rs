//! Map Studio Android/desktop entry and platform-neutral GUI.
pub mod model;
pub mod export;
pub mod storage;
pub mod tiles;
pub mod prefabs;
pub mod choices;
#[cfg(target_os = "android")]
mod android_export;
mod ui;

pub use ui::MapStudioApp;

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );

    android_export::set_app(app.clone());
    let save_path = app.internal_data_path().map(|p| p.join("map-studio-project.json"));
    let export_dir = app.external_data_path().map(|p| p.join("MapStudio"));

    let options = eframe::NativeOptions {
        android_app: Some(app),
        ..Default::default()
    };

    if let Err(err) = eframe::run_native(
        "Map Studio",
        options,
        Box::new(|cc| Ok(Box::new(MapStudioApp::with_paths(&cc.egui_ctx, save_path.clone(), export_dir.clone())))),
    ) {
        log::error!("Map Studio failed to start: {err}");
    }
}
