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
    use winapi::um::errhandlingapi::GetLastError;
    use winapi::um::winuser::{
        DrawMenuBar, EnumChildWindows, EnumWindows, GetClientRect, GetWindowTextW,
        IsWindowVisible, SetMenu, SetWindowLongPtrW, SetWindowPos, ShowWindow,
        GWL_STYLE, HWND_TOPMOST, SWP_FRAMECHANGED, SWP_NOACTIVATE, SW_HIDE,
        SW_RESTORE, WS_BORDER, WS_CAPTION, WS_DLGFRAME, WS_MAXIMIZEBOX,
        WS_MINIMIZEBOX, WS_SYSMENU, WS_THICKFRAME,
    };

    const DECORATION_STYLES: u32 = WS_CAPTION
        | WS_THICKFRAME
        | WS_MINIMIZEBOX
        | WS_MAXIMIZEBOX
        | WS_SYSMENU
        | WS_BORDER
        | WS_DLGFRAME;

    // Height threshold (pixels) for identifying toolbar-height child windows.
    const TOOLBAR_MAX_HEIGHT: i32 = 80;

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

    struct ControlsState {
        parent_width: i32,
    }

    unsafe extern "system" fn controls_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let state = unsafe { &*(lparam as *const ControlsState) };

        if unsafe { IsWindowVisible(hwnd) } == 0 {
            return 1;
        }

        let mut rect = unsafe { std::mem::zeroed() };
        if unsafe { GetClientRect(hwnd, &mut rect) } == 0 {
            return 1;
        }

        let h = rect.bottom - rect.top;
        let w = rect.right - rect.left;

        // Hide child windows that span most of the parent width and are toolbar-tall.
        if h > 0 && h <= TOOLBAR_MAX_HEIGHT && w >= state.parent_width / 2 {
            unsafe { ShowWindow(hwnd, SW_HIDE) };
        }

        1
    }

    unsafe fn hide_controls(hwnd: HWND, client_width: i32) {
        let state = ControlsState {
            parent_width: client_width,
        };
        unsafe {
            EnumChildWindows(
                hwnd,
                Some(controls_cb),
                &state as *const ControlsState as LPARAM,
            );
        }
    }

    fn snap_hwnd(hwnd: HWND, rect: &egui::Rect) -> SnapResult {
        unsafe {
            ShowWindow(hwnd, SW_RESTORE);

            let style = winapi::um::winuser::GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
            let new_style = (style & !DECORATION_STYLES) as isize;
            SetWindowLongPtrW(hwnd, GWL_STYLE, new_style);

            // SWP_NOZORDER must be absent so HWND_TOPMOST actually takes effect.
            let ok = SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                rect.left() as i32,
                rect.top() as i32,
                rect.width() as i32,
                rect.height() as i32,
                SWP_NOACTIVATE | SWP_FRAMECHANGED,
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
