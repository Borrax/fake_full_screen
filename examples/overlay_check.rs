// Headless verification of the Phase 2 hover-fade playback control bar
// (src/overlay.rs). Launches VLC, snaps it into a fixed rect, activates the
// Overlay for that rect, then drives the REAL desktop (SetCursorPos +
// synthesized clicks) from a background thread to prove:
//   (a) the bar (title "ffs_overlay_bar") appears, is visible, is
//       WS_EX_TOPMOST, and sits at the bottom of the snapped region once the
//       cursor hovers the region;
//   (b) clicking its play/pause button actually flips vlc_http::status().state;
//   (c) once the cursor leaves and the ~1.2s linger elapses, the bar's HWND
//       gains WS_EX_TRANSPARENT (click-through).
//
// Build then run the .exe DIRECTLY (not `cargo run`) with output redirected,
// since this blocks in the eframe event loop until the check thread exits:
//   cargo build --example overlay_check
//   ./target/debug/examples/overlay_check.exe > out.txt 2>&1
//   cat out.txt

use std::path::PathBuf;
use std::time::Duration;

use fake_full_screen::{overlay::Overlay, vlc};

const RX: f32 = 300.0;
const RY: f32 = 200.0;
const RW: f32 = 1000.0;
const RH: f32 = 560.0;

fn region() -> egui::Rect {
    egui::Rect::from_min_size(egui::pos2(RX, RY), egui::vec2(RW, RH))
}

struct CheckApp {
    launched: bool,
    overlay: Option<Overlay>,
}

impl eframe::App for CheckApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        if !self.launched {
            self.launched = true;
            let video = PathBuf::from(r"C:\Users\catarract\Desktop\test_video.mp4");
            println!("launch_vlc -> {:?}", vlc::launch_vlc(&video));
        }

        if self.overlay.is_none() && win::find_vlc().is_some() {
            let rect = region();
            println!("snap_vlc -> {:?}", vlc::snap_vlc(&rect));
            self.overlay = Some(Overlay::new(rect));
        }

        if let Some(o) = &mut self.overlay {
            o.show(&ctx);
        }

        ctx.request_repaint_after(Duration::from_millis(50));
    }
}

fn main() {
    #[cfg(not(windows))]
    {
        eprintln!("overlay_check is Windows-only");
        return;
    }

    #[cfg(windows)]
    {
        std::thread::spawn(win::checker_thread);

        let viewport = egui::ViewportBuilder::default()
            .with_title("overlay_check_host")
            .with_inner_size(egui::vec2(200.0, 100.0));
        let native_options = eframe::NativeOptions {
            viewport,
            ..Default::default()
        };
        let _ = eframe::run_native(
            "overlay_check_host",
            native_options,
            Box::new(|_cc| {
                Ok(Box::new(CheckApp {
                    launched: false,
                    overlay: None,
                }))
            }),
        );
    }
}

#[cfg(windows)]
mod win {
    use super::region;
    use fake_full_screen::{overlay, vlc_http};
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::process::Command;
    use std::thread::sleep;
    use std::time::Duration;

    use winapi::shared::minwindef::{BOOL, LPARAM};
    use winapi::shared::windef::{HWND, POINT, RECT};
    use winapi::um::winuser::{
        mouse_event, EnumWindows, GetCursorPos, GetWindowLongPtrW, GetWindowRect,
        GetWindowTextW, IsWindowVisible, SetCursorPos, GWL_EXSTYLE, MOUSEEVENTF_LEFTDOWN,
        MOUSEEVENTF_LEFTUP, WS_EX_TOPMOST, WS_EX_TRANSPARENT,
    };

    struct Find {
        needle: Vec<u16>,
        exact: bool,
        found: HWND,
    }

    unsafe extern "system" fn find_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let f = unsafe { &mut *(lparam as *mut Find) };
        if unsafe { IsWindowVisible(hwnd) } == 0 {
            return 1;
        }
        let mut buf = [0u16; 512];
        let len = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) } as usize;
        if len == 0 {
            return 1;
        }
        let title = &buf[..len];
        let matched = if f.exact {
            title == f.needle.as_slice()
        } else {
            title.windows(f.needle.len()).any(|w| w == f.needle.as_slice())
        };
        if matched {
            f.found = hwnd;
            return 0;
        }
        1
    }

    fn find_window(needle: &str, exact: bool) -> Option<HWND> {
        let needle: Vec<u16> = OsStr::new(needle).encode_wide().collect();
        let mut f = Find {
            needle,
            exact,
            found: std::ptr::null_mut(),
        };
        unsafe { EnumWindows(Some(find_cb), &mut f as *mut Find as LPARAM) };
        if f.found.is_null() {
            None
        } else {
            Some(f.found)
        }
    }

    pub fn find_vlc() -> Option<HWND> {
        find_window("VLC", false)
    }

    fn find_overlay() -> Option<HWND> {
        find_window(overlay::OVERLAY_TITLE, true)
    }

    fn window_rect(hwnd: HWND) -> (i32, i32, i32, i32) {
        let mut r: RECT = unsafe { std::mem::zeroed() };
        unsafe { GetWindowRect(hwnd, &mut r) };
        (r.left, r.top, r.right, r.bottom)
    }

    fn ex_style(hwnd: HWND) -> u32 {
        unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 }
    }

    fn set_cursor(x: i32, y: i32) {
        unsafe { SetCursorPos(x, y) };
    }

    fn get_cursor() -> (i32, i32) {
        let mut p = POINT { x: 0, y: 0 };
        unsafe { GetCursorPos(&mut p) };
        (p.x, p.y)
    }

    fn click_at(x: i32, y: i32) {
        set_cursor(x, y);
        sleep(Duration::from_millis(50));
        unsafe {
            mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0, 0);
            sleep_ms(40);
            mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0, 0);
        }
    }

    fn sleep_ms(ms: u64) {
        sleep(Duration::from_millis(ms));
    }

    fn kill_vlc() {
        let _ = Command::new("taskkill").args(["/F", "/IM", "vlc.exe"]).output();
    }

    pub fn checker_thread() {
        let r = region();
        let bar_top = (r.bottom() - 72.0) as i32;
        let bar_bottom = r.bottom() as i32;
        let bar_left = r.left() as i32;
        let bar_right = r.right() as i32;

        let saved = get_cursor();
        let mut all_pass = true;

        // Give VLC + snap + overlay time to come up.
        sleep_ms(5000);
        // Extra settle: wait until VLC is reporting "playing".
        for _ in 0..20 {
            if let Ok(s) = vlc_http::status() {
                if s.state == "playing" {
                    break;
                }
            }
            sleep_ms(300);
        }

        // (a) Hover -> bar appears, visible, topmost, positioned at the bottom
        // of the snapped region.
        set_cursor((r.left() + r.width() / 2.0) as i32, (r.top() + r.height() / 2.0) as i32);
        sleep_ms(900);

        let bar = find_overlay();
        let a_exists = bar.is_some();
        println!("CHECK a1 (overlay window exists): {}", if a_exists { "PASS" } else { "FAIL" });
        all_pass &= a_exists;

        let mut a_visible = false;
        let mut a_topmost = false;
        let mut a_rect_ok = false;
        if let Some(hwnd) = bar {
            a_visible = unsafe { IsWindowVisible(hwnd) } != 0;
            a_topmost = ex_style(hwnd) & WS_EX_TOPMOST != 0;
            let (l, t, rr, b) = window_rect(hwnd);
            println!("  overlay rect=({l},{t},{rr},{b}) expected bottom-of-region top~={bar_top} bottom~={bar_bottom} left~={bar_left} right~={bar_right}");
            a_rect_ok = (t - bar_top).abs() <= 20
                && (b - bar_bottom).abs() <= 20
                && (l - bar_left).abs() <= 20
                && (rr - bar_right).abs() <= 20;
        }
        println!("CHECK a2 (overlay visible): {}", if a_visible { "PASS" } else { "FAIL" });
        println!("CHECK a3 (overlay WS_EX_TOPMOST): {}", if a_topmost { "PASS" } else { "FAIL" });
        println!("CHECK a4 (overlay rect at region bottom): {}", if a_rect_ok { "PASS" } else { "FAIL" });
        all_pass &= a_visible && a_topmost && a_rect_ok;

        // (b) Click play/pause: sweep candidate x positions across the left
        // portion of the bar (button layout isn't pixel-exact, so probe).
        let before = vlc_http::status().map(|s| s.state).unwrap_or_default();
        let mut flipped = false;
        let py = bar_top + 36;
        for i in 0..30 {
            let px = bar_left + 15 + i * 6;
            click_at(px, py);
            sleep_ms(150);
            if let Ok(s) = vlc_http::status() {
                if !s.state.is_empty() && s.state != before {
                    flipped = true;
                    break;
                }
            }
        }
        println!(
            "CHECK b (play/pause click flips state, before='{before}'): {}",
            if flipped { "PASS" } else { "FAIL" }
        );
        all_pass &= flipped;

        // (c) Move far outside the region; after the fade-out linger, the bar
        // should be click-through (WS_EX_TRANSPARENT).
        set_cursor(50, 50);
        sleep_ms(2200);
        let mut c_transparent = false;
        if let Some(hwnd) = find_overlay() {
            c_transparent = ex_style(hwnd) & WS_EX_TRANSPARENT != 0;
        }
        println!(
            "CHECK c (overlay click-through after leaving+linger): {}",
            if c_transparent { "PASS" } else { "FAIL" }
        );
        all_pass &= c_transparent;

        set_cursor(saved.0, saved.1);

        println!(
            "\nRESULT: {}",
            if all_pass { "overlay checks PASS \u{2705}" } else { "overlay checks FAILED \u{274c}" }
        );

        kill_vlc();
        std::process::exit(0);
    }
}
