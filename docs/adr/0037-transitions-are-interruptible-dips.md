# 0037. Transitions: interruptible, a dip to black rather than a crossfade, instant under reduced motion

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-08-19 (#247, PR #257) |
| **Area** | UI |
| **Evidence** | `renderer.js` `LAYOUT_ANIM_MS` :2072, `layoutAnimMs` :2081, `:2064-2069, 2934-2962`; `styles.css:133, 180-189` |
| **egui verdict** | Keep, and reconsider the crossfade |
| **egui status** | Done |

## Context

At 6000x1200 an animation touches 7.2 Mpx per frame; #62 had shown
compositing is not the bottleneck. A true crossfade between inputs needs a
second `<video>` decoding the outgoing stream, doubling the decode.

## Decision

- **Dual <-> single:** 320 ms `flex-grow` + opacity collapse of the sibling
  (not `scaleX`, which distorts the picture).
- **Input switch:** 130 ms dip to black, hold while the stream is acquired,
  then snap in. Only the picker path fades; the number-key path does not
  (rationale not recorded).
- **Interruptible:** a token per side, so the newest switch wins ("pressing 2
  then 3 quickly must leave input 3 visible"); a second layout change
  cancels a pending hide.
- **Reduced motion:** both instant (`prefers-reduced-motion`).
- Freeze stays instant.

## Consequences

320 ms and 130 ms are "my choices, not measured optima", and the animations
were never watched on the wall (PR #257). A comment still says the switch
"fades back in over 180 ms"; it snaps.

## Porting to egui

**Keep** interruptibility, freeze-is-instant and a reduced-motion setting
(egui has no OS query for it; make it a setting). **Reconsider the
crossfade:** with one session per device both pictures are already textures,
so a crossfade costs a second textured quad, not a second decode. The
display-none/reflow tricks and `!important` against inline styles are DOM
artefacts.

Done (`transition.rs`): the 320 ms slide and the 130 ms dip, holding black
up to 2 s for the new input's first frame. Interruptible because neither is
a timeline: the layout is a position moving toward the setting, and a dip
remembers only what it is leaving. Every switch dips, number keys included.
Reduced motion is a setting (`reducedMotion`, Layout pane). The crossfade is
still not built.
