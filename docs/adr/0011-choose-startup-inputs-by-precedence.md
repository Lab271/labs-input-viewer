# 0011. Choose each half's input by a fixed precedence, and keep it across hot-plug

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-10-01 (PR #318, 8cb7e0a); 2026-10-02 (PR #324) |
| **Area** | Capture |
| **Evidence** | `renderer.js` `openInitialStreams` :4910-4940, `getVideoDevices` :654-766 (keep-side :713-747), `devicechange` :4282-4296, `ensureRightStream` :2096 |
| **egui verdict** | Keep |
| **egui status** | Partial |

## Context

Two incidents on the wall:

- **2026-10-01.** An EDID write in the Elgato utility fired `devicechange`
  with no hardware change. The re-pick started from null saved settings and
  relabelled the left half "Apple TV" while it still showed the presenter;
  the next health reopen would have switched the wall.
- **2026-10-02.** Starting in single view opened only the left stream, so
  `D` showed a black right half.

## Decision

For each half, in order:

1. Keep the current input if that device is still present (hot-plug).
2. Else the saved `leftDeviceId` / `rightDeviceId`.
3. Else the first enabled device (left), and the second enabled device or a
   duplicate of the first (right).
4. At startup only, `defaultInputId` overrides left -- and right in dual --
   if it is present and enabled.
5. With Multi-view off, right always follows left.

Settings are saved only after the inputs are chosen (writing null over the
saved choice is how the wall once forgot its input). On `devicechange`,
re-enumerate and immediately reopen any side that is `open-failed`, `ended`
or `no-frames`, skipping its backoff. Dual view always ensures a live right
stream.

## Consequences

A device-change event can fire with no hardware change; the precedence makes
that harmless.

## Porting to egui

**Keep.** Prototype: the precedence is `choose_sides()` in `app.rs`, unit
tested against these rules (startup default, hot-plug keeps the screen,
disabled default ignored, Multi-view off). The "not overwritten with None"
rule is in the frame loop's settings sync.

Missing: hot-plug is a 10 s re-enumeration poll, not an OS device-arrival
notification, and a device returning does not short-circuit the reopen
backoff.
