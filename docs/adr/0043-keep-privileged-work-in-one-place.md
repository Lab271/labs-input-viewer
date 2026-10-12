# 0043. Keep privileged work in one small, auditable place

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-08-05 (hardening, PR #129, #55); 2026-10-03 (origin parsing, PR #333) |
| **Area** | Platform |
| **Evidence** | `src/main/index.js:228-290`; `src/preload/index.js:6-55`; `index.html:6` (CSP); `SECURITY.md`; `test/settings-parsing.test.js:38-50` |
| **egui verdict** | Drop the mechanism; keep the principle |
| **egui status** | N/A |

## Context

#55 found that "no remote content" was "passing by accident": nothing
enforced it. A prefix check on `http://localhost:5174` also let through
`:51740` (#333). Main is CommonJS and requires `electron` at load, so it is
effectively untestable; logic was moved into pure renderer modules.

## Decision

- The renderer is an untrusted, sandboxed page: `sandbox`,
  `contextIsolation`, no `nodeIntegration`, security flags set explicitly so a
  future Electron default cannot weaken them.
- No navigation (parsed-origin check); `window.open` denied, http(s) handed
  to the system browser; only `media`, `audioCapture` and `videoCapture`
  permissions granted.
- A small named IPC surface (`window.electronAPI`): window, settings,
  diagnostics, reports, volume, and the two outbound-HTTP proxies. Handlers
  never throw across the boundary; they return `{ok, error}`. Main stays a
  "dumb pipe"; policy lives in testable modules.

## Consequences

Two dead channels remain (`get-settings-path`, `diag-log-reset`). The CSP
still allows `connect-src http://* https://*` and `unsafe-eval`, so the
sandbox, not the CSP, is the boundary.

## Porting to egui

**Drop** the mechanism: a native app has no web content to sandbox, no IPC
and no CORS. **Keep the principle:** policy in pure, tested modules; the few
operations that write files or change state on the LAN (settings, logs,
Art-Net, remote keyboard) in one thin platform layer; external links opened
in the system browser. The prototype follows this split (`settings.rs`,
`health.rs`, `render_stats.rs` and `settings_status.rs` are pure and tested).
