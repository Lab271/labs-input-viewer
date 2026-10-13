# 0006. The app and the wall's setup live in separate repos; the startup script owns launch order

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-10-01 (labs-videowall-config a98bb1a, ded2efa) |
| **Area** | Product |
| **Evidence** | labs-videowall-config README, `startup/README.md`, `startup/startup.ps1`, `edid/README.md`; `.claude/CLAUDE.md` |
| **egui verdict** | Keep |
| **egui status** | Partial |

## Context

The wall PC is Windows 11 with two PCIe Elgato 4K60 Pro MK.2 cards, NVIDIA
Mosaic and Scalable Warp. Getting it to a usable state is ordered: the
desktop must be 6000x1200 and stable before warping, and warping before the
app opens the cards.

## Decision

- `LAB271/labs-videowall-config` owns the machine: Mosaic, Warp, EDID,
  startup. Input Viewer is explicitly out of its scope ("deploy from its own
  repo"), and this repo is the application only.
- At logon `startup.ps1` waits for Mosaic (>= 6000x1200, stable for three
  polls), engages Warp, logs both cards' PnP status without blocking, then
  launches `C:\Program Files\Input Viewer\Input Viewer.exe`. It relaunches
  once if the app is gone after 20 s, runs unelevated, and takes a named
  mutex.
- It does **not** restart the cards: `pnputil /restart-device` returns 3010
  because `audiodg.exe` holds the audio endpoint. Recovering a card is the
  app's job ([0013](0013-reopen-by-releasing-the-device-with-two-backoffs.md)).

## Consequences

- The executable's path and name are an interface between the two repos.
- The Elgato utility and Input Viewer cannot hold the cards at the same time
  ("Close Input Viewer (it holds the cards)", `edid/README.md`).
- Since 2026-10-02 the presenter arrives over a 15 m HDMI cable, not
  HDBaseT (labs-videowall-config #5). Docs here still say HDBaseT.

## Porting to egui

**Keep** the split, but the interface **breaks** as things stand:

- `startup.ps1` launches `C:\Program Files\Input Viewer\Input Viewer.exe`.
  The prototype's Velopack installer installs per user under
  `%LocalAppData%\InputViewerEgui\` with `input-viewer.exe`.
- Note also that Electron's NSIS config is `perMachine: false`, whose
  default location is `%LocalAppData%\Programs`, not `C:\Program Files` --
  so how the wall's current install got there is itself undocumented
  ([open questions](open-questions.md)).

Decide the install location and change `startup.ps1` in the same release
that switches the wall, in labs-videowall-config.

## Decided 2026-10-13

D4: Velopack **per user**, `%LocalAppData%\InputViewer`, installed by
`setup.exe` as `sbpla`; updates need no admin. `startup.ps1` in
`LAB271/labs-videowall-config` launches it from there.
