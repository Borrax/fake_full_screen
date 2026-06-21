mod app;
mod region;
mod theme;

// On Windows suppress the console window that would otherwise pop up
// alongside the GUI application.
#[cfg(target_os = "windows")]
#[link_section = ".rsrc"]
#[used]
static _WINDOWS_SUBSYSTEM: () = ();

// Tell the Windows linker to use the Windows subsystem (no console).
#[cfg(target_os = "windows")]
mod windows_subsystem {
    // The attribute must live on main or on a dummy item in the crate root.
}

fn main() -> eframe::Result<()> {
    // On Windows we want a borderless window positioned to cover the full
    // primary monitor — this is "fake" fullscreen (the window stays a normal
    // top-level window, not an exclusive fullscreen surface).
    let viewport = egui::ViewportBuilder::default()
        .with_title("fake_full_screen")
        // Start maximised; the app.rs startup logic will then strip the title
        // bar and move the window to (0,0) so it covers the whole screen.
        .with_maximized(true)
        // No title bar, no resize border.
        .with_decorations(false)
        // Sits on top of the taskbar.
        .with_always_on_top()
        // Skip the taskbar button (optional — remove if you want one).
        .with_taskbar(false);

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
