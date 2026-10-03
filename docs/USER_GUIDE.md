# Input Viewer - User Guide

Input Viewer is a lightweight video input display application for viewing multiple video feeds simultaneously. Perfect for presenter booths and control rooms.

## Features

- **Dual/Single View** - Display one or two video inputs side by side
- **No-Signal Detection** - Automatically shows "NO SIGNAL" when input is disconnected
- **29 Screensavers** - GPU-rendered, shown when all inputs have no signal (5 min timeout)
- **Freeze Frame** - Pause the display without affecting the source
- **Audio Controls** - Control capture card volume and system output
- **Remote Keyboard** - Forward clicker presses to a presenter PC over network
- **Touch Support** - Tap-friendly dropdown menu

## Installation

Download the latest version from GitHub Releases:

- **macOS**: `Input-Viewer-X.X.X.dmg`
- **Windows**: `Input-Viewer-Setup-X.X.X.exe`

The app auto-updates when new versions are available.

## Presenter Booth Setup

1. **Connect your laptop** using the Thunderbolt (USB-C) cable in the presenter booth
   - The cable is marked with **"UP"** - this side should face up when plugging in
2. Wait about **10 seconds** for your screen to appear
3. This cable connects to a docking station, which is connected to the PC controlling the videowall
4. Your laptop screen will appear as an input in the Input Viewer app

## Basic Usage

### Selecting Inputs

1. Hover at the top of the screen (or tap the tab there) to bring up the input controls
2. They appear **over the wall itself**: each half shows what it is playing and a strip of
   inputs to choose from. Tap an input to put it on that half. The controls stay open, so
   you can change both halves in one go
3. Press **Close** at the bottom (or `Esc`) when you are done. They also close by themselves
   after 30 seconds without anything being touched, so they never cover an unattended wall

In **Single** view there is one strip, for the whole wall, and its inputs carry the number
keys `1`-`4` because a tap does the same as the key.

**Multi-view** (Settings > Inputs, on by default) is what lets each half show a different
input. With it off, dual view always shows one input on both halves and the controls have a
single strip: one tap and it is on the whole wall.

### View Modes

| Mode | Description |
|------|-------------|
| **Dual** | Two inputs side by side (for wide screens) |
| **Single** | One input fills the screen |

The app automatically selects the default mode based on your screen aspect ratio.

### Filling the whole wall from a laptop

The wall is 5:1 (6000x1200). A laptop fills it in single view at **3840 x 768**,
which is what it gets by default when plugged in. Any other shape is shown in full
with black at the sides, never stretched.

**Don't change the laptop's resolution while it is connected.** On this wall the
picture is lost on a resolution change and does not come back by itself. If it
happens, replug the HDMI cable at the capture card.

A tip suggesting 3840 x 768 for narrower pictures exists, but is off by default for
that reason. `"aspectHint": true` in `settings.json` turns it on.

The capture cards add black bars around anything that is not 16:9. Input Viewer
crops those away automatically, so a 3840 x 768 picture fills the wall rather than
sitting in a small band in the middle. To turn that off, set
`"cropLetterbox": false` in `settings.json`.

## Keyboard Shortcuts

Hover the **bottom** edge of the screen to bring up a legend of every shortcut, the same way
hovering the top edge brings up the input dropdown. The legend is generated from the app's own
bindings, so it always matches what the keys actually do.


| Key | Action |
|-----|--------|
| `1` - `4` | Select input 1-4 |
| `D` | Switch to Dual view |
| `S` | Switch to Single view |
| `Space` | Freeze/Unfreeze frame |
| `F` / `F11` | Toggle fullscreen |
| `Esc` | Exit fullscreen / Unfreeze / Close menus |
| `Q` | Quit application |
| `V` | Show or hide the screensaver immediately, without waiting out the 5-minute no-signal delay |
| `+` / `-` | Step forward or back through the screensavers while one is showing |
| `←` `→` `PgUp` `PgDn` | Remote keyboard (if enabled) |

## Settings

Open Settings with the gear in the capsule at the top of the input controls. It has four
sections in a side menu: **Inputs**, **Layout**, **Remote keyboard** and **Art-Net
lighting**. The menu shows what needs attention: a count of inputs without a no-signal
reference, and On/Off for the two integrations (orange when one is on but not working or
not filled in). Everything saves as you change it; there is no Apply button.

Each input in the controls shows a **snapshot** of what it is currently sending, taken
when the dropdown opens. These are stills, not live previews — they are as recent as the
moment you opened the panel. An input that is not currently on screen is sampled briefly to
take its picture; one that cannot be reached, or that has nothing plugged into it, keeps an
empty tile.

### Inputs

- **Multi-view** - Whether each half of the wall can show a different input (see above)
- **Key** - The number key that selects the input. Only enabled inputs are numbered, and
  only the first four have a key
- **On** - Disabled inputs are hidden from the controls and the number keys. One that is on
  the wall when you switch it off stays there until that half is switched
- **Name** - Leave empty to use the name the capture card reports
- **Startup** - The input shown when the app starts. Click it again to clear it
- **No-signal** - How many no-signal references the input has; click to see them

### Layout

A drawing of the wall shows the halves, the center gap and the side borders to scale.

- **Center gap** - Space between the two halves (dual view only)
- **Side borders** - Black borders on the left and right edges

### No-Signal Detection

Capture what your capture card shows when nothing is connected. This allows the app to detect "no signal" and show the overlay.

1. Disconnect the source from the capture card, so it shows its no-signal screen
2. Put that input on the wall
3. In Settings > Inputs, click its **No-signal** badge, then **Capture from left half** (or
   right half, whichever it is on)
4. The app remembers the picture. If the capture fails it says why

References belong to the capture card, not to a half of the wall, so they keep working when
the card moves to the other side.

## Volume Controls

In the input controls:

- **Volume** under each strip - The audio of that half of the wall. It belongs to the half,
  not to the input, so it stays put when you switch inputs
- **Output** in the capsule at the top - System volume

The output slider follows the system volume while the controls are open.

## Remote Keyboard

Control presentations on a remote PC using a wireless clicker.

### How It Works

```text
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Clicker   │────>│Input Viewer │────>│   Arduino   │────>│ Presenter   │
│  (RF/BT)    │     │     App     │     │  (ESP32-S3) │     │     PC      │
└─────────────┘     └─────────────┘     └─────────────┘     └─────────────┘
                         WiFi              USB HID
```

1. Press a button on your wireless clicker
2. Input Viewer receives the keypress
3. Input Viewer sends HTTP request to Arduino over WiFi
4. Arduino sends keypress to presenter PC via USB

### Setup

**In Input Viewer:**

1. Open **Settings** > **Remote Keyboard**
2. Enable the toggle
3. Enter **Hostname**: `space_keyboard` (or the Arduino's IP)
4. Enter **API Key**: the secret key configured on the Arduino

**Arduino side:**

The Arduino (ESP32-S3) must be:

- Connected to the same WiFi network
- Connected to the presenter PC via USB
- Configured with matching API key

### Supported Clickers

The app listens for multiple key types to support different clickers:

- Arrow keys (← →)
- Page Up / Page Down

## Touch Screen Support

For touch screen setups:

- **Tap** the tab at the top edge to bring up the input controls
- **Tap Close** at the bottom to put them away

## Screensavers

When all video feeds show "no signal" for 5 minutes, a screensaver appears. There are
**29**, one picked at random, and it changes every 10 minutes. The rotation never picks
the same one twice in a row.

Each screensaver also looks different every time it starts — the random choices inside
it are seeded from the clock — so the same one twice is not the same picture twice.

To exit the screensaver:

- Move the mouse
- Shake the mouse rapidly
- Press any key
- Touch the screen

You can also drive it by hand: **`V`** shows or hides it immediately rather than waiting
out the five-minute delay, and **`+`** / **`-`** step through the set in the order below.

### The split-flap board is not one of them

When a feed loses signal you first see a **split-flap departures board**. That is the
no-signal display, not a screensaver: it appears straight away rather than after five
minutes, it never comes up in the rotation, and `+` / `-` cannot reach it.

### The screensavers

Listed in rotation order, which is the order `+` and `-` step through.

<!-- SCREENSAVER-LIST -->
1. DVD Logo
2. Plasma
3. Flow Field
4. Raymarch Fractal
5. Julia Family
6. Burning Ship
7. Reaction Diffusion
8. Particle Swarm
9. White Particles
10. Boids
11. Strange Attractor
12. Voronoi
13. Metaballs
14. Game of Life
15. Matrix Rain
16. Starfield Warp
17. Pong
18. Truchet Tiles
19. Moire Interference
20. ASCII Doughnut
21. Double Pendulum
22. Wave Tank
23. Falling Sand
24. Frost
25. Tree Growth
26. Physarum
27. Aquarium
28. Bicycle Horizon
29. Weather<!-- /SCREENSAVER-LIST -->

### Weather and Art-Net

Two of these reach outside the machine, and **both are off until you turn them on**:

- **Weather** (number 30) draws the live conditions for a
  configured latitude and longitude. With it off, it is skipped in the rotation. It
  fetches from a public weather service every 15 minutes while enabled.
- **Art-Net reactive mode** is not a screensaver but affects them all: it sends the
  colour of whatever is on screen to a lighting service, so the room matches the wall.

Neither makes any network request until enabled and configured. See the Configuration
section of `README.md` for the settings and exactly what each one sends.

## Troubleshooting

| Problem | Solution |
|---------|----------|
| No video showing | Check capture card connection and permissions |
| "NO SIGNAL" not detecting | Re-capture the no-signal reference in Settings |
| Remote keyboard not working | Check WiFi connection and API key |
| Audio not working | Ensure capture card provides audio (not all do) |
| Touch not opening menu | Tap near the top center of the screen |

## System Requirements

- **macOS** 10.13+ or **Windows** 10+
- USB capture card (HDMI/SDI to USB)
- For remote keyboard: ESP32-S3 Arduino + WiFi network
