use eframe::NativeOptions;
use eframe::egui::Vec2;
use app::SoundApp;

mod app;
mod audio;
mod ui;

fn main() -> Result<(), eframe::Error> {
    let options = NativeOptions {
        initial_window_size: Some(Vec2::new(800.0, 600.0)),
        ..Default::default()
    };
    eframe::run_native(
        "Sound Editing Tool",
        options,
        Box::new(|_cc| Box::new(SoundApp::new())),
    )
}