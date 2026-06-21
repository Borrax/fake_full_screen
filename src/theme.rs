use egui::Context;

/// Which colour theme is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
}

impl Theme {
    /// Detect the OS preference via the `dark-light` crate.
    pub fn from_os() -> Self {
        match dark_light::detect() {
            Ok(dark_light::Mode::Light) => Theme::Light,
            _ => Theme::Dark, // Dark, Unspecified, or error → default dark
        }
    }

    /// Apply this theme to an egui context.
    pub fn apply(&self, ctx: &Context) {
        match self {
            Theme::Dark => ctx.set_visuals(egui::Visuals::dark()),
            Theme::Light => ctx.set_visuals(egui::Visuals::light()),
        }
    }

    /// Return the opposite theme.
    pub fn toggled(self) -> Self {
        match self {
            Theme::Dark => Theme::Light,
            Theme::Light => Theme::Dark,
        }
    }

    /// Label shown on the toggle button.
    pub fn label(self) -> &'static str {
        match self {
            Theme::Dark => "☀ Light mode",
            Theme::Light => "🌙 Dark mode",
        }
    }
}
