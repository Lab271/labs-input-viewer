# TODO - Input Viewer Electron

## ✅ Completed

- [x] Dual camera feed display (side by side)
- [x] Video quality (1920x1080 @ 30fps)
- [x] Center logo between feeds (scales with gap)
- [x] **D/S** buttons - Dual/Single view mode
- [x] **1-4** keys - Select input directly
- [x] Input name overlay on switch
- [x] **F11/F** - Toggle fullscreen
- [x] **Escape** - Exit fullscreen
- [x] **Q** - Quit application
- [x] Starts in fullscreen
- [x] Cursor hides after inactivity
- [x] **Freeze frame** - Space key to pause feed
  - Freeze indicator overlay (❙❙ FROZEN / ▶ LIVE)
- [x] **Settings persistence** - Save to settings.json
  - Remember selected inputs on restart
  - Remember layout preferences
- [x] **Edit input names** - Rename inputs in settings
- [x] **Enable/disable inputs** - Toggle switches in settings
- [x] Auto-updater with dialog prompts (uses public releases repo)
- [x] No-signal overlay (basic)
- [x] Screen aspect ratio detection (auto D/S on startup)
- [x] Dropdown panel (hover arrow at top)
- [x] Center gap slider (adjustable divider width)
- [x] Border width slider (left/right black borders)
- [x] Logo overlay in single view mode

---

## 🔴 P1 - Must Have

Essential features for release:

- [ ] **Update GitHub workflows** - Fix release.yml for electron-vite
  - Test build on macOS and Windows
  - Publish to input-viewer-releases repo
- [ ] **Windows compatibility** - Test and fix Windows-specific issues
  - Camera permissions handling
  - Path separators and file system
  - Code signing for Windows builds
- [ ] **Elgato no-signal detection** - Detect capture card "no signal" screen
  - Template matching for Elgato screens
  - Custom animated overlay when no signal detected
- [ ] **DVD-style screensaver** - When feeds lose signal
  - Bouncing logo animation
  - Color changes on bounce

---

## 🟠 P2 - Should Have

Important features from Python version:

- [ ] **Thumbnails panel** - T key to open
  - Live preview of all inputs
  - Click to select input
- [ ] **Auto-switch mode** - A key to toggle
  - Auto-switch when signal detected
  - Indicator overlay

---

## 🟡 P3 - Nice to Have

Polish and enhancement features:

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
