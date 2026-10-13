// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! `settings.json`, in the Electron app's format.
//!
//! Same keys and the same file layout as Electron's, so it can be read, edited
//! and copied around the same way on the wall. Lives in its own folder,
//! `<config>/Input Viewer (egui)/`, next to Electron's `<config>/Input Viewer/`,
//! so the two apps never write the same file.
//!
//! On first launch the Electron file is imported. What carries over is the
//! layout -- centerGap, borderWidth, layoutMode, multiView -- and every key this
//! app does not use (Art-Net, remote keyboard, volumes...), kept verbatim in
//! `extra` and written back unchanged. Input names do **not** carry over:
//! Electron keys `inputs` by Chromium's deviceId, a salted hash nothing outside
//! Chromium can reproduce, and stores no label to match on instead. Those
//! entries are kept but stay inert.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use crate::filelog::{log_error, log_info, log_warn};

/// Electron's `debouncedSaveSettings` delay: a slider drag writes once, after
/// it stops.
const SAVE_DEBOUNCE: Duration = Duration::from_millis(300);

pub const APP_DIR: &str = "Input Viewer (egui)";
/// Electron's `userData` folder name. Not the product name "Input Viewer":
/// with no top-level `productName` in package.json, Electron uses `name`.
/// Checked on the wall 2026-10-12: `%APPDATA%\input-viewer\`.
pub const ELECTRON_APP_DIR: &str = "input-viewer";

/// The ranges of Electron's sliders.
pub const CENTER_GAP_RANGE: std::ops::RangeInclusive<f32> = 20.0..=200.0;
pub const BORDER_WIDTH_RANGE: std::ops::RangeInclusive<f32> = 0.0..=200.0;
/// The logo's width when it is not fitted to the gap.
pub const LOGO_WIDTH_RANGE: std::ops::RangeInclusive<f32> = 20.0..=1200.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Layout {
    #[default]
    Dual,
    Single,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct InputSettings {
    /// Operator's name for the input; None shows the device's own label.
    pub name: Option<String>,
    pub enabled: bool,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Default for InputSettings {
    fn default() -> Self {
        Self { name: None, enabled: true, extra: Map::new() }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Electron also allows null ("detect from the screen"); read as dual.
    pub layout_mode: Option<Layout>,
    pub multi_view: bool,
    pub center_gap: f32,
    /// Crop the capture card's black bars (ADR 0015). On by default, as in
    /// Electron; `false` in settings.json turns it off.
    pub crop_letterbox: bool,
    /// Switches and layout changes are instant (ADR 0037). Electron read
    /// the OS `prefers-reduced-motion`; egui has no such query, so it is a
    /// setting. Electron ignores the key.
    pub reduced_motion: bool,
    /// The centre logo's width in px. None: fitted to the centre gap, as in
    /// Electron. Set, it may be wider than the gap and is drawn over the
    /// pictures. Electron ignores the key.
    pub logo_width: Option<f32>,
    /// Screensavers left out of the rotation, by display name. A list of the
    /// *disabled* ones, so a saver added later is on by default. Electron has
    /// no such setting and ignores the key.
    pub disabled_screensavers: Vec<String>,
    /// The remote keyboard (ADR 0040), under Electron's keys. The API key
    /// is plain text, as it is in Electron and in the firmware.
    pub remote_keyboard_enabled: bool,
    pub remote_keyboard_host: String,
    pub remote_keyboard_api_key: String,
    pub border_width: f32,
    pub left_device_id: Option<String>,
    pub right_device_id: Option<String>,
    /// The input both halves start on. None: the first enabled one.
    pub default_input_id: Option<String>,
    pub inputs: BTreeMap<String, InputSettings>,
    /// Everything this app does not use, preserved for the round trip.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            layout_mode: Some(Layout::Dual),
            multi_view: true,
            center_gap: 60.0,
            crop_letterbox: true,
            reduced_motion: false,
            logo_width: None,
            disabled_screensavers: Vec::new(),
            remote_keyboard_enabled: false,
            remote_keyboard_host: String::new(),
            remote_keyboard_api_key: String::new(),
            border_width: 0.0,
            left_device_id: None,
            right_device_id: None,
            default_input_id: None,
            inputs: BTreeMap::new(),
            extra: Map::new(),
        }
    }
}

impl Settings {
    pub fn layout(&self) -> Layout {
        self.layout_mode.unwrap_or_default()
    }

    /// The input's entry, created with defaults on first touch, as Electron's
    /// `state.settings.inputs[deviceId] ??= { enabled: true }`.
    pub fn input_mut(&mut self, id: &str) -> &mut InputSettings {
        self.inputs.entry(id.to_owned()).or_default()
    }

    pub fn is_enabled(&self, id: &str) -> bool {
        self.inputs.get(id).is_none_or(|i| i.enabled)
    }

    /// The operator's name if set and not blank.
    pub fn name_of(&self, id: &str) -> Option<&str> {
        self.inputs.get(id)?.name.as_deref().map(str::trim).filter(|n| !n.is_empty())
    }

    pub fn saver_enabled(&self, name: &str) -> bool {
        !self.disabled_screensavers.iter().any(|n| n == name)
    }

    pub fn set_saver_enabled(&mut self, name: &str, on: bool) {
        self.disabled_screensavers.retain(|n| n != name);
        if !on {
            self.disabled_screensavers.push(name.to_owned());
        }
    }

    /// Pull out-of-range values from a hand-edited file back into the sliders'
    /// range rather than rendering a 5000px gap.
    pub fn sanitise(&mut self) {
        let clamp = |v: f32, r: &std::ops::RangeInclusive<f32>, d: f32| {
            if v.is_finite() { v.clamp(*r.start(), *r.end()) } else { d }
        };
        self.center_gap = clamp(self.center_gap, &CENTER_GAP_RANGE, 60.0);
        self.border_width = clamp(self.border_width, &BORDER_WIDTH_RANGE, 0.0);
        self.logo_width = self.logo_width.filter(|w| w.is_finite()).map(|w| w.clamp(*LOGO_WIDTH_RANGE.start(), *LOGO_WIDTH_RANGE.end()));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Origin {
    Own,
    ImportedFromElectron(PathBuf),
    Defaults,
    /// The file did not parse. It was moved aside, not overwritten.
    Recovered { backup: PathBuf, error: String },
}

pub struct Store {
    path: Option<PathBuf>,
    dirty_since: Option<Instant>,
}

pub fn default_path() -> Option<PathBuf> {
    Some(dirs::config_dir()?.join(APP_DIR).join("settings.json"))
}

pub fn electron_path() -> Option<PathBuf> {
    Some(dirs::config_dir()?.join(ELECTRON_APP_DIR).join("settings.json"))
}

pub fn parse(text: &str) -> Result<Settings, serde_json::Error> {
    let mut s: Settings = serde_json::from_str(text)?;
    s.sanitise();
    Ok(s)
}

/// Load from `path`, else import from `electron`, else defaults.
pub fn load_from(path: &Path, electron: Option<&Path>) -> (Settings, Origin) {
    match std::fs::read_to_string(path) {
        Ok(text) => match parse(&text) {
            Ok(s) => return (s, Origin::Own),
            Err(e) => {
                let backup = path.with_extension(format!(
                    "json.corrupt-{}",
                    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
                ));
                let _ = std::fs::rename(path, &backup);
                return (Settings::default(), Origin::Recovered { backup, error: e.to_string() });
            }
        },
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            log_warn!("[Settings] cannot read {}: {e}", path.display());
        }
        Err(_) => {}
    }
    if let Some(electron) = electron
        && let Ok(text) = std::fs::read_to_string(electron)
        && let Ok(s) = parse(&text)
    {
        return (s, Origin::ImportedFromElectron(electron.to_owned()));
    }
    (Settings::default(), Origin::Defaults)
}

impl Store {
    /// `enabled: false` (mock mode) loads nothing and never writes: one mock run
    /// must not persist `mock-input-N` as the wall's real selection.
    pub fn open(enabled: bool) -> (Settings, Self) {
        let path = default_path().filter(|_| enabled);
        let Some(p) = &path else {
            return (Settings::default(), Self { path: None, dirty_since: None });
        };
        let (settings, origin) = load_from(p, electron_path().as_deref());
        match &origin {
            Origin::Own => log_info!("[Settings] loaded {}", p.display()),
            Origin::ImportedFromElectron(from) => log_info!("[Settings] imported layout from {}", from.display()),
            Origin::Defaults => log_info!("[Settings] none found, using defaults"),
            Origin::Recovered { backup, error } => {
                log_warn!("[Settings] {} did not parse ({error}); moved to {}", p.display(), backup.display())
            }
        }
        let mut store = Self { path, dirty_since: None };
        // An import is written straight away, so it happens exactly once.
        if !matches!(origin, Origin::Own) {
            store.write(&settings);
        }
        (settings, store)
    }

    /// Where settings are written; None in mock and review runs.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn mark_dirty(&mut self) {
        self.dirty_since = Some(Instant::now());
    }

    /// Write if a change has settled. Call every frame.
    pub fn tick(&mut self, settings: &Settings) {
        if self.dirty_since.is_some_and(|t| t.elapsed() >= SAVE_DEBOUNCE) {
            self.write(settings);
        }
    }

    pub fn flush(&mut self, settings: &Settings) {
        if self.dirty_since.is_some() {
            self.write(settings);
        }
    }

    fn write(&mut self, settings: &Settings) {
        self.dirty_since = None;
        let Some(path) = &self.path else { return };
        if let Err(e) = write_atomic(path, settings) {
            log_error!("[Settings] could not save {}: {e}", path.display());
        }
    }
}

/// Write to a temporary file and rename over the real one, so a crash or power
/// cut mid-write leaves the previous settings rather than half a file.
pub fn write_atomic(path: &Path, settings: &Settings) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(settings)? + "\n")?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A trimmed real Electron settings.json.
    const ELECTRON: &str = r#"{
      "leftDeviceId": "4f1c0e9a",
      "rightDeviceId": "4f1c0e9a",
      "layoutMode": "single",
      "centerGap": 80,
      "borderWidth": 12,
      "defaultInputId": null,
      "leftVolume": 0.5,
      "inputs": { "4f1c0e9a": { "name": "Booth", "enabled": true } },
      "multiView": false,
      "artnetEnabled": true,
      "artnetUrl": "http://relay.local",
      "artnetSceneBySaver": { "plasma": "warm" }
    }"#;

    #[test]
    fn reads_the_electron_format() {
        let s = parse(ELECTRON).unwrap();
        assert_eq!(s.layout(), Layout::Single);
        assert_eq!((s.center_gap, s.border_width, s.multi_view), (80.0, 12.0, false));
        assert_eq!(s.name_of("4f1c0e9a"), Some("Booth"));
    }

    #[test]
    fn keys_this_app_does_not_use_survive_a_round_trip() {
        let s = parse(ELECTRON).unwrap();
        let back: Value = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back["artnetUrl"], "http://relay.local");
        assert_eq!(back["artnetSceneBySaver"]["plasma"], "warm");
        assert_eq!(back["leftVolume"], 0.5);
        assert_eq!(back["layoutMode"], "single");
        assert_eq!(back["centerGap"], 80.0);
    }

    #[test]
    fn null_layout_and_missing_keys_get_defaults() {
        let s = parse(r#"{ "layoutMode": null }"#).unwrap();
        assert_eq!(s.layout(), Layout::Dual);
        assert!(s.multi_view);
        assert_eq!(s.center_gap, 60.0);
        assert!(s.is_enabled("never-seen"));
    }

    #[test]
    fn hand_edited_values_are_pulled_into_range() {
        let s = parse(r#"{ "centerGap": 5000, "borderWidth": -3 }"#).unwrap();
        assert_eq!((s.center_gap, s.border_width), (200.0, 0.0));
    }

    #[test]
    fn blank_names_fall_back_to_the_device_label() {
        let mut s = Settings::default();
        s.input_mut("a").name = Some("   ".into());
        assert_eq!(s.name_of("a"), None);
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("iv-settings-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn imports_electron_once_then_reads_its_own() {
        let dir = scratch("import");
        let (own, electron) = (dir.join("egui/settings.json"), dir.join("electron/settings.json"));
        std::fs::create_dir_all(electron.parent().unwrap()).unwrap();
        std::fs::write(&electron, ELECTRON).unwrap();

        let (s, origin) = load_from(&own, Some(&electron));
        assert!(matches!(origin, Origin::ImportedFromElectron(_)));
        write_atomic(&own, &s).unwrap();

        let (again, origin) = load_from(&own, Some(&electron));
        assert_eq!(origin, Origin::Own);
        assert_eq!(again, s);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_corrupt_file_is_moved_aside_not_overwritten() {
        let dir = scratch("corrupt");
        let own = dir.join("settings.json");
        std::fs::write(&own, "{ not json").unwrap();
        let (s, origin) = load_from(&own, None);
        assert_eq!(s, Settings::default());
        let Origin::Recovered { backup, .. } = origin else { panic!("{origin:?}") };
        assert_eq!(std::fs::read_to_string(backup).unwrap(), "{ not json");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
