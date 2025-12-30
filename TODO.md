# TODO - Future Improvements

## High Priority

- [ ] Complete Electron app implementation
- [ ] Add auto-updater with electron-updater
- [ ] Create installers for macOS (DMG), Windows (NSIS), Linux (AppImage)

## Medium Priority

- [ ] Add input source previews/thumbnails in settings
- [ ] Keyboard shortcut customization
- [ ] Multiple layout presets (side-by-side, picture-in-picture, etc.)

## Future Exploration

### Tauri Migration (Option 3)

Consider migrating from Electron to [Tauri](https://tauri.app/) for:

- **Smaller bundle size**: ~10MB vs Electron's ~150MB
- **Lower memory usage**: Uses system WebView instead of bundled Chromium
- **Built-in updater**: Similar to electron-updater
- **Rust backend**: Could potentially rewrite video processing in Rust for better performance

**Requirements:**
- Rust toolchain
- WebView2 (Windows) / WebKit (macOS/Linux)
- Video capture via Rust libraries (e.g., `nokhwa` crate)

**Migration path:**
1. Keep the same frontend (HTML/CSS/JS)
2. Rewrite IPC handlers in Rust
3. Implement video capture with `nokhwa` or similar
4. Use Tauri's updater plugin

## Ideas

- [ ] Recording functionality
- [ ] Streaming output (NDI, virtual camera)
- [ ] Audio level meters
- [ ] Color correction / brightness controls
- [ ] Custom overlay graphics
