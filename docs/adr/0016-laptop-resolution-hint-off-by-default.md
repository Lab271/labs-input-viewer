# 0016. Keep the "set your laptop to 3840x768" hint off by default

| | |
|---|---|
| **Status** | Accepted (premise weakened since 2026-10-02) |
| **Date** | 2026-10-01 (on in PR #314, off in PR #319) |
| **Area** | Signal |
| **Evidence** | `renderer.js:1129-1169`, `ASPECT_HINT_MS`; labs-videowall-config `edid/README.md` (08f8a82) |
| **egui verdict** | Keep |
| **egui status** | Not started |

## Context

Once cropping knew a laptop was sending a narrower-than-16:9 picture, a hint
could ask the presenter to switch to 3840x768. On the wall, changing
resolution dropped the HDBaseT link until the HDMI at the card was replugged:
picture lost for up to **156 s** over HDBaseT. On a 15 m HDMI cable there
were no drops in 20 switches. The EDID now makes 3840x768 a laptop's default
mode, and since 2026-10-02 the presenter link is HDMI, not HDBaseT.

## Decision

Off unless `aspectHint: true`. When on: single view only, for a cropped
source narrower than 16:9, for 15 s, 28 px at 6% from the top. The Apple TV
is native 16:9 and never triggers it.

## Consequences

The original reason for keeping it off -- the HDBaseT drop -- no longer
applies to the presenter link.

## Porting to egui

**Keep** off by default, behind the same setting. Revisit the default with
the wall's current cabling. Needs [0015](0015-crop-black-bars-by-snapping-to-shapes.md) first.
