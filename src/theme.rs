use egui::Context;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
}

impl Theme {
    pub fn from_os() -> Self {
        match dark_light::detect() {
            Ok(dark_light::Mode::Light) => Theme::Light,
            _ => Theme::Dark,
        }
    }

    pub fn apply(&self, ctx: &Context) {
        match self {
            Theme::Dark => ctx.set_visuals(egui::Visuals::dark()),
            Theme::Light => ctx.set_visuals(egui::Visuals::light()),
        }
    }

    pub fn toggled(self) -> Self {
        match self {
            Theme::Dark => Theme::Light,
            Theme::Light => Theme::Dark,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Theme::Dark => "☀ Light mode",
            Theme::Light => "🌙 Dark mode",
        }
    }
}
