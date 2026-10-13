# 0005. Single instance, display sleep and crash relaunch are not the app's job (by omission)

| | |
|---|---|
| **Status** | Open |
| **Date** | n/a -- never decided, observed as absent |
| **Area** | Product |
| **Evidence** | No `requestSingleInstanceLock`, `powerSaveBlocker` or `wakeLock` in `src/`; `index.js:300-306, 346-366`; `startup.ps1` relaunch-once |
| **egui verdict** | Change: single instance and no sleep in the app |
| **egui status** | Done (lock, awake) |

## Context

None of these was ever discussed in an issue or PR. What exists:

- A second launch opens a second window that competes for the same capture
  devices.
- Main only logs `render-process-gone`, `unresponsive` and
  `child-process-gone`; nothing restarts the app.
- The wall's startup script relaunches the app **once** if it is gone 20 s
  after launch, and takes a named mutex so the script itself does not run
  twice. It does not watch the app after that.
- Display sleep is presumably the wall OS's configuration.

## Decision

None recorded. In effect: rely on the startup script and the OS.

## Consequences

Two instances fighting over the Elgato cards is a possible failure. A crash
after the first 20 s leaves a dark wall until someone notices.

## Porting to egui

**Decide**, for each:

- **Single instance** -- cheap in Rust (a named mutex on Windows, a lock
  file elsewhere). Recommended: a second launch logs and exits, so the cards
  are never contended.
- **Display sleep** -- leave to the wall's OS config unless it is shown to
  be a problem.
- **Crash relaunch** -- belongs in the startup script or a Windows service
  wrapper, not in the app. If it moves there, record it in
  labs-videowall-config.

## Decided 2026-10-13

D5: the app holds a **single-instance lock** (a second copy hands over to the
first and exits) and **keeps the display awake**. Crash relaunch stays in
`startup.ps1`.

Built: `single_instance.rs` holds a loopback port as the lock (released by
the OS however the process dies); a second copy asks the first to come
forward and exits; a stranger on the port does not block the start. Test
runs skip it. `keepawake` holds display and idle-sleep assertions for the
app's lifetime (checked on macOS: `PreventUserIdleDisplaySleep` "Showing the
video wall"; Windows uses `SetThreadExecutionState`, to confirm on the wall).
