use crate::region::{Region, SplitDirection};
use crate::theme::Theme;
use crate::vlc::{snap_vlc, SnapResult};
use egui::{Align2, CentralPanel, Color32, Context, FontId, Panel, Sense, Stroke};

const DIVIDER_THICKNESS: f32 = 2.0;
const DIVIDER_COLOR: Color32 = Color32::from_rgba_premultiplied(180, 180, 180, 200);
const HOVER_FILL: Color32 = Color32::from_rgba_premultiplied(100, 149, 237, 60);
const SELECT_FILL: Color32 = Color32::from_rgba_premultiplied(100, 200, 120, 90);
const SELECT_BORDER: Color32 = Color32::from_rgba_premultiplied(80, 220, 100, 255);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Split,
    Select,
}

pub struct App {
    root: Region,
    split_dir: SplitDirection,
    theme: Theme,
    mode: Mode,
    // index into root.leaves() of the currently selected region
    selected: Option<usize>,
    // feedback shown in the toolbar after a snap attempt
    snap_status: Option<String>,
    startup_done: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let theme = Theme::from_os();
        theme.apply(&cc.egui_ctx);

        let screen = cc.egui_ctx.content_rect();
        Self {
            root: Region::new(screen),
            split_dir: SplitDirection::Vertical,
            theme,
            mode: Mode::Split,
            selected: None,
            snap_status: None,
            startup_done: false,
        }
    }
}

#[cfg(target_os = "windows")]
fn snap_to_primary_monitor(ctx: &Context) {
    use winapi::um::winuser::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

    let (w, h) = unsafe {
        (
            GetSystemMetrics(SM_CXSCREEN) as f32,
            GetSystemMetrics(SM_CYSCREEN) as f32,
        )
    };

    ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(0.0, 0.0)));
    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(w, h)));
}

#[cfg(not(target_os = "windows"))]
fn snap_to_primary_monitor(_ctx: &Context) {}

impl eframe::App for App {
    fn logic(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.theme.apply(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        if !self.startup_done {
            snap_to_primary_monitor(&ctx);
            self.startup_done = true;
        }

        Panel::top("toolbar").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("fake_full_screen");
                ui.separator();

                ui.label("Mode:");
                if ui
                    .selectable_label(self.mode == Mode::Split, "✂ Split")
                    .clicked()
                {
                    self.mode = Mode::Split;
                    self.selected = None;
                }
                if ui
                    .selectable_label(self.mode == Mode::Select, "🖱 Select")
                    .clicked()
                {
                    self.mode = Mode::Select;
                }

                ui.separator();

                if self.mode == Mode::Split {
                    ui.label("Dir:");
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
                }

                let snap_enabled = self.selected.is_some();
                ui.add_enabled_ui(snap_enabled, |ui| {
                    if ui.button("▶ Snap VLC").clicked() {
                        if let Some(idx) = self.selected {
                            let leaves = self.root.leaves();
                            if let Some(&rect) = leaves.get(idx) {
                                let status = match snap_vlc(&rect) {
                                    SnapResult::Ok => "VLC snapped ✓".to_owned(),
                                    SnapResult::NotFound => "VLC not found — is it open?".to_owned(),
                                    SnapResult::Error(code) => {
                                        format!("Win32 error {code}")
                                    }
                                };
                                self.snap_status = Some(status);
                            }
                        }
                    }
                });

                if let Some(ref msg) = self.snap_status {
                    ui.separator();
                    ui.label(msg);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✕").clicked()
                        || ctx.input(|i| i.key_pressed(egui::Key::Escape))
                    {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }

                    ui.separator();

                    if ui.button(self.theme.label()).clicked() {
                        self.theme = self.theme.toggled();
                    }
                });
            });
        });

        CentralPanel::default().show_inside(ui, |ui| {
            let canvas_rect = ui.available_rect_before_wrap();

            if let crate::region::Region::Leaf { rect } = &mut self.root {
                *rect = canvas_rect;
            }

            let painter = ui.painter();
            let pointer_pos = ctx.pointer_hover_pos();
            let leaves = self.root.leaves();

            let response = ui.allocate_rect(canvas_rect, Sense::click());

            for (idx, leaf_rect) in leaves.iter().enumerate() {
                let is_selected = self.selected == Some(idx);
                let is_hovered = pointer_pos.map_or(false, |p| leaf_rect.contains(p));

                if is_selected {
                    painter.rect_filled(*leaf_rect, 0.0, SELECT_FILL);
                } else if is_hovered {
                    painter.rect_filled(*leaf_rect, 0.0, HOVER_FILL);
                }

                let (border_color, border_width) = if is_selected {
                    (SELECT_BORDER, DIVIDER_THICKNESS + 1.0)
                } else {
                    (DIVIDER_COLOR, DIVIDER_THICKNESS)
                };
                painter.rect_stroke(
                    *leaf_rect,
                    0.0,
                    Stroke::new(border_width, border_color),
                    egui::StrokeKind::Inside,
                );

                let dim_label =
                    format!("{:.0} × {:.0}", leaf_rect.width(), leaf_rect.height());
                let hint = if is_selected {
                    " [selected]".to_owned()
                } else if self.mode == Mode::Select {
                    " [click to select]".to_owned()
                } else {
                    String::new()
                };
                painter.text(
                    leaf_rect.center(),
                    Align2::CENTER_CENTER,
                    format!("{dim_label}{hint}"),
                    FontId::proportional(12.0),
                    ui.visuals().text_color(),
                );
            }

            if response.clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    match self.mode {
                        Mode::Split => {
                            self.root.try_split(pos, self.split_dir);
                            // deselect — the split leaf no longer exists
                            self.selected = None;
                            self.snap_status = None;
                        }
                        Mode::Select => {
                            let new_sel = leaves
                                .iter()
                                .position(|r| r.contains(pos));
                            if new_sel == self.selected {
                                self.selected = None;
                            } else {
                                self.selected = new_sel;
                                self.snap_status = None;
                            }
                        }
                    }
                }
            }
        });
    }
}
