# 0052. Run everything that must not stop in `App::logic`, not in `ui`

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-10-12 |
| **Area** | Platform |
| **Evidence** | `input_viewer_egui/src/app.rs` `impl eframe::App`; eframe `App::logic` docs |
| **egui verdict** | Keep (egui-only decision) |
| **egui status** | Done |

## Context

eframe calls `App::ui` only while the window is visible. While it is hidden
or fully covered it runs no egui pass at all and calls `App::logic` instead
(whenever a repaint is requested).

The prototype first did its health sampling, reopening, device polling and
screensaver timing inside `ui`. Testing `--mock-fault` with the window behind
other windows, the app logged one health line at 2 s and then nothing: the
main thread was idle in the macOS event loop, `ui` was never called, and a
stalled input was never reopened. On the wall, any window over the app -- an
update prompt, the Elgato utility, a notification -- would have paused card
recovery the same way.

## Decision

- `logic`: device polling, stream sync (open, release, reopen), health
  sampling, screensaver timing, settings saves. It requests its own wake-up
  every 500 ms, because with every input stalled or failed no frames arrive
  to request one, and the reopen timers would never fire.
- `ui`: input handling, texture uploads, painting, dialogs.

## Consequences

Anything added later that must keep running unattended -- the log file's
day roll-over, the remote keyboard, Art-Net, the weather poll -- belongs in
`logic` or on its own thread, never in `ui`.

## Porting to egui

N/A: this ADR exists only because of egui.
