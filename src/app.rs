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
        }
    }
}

impl eframe::App for App {
    /// Theme application lives in `logic` so it runs even when the window is hidden.
    fn logic(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.theme.apply(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

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
