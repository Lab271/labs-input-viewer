// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! The thread that talks to the relay, and its HTTP transport.
//!
//! Electron sent every request through the main process (`artnet-send`),
//! because a `file://` renderer's JSON POST needs a CORS preflight the relay
//! never answers. Here there is no browser: one worker thread owns the
//! client, and the UI thread only sends it messages and reads back a status.
//! Nothing on the UI or render path ever waits on the LAN.
//!
//! The transport is the main process's handler: http(s) only, 2 s timeout,
//! no retry (the client decides when to try again), a non-2xx status is a
//! failure, and a GET whose body does not parse is a failure -- restoring
//! the room from an empty snapshot would leave it lit.

use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::Value;

use super::Config;
use super::client::{Catalogue, Client, Status, Transport};
use crate::filelog::log_info;
use crate::settings_status::is_relay_url;

const TIMEOUT: Duration = Duration::from_secs(2);

pub struct Http {
    agent: ureq::Agent,
}

impl Http {
    pub fn new() -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(TIMEOUT))
            .http_status_as_error(false)
            .build()
            .into();
        Self { agent }
    }
}

/// A bad settings value fails here rather than reaching the network:
/// file:// or a data: URL is never a lighting relay.
fn check(url: &str) -> Result<(), String> {
    if is_relay_url(url) { Ok(()) } else { Err(format!("refusing {url:?}: not an http(s) URL")) }
}

impl Transport for Http {
    fn post(&mut self, url: &str, body: &Value) -> Result<(), String> {
        check(url)?;
        let res = self.agent.post(url).send_json(body).map_err(|e| e.to_string())?;
        if res.status().is_success() { Ok(()) } else { Err(format!("HTTP {}", res.status().as_u16())) }
    }

    fn get(&mut self, url: &str) -> Result<Value, String> {
        check(url)?;
        let mut res = self.agent.get(url).call().map_err(|e| e.to_string())?;
        if !res.status().is_success() {
            return Err(format!("HTTP {}", res.status().as_u16()));
        }
        res.body_mut().read_json::<Value>().map_err(|e| format!("unparseable response: {e}"))
    }
}

pub enum Msg {
    Config(Config),
    /// A sample grid, and when it was read: the 1 Hz cap is measured against
    /// read times, so a reply that waited behind a slow request cannot make
    /// the next frame look early.
    Frame(Vec<u8>, Instant),
    Activate(String),
    Rotate(String),
    Release,
    ReadCatalogue,
}

/// What the Settings pane shows.
#[derive(Debug, Clone, Default)]
pub struct Shared {
    pub status: Status,
    pub catalogue: Option<Catalogue>,
}

pub struct Worker {
    tx: mpsc::Sender<Msg>,
    shared: Arc<Mutex<Shared>>,
}

impl Worker {
    pub fn start(cfg: Config, repaint: impl Fn() + Send + 'static) -> Self {
        Self::with_transport(Http::new(), cfg, repaint)
    }

    pub fn with_transport<T: Transport + Send + 'static>(transport: T, cfg: Config, repaint: impl Fn() + Send + 'static) -> Self {
        let (tx, rx) = mpsc::channel::<Msg>();
        let shared = Arc::new(Mutex::new(Shared::default()));
        let out = shared.clone();
        std::thread::Builder::new().name("artnet".into()).spawn(move || {
            let epoch = Instant::now();
            let mut client = Client::new(transport, cfg);
            log_info!("[Art-Net] worker started");
            while let Ok(first) = rx.recv() {
                // Everything queued so far, in order. Only the newest frame is
                // worth sending; an older one queued behind a slow request is
                // already stale.
                let mut batch = vec![first];
                batch.extend(rx.try_iter());
                let newest_frame = batch.iter().rposition(|m| matches!(m, Msg::Frame(..)));
                let mut catalogue = None;
                for (i, msg) in batch.into_iter().enumerate() {
                    match msg {
                        Msg::Config(c) => client.set_config(c),
                        Msg::Frame(rgba, at) if Some(i) == newest_frame => {
                            client.offer_frame(&rgba, at.saturating_duration_since(epoch).as_millis() as u64);
                        }
                        Msg::Frame(..) => {}
                        Msg::Activate(name) => client.activate(&name),
                        Msg::Rotate(name) => client.rotate(&name),
                        Msg::Release => client.release(),
                        Msg::ReadCatalogue => catalogue = client.read_catalogue(),
                    }
                }
                let mut s = out.lock().unwrap();
                s.status = client.status();
                if catalogue.is_some() {
                    s.catalogue = catalogue;
                }
                drop(s);
                repaint();
            }
        }).expect("spawning the artnet thread");
        Self { tx, shared }
    }

    pub fn send(&self, msg: Msg) {
        let _ = self.tx.send(msg);
    }

    /// A sender for the GL observer, which runs inside a paint callback.
    pub fn frames(&self) -> mpsc::Sender<Msg> {
        self.tx.clone()
    }

    pub fn shared(&self) -> Shared {
        self.shared.lock().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    /// One request as the relay saw it.
    #[derive(Debug)]
    struct Seen {
        method: String,
        path: String,
        content_type: Option<String>,
        body: Value,
    }

    /// A stand-in for artnet-relay on a local port: answers GET /status with
    /// `status_body`, every POST with 200, and reports what it received.
    fn fake_relay(status_body: &'static str, expect: usize) -> (String, mpsc::Receiver<Seen>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            for stream in listener.incoming().take(expect) {
                let mut stream = stream.unwrap();
                let mut buf = Vec::new();
                let mut chunk = [0u8; 4096];
                // Read the head, then exactly content-length bytes of body.
                let (head, mut body) = loop {
                    let n = stream.read(&mut chunk).unwrap();
                    buf.extend_from_slice(&chunk[..n]);
                    if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                        break (String::from_utf8_lossy(&buf[..i]).into_owned(), buf[i + 4..].to_vec());
                    }
                };
                let header = |name: &str| head.lines().find_map(|l| {
                    let (k, v) = l.split_once(':')?;
                    k.eq_ignore_ascii_case(name).then(|| v.trim().to_owned())
                });
                let len: usize = header("content-length").and_then(|v| v.parse().ok()).unwrap_or(0);
                while body.len() < len {
                    let n = stream.read(&mut chunk).unwrap();
                    body.extend_from_slice(&chunk[..n]);
                }
                let mut parts = head.split_whitespace();
                let (method, path) = (parts.next().unwrap().to_owned(), parts.next().unwrap().to_owned());
                let reply = if method == "GET" { status_body } else { r#"{"ok":true}"# };
                let _ = stream.write_all(format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{reply}",
                    reply.len()).as_bytes());
                let body = serde_json::from_slice(&body).unwrap_or(Value::Null);
                let _ = tx.send(Seen { method, path, content_type: header("content-type"), body });
            }
        });
        (base, rx)
    }

    fn next(rx: &mpsc::Receiver<Seen>) -> Seen {
        rx.recv_timeout(Duration::from_secs(5)).expect("the relay saw no request")
    }

    /// End to end through the real HTTP transport and the worker thread: the
    /// snapshot GET, a ColorRequest POST to /all with a JSON body, and the
    /// restore on release.
    #[test]
    fn talks_to_the_relay_in_the_shape_it_expects() {
        let status = r#"{"strips":[{"name":"u0_01","rgb":[40,10,80],"brightness":0.6}],"scenes":["warm_wit"]}"#;
        let (base, seen) = fake_relay(status, 3);
        let cfg = Config { enabled: true, url: base, ..Config::default() };
        let worker = Worker::with_transport(Http::new(), cfg, || {});

        worker.send(Msg::Activate("Frost".into()));
        let get = next(&seen);
        assert_eq!((get.method.as_str(), get.path.as_str()), ("GET", "/status"));

        worker.send(Msg::Frame(crate::artnet::colour::tests::solid(0, 200, 255, 2048), Instant::now()));
        let post = next(&seen);
        assert_eq!((post.method.as_str(), post.path.as_str()), ("POST", "/all"));
        // ureq adds "; charset=utf-8"; FastAPI reads the body as JSON either way.
        assert!(post.content_type.as_deref().is_some_and(|c| c.starts_with("application/json")), "{:?}", post.content_type);
        let mut keys: Vec<_> = post.body.as_object().unwrap().keys().cloned().collect();
        keys.sort();
        assert_eq!(keys, ["b", "brightness", "g", "r", "transition_ms"]);
        assert!(post.body["b"].as_u64().unwrap() > 200);
        // maxBrightness defaults to main's 0.8.
        assert!(post.body["brightness"].as_f64().unwrap() <= 0.8);

        worker.send(Msg::Release);
        let restore = next(&seen);
        assert_eq!((restore.method.as_str(), restore.path.as_str()), ("POST", "/all"));
        assert_eq!(restore.body["r"], 40);
        assert_eq!(restore.body["brightness"], 0.6);

        // The status reaches the UI side.
        let deadline = Instant::now() + Duration::from_secs(5);
        while worker.shared().status.sent == 0 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        let s = worker.shared().status;
        assert_eq!((s.sent, s.failures), (1, 0));
    }

    #[test]
    fn a_status_body_that_does_not_parse_is_a_failed_read() {
        let (base, _seen) = fake_relay("<html>not json</html>", 1);
        let err = Http::new().get(&format!("{base}/status")).unwrap_err();
        assert!(err.starts_with("unparseable response"), "{err}");
    }

    #[test]
    fn refuses_anything_but_http() {
        let mut http = Http::new();
        assert!(http.post("file:///etc/passwd", &Value::Null).is_err());
        assert!(http.get("relay.lan/status").is_err());
    }

    #[test]
    fn an_unreachable_relay_fails_within_the_timeout() {
        // A bound-but-silent port: accepts and never answers.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://127.0.0.1:{}/all", listener.local_addr().unwrap().port());
        let started = Instant::now();
        assert!(Http::new().post(&url, &Value::Null).is_err());
        assert!(started.elapsed() < TIMEOUT + Duration::from_secs(1));
        drop(listener);
    }
}
