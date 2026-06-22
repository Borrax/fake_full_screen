//! Win32 helpers for finding VLC's window and snapping it into a target rect.
//!
//! Non-Windows targets get no-op stubs so the crate compiles on Linux / macOS.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapResult {
    Ok,
    NotFound,
    // Win32 error code
    Error(u32),
}

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
        let state = &mut *(lparam as *mut FindState);

        if IsWindowVisible(hwnd) == 0 {
            return 1;
        }

        let mut buf = [0u16; 512];
        let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) as usize;
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

    fn snap_hwnd(hwnd: HWND, rect: &egui::Rect) -> SnapResult {
        unsafe {
            ShowWindow(hwnd, SW_RESTORE);

            let style = winapi::um::winuser::GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
            let new_style = (style & !DECORATION_STYLES) as isize;
            SetWindowLongPtrW(hwnd, GWL_STYLE, new_style);

            let ok = SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                rect.left() as i32,
                rect.top() as i32,
                rect.width() as i32,
                rect.height() as i32,
                SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );

            if ok == 0 {
                SnapResult::Error(GetLastError())
            } else {
                SnapResult::Ok
            }
        }
    }

    pub fn snap_vlc(rect: &egui::Rect) -> SnapResult {
        match find_window("VLC") {
            Some(h) => snap_hwnd(h, rect),
            None => SnapResult::NotFound,
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    use super::SnapResult;

    pub fn snap_vlc(_rect: &egui::Rect) -> SnapResult {
        SnapResult::NotFound
    }
}

pub fn snap_vlc(rect: &egui::Rect) -> SnapResult {
    imp::snap_vlc(rect)
}
