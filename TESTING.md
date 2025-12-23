# HDMI Viewer - Feature Testing Checklist

Rate each feature: **1** (not working), **2** (works but not good), **3** (works fine)

---

## Core Video Features

| # | Feature | Rating | Comment |
|---|---------|--------|---------|
| 1 | Dual camera feed display (side by side) | 3 | |
| 2 | Left feed video quality | 3 | |
| 3 | Right feed video quality | 3 | |
| 4 | Frame rate / smoothness | 2 | |
| 5 | Center logo displays between feeds | 3 | |
| 6 | Side margins (black space on edges) | 3 | |

---

## Layout Modes

| # | Feature | Rating | Comment |
|---|---------|--------|---------|
| 7 | **D** key - Switch to dual view | 3 | |
| 8 | **L** key - Switch to single left view (centered) | 3 | |
| 9 | **R** key - Switch to single right view (centered) | 3 | |
| 10 | Layout switching animation/transition | | |

---

## Input Selection

| # | Feature | Rating | Comment |
|---|---------|--------|---------|
| 11 | **1** key - Select input 1 | | |
| 12 | **2** key - Select input 2 | | |
| 13 | **3** key - Select input 3 | | |
| 14 | **4** key - Select input 4 | | |
| 15 | **T** key - Open thumbnails panel | | |
| 16 | Thumbnails panel - click to select input | | |
| 17 | Thumbnails panel - shows input names | | |
| 18 | Thumbnails panel - enabled/disabled styling | | |
| 19 | Input name overlay appears on switch | | |
| 20 | Input name overlay auto-hides | | |

---

## Fullscreen & Window

| # | Feature | Rating | Comment |
|---|---------|--------|---------|
| 21 | **F11** key - Toggle fullscreen | | |
| 22 | **F** key - Toggle fullscreen | | |
| 23 | **Escape** key - Exit fullscreen | | |
| 24 | **Q** key - Quit application | | |
| 25 | Window resize handling | | |
| 26 | Starts in fullscreen (if configured) | | |

---

## Cursor Auto-Hide

| # | Feature | Rating | Comment |
|---|---------|--------|---------|
| 27 | Cursor hides after inactivity | 2 | Only after clicking on an icon/settings |
| 28 | Cursor shows on mouse shake | 1 | |
| 29 | Cursor shows on mouse movement | 1 | |

---

## Freeze Frame

| # | Feature | Rating | Comment |
|---|---------|--------|---------|
| 30 | **Space** key - Toggle freeze frame | | |
| 31 | Freeze indicator overlay (❙❙ FROZEN) | | |
| 32 | Unfreeze shows (▶ LIVE) | | |
| 33 | Frozen frame maintains quality | | |

---

## Auto-Switch

| # | Feature | Rating | Comment |
|---|---------|--------|---------|
| 34 | **A** key - Toggle auto-switch | | |
| 35 | Auto-switch indicator overlay | | |
| 36 | Auto-switches when signal detected | | |

---

## No-Signal Detection

| # | Feature | Rating | Comment |
|---|---------|--------|---------|
| 37 | Detects Elgato "no signal" screen | | |
| 38 | Shows no-signal animation/message | | |
| 39 | Recovers when signal returns | | |
| 40 | Detection speed (quick vs slow) | | |

---

## Screensaver

| # | Feature | Rating | Comment |
|---|---------|--------|---------|
| 41 | Screensaver activates after both feeds lose signal | | |
| 42 | DVD-style bouncing logo animation | | |
| 43 | Logo color changes on bounce | | |
| 44 | Screensaver deactivates when signal returns | | |

---

## Settings Panel

| # | Feature | Rating | Comment |
|---|---------|--------|---------|
| 45 | **⚙** icon - Click to open settings | | |
| 46 | Settings panel appears centered | | |
| 47 | **✕** button - Close settings | | |
| 48 | Edit input name | | |
| 49 | Input name saves correctly | | |
| 50 | Enable/disable input toggle | | |
| 51 | Set default input toggle | | |
| 52 | Only one input can be default | | |
| 53 | At least one input stays enabled | | |
| 54 | Changes apply immediately | | |

---

## Info Panel

| # | Feature | Rating | Comment |
|---|---------|--------|---------|
| 55 | **ⓘ** icon - Hover to show info | | |
| 56 | Info panel shows keyboard shortcuts | | |
| 57 | Info panel hides on mouse leave | | |

---

## Audio Panel

| # | Feature | Rating | Comment |
|---|---------|--------|---------|
| 58 | **♪** icon - Click to open audio panel | | |
| 59 | Input volume slider | | |
| 60 | System volume slider | | |
| 61 | Mute input button | | |
| 62 | Mute system button | | |
| 63 | Volume changes apply | | |

---

## Test Modes (CLI)

| # | Feature | Rating | Comment |
|---|---------|--------|---------|
| 64 | `--mock` / `-m` - Mock video sources | | |
| 65 | `--switch-signals` / `-s` - Cycling signal test | | |
| 66 | `--no-signal` / `-n` - Always no-signal test | | |
| 67 | `--verbose` / `-v` - Verbose logging | | |
| 68 | **N** key - Toggle signal (test mode only) | | |

---

## Visual Polish

| # | Feature | Rating | Comment |
|---|---------|--------|---------|
| 69 | Icon hover effects (brighten on hover) | | |
| 70 | Toggle switch animation | | |
| 71 | Panel styling (rounded corners, transparency) | | |
| 72 | Overall dark theme consistency | | |

---

## Performance

| # | Feature | Rating | Comment |
|---|---------|--------|---------|
| 73 | CPU usage (acceptable?) | | |
| 74 | Memory usage (acceptable?) | | |
| 75 | Startup time | | |
| 76 | No lag when switching inputs | | |

---

## Settings Persistence

| # | Feature | Rating | Comment |
|---|---------|--------|---------|
| 77 | Settings saved to settings.json | | |
| 78 | Settings load on app restart | | |
| 79 | Default input remembered | | |

---

## Summary

| Category | Working (3) | Partial (2) | Broken (1) |
|----------|-------------|-------------|------------|
| Core Video | | | |
| Layout Modes | | | |
| Input Selection | | | |
| Window/Fullscreen | | | |
| Cursor | | | |
| Freeze Frame | | | |
| Auto-Switch | | | |
| No-Signal | | | |
| Screensaver | | | |
| Settings Panel | | | |
| Info Panel | | | |
| Audio Panel | | | |
| Test Modes | | | |
| Visual Polish | | | |
| Performance | | | |
| Persistence | | | |

**Total: ___ / 79 features tested**

---

## Notes / Bugs Found

<!-- Add any bugs or issues discovered during testing -->

1. 
2. 
3. 

---

## Test Environment

- **OS**: 
- **Display Resolution**: 
- **Capture Cards**: 
- **Python Version**: 
- **Date Tested**: 
