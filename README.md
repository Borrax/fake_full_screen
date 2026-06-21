<div align="center">
  <img src="https://www.rust-lang.org/logos/rust-logo-512x512.png" width="120" alt="Rust logo"/>
  <h1>fake_full_screen</h1>
  <p>Force VLC (or any window) into a borderless fake-fullscreen for a chosen region of your screen.</p>
</div>

---

## What it does

`fake_full_screen` lets you draw one or more rectangular regions on your screen.
Each region can host a window (e.g. VLC) that is resized, repositioned, and made
borderless so it fills that region exactly — a "fake" fullscreen inside an arbitrary
rectangle rather than the whole monitor.

## Features

- **Dark / light theme** — detected automatically from the OS, with a manual toggle in the UI
- **Interactive region editor** — start with one box covering the full available screen
- **Split anywhere** — click inside a region to split it vertically or horizontally
  (direction chosen via toolbar button)
- **Recursive splitting** — keep splitting sub-regions as deep as you like
  (minimum region size is enforced so regions never become too small to use)
- **Linux / X11** — built for the Linux desktop environment

## Building

```bash
cargo build --release
./target/release/fake_full_screen
```

> Requires Rust 1.70+ and an X11 or Wayland display.

## Usage

1. Run the application — the full available screen area appears as a single region.
2. Choose **Vertical** or **Horizontal** split from the toolbar.
3. Click inside a region at the point where you want to split it.
4. Repeat for any sub-region.
5. Select a region and assign a window to it (future feature — coming soon).
