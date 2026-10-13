# 0040. Forward the presenter's clicker to a networked keyboard

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-01-26 (#26, f1f2dd7; PgUp/PgDn 3d8f6fb, 24473b7); through main 2026-10-03 (PR #334) |
| **Area** | UI |
| **Evidence** | `remote_keyboard/source.ino`; `settings-status.js` `remoteKeyUrl` :25-34, `remoteKeyboardStatus`; `src/main/index.js:506-535`; #32 |
| **egui verdict** | Keep |
| **egui status** | Partial |

## Context

Presenters bring a clicker. Its presses arrive at the wall PC, but the
slides are on the presenter's laptop. An ESP32-S3 plugged into the laptop
acts as a USB keyboard and listens on the network, so no software runs on
the presenter's machine.

## Decision

- **Firmware contract:** mDNS hostname (`space_keyboard`), HTTP on port 80.
  `GET /left` and `GET /right` require an `X-API-Key` header and tap
  PageUp / PageDown. `GET /` serves an unauthenticated page.
- **App:** PageUp/Left and PageDown/Right, when enabled, become
  `GET http://<host>[.local]/<left|right>` with the key. A bare hostname gets
  `.local` **before** the scheme is added (fixed in #334). 2 s timeout, no
  retry.
- Requests go through the main process because a `file://` renderer `fetch`
  with `X-API-Key` needs a CORS preflight the device does not answer.
- The status line reports the last press, kept in memory only. The key field
  is masked with Show/Hide. A presenter debug overlay keeps 8 lines (#32).

## Consequences

The API key is plain text in `settings.json` and in the firmware source; the
transport is plain HTTP on the LAN. The Settings copy says "arrow key
presses", but PgUp/PgDn are the primary keys.

## Porting to egui

**Keep** the firmware contract, the `.local` rule, timeouts, the status line
and the shortcut rows. CORS and the main-process hop disappear. **Verify
mDNS:** Chromium resolves `.local` on Windows itself; a Rust HTTP client uses
the OS resolver, which may not.

Done (`remote_key.rs`): the URL rule, a worker thread with ureq (2 s, no
retry, `[RemoteKey]` log lines), the status line, the Settings pane with a
masked key, and the legend rows. The chips read `Left / PgUp`: egui's
default fonts have no arrow glyphs. A loopback test checks the header and
the 200/401 handling. Still to do: confirm `.local` resolves on the wall,
and the presenter debug overlay.
