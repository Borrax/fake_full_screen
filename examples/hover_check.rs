// Headless-ish integration check for the fullscreen HOVER controls: launches
// VLC, snaps it into a rect, then programmatically moves the cursor over the
// snapped region and polls VLC's top-level windows for the floating fullscreen
// controller. Prints whether the controller actually appears on hover.
//
//   cargo run --example hover_check -- <x> <y> <w> <h>
//
// Drives the real desktop (SetCursorPos), so run it on the machine's session.
use std::path::PathBuf;

fn main() {
    #[cfg(not(windows))]
    eprintln!("hover_check is Windows-only");
    #[cfg(windows)]
    win::run();
}

#[cfg(windows)]
mod win {
    use std::ffi::{OsStr, OsString};
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::path::PathBuf;

    use winapi::shared::minwindef::{BOOL, DWORD, LPARAM};
    use winapi::shared::windef::{HWND, POINT, RECT};
    use winapi::um::winuser::{
        EnumWindows, GetClassNameW, GetCursorPos, GetWindowRect, GetWindowTextW,
        GetWindowThreadProcessId, IsWindowVisible, SetCursorPos,
    };

    #[derive(Clone)]
    struct WinInfo {
        hwnd: usize,
        class: String,
        title: String,
        rect: (i32, i32, i32, i32), // left, top, right, bottom
    }
    impl WinInfo {
        fn w(&self) -> i32 {
            self.rect.2 - self.rect.0
        }
        fn h(&self) -> i32 {
            self.rect.3 - self.rect.1
        }
        fn desc(&self) -> String {
            format!(
                "hwnd={:#x} {}x{} @({},{}) class='{}' title='{}'",
                self.hwnd,
                self.w(),
                self.h(),
                self.rect.0,
                self.rect.1,
                self.class,
                self.title
            )
        }
    }

    struct Collect {
        pid: DWORD,
        out: Vec<WinInfo>,
    }

    fn wide_to_string(buf: &[u16]) -> String {
        OsString::from_wide(buf).to_string_lossy().into_owned()
    }

    unsafe extern "system" fn collect_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let c = unsafe { &mut *(lparam as *mut Collect) };
        if unsafe { IsWindowVisible(hwnd) } == 0 {
            return 1;
        }
        let mut pid: DWORD = 0;
        unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
        if pid != c.pid {
            return 1;
        }

        let mut cbuf = [0u16; 256];
        let clen = unsafe { GetClassNameW(hwnd, cbuf.as_mut_ptr(), cbuf.len() as i32) } as usize;
        let class = wide_to_string(&cbuf[..clen]);

        let mut tbuf = [0u16; 256];
        let tlen = unsafe { GetWindowTextW(hwnd, tbuf.as_mut_ptr(), tbuf.len() as i32) } as usize;
        let title = wide_to_string(&tbuf[..tlen]);

        let mut r: RECT = unsafe { std::mem::zeroed() };
        unsafe { GetWindowRect(hwnd, &mut r) };

        c.out.push(WinInfo {
            hwnd: hwnd as usize,
            class,
            title,
            rect: (r.left, r.top, r.right, r.bottom),
        });
        1
    }

    fn windows_for_pid(pid: DWORD) -> Vec<WinInfo> {
        let mut c = Collect {
            pid,
            out: Vec::new(),
        };
        unsafe { EnumWindows(Some(collect_cb), &mut c as *mut Collect as LPARAM) };
        c.out
    }

    struct Find {
        needle: Vec<u16>,
        hwnd: HWND,
        pid: DWORD,
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
        if title.windows(f.needle.len()).any(|w| w == f.needle.as_slice()) {
            f.hwnd = hwnd;
            unsafe { GetWindowThreadProcessId(hwnd, &mut f.pid) };
            return 0;
        }
        1
    }

    fn find_vlc() -> Option<(HWND, DWORD)> {
        let needle: Vec<u16> = OsStr::new("VLC").encode_wide().collect();
        let mut f = Find {
            needle,
            hwnd: std::ptr::null_mut(),
            pid: 0,
        };
        unsafe { EnumWindows(Some(find_cb), &mut f as *mut Find as LPARAM) };
        if f.hwnd.is_null() {
            None
        } else {
            Some((f.hwnd, f.pid))
        }
    }

    fn get_cursor() -> (i32, i32) {
        let mut p = POINT { x: 0, y: 0 };
        unsafe { GetCursorPos(&mut p) };
        (p.x, p.y)
    }

    // A fullscreen-controller candidate: a visible VLC-owned window that is not
    // the main window, wide, and short (the FSC is a horizontal control strip).
    fn fsc_candidates<'a>(wins: &'a [WinInfo], main: usize) -> Vec<&'a WinInfo> {
        wins.iter()
            .filter(|w| w.hwnd != main && w.w() > 300 && w.h() > 0 && w.h() < 160)
            .collect()
    }

    pub fn run() {
        let args: Vec<String> = std::env::args().collect();
        let x: f32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(300.0);
        let y: f32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(200.0);
        let w: f32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1000.0);
        let h: f32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(560.0);

        let video = PathBuf::from(r"C:\Users\catarract\Desktop\test_video.mp4");
        println!(
            "launch_vlc -> {:?}",
            fake_full_screen::vlc::launch_vlc(&video)
        );
        std::thread::sleep(std::time::Duration::from_millis(3000));

        let (main_hwnd, pid) = match find_vlc() {
            Some(v) => v,
            None => {
                println!("FAIL: no VLC window found");
                return;
            }
        };
        println!("VLC main hwnd={:#x} pid={}", main_hwnd as usize, pid);

        let rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h));
        println!(
            "snap_vlc({rect:?}) -> {:?}",
            fake_full_screen::vlc::snap_vlc(&rect)
        );
        std::thread::sleep(std::time::Duration::from_millis(2800)); // past re-assert passes

        let before = windows_for_pid(pid);
        println!("\n-- VLC windows BEFORE hover ({}) --", before.len());
        for wi in &before {
            println!("  {}", wi.desc());
        }

        // Save cursor, then nudge across the snapped region near the bottom (where
        // the controller lives), polling for the FSC while the mouse is moving —
        // it auto-hides once the pointer goes stationary.
        let saved = get_cursor();
        let cx = x as i32 + w as i32 / 2;
        let mut found: Option<WinInfo> = None;
        for i in 0..40 {
            let px = cx + ((i % 4) as i32 - 2) * 8;
            let py = y as i32 + h as i32 - 45 - (i % 3) as i32 * 6;
            unsafe { SetCursorPos(px, py) };
            std::thread::sleep(std::time::Duration::from_millis(70));
            let now = windows_for_pid(pid);
            if let Some(f) = fsc_candidates(&now, main_hwnd as usize).first() {
                found = Some((*f).clone());
                break;
            }
        }

        let after = windows_for_pid(pid);
        unsafe { SetCursorPos(saved.0, saved.1) }; // restore cursor

        println!("\n-- VLC windows AFTER hover ({}) --", after.len());
        for wi in &after {
            println!("  {}", wi.desc());
        }

        println!();
        match found {
            Some(f) => println!("RESULT: fullscreen controller appeared on hover ✅  {}", f.desc()),
            None => println!("RESULT: no fullscreen controller detected on hover ❌"),
        }
    }
}
