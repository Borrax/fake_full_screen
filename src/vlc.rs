//! Win32 helpers for finding VLC's window and snapping it into a target rect.
//!
//! On non-Windows targets every public function is a no-op stub so the crate
//! compiles cleanly on Linux / macOS for development.

/// Result of attempting to find and snap VLC.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapResult {
    /// VLC window found and successfully repositioned.
    Ok,
    /// No VLC window could be found.
    NotFound,
    /// Win32 call failed (error code attached).
    Error(u32),
}

// ── Real implementation (Windows only) ───────────────────────────────────────

#[cfg(target_os = "windows")]
mod imp {
    use super::SnapResult;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    use winapi::shared::minwindef::{BOOL, LPARAM};
    use winapi::shared::windef::HWND;
    use winapi::um::winuser::{
        EnumWindows, GetLastError, GetWindowTextW, IsWindowVisible, SetWindowLongPtrW,
        SetWindowPos, ShowWindow, GWL_STYLE, HWND_TOPMOST, SWP_FRAMECHANGED,
        SWP_NOACTIVATE, SWP_NOZORDER, SW_RESTORE, WS_BORDER, WS_CAPTION, WS_DLGFRAME,
        WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_SYSMENU, WS_THICKFRAME,
    };

    /// Window style bits that make up a normal decorated frame — we strip all
    /// of these to produce a borderless window.
    const DECORATION_STYLES: u32 = WS_CAPTION
        | WS_THICKFRAME
        | WS_MINIMIZEBOX
        | WS_MAXIMIZEBOX
        | WS_SYSMENU
        | WS_BORDER
        | WS_DLGFRAME;

    // ── EnumWindows callback ──────────────────────────────────────────────────

    /// State threaded through the `EnumWindows` callback via `LPARAM`.
    struct FindState {
        needle: Vec<u16>, // UTF-16 substring to look for in window titles
        found: HWND,
    }

    /// `EnumWindows` callback.  Searches visible top-level windows for one
    /// whose title contains the needle string.
    unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
        // Safety: lparam is always a valid *mut FindState we own.
        let state = &mut *(lparam as *mut FindState);

        if IsWindowVisible(hwnd) == 0 {
            return 1; // keep enumerating
        }

        let mut buf = [0u16; 512];
        let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) as usize;
        if len == 0 {
            return 1;
        }

        let title = &buf[..len];
        // Substring search: look for every u16 in needle appearing in title.
        if title
            .windows(state.needle.len())
            .any(|w| w == state.needle.as_slice())
        {
            state.found = hwnd;
            return 0; // stop enumerating
        }

        1 // keep going
    }

    /// Find the first visible window whose title contains `title_fragment`.
    fn find_window(title_fragment: &str) -> Option<HWND> {
        // Encode the search string to UTF-16 (without a null terminator — we
        // are doing a substring search, not passing it to a Win32 API).
        let needle: Vec<u16> = OsStr::new(title_fragment).encode_wide().collect();
        let mut state = FindState {
            needle,
            found: std::ptr::null_mut(),
        };

        unsafe {
            EnumWindows(Some(enum_cb), &mut state as *mut FindState as LPARAM);
        }

        if state.found.is_null() {
            None
        } else {
            Some(state.found)
        }
    }

    /// Strip decorations from `hwnd` and move / resize it to cover `rect`.
    ///
    /// `rect` is in egui logical pixels relative to the top-left of the
    /// primary monitor (which is where our fake-fullscreen window lives).
    fn snap_hwnd(hwnd: HWND, rect: &egui::Rect) -> SnapResult {
        unsafe {
            // 1. Restore the window first (in case it is minimised).
            ShowWindow(hwnd, SW_RESTORE);

            // 2. Read current style and strip decoration bits.
            let style = winapi::um::winuser::GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
            let new_style = (style & !DECORATION_STYLES) as isize;
            SetWindowLongPtrW(hwnd, GWL_STYLE, new_style);

            // 3. Move and resize.
            let x = rect.left() as i32;
            let y = rect.top() as i32;
            let w = rect.width() as i32;
            let h = rect.height() as i32;

            let ok = SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                x,
                y,
                w,
                h,
                SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );

            if ok == 0 {
                SnapResult::Error(GetLastError())
            } else {
                SnapResult::Ok
            }
        }
    }

    /// Find VLC and snap it into `rect`.
    ///
    /// Tries several title substrings VLC uses across versions and languages.
    pub fn snap_vlc(rect: &egui::Rect) -> SnapResult {
        // VLC's window title always contains "VLC" somewhere.
        let hwnd = match find_window("VLC") {
            Some(h) => h,
            None => return SnapResult::NotFound,
        };
        snap_hwnd(hwnd, rect)
    }
}

// ── Stub implementation (non-Windows) ────────────────────────────────────────

#[cfg(not(target_os = "windows"))]
mod imp {
    use super::SnapResult;

    pub fn snap_vlc(_rect: &egui::Rect) -> SnapResult {
        SnapResult::NotFound
    }
}

// ── Public re-export ──────────────────────────────────────────────────────────

/// Find VLC's window and snap it (borderless) into the given screen rectangle.
pub fn snap_vlc(rect: &egui::Rect) -> SnapResult {
    imp::snap_vlc(rect)
}
