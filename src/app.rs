use crate::region::{Region, SplitDirection};
use crate::theme::Theme;
use egui::{Align2, CentralPanel, Color32, Context, FontId, Rect, Sense, Stroke, Vec2};

/// Thickness of the dividing lines drawn between regions.
const DIVIDER_THICKNESS: f32 = 2.0;
/// Colour of dividing lines (semi-transparent white works on both themes).
const DIVIDER_COLOR: Color32 = Color32::from_rgba_premultiplied(180, 180, 180, 200);
/// Fill colour for hovered leaf regions.
const HOVER_FILL: Color32 = Color32::from_rgba_premultiplied(100, 149, 237, 60); // cornflower tint

/// Top-level application state.
pub struct App {
    /// Root of the region tree.
    root: Region,
    /// Currently selected split direction for the next click.
    split_dir: SplitDirection,
    /// Active colour theme.
    theme: Theme,
}

impl App {
    /// Create the app, initialising the root region to the available screen area.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let theme = Theme::from_os();
        theme.apply(&cc.egui_ctx);

        // Use the full available rect as the initial region.
        let screen = cc.egui_ctx.available_rect();
        Self {
            root: Region::new(screen),
            split_dir: SplitDirection::Vertical,
            theme,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // Always apply the current theme so hot-toggles take effect immediately.
        self.theme.apply(ctx);

        // ── Toolbar ──────────────────────────────────────────────────────────
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("fake_full_screen");
                ui.separator();

                // Split-direction toggle
                ui.label("Split:");
                if ui
                    .selectable_label(
                        self.split_dir == SplitDirection::Vertical,
                        "Vertical",
                    )
                    .clicked()
                {
                    self.split_dir = SplitDirection::Vertical;
                }
                if ui
                    .selectable_label(
                        self.split_dir == SplitDirection::Horizontal,
                        "Horizontal",
                    )
                    .clicked()
                {
                    self.split_dir = SplitDirection::Horizontal;
                }

                ui.separator();

                // Theme toggle
                if ui.button(self.theme.label()).clicked() {
                    self.theme = self.theme.toggled();
                }
            });
        });

        // ── Canvas ───────────────────────────────────────────────────────────
        CentralPanel::default().show(ctx, |ui| {
            let canvas_rect = ui.available_rect_before_wrap();

            // Sync root rect to available canvas size on first frame or resize.
            if let crate::region::Region::Leaf { rect } = &mut self.root {
                *rect = canvas_rect;
            }

            let painter = ui.painter();
            let pointer_pos = ctx.pointer_hover_pos();

            // Collect leaf rects for interaction.
            let leaves = self.root.leaves();

            for leaf_rect in &leaves {
                // Highlight the hovered leaf.
                if let Some(p) = pointer_pos {
                    if leaf_rect.contains(p) {
                        painter.rect_filled(*leaf_rect, 0.0, HOVER_FILL);
                    }
                }

                // Draw leaf border.
                painter.rect_stroke(
                    *leaf_rect,
                    0.0,
                    Stroke::new(DIVIDER_THICKNESS, DIVIDER_COLOR),
                    egui::StrokeKind::Inside,
                );

                // Draw a small label with the region size.
                let label = format!(
                    "{:.0} × {:.0}",
                    leaf_rect.width(),
                    leaf_rect.height()
                );
                painter.text(
                    leaf_rect.center(),
                    Align2::CENTER_CENTER,
                    label,
                    FontId::proportional(12.0),
                    ui.visuals().text_color(),
                );
            }

            // Detect clicks and attempt a split.
            let response = ui.allocate_rect(canvas_rect, Sense::click());
            if response.clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    self.root.try_split(pos, self.split_dir);
                }
            }
        });
    }
}
