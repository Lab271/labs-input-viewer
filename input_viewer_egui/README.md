# Input Viewer — egui prototype

A Rust/[egui](https://github.com/emilk/egui) prototype of Input Viewer, next to
the Electron app in `../input_viewer_electron`. One native binary (~16 MB release on macOS), no
Chromium.

```sh
cargo run --release -- --mock                 # 4 synthetic inputs, no hardware
cargo run --release -- --mock=2 --no-signal   # split-flap board
cargo run --release -- --no-signal --screensaver-delay=0
cargo run --release -- --mock=1 --no-signal --screensaver-delay=0 --screensaver=plasma
cargo run --release                           # real capture devices (nokhwa)
cargo run --release -- --test-pattern --hud    # two 4K60 inputs, frame-rate overlay
cargo test
cargo run --release --example camera_probe   # can each camera deliver a frame?
```

The test-mode flags behave as in the Electron app (#248), and `--mock` never
writes settings.

## What is ported

| Feature | Electron source | Here |
|---|---|---|
| Dual / single view, Multi-view, per-half pickers (`pickerPlan`) | `renderer.js` | `app.rs` |
| Pickers over the top edge, closes 48px below or after 30s idle | dropdown 2b | `app.rs` |
| Shortcut table as single source, legend on the bottom edge | `shortcuts.js` | `shortcuts.rs` |
| Mock inputs as real frames through the capture path | `mock-capture.js` | `capture.rs` |
| Stall / flat-frame health with reopen backoff | `stream-health.js` | `health.rs` |
| Split-flap board, screensaver after delay, 10-min rotation, seeded + logged | `split-flap.js`, `registry.js` | `screensaver.rs` |
| Plasma saver + bloom / ACES / dither chain, same GLSL | `plasma.js`, `post-fx.js` | `plasma.rs`, `shaders/` |
| Freeze, fullscreen, Esc does one thing per press | `renderer.js` | `app.rs` |
| Settings dialog: Inputs and Layout panes | `index.html`, `settings-status.js` | `app/settings_ui.rs`, `settings_status.rs` |
| `settings.json`, same keys, Electron's imported once | `settings.json` | `settings.rs` |
| Logo column between the halves, side borders | `#center-divider`, `.edge-border` | `app.rs` |

## Not ported

Reference-screenshot no-signal detection, black-bar cropping, audio/volume,
the remote keyboard and Art-Net (features and their Settings panes), weather,
WebGPU compositing — and Plasma plus 3 painter-drawn screensavers instead
of ~30 WebGL ones. Picker tiles show the last frame seen on screen, not a fresh
snapshot sweep (#242).

## Headless screenshots

`screencapture` needs Screen Recording permission; this reads egui's own
framebuffer instead:

```sh
INPUT_VIEWER_SCREENSHOT=out.png INPUT_VIEWER_OPEN=dropdown cargo run -- --mock
```

## Screensavers on the GPU

eframe runs on its **glow** (OpenGL) backend rather than the default wgpu, so
the Electron savers' GLSL runs unchanged apart from the `#version` line. The
shaders in `shaders/` are generated, not hand-copied:

```sh
node input_viewer_egui/shaders/extract.mjs
```

Porting another shader saver means adding it to `extract.mjs` and a variant in
`screensaver.rs`. The post-FX chain in `plasma.rs` is already generic.

## Cameras on macOS

Camera permission belongs to the app that launched the process. Run from
Terminal or iTerm to get the prompt; launched from another app that cannot
show one, access is silently denied. **OBS Virtual Camera does not work**:
it only offers BGRA, which nokhwa's AVFoundation backend rejects.

## Releases and auto-update

Built and published by [`release-egui.yml`](../.github/workflows/release-egui.yml)
with [Velopack](https://velopack.io), the counterpart of electron-builder +
electron-updater:

| | Electron | egui |
|---|---|---|
| Windows installer | NSIS `Setup.exe`, per user | Velopack `Setup.exe`, per user, no admin |
| macOS | universal DMG | universal `.pkg` (unsigned, ad-hoc signed binary) |
| Updates | full download | delta packages against the previous release |
| Update UX | check 3s after start, Download / Later, Restart now / Later, install on quit | same, as an egui dialog |
| Tags | `v*`, marked latest | `egui-v*`, **never** marked latest |

```sh
gh workflow run release-egui.yml                 # dry run: installers as artifacts
gh workflow run release-egui.yml -f publish=true # tag + GitHub release (main only)
```

Versions come from Conventional Commits touching `input_viewer_egui/` since
the last `egui-v*` tag (or pick `-f bump=minor`). There is no bump commit:
the tag is the source of truth, and `Cargo.toml` stays at `0.0.0` in git.

Sharing the repo with the Electron app is the delicate part:

- **Never "latest".** electron-updater resolves `/releases/latest`; an egui
  release there would leave every Electron install unable to find its
  `latest.yml`. The workflow publishes with `make_latest: false`.
- **Not Velopack's `GithubSource`.** It looks at only the ten most recent
  releases, which a burst of Electron releases would push the egui ones out of.
  `update.rs` finds the newest `egui-v*` release itself and reads its feed.
- **`release.yml` ignores `egui-v*` tags** (`git describe --match 'v[0-9]*'`),
  otherwise an egui release would reset the Electron commit analysis.

Updates only run in an installed copy; `cargo run` logs
`[updater] disabled` and carries on, like Electron's `isDev`.

## Settings

`<config>/Input Viewer (egui)/settings.json`, next to the Electron app's
`<config>/Input Viewer/settings.json` and in the same format. On first launch
the Electron file is imported once:

- **Carries over:** `centerGap`, `borderWidth`, `layoutMode`, `multiView`, and
  every key this app does not use yet (Art-Net, remote keyboard, volumes),
  written back unchanged.
- **Does not:** input names, enabled flags and the default input. Electron
  keys `inputs` by Chromium's `deviceId`, a salted hash nothing outside
  Chromium can reproduce, and stores no label to match on.

Inputs here are keyed by AVFoundation's `uniqueID` (macOS) or the device path
(Windows), so a name stays with its card across restarts and replugging.

Every change is caught in one place, by comparing settings at the end of each
frame, and written 300ms after it settles, atomically. `--mock` and review runs
neither read nor write the file. A file that does not parse is moved aside to
`settings.json.corrupt-<time>` rather than overwritten.

## Render test: `--test-pattern --hud`

`--test-pattern[=WxH@FPS]` (default `3840x2160@60`, also `4k30`, `1080p60`)
gives the mock inputs a motion test pattern, and implies two of them: the
wall's two 4K60 cards. `--hud` overlays, per half, the source and shown frame
rates, frames skipped, and the delay from capture to upload, and logs a
`[render]` line every 5s.

What each part of the pattern shows is listed in `src/testpattern.rs`: lamp
row and frame-number strip for drops, a red/cyan sweep for tearing and
blending, corner marks for cropping, 1px line pairs for scaling, ramps for
banding.

Measured on an M3 Pro, two 3840x2160@60 inputs, window at ~2.2x downscale:

| | shown | skipped/s | capture→upload | UI |
|---|---|---|---|---|
| bilinear | 60.0 fps | 0 | ~7 ms | ~103 fps |
| trilinear mipmaps (shipped) | 60.0 fps | 0 | ~7 ms | ~95 fps |

Bilinear turned the 1px line pairs into moiré; mipmaps render them as the
flat grey they should be. 14-17 frames are skipped in the first second while
textures are first allocated; none after.

Frames reach egui already converted, on the capture thread, as a shared
`Arc<ColorImage>` -- no per-pixel pass on the UI thread.
