# TODO - Input Viewer Electron

## ✅ Completed

- [x] Dual camera feed display (side by side)
- [x] Video quality (1920x1080 @ 30fps)
- [x] Center logo between feeds
- [x] Layout gap slider
- [x] **D/L/R** keys - Layout switching
- [x] **1-4** keys - Select input directly
- [x] Input name overlay on switch
- [x] **F11/F** - Toggle fullscreen
- [x] **Escape** - Exit fullscreen
- [x] **Q** - Quit application
- [x] Starts in fullscreen
- [x] Cursor hides after inactivity
- [x] Settings panel with input selection
- [x] Info panel with keyboard shortcuts
- [x] Auto-updater (check, download, notify)
- [x] No-signal overlay (basic)

---

## 🔴 P1 - Must Have

Essential features for basic usability:

- [ ] **Freeze frame** - Space key to pause feed
  - Freeze indicator overlay (❙❙ FROZEN / ▶ LIVE)
- [ ] **Settings persistence** - Save to settings.json
  - Remember selected inputs on restart
  - Remember layout gap preference
- [ ] **Edit input names** - Rename inputs in settings
- [ ] **Enable/disable inputs** - Toggle switches in settings

---

## 🟠 P2 - Should Have

Important features from Python version:

- [ ] **Thumbnails panel** - T key to open
  - Live preview of all inputs
  - Click to select input
- [ ] **No-signal detection** - Template matching
  - Detect capture card "no signal" screen
  - Animated no-signal icon (spinning)
- [ ] **Auto-switch mode** - A key to toggle
  - Auto-switch when signal detected
  - Indicator overlay

---

## 🟡 P3 - Nice to Have

Polish and enhancement features:

- [ ] **Screensaver** - When both feeds lose signal
  - DVD-style bouncing logo
  - Color changes on bounce
- [ ] **Layout animations** - Smooth transitions
- [ ] **Cursor shake detection** - Show cursor on shake
- [ ] **Audio controls** - ♪ panel
  - Volume sliders
  - Mute buttons

---

## 🔵 P4 - Future Ideas

- [ ] Test mode flags (`--mock`, `--no-signal`)
- [ ] Keyboard shortcut customization
- [ ] Picture-in-picture layout
- [ ] Recording functionality
- [ ] Streaming output (NDI, virtual camera)
- [ ] Color correction / brightness controls
- [ ] Restart to install update button

---

## 🔮 Future Exploration: Tauri

Consider migrating to [Tauri](https://tauri.app/) for:

- **~10MB** bundle vs Electron's ~150MB
- **Lower memory** - System WebView vs Chromium
- **Rust backend** - Better performance potential

Migration: Keep frontend, rewrite IPC in Rust, use `nokhwa` for video capture.
