# 0002. Port Input Viewer to Rust and egui

| | |
|---|---|
| **Status** | Proposed |
| **Date** | 2026-10-12 |
| **Area** | Product |
| **Evidence** | #249 (Tauri, "recorded, not recommended"), #225, #62, `input_viewer_egui/` |
| **egui verdict** | Decide |
| **egui status** | Prototype |

## Context

The Electron app works and is in production on the wall. The case for moving
is about what Electron costs on a single-purpose wall PC:

- **Size and update surface.** A 95 MB installer and a 211 MB DMG (#249),
  carrying a Chromium whose security fixes reach the wall only by
  redeploying (labs-videowall-config `docs/software.md`), while Dependabot's
  Electron bumps never trigger a release on their own
  ([0045](0045-release-by-hand-bump-from-commits.md)).
- **Workarounds that exist only because the UI is a web page.** CORS
  proxies through the main process ([0043](0043-keep-privileged-work-in-one-place.md)),
  WebRTC constraint negotiation that once locked a card at 640x480 (#34),
  one capture session shared between two `<video>` elements
  ([0008](0008-one-capture-session-per-device.md)), PowerShell compiling C#
  on every volume read (#283), `object-view-box` for cropping.
- **A device API that Chromium hides.** #340 wants the source resolution
  from the Elgato driver instead of guessing it from black bars; that is a
  KS property query, out of reach from a renderer.

#249 looked at Tauri and recommended against it. Its argument was that the
cost is not the IPC but the ~30 WebGL2 shaders tuned against ANGLE, plus
rebuilding auto-update (#86) and signing (#56). Those are the questions a
port has to answer.

## Decision (proposed)

Port to Rust with eframe/egui on the glow (OpenGL) backend, keeping the
product behaviour these ADRs record and dropping the browser workarounds.

The prototype in `input_viewer_egui/` was built to test #249's objections:

| #249's concern | Prototype result |
|---|---|
| Shaders tuned against ANGLE | Plasma and its post-FX chain run as the **same GLSL**, extracted from the JS at build time (`shaders/extract.mjs`), on desktop GL 3.3. Six of 29 savers are pure fragment shaders and port this way; the rest need their JS orchestration rewritten ([0026](0026-share-glsl-and-work-in-world-space.md)). |
| Compositing cost | Two synthetic 3840x2160@60 inputs: 60.0 fps shown, 0 skipped/s, ~7 ms capture-to-upload, UI ~95 fps (M3 Pro, `--test-pattern --hud`). #62 had already shown compositing is not the bottleneck in Electron either. |
| Auto-update | Velopack with a custom update source, alongside the Electron releases ([0048](0048-package-and-update-from-the-public-repo.md)). Built, linted, **not yet run in CI**. |
| Capture | `nokhwa` (AVFoundation / Media Foundation). Compiles; frames flow from mock inputs. **Not tested against an Elgato 4K60 Pro MK.2**, and nokhwa rejects BGRA-only devices (OBS Virtual Camera). |

The release binary is ~16 MB.

## Consequences

- Everything in "egui status: Not started" in the [index](README.md) is the
  remaining work; the biggest pieces are audio, black-bar cropping,
  reference detection, 23 screensavers, Art-Net and the remote keyboard.
- Two codebases during the transition. The GLSL stays single-sourced in the
  Electron tree until the port owns it.
- **Go/no-go gate:** run `--test-pattern --hud` and real capture on the wall's
  Windows host with both Elgato cards. Media Foundation behaviour through
  nokhwa is the largest unknown, and every figure so far is from a Mac.

## Porting to egui

This is the decision the rest depend on. It stays Proposed until the wall
test above has been run.
