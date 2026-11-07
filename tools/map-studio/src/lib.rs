//! Map Studio Android/desktop entry and platform-neutral GUI.
pub mod model;
mod ui;

pub use ui::MapStudioApp;

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );

    let options = eframe::NativeOptions {
        android_app: Some(app),
        ..Default::default()
    };

    if let Err(err) = eframe::run_native(
        "Map Studio",
        options,
        Box::new(|cc| Ok(Box::new(MapStudioApp::new(&cc.egui_ctx)))),
    ) {
        log::error!("Map Studio failed to start: {err}");
    }
}
