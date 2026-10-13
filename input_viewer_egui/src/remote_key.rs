// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! The remote keyboard (ADR 0040): PageUp/Left and PageDown/Right on the wall
//! become `GET http://<host>/left|right` with an `X-API-Key`, and the ESP32
//! taps PageUp/PageDown on the presenter's laptop.
//!
//! Electron sent these from the main process because a `file://` renderer
//! fetch with a custom header needs a CORS preflight the device never
//! answers. Here there is no browser and no preflight: one worker thread
//! sends presses in order, with Electron's 2 s timeout and no retry. A
//! press is a slide change, so a late retry would be a wrong one.
//!
//! `.local` names go to the OS resolver. Windows 10 1809+ resolves mDNS
//! itself; Chromium did it on its own, so the wall still has to confirm it.

use std::sync::mpsc;
use std::time::Duration;

use chrono::{DateTime, Local};

use crate::filelog::{log_info, log_warn};

const TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
}

impl Direction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
        }
    }
}

/// The outcome of the last press, for the status line. In memory only.
#[derive(Debug, Clone, PartialEq)]
pub struct Last {
    pub ok: bool,
    pub direction: Direction,
    pub status: Option<u16>,
    pub error: Option<String>,
    pub at: DateTime<Local>,
}

/// The URL a press goes to. A bare name gets `.local`, decided on the host
/// as typed, BEFORE a scheme is added: Electron once prefixed `http://`
/// first, after which "no dot, no colon" never held (#334).
pub fn url(host: &str, direction: Direction) -> Option<String> {
    let h = host.trim();
    if h.is_empty() {
        return None;
    }
    let (scheme, after) = ["http://", "https://"].iter()
        .find_map(|s| h.get(..s.len()).filter(|p| p.eq_ignore_ascii_case(s)).map(|p| (p, &h[s.len()..])))
        .unwrap_or(("http://", h));
    let (name, rest) = after.find('/').map_or((after, ""), |i| after.split_at(i));
    let suffix = if name.contains(['.', ':']) || name.eq_ignore_ascii_case("localhost") { "" } else { ".local" };
    Some(format!("{scheme}{name}{suffix}{}/{}", rest.trim_end_matches('/'), direction.as_str()))
}

struct Press {
    url: String,
    api_key: String,
    direction: Direction,
}

/// Sends presses on a worker thread; results come back through `poll`.
pub struct Sender {
    tx: mpsc::Sender<Press>,
    rx: mpsc::Receiver<Last>,
}

impl Sender {
    pub fn new(repaint: impl Fn() + Send + 'static) -> Self {
        let (tx, jobs) = mpsc::channel::<Press>();
        let (done, rx) = mpsc::channel();
        std::thread::Builder::new().name("remote-key".into()).spawn(move || {
            let agent: ureq::Agent = ureq::Agent::config_builder()
                .timeout_global(Some(TIMEOUT))
                .http_status_as_error(false)
                .build()
                .into();
            for p in jobs {
                let result = agent.get(&p.url).header("X-API-Key", &p.api_key).call();
                let last = match result {
                    Ok(r) => {
                        let status = r.status().as_u16();
                        Last { ok: r.status().is_success(), direction: p.direction, status: Some(status), error: None, at: Local::now() }
                    }
                    Err(e) => Last { ok: false, direction: p.direction, status: None, error: Some(e.to_string()), at: Local::now() },
                };
                match (&last.status, &last.error) {
                    (Some(s), _) if last.ok => log_info!("[RemoteKey] {} -> HTTP {s}", p.url),
                    (Some(s), _) => log_warn!("[RemoteKey] {} -> HTTP {s}", p.url),
                    (None, e) => log_warn!("[RemoteKey] {} failed: {}", p.url, e.as_deref().unwrap_or("?")),
                }
                if done.send(last).is_err() {
                    break;
                }
                repaint();
            }
        }).expect("spawning the remote-key thread");
        Self { tx, rx }
    }

    pub fn send(&self, url: String, api_key: String, direction: Direction) {
        let _ = self.tx.send(Press { url, api_key, direction });
    }

    /// The newest result since the last poll, if any.
    pub fn poll(&self) -> Option<Last> {
        self.rx.try_iter().last()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Direction::{Left, Right};

    #[test]
    fn adds_local_to_a_bare_name() {
        assert_eq!(url("space_keyboard", Left).unwrap(), "http://space_keyboard.local/left");
    }

    #[test]
    fn leaves_a_dot_a_port_or_localhost_alone() {
        assert_eq!(url("space_keyboard.local", Right).unwrap(), "http://space_keyboard.local/right");
        assert_eq!(url("10.0.0.7", Left).unwrap(), "http://10.0.0.7/left");
        assert_eq!(url("kbd:8080", Left).unwrap(), "http://kbd:8080/left");
        assert_eq!(url("localhost", Left).unwrap(), "http://localhost/left");
    }

    #[test]
    fn keeps_a_typed_scheme_and_still_suffixes_after_it() {
        assert_eq!(url("https://kbd.example", Left).unwrap(), "https://kbd.example/left");
        assert_eq!(url("http://kbd", Left).unwrap(), "http://kbd.local/left");
    }

    #[test]
    fn trims_whitespace_and_a_trailing_slash() {
        assert_eq!(url("  kbd.lan/  ", Right).unwrap(), "http://kbd.lan/right");
    }

    /// A stand-in for the ESP32: 200 with the right key, 401 without.
    #[test]
    fn sends_the_key_and_reports_the_status() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for stream in listener.incoming().take(2) {
                let mut stream = stream.unwrap();
                let mut buf = [0u8; 2048];
                let n = stream.read(&mut buf).unwrap();
                let req = String::from_utf8_lossy(&buf[..n]).to_ascii_lowercase();
                let ok = req.starts_with("get /left ") && req.contains("x-api-key: secret\r\n");
                let status = if ok { "200 OK" } else { "401 Unauthorized" };
                let _ = stream.write_all(format!("HTTP/1.1 {status}\r\ncontent-length: 0\r\nconnection: close\r\n\r\n").as_bytes());
            }
        });
        let sender = Sender::new(|| {});
        let host = format!("127.0.0.1:{port}");
        let wait = || (0..200).find_map(|_| {
            std::thread::sleep(Duration::from_millis(10));
            sender.poll()
        }).expect("no result");
        sender.send(url(&host, Left).unwrap(), "secret".into(), Left);
        let good = wait();
        assert_eq!((good.ok, good.status), (true, Some(200)));
        sender.send(url(&host, Left).unwrap(), "wrong".into(), Left);
        let bad = wait();
        assert_eq!((bad.ok, bad.status), (false, Some(401)));
    }

    #[test]
    fn none_without_a_host() {
        assert_eq!(url("", Left), None);
        assert_eq!(url("   ", Left), None);
    }
}
