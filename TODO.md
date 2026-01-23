# TODO - Input Viewer Electron

## Completed

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
- [x] No-signal detection with reference screenshot capture
- [x] Screen aspect ratio detection (auto D/S on startup)
- [x] Dropdown panel (hover arrow at top)
- [x] Center gap slider (adjustable divider width)
- [x] Border width slider (left/right black borders)
- [x] Logo overlay in single view mode
- [x] **DVD-style screensaver** - When feeds lose signal
  - Bouncing logo animation
  - Color changes on bounce
  - 5-minute delay before activation
- [x] **CI/CD Pipeline** - Automated releases
  - Build on macOS and Windows
  - Auto version bump from conventional commits
  - Publish to input-viewer-releases repo
- [x] **Windows compatibility** - Tested and working
  - Camera permissions handling
  - Windows installer (.exe)
  - Auto-updater works on Windows

---

## High Priority

### User Experience

- [ ] **Thumbnails panel** - T key to open
  - Live preview of all inputs
  - Click to select input
- [ ] **Auto-switch mode** - A key to toggle
  - Auto-switch when signal detected
  - Indicator overlay

---

## Medium Priority

### Polish

- [ ] **Layout animations** - Smooth transitions between views
- [ ] **Cursor shake detection** - Show cursor on shake movement
- [ ] **Audio controls** - Volume panel
  - Volume sliders per input
  - Mute buttons

---

## Low Priority / Future Ideas

- [ ] **Multi-view mode** - Different inputs on each screen panel
- [ ] **Picture-in-picture layout** - Small preview in corner
- [ ] **Keyboard shortcut customization** - User-configurable keys
- [ ] **Touchscreen controls** - Enhanced touch support
- [ ] **CI testing** - Automated tests in pipeline
  - Unit tests for core logic
  - Integration tests for video capture
- [ ] Test mode flags (`--mock`, `--no-signal`)

---

## Future Exploration: Tauri

Consider migrating to [Tauri](https://tauri.app/) for:

- **~10MB** bundle vs Electron's ~150MB
- **Lower memory** - System WebView vs Chromium
- **Rust backend** - Better performance potential

Migration: Keep frontend, rewrite IPC in Rust, use `nokhwa` for video capture.
