# 0007. Capture at the card's native resolution, and log what was negotiated

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-01-16 (#5, #7, #8, #9); 2026-04-16 (#34); 2026-08-09 (#152, #166) |
| **Area** | Capture |
| **Evidence** | `renderer.js:855-860` (constraints), `:875-883` (video-only fallback), `:899-945` (low-res retry); 427ae10, afd9a22, a158a56, 5cb79c0, 75c56d7 |
| **egui verdict** | Adapt |
| **egui status** | Partial |

## Context

The constraint went through five versions in a day and more later: ideal
1920x1080@30, ideal 4096x2160@60, `min` constraints (the stream "failed
entirely"), `exact` 1920x1200 (too rigid), then ideal 4096x2160 with a retry.

Two separate problems hid behind "the card opens at 640x480":

- A permission-probe MediaStream that was never stopped locked an Elgato Cam
  Link at 640x480 (#34). Some GPU flags added to fix "poor quality" were
  partly that misdiagnosis and were removed again
  ([0018](0018-composite-on-the-gpu.md)).
- Some USB cards really do negotiate low by default.

The wall's PCIe 4K60 Pro MK.2 always negotiates 3840x2160 (measured
2026-10-01). Why the request says 4096 rather than 3840 is not recorded.

## Decision

- Request `{deviceId: exact, width: ideal 4096, height: ideal 2160,
  frameRate: ideal 60}`.
- If the result is <= 640 wide: stop the tracks, wait 300 ms, retry at
  min(capabilities, 1920x1080) -- capped there "to prefer uncompressed
  formats over MJPEG" on USB cards. Missing `getCapabilities()` means
  "unknown, try anyway" (#152). If that fails, open with no constraints.
- Log the settings and capabilities that were agreed.

## Consequences

The retry path only matters for USB and virtual cameras. It carries a bug:
it requests audio with the **video** device id (`renderer.js:940-942`), the
mistake #151 fixed on the main path ([open questions](open-questions.md)).

## Porting to egui

**Adapt.** Constraint negotiation is a WebRTC concept; natively the formats
are enumerable. Keep the product rule -- native resolution, uncompressed
where offered, log the chosen format -- and replace the guesswork with an
explicit choice from the device's format list.

**Lesson to keep from #34:** release any probe stream before opening the
real one.

Prototype: `capture.rs` asks nokhwa for `AbsoluteHighestResolution` in RGBA
and logs `stream: WxH`. It does not prefer uncompressed over MJPEG, does not
log the frame rate or pixel format, and fails outright on devices that only
offer BGRA (OBS Virtual Camera: `Unknown FourCharCode 1111970369`).
