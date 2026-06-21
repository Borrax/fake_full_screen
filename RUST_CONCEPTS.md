# Rust Concepts Used in fake_full_screen

## Enums with data (algebraic data types)

Rust enums can carry data in each variant — this is different from C/Python enums.

```rust
enum Region {
    Leaf { rect: egui::Rect },          // struct-like variant
    Split {
        direction: SplitDirection,
        ratio: f32,
        children: Box<[Region; 2]>,     // boxed array (heap-allocated)
    },
}
```

This forms a **recursive tree** — `Split` owns two child `Region`s.

---

## Box<T> — heap allocation

`Box<T>` places a value on the heap and gives you a smart pointer to it.
It is needed here because a recursive enum would have infinite size on the stack
without indirection. `Box<[Region; 2]>` stores a fixed-size array of two Regions
on the heap.

---

## Pattern matching (`match`)

`match` in Rust is exhaustive — the compiler forces you to handle every variant.
You can destructure enum fields right in the pattern:

```rust
match self {
    Region::Leaf { rect } => { /* rect is &mut egui::Rect */ }
    Region::Split { children, .. } => { /* .. ignores unused fields */ }
}
```

---

## Traits

Traits are Rust's version of interfaces / type classes. We implement `eframe::App`:

```rust
impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) { ... }
}
```

`fn ui` is the only **required** method; others like `fn logic` have default no-op
implementations you can optionally override.

---

## Ownership and borrowing

- `&T` — shared (immutable) reference; many can exist simultaneously.
- `&mut T` — exclusive (mutable) reference; only one can exist at a time.
- Values without `&` are **moved** — ownership transfers to the new location.

Example in `try_split`:

```rust
pub fn try_split(&mut self, point: egui::Pos2, direction: SplitDirection) -> bool
//               ^^^^^  mutable borrow of self so we can modify the tree in place
```

---

## Closures

Anonymous functions captured from surrounding scope:

```rust
eframe::run_native(
    "fake_full_screen",
    options,
    Box::new(|cc| Ok(Box::new(App::new(cc)))),
    //        ^^^^^ closure receiving CreationContext
)
```

---

## `#[derive(...)]`

Automatically generates trait implementations:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection { Vertical, Horizontal }
```

- `Debug` — enables `{:?}` formatting
- `Clone` / `Copy` — Copy makes the type cheaply duplicated (no move)
- `PartialEq` / `Eq` — enables `==` comparisons

---

## Constants

```rust
const MIN_REGION_SIZE: f32 = 80.0;
```

Must have an explicit type. Evaluated at compile time. Convention: SCREAMING_SNAKE_CASE.

---

## Result<T, E> and the `?` operator

Many fallible functions return `Result<T, E>`.
The `?` operator unwraps the `Ok` value or returns the error early:

```rust
fn main() -> eframe::Result<()> {
    eframe::run_native(...)?;  // returns error if run_native fails
    Ok(())
}
```

`dark_light::detect()` also returns `Result<Mode, Error>` — we match on it:

```rust
match dark_light::detect() {
    Ok(dark_light::Mode::Light) => Theme::Light,
    _ => Theme::Dark,
}
```

---

## `pub` visibility

- `pub` — visible everywhere
- (no modifier) — visible only within the current module
- `pub(crate)` — visible within the crate but not to external users

---

## Modules

`mod region;` tells Rust to look for `src/region.rs` (or `src/region/mod.rs`)
and compile it as a submodule. `use crate::region::Region;` brings `Region`
into scope from the crate root.

---

## Conditional compilation (`#[cfg(...)]`)

`#[cfg(target_os = "windows")]` compiles a block only on Windows.
Used in `vlc.rs` to provide a real Win32 implementation on Windows and a
no-op stub on every other platform:

```rust
#[cfg(target_os = "windows")]
mod imp {
    pub fn snap_vlc(rect: &egui::Rect) -> SnapResult { /* real Win32 code */ }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    pub fn snap_vlc(_rect: &egui::Rect) -> SnapResult { SnapResult::NotFound }
}
```

The crate-level attribute `#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]`
uses the same mechanism to suppress the console window on Windows without
affecting other targets.

---

## `unsafe` blocks

Some Win32 API calls require `unsafe` because Rust cannot verify their safety
contracts at compile time:

```rust
let (w, h) = unsafe {
    (
        GetSystemMetrics(SM_CXSCREEN) as f32,
        GetSystemMetrics(SM_CYSCREEN) as f32,
    )
};
```

The `unsafe` block is a promise to the compiler that *you* have checked that the
FFI call is safe to make here (correct arguments, valid pointers, etc.).

---

## FFI and extern "system" callbacks

Win32's `EnumWindows` requires a C-ABI callback:

```rust
unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
    // ...
}
```

`extern "system"` tells Rust to use the Windows calling convention (stdcall on
32-bit, the same as C on 64-bit). `LPARAM` is a pointer-sized integer used here
to carry a raw pointer to our `FindState` struct — a classic C-style pattern
that Rust can express but requires `unsafe` to dereference.

---

## `Option<T>` and `if let`

`Option<T>` represents a value that may or may not be present — Rust's safe
alternative to null pointers.

```rust
// Unwrap only if Some:
if let Some(idx) = self.selected {
    let leaves = self.root.leaves();
    // idx is valid here
}

// Provide a default with map_or:
let is_hovered = pointer_pos.map_or(false, |p| leaf_rect.contains(p));
```

Used throughout for the selected region index, hover position, and click position.

---

## Iterator methods (`position`, `enumerate`)

Rust iterators expose a rich set of functional combinators:

```rust
// Find the index of the first leaf that contains the click point:
let new_sel = leaves.iter().position(|r| r.contains(pos));

// Iterate with index:
for (idx, leaf_rect) in leaves.iter().enumerate() { ... }
```

`position` returns `Option<usize>` — it is `None` if no element matched.

---

## `String` vs `&str`

- `&str` — a borrowed string slice; no ownership, no allocation.
- `String` — an owned, heap-allocated, growable string.

In `snap_status: Option<String>` we store an owned `String` because the message
outlives the function that created it. `format!(...)` always returns a `String`.

---

## `add_enabled_ui`

egui's `add_enabled_ui(enabled, |ui| { ... })` greys out and disables all widgets
inside the closure when `enabled` is `false` — used here to disable the
"Snap VLC" button until a region has been selected:

```rust
ui.add_enabled_ui(snap_enabled, |ui| {
    if ui.button("▶ Snap VLC").clicked() { ... }
});
```
