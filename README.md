<div align="center">
  <img src="https://www.rust-lang.org/logos/rust-logo-512x512.png" width="120" alt="Rust logo"/>
  <h1>fake_full_screen</h1>
  <p>Force VLC into a borderless fake-fullscreen for any chosen region of your screen.</p>
</div>

---

## What it does

`fake_full_screen` is a borderless overlay that covers your entire primary
monitor.  You divide the screen into rectangular regions by clicking, then
select a region and hit **Snap VLC** — the app finds VLC's window, strips its
decorations, and repositions it to fill that region exactly.

## Features

- **Dark / light theme** — detected from the OS, with a manual toggle
- **Interactive region editor** — one box covering the whole screen to start
- **Split anywhere** — click inside a region to split it vertically or
  horizontally at the clicked point
- **Recursive splitting** — sub-regions can be split again; minimum size is
  enforced so panes never become unusable
- **Select mode** — switch from splitting to selecting; click a leaf to
  highlight it (click again to deselect)
- **Snap VLC** — with a region selected, click **▶ Snap VLC** to:
  1. Find VLC's window by title substring search (`EnumWindows`)
  2. Strip its title bar and resize borders (`SetWindowLongPtrW`)
  3. Move and resize it to cover the selected region (`SetWindowPos`)
- **Escape / ✕** — close the overlay

## Building

```powershell
cargo build --release
.\target\release\fake_full_screen.exe
```

Requires Rust 1.80+ and Windows (Win32 API used for window manipulation).
The crate compiles on Linux/macOS too (VLC snap is a no-op stub there).

## Usage

1. Run the app — the full primary monitor appears as a single region.
2. **Split mode** (default): choose Vertical or Horizontal, then click
   inside a region to split it at that point.  Repeat as needed.
3. Switch to **Select mode** in the toolbar.
4. Click the region you want VLC to fill — it highlights green.
5. Open VLC (if not already running).
6. Click **▶ Snap VLC** — VLC moves and goes borderless inside the region.
7. Press **Escape** or **✕** to exit the overlay.

## How the VLC snap works (Windows)

```
EnumWindows  →  find HWND whose title contains "VLC"
                    ↓
SetWindowLongPtrW(hwnd, GWL_STYLE, style & !DECORATION_BITS)
                    ↓
SetWindowPos(hwnd, HWND_TOPMOST, x, y, w, h, SWP_FRAMECHANGED)
```

`DECORATION_BITS` covers `WS_CAPTION | WS_THICKFRAME | WS_MINIMIZEBOX |
WS_MAXIMIZEBOX | WS_SYSMENU | WS_BORDER | WS_DLGFRAME`.

## Project structure

```
src/
  main.rs     entry point, NativeOptions, Windows subsystem pragma
  app.rs      App struct, mode/selection state, toolbar, canvas
  region.rs   binary-tree Region type, split logic
  theme.rs    OS theme detection, dark/light toggle
  vlc.rs      Win32 window-find + borderless snap (cfg-gated)
```
