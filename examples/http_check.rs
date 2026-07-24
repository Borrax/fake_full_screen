// Headless verification of the VLC HTTP control backend: launches VLC, waits for
// the interface, then toggles pause and seeks, asserting the reported state
// actually changes. Proves the control plane the hover overlay will drive.
//
//   cargo run --example http_check
use fake_full_screen::{vlc, vlc_http};
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;

fn main() {
    let video = PathBuf::from(r"C:\Users\catarract\Desktop\test_video.mp4");
    println!("launch_vlc -> {:?}", vlc::launch_vlc(&video));

    // Wait for the HTTP interface to come up and playback to start.
    let mut started = false;
    for _ in 0..24 {
        sleep(Duration::from_millis(500));
        match vlc_http::status() {
            Ok(s) if s.state == "playing" => {
                println!(
                    "playing: time={}s length={}s volume={}",
                    s.time, s.length, s.volume
                );
                started = true;
                break;
            }
            Ok(s) => println!("state={} (waiting for playing)", s.state),
            Err(_) => println!("waiting for HTTP interface..."),
        }
    }
    if !started {
        println!("FAIL: never reached playing state");
        return;
    }

    let mut pass = true;

    println!("\n-> play_pause (expect paused)");
    let _ = vlc_http::play_pause();
    sleep(Duration::from_millis(800));
    match vlc_http::status() {
        Ok(s) => {
            println!("   state={}", s.state);
            pass &= s.state == "paused";
        }
        Err(e) => {
            println!("   status err: {e}");
            pass = false;
        }
    }

    println!("-> play_pause (expect playing)");
    let _ = vlc_http::play_pause();
    sleep(Duration::from_millis(800));
    match vlc_http::status() {
        Ok(s) => {
            println!("   state={}", s.state);
            pass &= s.state == "playing";
        }
        Err(e) => {
            println!("   status err: {e}");
            pass = false;
        }
    }

    let before = vlc_http::status().map(|s| s.time).unwrap_or(0);
    println!("-> seek +30 (time before={before}s)");
    let _ = vlc_http::seek(30);
    sleep(Duration::from_millis(800));
    let after = vlc_http::status().map(|s| s.time).unwrap_or(before);
    println!("   time after={after}s");
    pass &= after >= before + 20;

    let vbefore = vlc_http::status().map(|s| s.volume).unwrap_or(0);
    println!("-> volume_delta -40 (vol before={vbefore})");
    let _ = vlc_http::volume_delta(-40);
    sleep(Duration::from_millis(600));
    let vafter = vlc_http::status().map(|s| s.volume).unwrap_or(vbefore);
    println!("   vol after={vafter}");
    pass &= vafter < vbefore;

    println!(
        "\nRESULT: {}",
        if pass {
            "VLC HTTP control backend works \u{2705}"
        } else {
            "control backend FAILED \u{274c}"
        }
    );
}
