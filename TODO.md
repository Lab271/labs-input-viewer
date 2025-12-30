# TODO - Input Viewer Electron

## Feature Parity with Python Version

### Core Video Features
- [x] Dual camera feed display (side by side)
- [x] Video quality (1920x1080 @ 30fps)
- [x] Center logo between feeds
- [x] Side margins (black space on edges)
- [x] Layout gap slider

### Layout Modes
- [x] **D** key - Dual view
- [x] **L** key - Single left view (centered)
- [x] **R** key - Single right view (centered)
- [ ] Layout switching animation/transition

### Input Selection
- [x] **1-4** keys - Select input directly
- [x] Input name overlay on switch
- [x] Input name auto-hides
- [ ] **T** key - Open thumbnails panel
- [ ] Thumbnails panel with live previews
- [ ] Click thumbnail to select input

### Fullscreen & Window
- [x] **F11** / **F** key - Toggle fullscreen
- [x] **Escape** key - Exit fullscreen
- [x] **Q** key - Quit application
- [x] Window resize handling
- [x] Starts in fullscreen

### Cursor Management
- [x] Cursor hides after inactivity
- [ ] Cursor shows on mouse shake
- [x] Cursor shows on mouse movement

### Freeze Frame
- [ ] **Space** key - Toggle freeze frame
- [ ] Freeze indicator overlay (❙❙ FROZEN)
- [ ] Unfreeze shows (▶ LIVE)
- [ ] Frozen frame maintains quality

### Auto-Switch
- [ ] **A** key - Toggle auto-switch mode
- [ ] Auto-switch indicator overlay
- [ ] Auto-switches when signal detected on other input

### No-Signal Detection
- [x] Shows no-signal overlay
- [ ] Animated no-signal icon (spinning/pulsing)
- [ ] Template matching for capture card "no signal" screen
- [ ] Detection speed optimization

### Screensaver
- [ ] Activates after both feeds lose signal
- [ ] DVD-style bouncing logo animation
- [ ] Logo color changes on bounce
- [ ] Deactivates when signal returns

### Settings Panel
- [x] **⚙** icon to open settings
- [x] Settings panel UI
- [x] **✕** button to close
- [x] Select input for left/right feed
- [x] Layout gap slider
- [ ] Edit input names
- [ ] Enable/disable input toggle
- [ ] Set default input toggle
- [ ] Changes persist to settings.json

### Info Panel
- [x] **ⓘ** icon - Click to show info
- [x] Info panel shows keyboard shortcuts
- [x] Info panel hides on click outside

### Audio Panel
- [ ] **♪** icon - Audio settings
- [ ] Input volume slider
- [ ] System volume slider
- [ ] Mute input button
- [ ] Mute system button

### Test Modes (CLI)
- [ ] `--mock` - Mock video sources
- [ ] `--switch-signals` - Cycling signal test
- [ ] `--no-signal` - Always no-signal test
- [ ] `--verbose` - Verbose logging
- [ ] **N** key - Toggle signal (test mode)

### Auto-Updater
- [x] Check for updates on startup
- [x] Download update in background
- [x] Show update notification
- [ ] Restart to install update button

---

## Future Improvements

### High Priority
- [ ] Thumbnails panel with live previews
- [ ] Freeze frame functionality
- [ ] No-signal detection with template matching
- [ ] Settings persistence to file

### Medium Priority
- [ ] Auto-switch mode
- [ ] Animated no-signal icon
- [ ] Screensaver (bouncing logo)
- [ ] Audio controls

### Nice to Have
- [ ] Keyboard shortcut customization
- [ ] Multiple layout presets (picture-in-picture)
- [ ] Recording functionality
- [ ] Streaming output (NDI, virtual camera)
- [ ] Color correction / brightness controls
- [ ] Custom overlay graphics

---

## Future Exploration

### Tauri Migration

Consider migrating from Electron to [Tauri](https://tauri.app/) for:

- **Smaller bundle size**: ~10MB vs Electron's ~150MB
- **Lower memory usage**: Uses system WebView instead of bundled Chromium
- **Built-in updater**: Similar to electron-updater
- **Rust backend**: Better performance potential

**Migration path:**
1. Keep the same frontend (HTML/CSS/JS)
2. Rewrite IPC handlers in Rust
3. Implement video capture with `nokhwa` or similar
4. Use Tauri's updater plugin
