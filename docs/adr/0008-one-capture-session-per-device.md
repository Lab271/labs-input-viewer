# 0008. One capture session per device, shared by both halves

| | |
|---|---|
| **Status** | Accepted (Electron shape: one track per side) |
| **Date** | 2026-01-11 (69fa6d5, detection per unique device); 2026-08-20 (#261) |
| **Area** | Capture |
| **Evidence** | `renderer.js` `startVideoStream` :768-985, `selectInputForBoth` :2924, `getUniqueActiveDevices` :4855, detection mirroring :4758-4769 |
| **egui verdict** | Adapt |
| **egui status** | Done |

## Context

The wall usually shows the same card on both halves (`.claude/CLAUDE.md`).
Chromium shares one capture session between tracks on the same device,
which is why opening the device twice works -- and why a reopen must release
both sides first ([0013](0013-reopen-by-releasing-the-device-with-two-backoffs.md)).

## Decision

- Each side owns a MediaStream and a `<video>`, even when both show the
  same device. Switching a side stops that side's previous tracks.
- Detection runs once per unique device and mirrors the verdict to the
  twin side.
- A per-side generation token (`streamGen`) stops a late reopen from
  overwriting a switch the operator made meanwhile.

## Consequences

With the same device on both sides, each side has its own audio track; the
audio may play twice. Nothing addresses or documents that
([open questions](open-questions.md)).

## Porting to egui

**Adapt** to the native shape the Electron one approximates: open **one**
session per physical device and fan its frames out to both halves. That
makes the "release every side first" workaround implicit, and gives one
health verdict and one audio stream per device for free.

Prototype: done. `slots: HashMap<device_id, Slot>`; both halves draw from
the same texture; streams no visible side needs are dropped each frame
(`sync_streams`). The generation-token concern disappears because a slot is
keyed by device, not by side.
