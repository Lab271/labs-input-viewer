# nokhwa-bindings-windows 0.4.6, patched for Input Viewer

Vendored from crates.io (Apache-2.0, as upstream) and wired in through
`[patch.crates-io]` in `input_viewer_egui/Cargo.toml`. Every change is marked
`PATCHED (Input Viewer)` in `src/lib.rs`.

Found on the wall (2026-10-12): both Elgato 4K60 Pro MK.2 cards delivered
~13 fps to the egui prototype while Chromium, on the same cards in the same
state, negotiated 3840x2160 @ 59.94 and measured 60 fps.

1. **Fractional rates were discarded.** Rates are read as
   `numerator << 32 | denominator`, and any denominator other than 1 zeroed the
   rate. 59.94 is 60000/1001, so the cards' real top rate never reached the
   format list. Now rounded to whole fps (`ratio_fps`).
2. **`set_format` could pick a low rate.** A native media type matched when its
   range max, own rate or range min equalled the request, and was then set as
   is -- with its own `MF_MT_FRAME_RATE`, which on these cards can be any step
   of a ladder down to 1 fps. A type whose own rate matches now wins; a type
   that only covers the rate in its range is a fallback, with `MF_MT_FRAME_RATE`
   set explicitly (to the range max's exact ratio when that rounds to the
   request, so 60000/1001 stays 60000/1001).
3. **The current rate read as the denominator.** `format_refreshed` did
   `fps as u32` on the packed value, keeping the low 32 bits: 60000/1001 read
   as 1001.

Worth offering upstream (l1npengtul/nokhwa) once confirmed on the hardware.
