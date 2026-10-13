// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! One copy at a time (D5, ADR 0005).
//!
//! Two copies on the wall would both open the capture cards, and a card
//! serves one session: the second copy shows black and the first may lose its
//! picture. That happens when someone double-clicks the shortcut while the
//! startup script has already launched the app.
//!
//! The lock is a listening socket on a fixed loopback port, which the OS
//! releases however the process dies -- a lock file would survive a crash and
//! block the next start. A second copy finds the port taken, asks the first to
//! come to the front, and exits. If the port belongs to some other program,
//! the connection is not answered with the handshake and the app starts
//! anyway: never refusing to start is worth more than the guarantee.
//!
//! Test runs (`--mock`, `--test-pattern`, review screenshots) skip the lock,
//! so they can run beside the real app.

use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::sync::Mutex;
use std::time::Duration;

use crate::filelog::{log_info, log_warn};

const PORT: u16 = 47_291;
const HELLO: &str = "input-viewer show";
const ACK: &str = "input-viewer ok";

pub enum Outcome {
    /// This is the only copy; `serve` once a window exists.
    First,
    /// Another copy was asked to come forward; exit now.
    Second,
}

static LISTENER: Mutex<Option<TcpListener>> = Mutex::new(None);

pub fn acquire() -> Outcome {
    match TcpListener::bind((Ipv4Addr::LOCALHOST, PORT)) {
        Ok(listener) => {
            *LISTENER.lock().unwrap() = Some(listener);
            Outcome::First
        }
        Err(e) => match hand_over(PORT) {
            Ok(()) => {
                log_info!("[App] already running; asked it to come to the front, exiting");
                Outcome::Second
            }
            Err(why) => {
                log_warn!("[App] port {PORT} is taken ({e}) but not by Input Viewer ({why}); starting without the single-instance lock");
                Outcome::First
            }
        },
    }
}

fn hand_over(port: u16) -> std::io::Result<()> {
    let mut stream = TcpStream::connect_timeout(&(Ipv4Addr::LOCALHOST, port).into(), Duration::from_secs(1))?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    writeln!(stream, "{HELLO}")?;
    let mut reply = String::new();
    BufReader::new(stream).read_line(&mut reply)?;
    if reply.trim() == ACK {
        Ok(())
    } else {
        Err(std::io::Error::other("unexpected reply"))
    }
}

/// Answer later copies by bringing this window forward.
pub fn serve(ctx: eframe::egui::Context) {
    let Some(listener) = LISTENER.lock().unwrap().take() else { return };
    let _ = std::thread::Builder::new().name("single-instance".into()).spawn(move || {
        answer(&listener, || {
            log_info!("[App] a second copy was started; bringing this one forward");
            use eframe::egui::ViewportCommand;
            ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(ViewportCommand::Focus);
            ctx.request_repaint();
        });
    });
}

/// The listening side of the handshake; runs until the listener fails.
fn answer(listener: &TcpListener, on_show: impl Fn()) {
    for stream in listener.incoming().flatten() {
        let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
        let mut line = String::new();
        if BufReader::new(&stream).read_line(&mut line).is_err() || line.trim() != HELLO {
            continue;
        }
        let _ = writeln!(&stream, "{ACK}");
        on_show();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn listener() -> (TcpListener, u16) {
        let l = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = l.local_addr().unwrap().port();
        (l, port)
    }

    #[test]
    fn a_second_copy_hands_over_to_the_first() {
        let (l, port) = listener();
        let shown = Arc::new(AtomicUsize::new(0));
        let s = shown.clone();
        std::thread::spawn(move || answer(&l, || { s.fetch_add(1, Ordering::SeqCst); }));
        hand_over(port).unwrap();
        hand_over(port).unwrap();
        assert_eq!(shown.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn a_stranger_on_the_port_is_not_mistaken_for_us() {
        let (l, port) = listener();
        std::thread::spawn(move || {
            if let Ok((mut s, _)) = l.accept() {
                let _ = writeln!(s, "HTTP/1.1 400 Bad Request");
            }
        });
        assert!(hand_over(port).is_err());
    }
}
