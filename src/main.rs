mod app;
mod region;
mod theme;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("fake_full_screen")
            .with_inner_size([1024.0, 768.0]),
        ..Default::default()
    };

    eframe::run_native(
        "fake_full_screen",
        native_options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
