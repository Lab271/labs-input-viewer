# 0018. Composite video on the GPU, with good downscaling

| | |
|---|---|
| **Status** | Accepted (Electron: CSS `<video>`; WebGPU path evaluated and not adopted) |
| **Date** | 2026-01-16 (#6 GPU switches); 2026-04-16 (#34); 2026-08-19 (#62 closed) |
| **Area** | Signal |
| **Evidence** | `src/main/index.js:10-12`; `styles.css:147-154`; `gpu-compositor.js`; `renderer.js:4378-4473`; `scripts/bench-compositing.mjs`; #62, PRs #129, #137, #200 |
| **egui verdict** | Drop (decided by the platform) |
| **egui status** | Done |

## Context

Electron composites with CSS: two `<video>` elements with `object-fit:
contain` and `image-rendering: high-quality`, plus Chromium switches
`ignore-gpu-blocklist` and `enable-gpu-rasterization`. Two other switches
added for quality, `enable-accelerated-mjpeg-decode` and
`enable-native-gpu-memory-buffers`, were removed because they "could
interfere with UVC device format negotiation" (#34).

#62 benchmarked three paths at 6000x1200 on an M3 Pro:

| Path | Quiet | With a 120 ms/s main-thread stall |
|---|---|---|
| CSS | 20 fps (capture-bound) | 18.9 |
| WebGPU in-page | 120 composites/s | 107.2 |
| OffscreenCanvas worker | 120 | 120.1 |

"Compositing is not the bottleneck." WebGPU stayed opt-in
(`gpuCompositing`), off on the wall, without the crop, and silently drops a
third feed.

## Decision

CSS compositing, GPU-accelerated through Chromium switches; WebGPU as an
experiment only.

## Consequences

On the wall, `gpu-report.txt` confirms ANGLE on the NVIDIA RTX PRO 4000 with
GPU compositing enabled (#271).

## Porting to egui

**Drop** -- a native egui app **is** a GPU compositor; the switches, the CSS
and the WebGPU experiment have nothing to carry over.

**One finding from the port to keep as a rule:** plain bilinear filtering
skips source pixels beyond 2x reduction, and a 4K card on a ~2133x1200 half
is 1.8x down. With `--test-pattern` the 1px line pairs turned to moiré.
The prototype uses trilinear mipmaps for video textures (`VIDEO_TEXTURE` in
`app.rs`): the lines render as flat grey, and two 4K60 inputs still run at
60 fps with no skipped frames. Chromium's `high-quality` hid this in
Electron.

Also keep #34's lesson: render settings must never affect capture format
negotiation.

**YUY2 is converted on the GPU** (`gpu_video.rs`, measured on the wall
2026-10-12). Converting each 4K YUY2 frame to RGBA on one CPU core took
~49 ms on the wall's Threadripper and capped capture at ~12 fps. Now the
card's bytes are uploaded as a half-width texture and a shader writes the
RGBA picture (BT.709 limited range; `yuv.rs` is the CPU twin, with tests):
half the bytes uploaded, nothing per pixel on the CPU. On the wall: capture
~40 fps, UI 60 fps at full wall size (was ~30).
