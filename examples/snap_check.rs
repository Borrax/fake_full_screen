// Throwaway integration check: drives the real vlc module against a running
// VLC and prints the resulting window rect. Run with:
//   cargo run --example snap_check -- <x> <y> <w> <h>
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let x: f32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(300.0);
    let y: f32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(200.0);
    let w: f32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(800.0);
    let h: f32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(450.0);

    let video = PathBuf::from(r"C:\Users\catarract\Desktop\test_video.mp4");
    println!("launch_vlc -> {:?}", fake_full_screen::vlc::launch_vlc(&video));

    std::thread::sleep(std::time::Duration::from_millis(2500));

    let rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h));
    println!("snap_vlc({rect:?}) -> {:?}", fake_full_screen::vlc::snap_vlc(&rect));

    // Wait past the delayed re-assert passes (120/300/600ms) before exit.
    std::thread::sleep(std::time::Duration::from_millis(1200));
    println!("done");
}
