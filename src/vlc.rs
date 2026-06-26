//! Win32 helpers for finding VLC's window and snapping it into a target rect,
//! plus launching VLC in a chrome-free state.
//!
//! Non-Windows targets get no-op stubs so the crate compiles on Linux / macOS.

use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapResult {
    Ok,
    NotFound,
    // Win32 error code
    Error(u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchResult {
    Ok,
    // VLC executable could not be located on disk
    NotInstalled,
    // std::process spawn error, stringified
    Error(String),
}

// VLC flags that suppress all of VLC's own UI and stop it fighting our snap:
//   --qt-minimal-view      drops the menu bar and the bottom controls/seek bar
//   --fullscreen           VLC enters its own fullscreen mode so the floating
//                          fullscreen controller appears on hover and auto-hides
//   --no-qt-video-autoresize  stops VLC resizing its window to the native video
//                          size, which otherwise undoes our SetWindowPos
//   --no-video-title-show / --no-osd  suppress overlay text
// Verified on the test machine: this leaves only the OS titlebar, which
// snap_vlc then strips, and the snapped rect holds.
const MINIMAL_VLC_ARGS: &[&str] = &[
    "--qt-minimal-view",
    "--fullscreen",
    "--no-qt-video-autoresize",
    "--no-video-title-show",
    "--no-osd",
    "--loop",
];

#[cfg(target_os = "windows")]
mod imp {
    use super::{LaunchResult, SnapResult, MINIMAL_VLC_ARGS};
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use winapi::shared::minwindef::{BOOL, LPARAM};
    use winapi::shared::windef::HWND;
    use winapi::um::errhandlingapi::GetLastError;
    use winapi::um::winuser::{
        DrawMenuBar, EnumWindows, GetWindowLongPtrW, GetWindowTextW, IsWindowVisible, SetMenu,
        SetWindowLongPtrW, SetWindowPos, ShowWindow, GWL_STYLE, HWND_TOPMOST, SWP_FRAMECHANGED,
        SWP_NOACTIVATE, SW_RESTORE, WS_BORDER, WS_CAPTION, WS_DLGFRAME, WS_MAXIMIZEBOX,
        WS_MINIMIZEBOX, WS_SYSMENU, WS_THICKFRAME,
    };

    const DECORATION_STYLES: u32 = WS_CAPTION
        | WS_THICKFRAME
        | WS_MINIMIZEBOX
        | WS_MAXIMIZEBOX
        | WS_SYSMENU
        | WS_BORDER
        | WS_DLGFRAME;

    struct FindState {
        needle: Vec<u16>,
        found: HWND,
    }

    unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let state = unsafe { &mut *(lparam as *mut FindState) };

        if unsafe { IsWindowVisible(hwnd) } == 0 {
            return 1;
        }

        let mut buf = [0u16; 512];
        let len = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) } as usize;
        if len == 0 {
            return 1;
        }

        let title = &buf[..len];
        if title
            .windows(state.needle.len())
            .any(|w| w == state.needle.as_slice())
        {
            state.found = hwnd;
            return 0;
        }

        1
    }

    fn find_window(title_fragment: &str) -> Option<HWND> {
        // encode without null terminator — we're doing a substring search
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

    unsafe fn hide_menu(hwnd: HWND) {
        unsafe {
            SetMenu(hwnd, std::ptr::null_mut());
            DrawMenuBar(hwnd);
        }
    }

    // SWP_NOZORDER must be absent so HWND_TOPMOST actually takes effect.
    unsafe fn place_window(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) -> i32 {
        unsafe {
            SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                x,
                y,
                w,
                h,
                SWP_NOACTIVATE | SWP_FRAMECHANGED,
            )
        }
    }

    fn snap_hwnd(hwnd: HWND, rect: &egui::Rect) -> SnapResult {
        let x = rect.left() as i32;
        let y = rect.top() as i32;
        let w = rect.width() as i32;
        let h = rect.height() as i32;

        unsafe {
            ShowWindow(hwnd, SW_RESTORE);

            // Belt-and-suspenders: VLC launched with --qt-minimal-view has no
            // menu, but stripping it is harmless if one is ever present.
            hide_menu(hwnd);

            let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
            let new_style = (style & !DECORATION_STYLES) as isize;
            SetWindowLongPtrW(hwnd, GWL_STYLE, new_style);

            if place_window(hwnd, x, y, w, h) == 0 {
                return SnapResult::Error(GetLastError());
            }
        }

        // VLC's Qt event loop runs layout passes for the first few seconds after
        // the window appears, each resizing the top-level window back toward
        // fullscreen and undoing the SetWindowPos above. Re-assert the same
        // placement repeatedly across that settling window so the final state
        // sticks regardless of when the user snaps. HWND is passed as usize
        // because raw pointers are not Send; we rebuild it inside the thread.
        let hwnd_bits = hwnd as usize;
        std::thread::spawn(move || {
            let hwnd = hwnd_bits as HWND;
            for delay in [120u64, 250, 500, 800, 1200, 1800, 2500] {
                std::thread::sleep(std::time::Duration::from_millis(delay));
                unsafe { place_window(hwnd, x, y, w, h) };
            }
        });

        SnapResult::Ok
    }

    pub fn snap_vlc(rect: &egui::Rect) -> SnapResult {
        match find_window("VLC") {
            Some(h) => snap_hwnd(h, rect),
            None => SnapResult::NotFound,
        }
    }

    fn vlc_exe() -> Option<PathBuf> {
        // Standard install locations; fall back to PATH lookup via the bare name.
        let candidates = [
            r"C:\Program Files\VideoLAN\VLC\vlc.exe",
            r"C:\Program Files (x86)\VideoLAN\VLC\vlc.exe",
        ];
        for c in candidates {
            let p = PathBuf::from(c);
            if p.exists() {
                return Some(p);
            }
        }
        // Let the OS resolve it from PATH as a last resort.
        Some(PathBuf::from("vlc.exe"))
    }

    pub fn launch_vlc(video: &Path) -> LaunchResult {
        let Some(exe) = vlc_exe() else {
            return LaunchResult::NotInstalled;
        };

        match Command::new(exe).args(MINIMAL_VLC_ARGS).arg(video).spawn() {
            Ok(_) => LaunchResult::Ok,
            Err(e) => LaunchResult::Error(e.to_string()),
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    use super::{LaunchResult, SnapResult};
    use std::path::Path;

    pub fn snap_vlc(_rect: &egui::Rect) -> SnapResult {
        SnapResult::NotFound
    }

    pub fn launch_vlc(_video: &Path) -> LaunchResult {
        LaunchResult::NotInstalled
    }
}

pub fn snap_vlc(rect: &egui::Rect) -> SnapResult {
    imp::snap_vlc(rect)
}

pub fn launch_vlc(video: &Path) -> LaunchResult {
    imp::launch_vlc(video)
}
