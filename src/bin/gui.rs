// src/bin/gui.rs
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
use bb_scrape::config::state::AppState;
use bb_scrape::gui::run;
use eframe::egui::{ IconData, ViewportBuilder };

fn app_icon() -> IconData {
    let rgba = image::load_from_memory(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/brutalball.png"
    )))
    .unwrap()
    .to_rgba8();
    let (w, h) = rgba.dimensions();
    IconData { rgba: rgba.into_raw(), width: w, height: h }
}

fn main() {
    let options = eframe::NativeOptions {
        // eframe 0.32: icon set via viewport builder
        viewport: ViewportBuilder::default().with_icon(app_icon()),
        ..Default::default()
    };

    if let Err(e) = run(AppState::default(), options) {
        eprintln!("GUI failed: {}", e);
        std::process::exit(1);
    }
}
