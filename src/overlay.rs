//! Custom hover-fade playback control bar shown over a snapped VLC instance.
//!
//! Once VLC is snapped into a sub-rect (see `vlc.rs`) it is chrome-free and has
//! no native controls in that windowed state, so this bar is the only way to
//! drive playback. It renders as a second egui *immediate* viewport (its own
//! native window) positioned along the bottom of the snapped region, fades in
//! only while the real (OS) cursor is hovering the region, and drives playback
//! entirely through `vlc_http`.
//!
//! Because the bar's window physically overlaps VLC's window, egui's own
//! pointer position is useless here (VLC, not us, has the mouse focus most of
//! the time) — we poll the *global* cursor position with `GetCursorPos`
//! instead. And because the bar must not steal clicks from VLC while hidden,
//! we toggle `WS_EX_TRANSPARENT` on the bar's HWND directly.

use egui::{
    Color32, Context, CornerRadius, Pos2, Rect, Stroke, Vec2, ViewportBuilder, ViewportClass,
    ViewportId,
};
use std::sync::Once;
use std::time::{Duration, Instant};

use crate::vlc_http::{self, Status};

const BAR_HEIGHT: f32 = 72.0;
const FADE_IN_SECS: f32 = 0.15;
const FADE_OUT_SECS: f32 = 0.35;
/// Hide the bar this long after the last in-region mouse movement or control use.
const HIDE_DELAY: Duration = Duration::from_secs(3);
const STATUS_POLL_INTERVAL: Duration = Duration::from_millis(250);
const REPAINT_INTERVAL: Duration = Duration::from_millis(80);
pub const OVERLAY_TITLE: &str = "ffs_overlay_bar";

/// Keeps egui's update loop ticking even while the app is unfocused/occluded by
/// VLC. Without this, `request_repaint_after` alone is not delivered reliably in
/// the background, so the idle fade-out only runs when another event wakes the
/// event loop (e.g. clicking a different window). One detached pump per process.
static REPAINT_PUMP: Once = Once::new();

fn ensure_repaint_pump(ctx: &Context) {
    REPAINT_PUMP.call_once(|| {
        let ctx = ctx.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_millis(100));
            ctx.request_repaint();
        });
    });
}

/// Playback control bar overlay for a single snapped region.
pub struct Overlay {
    region: Rect,
    viewport_id: ViewportId,

    // Last in-region mouse movement or control interaction; the bar stays up
    // until HIDE_DELAY after this.
    last_activity: Instant,
    last_cursor_pos: Option<(i32, i32)>,
    opacity: f32,
    last_frame: Instant,
    click_through: bool, // current WS_EX_TRANSPARENT state applied to the HWND

    status: Status,
    last_status_poll: Instant,

    // HWND of the overlay window, cached as usize (HWND is not Send/Sync, and
    // we never move this off the UI thread, but usize keeps the struct plain).
    hwnd: Option<usize>,

    // While the user is dragging the seek/volume sliders we show the dragged
    // value instead of the polled one, and commit on release.
    seek_drag: Option<f32>,
    vol_drag: Option<f32>,
}

impl Overlay {
    pub fn new(region: Rect) -> Self {
        let now = Instant::now();
        Self {
            region,
            viewport_id: ViewportId::from_hash_of("ffs_overlay_bar_viewport"),
            // Start hidden: treat the last activity as already past HIDE_DELAY.
            last_activity: now.checked_sub(HIDE_DELAY).unwrap_or(now),
            last_cursor_pos: None,
            opacity: 0.0,
            last_frame: now,
            click_through: true,
            status: Status::default(),
            last_status_poll: now - STATUS_POLL_INTERVAL,
            hwnd: None,
            seek_drag: None,
            vol_drag: None,
        }
    }

    pub fn bar_rect(&self) -> Rect {
        let h = BAR_HEIGHT.min(self.region.height());
        Rect::from_min_size(
            Pos2::new(self.region.left(), self.region.bottom() - h),
            Vec2::new(self.region.width(), h),
        )
    }

    /// Call once per frame while this overlay should be alive.
    pub fn show(&mut self, ctx: &Context) {
        ctx.request_repaint_after(REPAINT_INTERVAL);
        ensure_repaint_pump(ctx);

        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;

        // Hover gating using the GLOBAL cursor, not egui's pointer (VLC's window
        // covers the region and owns the OS cursor most of the time). The bar
        // stays up while the mouse MOVES within the region and hides only after
        // HIDE_DELAY of stillness (control use also counts, applied below).
        let cursor = win::global_cursor_pos();
        let inside = cursor.is_some_and(|(x, y)| self.region.contains(Pos2::new(x as f32, y as f32)));
        let moved = cursor.is_some() && cursor != self.last_cursor_pos;
        self.last_cursor_pos = cursor;
        if inside && moved {
            self.last_activity = now;
        }
        let target = if now.duration_since(self.last_activity) < HIDE_DELAY {
            1.0
        } else {
            0.0
        };

        let tau = if target > self.opacity { FADE_IN_SECS } else { FADE_OUT_SECS };
        let ease = 1.0 - (-dt / tau).exp();
        self.opacity += (target - self.opacity) * ease;
        if (self.opacity - target).abs() < 0.003 {
            self.opacity = target;
        }
        self.opacity = self.opacity.clamp(0.0, 1.0);

        // Throttled status poll (blocking HTTP call — keep it infrequent).
        if now.duration_since(self.last_status_poll) >= STATUS_POLL_INTERVAL {
            self.last_status_poll = now;
            if let Ok(s) = vlc_http::status() {
                self.status = s;
            }
        }

        let bar_rect = self.bar_rect();
        let builder = ViewportBuilder::default()
            .with_title(OVERLAY_TITLE)
            .with_decorations(false)
            .with_transparent(true)
            .with_taskbar(false)
            .with_resizable(false)
            .with_always_on_top()
            .with_position(bar_rect.min)
            .with_inner_size(bar_rect.size())
            .with_active(false);

        let status = self.status.clone();
        let opacity = self.opacity;
        let seek_drag = &mut self.seek_drag;
        let vol_drag = &mut self.vol_drag;
        let mut interacted = false;
        let interacted_ref = &mut interacted;

        ctx.show_viewport_immediate(self.viewport_id, builder, move |ui, _class: ViewportClass| {
            ui.ctx().request_repaint_after(REPAINT_INTERVAL);
            ui.set_opacity(opacity);
            *interacted_ref = paint_bar(ui, &status, seek_drag, vol_drag);
        });

        // Using a control (click/drag) keeps the bar up regardless of movement.
        if interacted {
            self.last_activity = now;
        }

        // Click-through + re-assert topmost. VLC is topmost too, so we redo
        // this every frame to keep the bar above it.
        //
        // Gate click-through on the HOVER state (`target`), not the residual
        // opacity: the moment the cursor leaves and the linger elapses we want
        // clicks to pass through to VLC, even while the bar is still visually
        // fading out. (Gating on opacity left the bar swallowing clicks for the
        // full ~2.5s ease-out.)
        let want_click_through = target < 0.5;
        let newly_found = self.hwnd.is_none();
        if newly_found {
            self.hwnd = win::find_overlay_hwnd();
        }
        if let Some(hwnd) = self.hwnd {
            // Force the ex-style on first acquisition so the HWND's real state
            // matches `self.click_through` (which is otherwise assumed, never
            // applied, on the opening frames).
            if newly_found || want_click_through != self.click_through {
                win::set_click_through(hwnd, want_click_through);
                self.click_through = want_click_through;
            }
            win::set_topmost(hwnd);
        }
    }
}

fn paint_bar(
    ui: &mut egui::Ui,
    status: &Status,
    seek_drag: &mut Option<f32>,
    vol_drag: &mut Option<f32>,
) -> bool {
    let mut interacted = false;
    let rect = ui.max_rect();
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(0), Color32::from_rgba_premultiplied(18, 18, 18, 210));
    painter.rect_stroke(
        rect,
        CornerRadius::same(0),
        Stroke::new(1.0, Color32::from_rgba_premultiplied(255, 255, 255, 40)),
        egui::StrokeKind::Inside,
    );

    ui.horizontal_centered(|ui| {
        ui.add_space(12.0);

        if ui.button("⏮").clicked() {
            interacted = true;
            let _ = vlc_http::seek(-10);
        }
        let play_label = if status.state == "playing" { "⏸" } else { "▶" };
        if ui.button(play_label).clicked() {
            interacted = true;
            let _ = vlc_http::play_pause();
        }
        if ui.button("⏭").clicked() {
            interacted = true;
            let _ = vlc_http::seek(10);
        }

        ui.add_space(12.0);

        let mut pos = seek_drag.unwrap_or(status.position);
        let seek_resp = ui.add(
            egui::Slider::new(&mut pos, 0.0..=1.0)
                .show_value(false)
                .trailing_fill(true),
        );
        if seek_resp.dragged() {
            interacted = true;
            *seek_drag = Some(pos);
        }
        if seek_resp.drag_stopped() {
            interacted = true;
            let target_secs = (pos * status.length as f32) as i64;
            let delta = target_secs - status.time;
            let _ = vlc_http::seek(delta);
            *seek_drag = None;
        }
        ui.label(format!(
            "{}:{:02} / {}:{:02}",
            status.time / 60,
            status.time % 60,
            status.length / 60,
            status.length % 60
        ));

        ui.add_space(12.0);
        ui.separator();

        ui.label("🔊");
        let cur_vol_pct = (status.volume as f32 / 256.0 * 100.0).clamp(0.0, 200.0);
        let mut vol = vol_drag.unwrap_or(cur_vol_pct);
        let vol_resp = ui.add(
            egui::Slider::new(&mut vol, 0.0..=150.0)
                .show_value(false)
                .fixed_decimals(0),
        );
        if vol_resp.dragged() {
            interacted = true;
            *vol_drag = Some(vol);
        }
        if vol_resp.drag_stopped() {
            interacted = true;
            let delta = ((vol - cur_vol_pct) / 100.0 * 256.0) as i64;
            let _ = vlc_http::volume_delta(delta);
            *vol_drag = None;
        }

        ui.add_space(12.0);
    });

    interacted
}

#[cfg(target_os = "windows")]
mod win {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    use winapi::shared::minwindef::{BOOL, LPARAM};
    use winapi::shared::windef::{HWND, POINT};
    use winapi::um::winuser::{
        EnumWindows, GetCursorPos, GetWindowLongPtrW, GetWindowTextW, IsWindowVisible,
        SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE,
        SWP_NOSIZE, WS_EX_LAYERED, WS_EX_TRANSPARENT,
    };

    struct FindState {
        needle: Vec<u16>,
        found: HWND,
    }

    unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let state = unsafe { &mut *(lparam as *mut FindState) };
        if unsafe { IsWindowVisible(hwnd) } == 0 {
            return 1;
        }
        let mut buf = [0u16; 256];
        let len = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) } as usize;
        if len == 0 {
            return 1;
        }
        if &buf[..len] == state.needle.as_slice() {
            state.found = hwnd;
            return 0;
        }
        1
    }

    pub fn find_overlay_hwnd() -> Option<usize> {
        let needle: Vec<u16> = OsStr::new(super::OVERLAY_TITLE).encode_wide().collect();
        let mut state = FindState {
            needle,
            found: std::ptr::null_mut(),
        };
        unsafe { EnumWindows(Some(enum_cb), &mut state as *mut FindState as LPARAM) };
        if state.found.is_null() {
            None
        } else {
            Some(state.found as usize)
        }
    }

    pub fn set_click_through(hwnd: usize, enable: bool) {
        let hwnd = hwnd as HWND;
        unsafe {
            let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
            // WS_EX_LAYERED stays set permanently once applied — removing it
            // can disrupt the per-pixel-alpha compositing the transparent
            // viewport relies on; only WS_EX_TRANSPARENT is toggled.
            let new_style = if enable {
                style | WS_EX_TRANSPARENT | WS_EX_LAYERED
            } else {
                (style | WS_EX_LAYERED) & !WS_EX_TRANSPARENT
            };
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_style as isize);
        }
    }

    pub fn set_topmost(hwnd: usize) {
        let hwnd = hwnd as HWND;
        unsafe {
            SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }

    pub fn global_cursor_pos() -> Option<(i32, i32)> {
        let mut p = POINT { x: 0, y: 0 };
        let ok = unsafe { GetCursorPos(&mut p) };
        if ok == 0 {
            None
        } else {
            Some((p.x, p.y))
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod win {
    pub fn find_overlay_hwnd() -> Option<usize> {
        None
    }
    pub fn set_click_through(_hwnd: usize, _enable: bool) {}
    pub fn set_topmost(_hwnd: usize) {}
    pub fn global_cursor_pos() -> Option<(i32, i32)> {
        None
    }
}
