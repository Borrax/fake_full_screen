#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod app;
mod region;
mod theme;
mod vlc;

fn main() -> eframe::Result<()> {
    let viewport = egui::ViewportBuilder::default()
        .with_title("fake_full_screen")
        .with_maximized(true)
        .with_decorations(false);

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "fake_full_screen",
        native_options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
