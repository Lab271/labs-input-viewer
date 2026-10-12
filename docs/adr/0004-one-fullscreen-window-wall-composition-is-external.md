# 0004. One borderless fullscreen window; the wall's composition is someone else's job

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2025-12-30 (3b5bd3d) |
| **Area** | Product |
| **Evidence** | `src/main/index.js:217-243, 537-548`; labs-videowall-config `startup/startup.ps1` |
| **egui verdict** | Keep |
| **egui status** | Partial |

## Context

The wall is several projectors made to look like one 6000x1200 display by
NVIDIA Mosaic and Scalable Warp, which the wall's own startup script
engages before launching the app ([0006](0006-app-and-wall-setup-live-in-separate-repos.md)).

## Decision

- One window, black background, borderless on Windows and Linux
  (`hiddenInset` title bar on macOS), fullscreen on whichever display the OS
  gives it.
- No display selection, no multi-monitor logic, no kiosk mode, no
  always-on-top. Rationale for avoiding kiosk mode is not recorded.
- The operator can leave fullscreen (Esc, F, F11) and quit (Q).

## Consequences

- Entirely dependent on Mosaic. If Mosaic is not up at launch, the app is
  fullscreen on whatever single display exists -- which is why the startup
  script waits for a stable 6000x1200 desktop first.
- Nothing pins the window above other windows.

## Porting to egui

**Keep.** `ViewportCommand::Fullscreen` already backs F/F11/Esc and
`ViewportCommand::Close` backs Q. The prototype still opens as a 1500x500
window rather than fullscreen and borderless; set
`ViewportBuilder::with_fullscreen(true).with_decorations(false)` for
release builds, keep the windowed default for development.
