//! Minimal, dependency-free client for VLC's HTTP control interface.
//!
//! VLC is launched with `--extraintf http` (see `launch_args`), exposing a
//! localhost-only control API at `/requests/status.json`. We drive playback from
//! our own hover overlay through this, because VLC's native hover controller is
//! only available in true fullscreen and disappears once we snap VLC into a
//! windowed sub-rect.
//!
//! Auth is HTTP Basic with an empty username and the password below. The
//! interface binds to 127.0.0.1 only, so a fixed local password is acceptable.

use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::time::Duration;

const HTTP_HOST: &str = "127.0.0.1";
const HTTP_PORT: u16 = 9010;
const HTTP_PASSWORD: &str = "ffs_ctl";
const TIMEOUT: Duration = Duration::from_millis(1500);

/// Extra VLC CLI args that enable the HTTP control interface.
pub fn launch_args() -> Vec<String> {
    vec![
        "--extraintf".into(),
        "http".into(),
        "--http-host".into(),
        HTTP_HOST.into(),
        "--http-port".into(),
        HTTP_PORT.to_string(),
        "--http-password".into(),
        HTTP_PASSWORD.into(),
    ]
}

#[derive(Debug, Clone, Default)]
pub struct Status {
    pub state: String, // "playing" | "paused" | "stopped"
    pub position: f32, // 0.0 ..= 1.0
    pub time: i64,     // seconds elapsed
    pub length: i64,   // seconds total
    pub volume: i64,   // raw VLC volume (256 == 100%)
}

fn base64(input: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        out.push(T[(b0 >> 2) as usize] as char);
        out.push(T[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        out.push(if chunk.len() > 1 {
            T[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            T[(b2 & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    out
}

fn http_get(path: &str) -> io::Result<String> {
    let mut stream = TcpStream::connect((HTTP_HOST, HTTP_PORT))?;
    stream.set_read_timeout(Some(TIMEOUT))?;
    stream.set_write_timeout(Some(TIMEOUT))?;

    let auth = base64(format!(":{HTTP_PASSWORD}").as_bytes());
    let req = format!(
        "GET {path} HTTP/1.0\r\nHost: {HTTP_HOST}\r\nAuthorization: Basic {auth}\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(req.as_bytes())?;

    let mut buf = String::new();
    stream.read_to_string(&mut buf)?;
    // Split off the response headers; return just the body.
    Ok(buf
        .split_once("\r\n\r\n")
        .map(|(_, body)| body.to_string())
        .unwrap_or(buf))
}

fn json_str(body: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\":\"");
    let start = body.find(&pat)? + pat.len();
    let rest = &body[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn json_num(body: &str, key: &str) -> Option<f64> {
    let pat = format!("\"{key}\":");
    let start = body.find(&pat)? + pat.len();
    let rest = &body[start..];
    let end = rest
        .find(|c: char| !matches!(c, '0'..='9' | '.' | '-' | '+' | 'e' | 'E'))
        .unwrap_or(rest.len());
    rest[..end].trim().parse::<f64>().ok()
}

/// Read current playback status. Errors while VLC's interface is still starting.
pub fn status() -> io::Result<Status> {
    let body = http_get("/requests/status.json")?;
    Ok(Status {
        state: json_str(&body, "state").unwrap_or_default(),
        position: json_num(&body, "position").unwrap_or(0.0) as f32,
        time: json_num(&body, "time").unwrap_or(0.0) as i64,
        length: json_num(&body, "length").unwrap_or(0.0) as i64,
        volume: json_num(&body, "volume").unwrap_or(0.0) as i64,
    })
}

fn command(cmd: &str) -> io::Result<()> {
    http_get(&format!("/requests/status.json?command={cmd}")).map(|_| ())
}

fn command_val(cmd: &str, val: &str) -> io::Result<()> {
    http_get(&format!("/requests/status.json?command={cmd}&val={val}")).map(|_| ())
}

/// Toggle play/pause.
pub fn play_pause() -> io::Result<()> {
    command("pl_pause")
}

/// Seek by a relative number of seconds (positive = forward). The literal '+'
/// must be percent-encoded so VLC reads it as a relative seek.
pub fn seek(delta_secs: i64) -> io::Result<()> {
    let val = if delta_secs >= 0 {
        format!("%2B{delta_secs}")
    } else {
        format!("-{}", delta_secs.abs())
    };
    command_val("seek", &val)
}

/// Adjust volume by a relative raw amount (VLC scale: 256 == 100%).
pub fn volume_delta(delta: i64) -> io::Result<()> {
    let val = if delta >= 0 {
        format!("%2B{delta}")
    } else {
        format!("-{}", delta.abs())
    };
    command_val("volume", &val)
}

/// Toggle VLC fullscreen.
pub fn toggle_fullscreen() -> io::Result<()> {
    command("fullscreen")
}
