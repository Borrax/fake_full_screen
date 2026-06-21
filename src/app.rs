use crate::region::{Region, SplitDirection};
use crate::theme::Theme;
use egui::{Align2, CentralPanel, Color32, Context, FontId, Panel, Sense, Stroke};

/// Thickness of the dividing lines drawn between regions.
const DIVIDER_THICKNESS: f32 = 2.0;
/// Colour of dividing lines (semi-transparent, works on both themes).
const DIVIDER_COLOR: Color32 = Color32::from_rgba_premultiplied(180, 180, 180, 200);
/// Fill colour for hovered leaf regions.
const HOVER_FILL: Color32 = Color32::from_rgba_premultiplied(100, 149, 237, 60);

/// Top-level application state.
pub struct App {
    root: Region,
    split_dir: SplitDirection,
    theme: Theme,
    /// Tracks whether the one-time startup viewport snap has been issued.
    startup_done: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let theme = Theme::from_os();
        theme.apply(&cc.egui_ctx);

        let screen = cc.egui_ctx.screen_rect();
        Self {
            root: Region::new(screen),
            split_dir: SplitDirection::Vertical,
            theme,
            startup_done: false,
        }
    }
}

// ── Windows-only: Win32 helper to position the window over the primary monitor ──
//
// eframe's `with_maximized` + `with_decorations(false)` is usually enough, but
// some Windows display drivers / DPI scaling configurations leave the window
// short of the taskbar area.  Calling SetWindowPos directly with the monitor
// work-area fixes that.
#[cfg(target_os = "windows")]
fn snap_to_primary_monitor(ctx: &Context) {
    use winapi::um::winuser::{
        GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN,
    };

    // Read the primary monitor resolution in logical pixels via the Win32 API.
    // We convert to f32 for egui's viewport commands.
    let (w, h) = unsafe {
        (
            GetSystemMetrics(SM_CXSCREEN) as f32,
            GetSystemMetrics(SM_CYSCREEN) as f32,
        )
    };

    // Use egui's cross-platform viewport commands to reposition and resize.
    ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(0.0, 0.0)));
    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(w, h)));
}

#[cfg(not(target_os = "windows"))]
fn snap_to_primary_monitor(_ctx: &Context) {
    // No-op on non-Windows: the Linux/macOS paths use a normal window.
}

impl eframe::App for App {
    /// Theme application lives in `logic` so it runs even when the window is hidden.
    fn logic(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.theme.apply(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // On the very first rendered frame, issue the Win32 snap so the
        // borderless window covers the full primary monitor exactly.
        if !self.startup_done {
            snap_to_primary_monitor(&ctx);
            self.startup_done = true;
        }

        // ── Toolbar ──────────────────────────────────────────────────────────
        Panel::top("toolbar").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("fake_full_screen");
                ui.separator();

                ui.label("Split:");
                if ui
                    .selectable_label(self.split_dir == SplitDirection::Vertical, "Vertical")
                    .clicked()
                {
                    self.split_dir = SplitDirection::Vertical;
                }
                if ui
                    .selectable_label(self.split_dir == SplitDirection::Horizontal, "Horizontal")
                    .clicked()
                {
                    self.split_dir = SplitDirection::Horizontal;
                }

                ui.separator();

                if ui.button(self.theme.label()).clicked() {
                    self.theme = self.theme.toggled();
                }

                ui.separator();

                // Escape key exits the fake-fullscreen window.
                if ui.button("✕  Exit").clicked()
                    || ctx.input(|i| i.key_pressed(egui::Key::Escape))
                {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        });

        // ── Canvas ───────────────────────────────────────────────────────────
        CentralPanel::default().show_inside(ui, |ui| {
            let canvas_rect = ui.available_rect_before_wrap();

            // Keep the root leaf in sync with the canvas size.
            if let crate::region::Region::Leaf { rect } = &mut self.root {
                *rect = canvas_rect;
            }

            let painter = ui.painter();
            let pointer_pos = ctx.pointer_hover_pos();
            let leaves = self.root.leaves();

            for leaf_rect in &leaves {
                if let Some(p) = pointer_pos {
                    if leaf_rect.contains(p) {
                        painter.rect_filled(*leaf_rect, 0.0, HOVER_FILL);
                    }
                }

                painter.rect_stroke(
                    *leaf_rect,
                    0.0,
                    Stroke::new(DIVIDER_THICKNESS, DIVIDER_COLOR),
                    egui::StrokeKind::Inside,
                );

                let label = format!("{:.0} × {:.0}", leaf_rect.width(), leaf_rect.height());
                painter.text(
                    leaf_rect.center(),
                    Align2::CENTER_CENTER,
                    label,
                    FontId::proportional(12.0),
                    ui.visuals().text_color(),
                );
            }

            let response = ui.allocate_rect(canvas_rect, Sense::click());
            if response.clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    self.root.try_split(pos, self.split_dir);
                }
            }
        });
    }
}
