// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! The Settings dialog: a side-nav modal, like the Electron one, with a pane
//! per feature that exists here -- Inputs, Layout, Remote keyboard, Art-Net
//! lighting. A pane for something that does nothing would suggest it works.
//!
//! Everything edits `self.settings` in place. Saving is not done here: the
//! frame loop notices the change and writes it, debounced.

use super::*;
use crate::settings::{BORDER_WIDTH_RANGE, CENTER_GAP_RANGE, LOGO_WIDTH_RANGE};
use crate::artnet::target::{saver_select_value, set_saver_mode};
use crate::settings_status::{self as status, ArtnetCounts, InputState, Tone};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Inputs,
    Layout,
    RemoteKeyboard,
    Screensavers,
    Artnet,
}

/// index.html's target options. group:/strip: targets are site-specific and
/// stay file-only; one set by hand is kept as an extra entry rather than
/// coerced to the first option, which would silently repoint the room the
/// moment someone opened this pane to change something unrelated.
const ARTNET_TARGETS: &[(&str, &str)] = &[
    ("all", "All fixtures \u{2014} one flat colour"),
    ("spatial", "Spatial \u{2014} colours follow the wall"),
    ("effect:spot", "Spot \u{2014} follows the bright part of the wall"),
    ("effect:ripple", "Ripple"),
    ("effect:plasma", "Plasma"),
    ("effect:blobs", "Blobs"),
    ("effect:aurora", "Aurora"),
];

/// The target dropdown's entries for `current`: the declared ones, plus the
/// current value if it is not among them, labelled as from settings.json.
pub(super) fn artnet_target_options(current: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = ARTNET_TARGETS.iter().map(|(v, l)| ((*v).to_owned(), (*l).to_owned())).collect();
    if !out.iter().any(|(v, _)| v == current) {
        out.push((current.to_owned(), format!("{current} (set in settings.json)")));
    }
    out
}

/// The per-saver dropdown's entries: Reactive, Spatial, Off, the relay's
/// scenes and effects, and anything already configured that the relay did
/// not offer -- a saved mapping is never silently rewritten for want of a
/// network.
pub(super) fn artnet_saver_options(current: &str, configured: &[String], catalogue: Option<&crate::artnet::client::Catalogue>) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = vec![
        ("reactive".into(), "Reactive".into()),
        ("spatial".into(), "Spatial \u{2014} colours follow the wall".into()),
        ("off".into(), "Leave lights alone".into()),
    ];
    if let Some(c) = catalogue {
        out.extend(c.scenes.iter().map(|n| (format!("scene:{n}"), format!("Scene: {n}"))));
        out.extend(c.effects.iter().map(|n| (format!("effect:{n}"), format!("Effect: {n}"))));
    }
    for extra in std::iter::once(current).chain(configured.iter().map(String::as_str)) {
        if !out.iter().any(|(v, _)| v == extra) {
            out.push((extra.to_owned(), format!("{extra} (configured)")));
        }
    }
    out
}

const STILL: Vec2 = vec2(96.0, 54.0);
const NAV_WIDTH: f32 = 180.0;
const BODY_HEIGHT: f32 = 440.0;
const NAME_WIDTH: f32 = 280.0;

fn tone_color(tone: Tone) -> Color32 {
    match tone {
        Tone::Ok => Color32::from_rgb(80, 200, 120),
        Tone::Warn => Color32::from_rgb(245, 180, 60),
        Tone::Off => Color32::from_gray(110),
    }
}

/// A status dot, sized to sit on a text line.
fn dot(ui: &mut egui::Ui, tone: Tone) {
    let (r, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
    ui.painter().circle_filled(r.center(), 4.0, tone_color(tone));
}

impl InputViewer {
    fn input_state(&self, id: &str) -> InputState {
        let on_screen = self.visible_sides().iter().any(|&i| self.sides[i].as_deref() == Some(id));
        if !on_screen {
            return InputState::NotOnScreen;
        }
        let Some(slot) = self.slots.get(id) else { return InputState::Opening };
        let Some(stream) = &slot.stream else { return InputState::Reopening };
        match slot.health.status() {
            Status::Ok => match stream.latest() {
                Some(f) => InputState::Live { width: f.width(), height: f.height() },
                None => InputState::Opening,
            },
            Status::Idle | Status::Opening => InputState::Opening,
            Status::Dark => InputState::Dark,
            Status::NoFrames => InputState::Failed("No frames since it opened".into()),
            Status::Stalled => InputState::Failed("Frames stopped".into()),
            Status::Ended => InputState::Failed("The stream ended".into()),
            Status::OpenFailed => InputState::Failed(format!("Could not open: {}", slot.health.info().error.unwrap_or_default())),
        }
    }

    pub(super) fn settings_dialog(&mut self, ctx: &egui::Context, wall: Rect) {
        let Some(mut pane) = self.settings_pane else { return };
        let states: Vec<(bool, InputState)> = self
            .devices
            .iter()
            .map(|d| (self.settings.is_enabled(&d.id), self.input_state(&d.id)))
            .collect();
        let mut close = false;

        let modal = egui::Modal::new(egui::Id::new("settings")).show(ctx, |ui| {
            ui.set_width(820.0_f32.min(wall.width() - 64.0));
            ui.horizontal(|ui| {
                ui.heading("Settings");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    close = ui.button("Close").clicked();
                });
            });
            ui.separator();
            // Fixed height: inside a modal there is no parent to fill, so
            // anything asking for "all the height" -- a vertical separator,
            // say -- takes the whole window instead.
            // Fits a laptop-sized window as well as the wall: header and
            // footer take about 110pt between them.
            let body_height = BODY_HEIGHT.min(wall.height() - 140.0).max(200.0);
            let body = vec2(ui.available_width(), body_height);
            ui.allocate_ui_with_layout(body, egui::Layout::left_to_right(egui::Align::Min), |ui| {
                ui.allocate_ui_with_layout(vec2(NAV_WIDTH, body_height), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(NAV_WIDTH);
                    let (inputs_text, inputs_tone) = status::inputs_nav(&states);
                    nav_item(ui, &mut pane, Pane::Inputs, "Inputs", &inputs_text, inputs_tone);
                    let layout_text = match self.settings.layout() {
                        Layout::Dual => "Dual",
                        Layout::Single => "Single",
                    };
                    nav_item(ui, &mut pane, Pane::Layout, "Layout", layout_text, Tone::Off);
                    let (rk_text, rk_tone, _) = self.remote_status();
                    nav_item(ui, &mut pane, Pane::RemoteKeyboard, "Remote keyboard", rk_text, rk_tone);
                    let on = screensaver::NAMES.iter().filter(|n| self.settings.saver_enabled(n)).count();
                    let savers_text = format!("{on} of {}", screensaver::NAMES.len());
                    nav_item(ui, &mut pane, Pane::Screensavers, "Screensavers", &savers_text,
                        if on == 0 { Tone::Warn } else { Tone::Off });
                    let (an_text, an_tone, _) = self.artnet_status();
                    nav_item(ui, &mut pane, Pane::Artnet, "Art-Net lighting", an_text, an_tone);
                });
                let (line, _) = ui.allocate_exact_size(vec2(17.0, body_height), Sense::hover());
                ui.painter().vline(line.center().x, line.y_range(), ui.visuals().widgets.noninteractive.bg_stroke);
                let width = ui.available_width();
                ui.allocate_ui_with_layout(vec2(width, body_height), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(width);
                    match pane {
                        Pane::Inputs => self.inputs_pane(ui, &states),
                        Pane::Layout => self.layout_pane(ui, wall),
                        Pane::RemoteKeyboard => self.remote_keyboard_pane(ui),
                        Pane::Screensavers => self.screensavers_pane(ui),
                        Pane::Artnet => self.artnet_pane(ui),
                    }
                });
            });
            ui.separator();
            let saved = match self.store.path() {
                Some(p) => format!("Changes save automatically to {}", p.display()),
                None => "Test mode: changes apply now but are not saved.".into(),
            };
            ui.label(egui::RichText::new(saved).small().weak());
            // Updates are silent (D3); this is the one place they show.
            let version = format!("Version {}", env!("CARGO_PKG_VERSION"));
            let update = match self.updater.as_ref().map(crate::update::Updater::status) {
                None => format!("{version}. Not installed by the installer, so it does not update itself."),
                Some(crate::update::Status::Idle) => format!("{version}. Updates install by themselves."),
                Some(crate::update::Status::Downloading(v)) => format!("{version}. Downloading {v}…"),
                Some(crate::update::Status::Ready(v)) => format!("{version}. {v} is downloaded and installs when the app next restarts."),
            };
            ui.label(egui::RichText::new(update).small().weak());
        });

        self.settings_pane = if close || modal.should_close() { None } else { Some(pane) };
    }

    fn inputs_pane(&mut self, ui: &mut egui::Ui, states: &[(bool, InputState)]) {
        ui.label(egui::RichText::new("Inputs").strong());
        ui.label(egui::RichText::new(
            "Name an input to show that name instead of the device's. Disabled inputs are \
             left out of the pickers and the number keys.").small().weak());
        ui.add_space(6.0);

        if self.devices.is_empty() {
            ui.weak("No capture devices found.");
            return;
        }

        // Start-up input: "last used" is Electron's null defaultInputId.
        ui.horizontal(|ui| {
            ui.label("At start-up show");
            let mut last_used = self.settings.default_input_id.is_none();
            if ui.radio_value(&mut last_used, true, "the inputs last on screen").clicked() {
                self.settings.default_input_id = None;
            }
            ui.weak("or the one marked below");
        });
        ui.add_space(4.0);

        let devices = self.devices.clone();
        let mut enabled_index = 0;
        egui::ScrollArea::vertical().max_height(ui.available_height()).show(ui, |ui| {
            egui::Grid::new("settings-inputs").num_columns(6).spacing([12.0, 10.0]).show(ui, |ui| {
                ui.weak("Key");
                ui.weak("");
                ui.weak("Name");
                ui.weak("Enabled");
                ui.weak("Start-up");
                ui.weak("No-signal references");
                ui.end_row();

                for (d, (enabled, state)) in devices.iter().zip(states) {
                    let key = status::input_key_label(enabled.then_some(enabled_index));
                    if *enabled {
                        enabled_index += 1;
                    }
                    ui.label(egui::RichText::new(key).monospace().strong());

                    let (r, _) = ui.allocate_exact_size(STILL, Sense::hover());
                    match self.stills.get(&d.id) {
                        Some(t) => {
                            ui.painter().image(t.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                        }
                        None => {
                            ui.painter().rect_filled(r, 4.0, Color32::from_gray(30));
                        }
                    }

                    ui.vertical(|ui| {
                        // A grid column shrinks to its content's minimum,
                        // which for a text field is a few characters.
                        ui.set_min_width(NAME_WIDTH);
                        let mut name = self.settings.inputs.get(&d.id).and_then(|i| i.name.clone()).unwrap_or_default();
                        let edit = ui.add(egui::TextEdit::singleline(&mut name).hint_text(&d.label).desired_width(NAME_WIDTH));
                        if edit.changed() {
                            self.settings.input_mut(&d.id).name = (!name.is_empty()).then_some(name);
                        }
                        // Trimmed when the field is left, as Electron's change
                        // event does -- not per keystroke, which would eat the
                        // space while typing a two-word name.
                        if edit.lost_focus()
                            && let Some(n) = self.settings.input_mut(&d.id).name.as_mut()
                        {
                            *n = n.trim().to_owned();
                            if n.is_empty() {
                                self.settings.input_mut(&d.id).name = None;
                            }
                        }
                        let (tone, line) = status::input_status(*enabled, state);
                        ui.horizontal(|ui| {
                            dot(ui, tone);
                            let what = if self.settings.name_of(&d.id).is_some() {
                                format!("{} · {line}", d.label)
                            } else {
                                line
                            };
                            ui.label(egui::RichText::new(what).small().weak());
                        });
                    });

                    let mut on = *enabled;
                    if ui.checkbox(&mut on, "").changed() {
                        self.settings.input_mut(&d.id).enabled = on;
                    }

                    let is_default = self.settings.default_input_id.as_deref() == Some(d.id.as_str());
                    if ui.add_enabled(*enabled, egui::RadioButton::new(is_default, "")).clicked() {
                        self.settings.default_input_id = Some(d.id.clone());
                    }

                    self.references_cell(ui, &d.id);
                    ui.end_row();
                }
            });
        });
    }

    /// The input's no-signal references: how many, capture one, clear them.
    /// Capture is from whichever half shows the device, as in Electron.
    fn references_cell(&mut self, ui: &mut egui::Ui, id: &str) {
        let count = self.references.get(id).map_or(0, |r| r.len());
        let half = match (self.sides[0].as_deref() == Some(id), self.sides[1].as_deref() == Some(id)) {
            (true, _) => Some("left"),
            (false, true) => Some("right"),
            _ => None,
        };
        let has_frame = self.slots.get(id).and_then(|s| s.stream.as_ref()).and_then(Stream::latest).is_some();
        ui.horizontal(|ui| {
            ui.label(match count {
                0 => "None".to_owned(),
                1 => "1".to_owned(),
                n => n.to_string(),
            });
            let label = half.map_or_else(|| "Capture".to_owned(), |h| format!("Capture from {h} half"));
            let capture = ui.add_enabled(half.is_some() && has_frame, egui::Button::new(label))
                .on_hover_text("While the card shows its own no-signal screen: from then on, a picture that \
                    looks like this shows the split-flap board. A card has several such screens \
                    (no cable, unsupported mode, HDCP); capture each.")
                .on_disabled_hover_text("Put this input on a half first.");
            if capture.clicked() {
                self.capture_reference(id);
            }
            if count > 0 && ui.button("Clear").clicked() {
                self.clear_references(id);
            }
        });
    }

    fn screensavers_pane(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("Screensavers").strong());
        ui.label(egui::RichText::new(
            "After five minutes without a signal the wall rotates through the ticked savers, a new one \
             every ten minutes. With none ticked, the split-flap board stays up.").weak());
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui.button("All").clicked() {
                self.settings.disabled_screensavers.clear();
            }
            if ui.button("None").clicked() {
                self.settings.disabled_screensavers = screensaver::NAMES.iter().map(|n| (*n).to_owned()).collect();
            }
        });
        ui.add_space(6.0);
        let running = self.saver.as_ref().map(Screensaver::name);
        egui::ScrollArea::vertical().show(ui, |ui| {
            for name in screensaver::NAMES {
                ui.horizontal(|ui| {
                    let mut on = self.settings.saver_enabled(name);
                    if ui.checkbox(&mut on, *name).changed() {
                        self.settings.set_saver_enabled(name, on);
                    }
                    if running == Some(name) {
                        ui.label(egui::RichText::new("showing now").small().weak());
                    }
                });
            }
        });
    }

    fn remote_status(&self) -> (&'static str, Tone, String) {
        let s = &self.settings;
        status::remote_keyboard_status(s.remote_keyboard_enabled, &s.remote_keyboard_host,
            &s.remote_keyboard_api_key, self.remote_last.as_ref())
    }

    fn remote_keyboard_pane(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("Remote keyboard").strong());
        ui.label(egui::RichText::new(
            "Turns Left/PgUp and Right/PgDn on this machine into PageUp and PageDown on the presenter's \
             laptop, through the ESP32 keyboard plugged into it.").weak());
        ui.add_space(6.0);
        ui.checkbox(&mut self.settings.remote_keyboard_enabled, "Send presentation keys to the remote keyboard");
        ui.add_space(6.0);
        ui.add_enabled_ui(self.settings.remote_keyboard_enabled, |ui| {
            egui::Grid::new("settings-remote").num_columns(2).spacing([16.0, 10.0]).show(ui, |ui| {
                ui.label("Hostname");
                ui.add(egui::TextEdit::singleline(&mut self.settings.remote_keyboard_host)
                    .hint_text("space_keyboard").desired_width(280.0));
                ui.end_row();
                ui.label("API key");
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.settings.remote_keyboard_api_key)
                        .password(!self.show_api_key).desired_width(280.0));
                    if ui.button(if self.show_api_key { "Hide" } else { "Show" }).clicked() {
                        self.show_api_key = !self.show_api_key;
                    }
                });
                ui.end_row();
            });
            ui.label(egui::RichText::new("A name without a dot gets .local.").small().weak());
        });
        let (_, tone, line) = self.remote_status();
        if !line.is_empty() {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                dot(ui, tone);
                ui.label(line);
            });
        }
    }

    fn artnet_status(&self) -> (&'static str, Tone, String) {
        let shared = self.artnet.as_ref().map(crate::artnet::worker::Worker::shared);
        let counts = shared.as_ref().map(|s| ArtnetCounts {
            sent: s.status.sent,
            failures: s.status.failures,
            last_error: s.status.last_error.as_deref(),
        });
        status::artnet_status(self.settings.artnet_enabled, &self.settings.artnet_url, counts)
    }

    fn artnet_pane(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("Art-Net lighting").strong());
        ui.label(egui::RichText::new(
            "Drive the room lighting from the active screensaver. Nothing is sent until this is enabled \
             and a URL is given \u{2014} there is no default relay.").weak());
        ui.add_space(6.0);
        ui.checkbox(&mut self.settings.artnet_enabled, "Enable Art-Net");
        if !self.settings.artnet_enabled {
            return;
        }
        let shared = self.artnet.as_ref().map(crate::artnet::worker::Worker::shared);
        ui.add_space(6.0);
        egui::ScrollArea::vertical().max_height(ui.available_height()).auto_shrink([false, true]).show(ui, |ui| {
            let mut editing_url = false;
            egui::Grid::new("settings-artnet").num_columns(2).spacing([16.0, 10.0]).show(ui, |ui| {
                ui.label("Relay URL");
                let url = ui.add(egui::TextEdit::singleline(&mut self.settings.artnet_url)
                    .hint_text("https://spacelights.lab271.io").desired_width(320.0));
                editing_url = url.has_focus();
                if url.lost_focus() {
                    self.settings.artnet_url = self.settings.artnet_url.trim().to_owned();
                }
                ui.end_row();

                ui.label("");
                let (_, tone, line) = self.artnet_status();
                ui.horizontal(|ui| {
                    dot(ui, tone);
                    ui.label(line);
                });
                ui.end_row();

                ui.label("Target");
                let options = artnet_target_options(&self.settings.artnet_target);
                let shown = options.iter().find(|(v, _)| *v == self.settings.artnet_target).map_or("", |(_, l)| l.as_str()).to_owned();
                egui::ComboBox::from_id_salt("artnet-target").width(320.0).selected_text(shown).show_ui(ui, |ui| {
                    for (value, label) in &options {
                        ui.selectable_value(&mut self.settings.artnet_target, value.clone(), label);
                    }
                });
                ui.end_row();

                ui.label("Release scene");
                ui.add(egui::TextEdit::singleline(&mut self.settings.artnet_release_scene)
                    .hint_text("Leave empty to keep the last colour").desired_width(320.0));
                ui.end_row();

                ui.label("Max brightness");
                percent_slider(ui, &mut self.settings.artnet_max_brightness);
                ui.end_row();

                // Depth only means anything for the spot.
                if self.settings.artnet_target == "effect:spot" {
                    ui.label("Spot depth");
                    percent_slider(ui, &mut self.settings.artnet_spot_depth);
                    ui.end_row();
                }
            });

            if let Some(s) = shared.as_ref().map(|s| &s.status) {
                last_send(ui, s);
            }

            // Scene names come from the relay once per URL, when the field
            // is not being typed in -- Electron once asked on every keystroke.
            let url = self.settings.artnet_url.trim().to_owned();
            if !editing_url && status::is_relay_url(&url) && self.artnet_catalogue_for.as_deref() != Some(url.as_str())
                && let Some(worker) = &self.artnet
            {
                worker.send(crate::artnet::worker::Msg::ReadCatalogue);
                self.artnet_catalogue_for = Some(url);
            }

            ui.add_space(12.0);
            let map = &self.settings.artnet_scene_by_saver;
            let customised = screensaver::NAMES.iter().filter(|n| map.contains_key(**n)).count();
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Per screensaver").strong());
                ui.label(egui::RichText::new(if customised == 0 { "all default".to_owned() } else { format!("{customised} customised") }).small().weak());
            });
            ui.label(egui::RichText::new(
                "Each screensaver can drive the lights from its own picture (the default), hold a fixed scene, run an \
                 effect, or leave the room alone. When the signal comes back the room returns to however it was before \
                 the screensaver started.").small().weak());
            ui.add_space(4.0);
            let configured: Vec<String> = map.values().filter_map(|v| v.as_str())
                .filter(|v| v.starts_with("scene:") || v.starts_with("effect:")).map(str::to_owned).collect();
            let catalogue = shared.as_ref().and_then(|s| s.catalogue.as_ref());
            egui::Grid::new("settings-artnet-savers").num_columns(2).spacing([16.0, 6.0]).show(ui, |ui| {
                for saver in screensaver::NAMES {
                    let current = saver_select_value(saver, &self.settings.artnet_scene_by_saver);
                    let custom = self.settings.artnet_scene_by_saver.contains_key(*saver);
                    ui.label(if custom { egui::RichText::new(*saver).strong() } else { egui::RichText::new(*saver) });
                    let options = artnet_saver_options(&current, &configured, catalogue);
                    let shown = options.iter().find(|(v, _)| *v == current).map_or(current.clone(), |(_, l)| l.clone());
                    let mut picked = current.clone();
                    egui::ComboBox::from_id_salt(("artnet-saver", *saver)).width(260.0).selected_text(shown).show_ui(ui, |ui| {
                        for (value, label) in &options {
                            ui.selectable_value(&mut picked, value.clone(), label);
                        }
                    });
                    if picked != current {
                        set_saver_mode(&mut self.settings.artnet_scene_by_saver, saver, &picked);
                    }
                    ui.end_row();
                }
            });
        });
    }

    fn layout_pane(&mut self, ui: &mut egui::Ui, wall: Rect) {
        ui.label(egui::RichText::new("Layout").strong());
        ui.add_space(6.0);

        egui::Grid::new("settings-layout").num_columns(2).spacing([16.0, 10.0]).show(ui, |ui| {
            ui.label("View");
            ui.horizontal(|ui| {
                let mut layout = self.settings.layout();
                ui.selectable_value(&mut layout, Layout::Dual, "Dual");
                ui.selectable_value(&mut layout, Layout::Single, "Single");
                self.settings.layout_mode = Some(layout);
            });
            ui.end_row();

            ui.label("Multi-view");
            let mut multi = self.settings.multi_view;
            if ui.checkbox(&mut multi, "Each half can show a different input").changed() {
                self.set_multi_view(multi);
            }
            ui.end_row();

            ui.label("Motion");
            ui.checkbox(&mut self.settings.reduced_motion, "Reduce motion: switch inputs and views instantly");
            ui.end_row();

            ui.label("Centre gap");
            ui.add(egui::Slider::new(&mut self.settings.center_gap, CENTER_GAP_RANGE).step_by(1.0).suffix(" px"));
            ui.end_row();

            ui.label("Side borders");
            ui.add(egui::Slider::new(&mut self.settings.border_width, BORDER_WIDTH_RANGE).step_by(1.0).suffix(" px"));
            ui.end_row();

            ui.label("Logo");
            ui.horizontal(|ui| {
                let mut fit = self.settings.logo_width.is_none();
                if ui.checkbox(&mut fit, "Fit the centre gap").changed() {
                    // Unticking starts from the size it has now, not a jump.
                    self.settings.logo_width = if fit { None } else { Some((self.settings.center_gap * 0.9).max(*LOGO_WIDTH_RANGE.start())) };
                }
                if let Some(w) = &mut self.settings.logo_width {
                    ui.add(egui::Slider::new(w, LOGO_WIDTH_RANGE).step_by(1.0).logarithmic(true).suffix(" px"));
                }
            });
            ui.end_row();
        });

        ui.add_space(12.0);
        ui.label(egui::RichText::new("The wall behind this dialog follows as you drag; the diagram is the whole wall to scale.").small().weak());
        ui.add_space(4.0);
        self.layout_diagram(ui, wall);
    }

    /// The wall to scale, built from the same geometry the wall is painted
    /// with, so the two cannot disagree.
    fn layout_diagram(&self, ui: &mut egui::Ui, wall: Rect) {
        let width = ui.available_width().min(560.0);
        let size = vec2(width, width * wall.height() / wall.width().max(1.0));
        let (area, _) = ui.allocate_exact_size(size, Sense::hover());
        let s = area.width() / wall.width().max(1.0);
        let map = |r: Rect| Rect::from_min_max(area.min + (r.min - wall.min) * s, area.min + (r.max - wall.min) * s);
        let p = ui.painter();

        p.rect_filled(area, 2.0, Color32::BLACK);
        let inner = wall_rect(wall, self.settings.border_width);
        let mut logo_at = None;
        let halves: Vec<(usize, Rect)> = match self.settings.layout() {
            Layout::Single => vec![(0, inner)],
            Layout::Dual => {
                let (l, divider, r) = dual_rects(inner, self.settings.center_gap);
                logo_at = Some(logo_rect(divider, self.logo.as_ref().map_or(Vec2::ZERO, Logo::size), self.settings.logo_width));
                vec![(0, l), (1, r)]
            }
        };
        for (i, half) in halves {
            let r = map(half);
            p.rect_filled(r, 0.0, Color32::from_rgb(34, 38, 48));
            let text = self.label_of(self.sides[i].as_ref());
            p.text(r.center(), Align2::CENTER_CENTER, text, FontId::proportional(12.0), Color32::from_gray(200));
        }
        if let (Some(logo), Some(r)) = (&self.logo, logo_at) {
            let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
            p.image(logo.mipmapped.id(), map(r), uv, Color32::WHITE);
        }
        p.rect_stroke(area, 2.0, Stroke::new(1.0, Color32::from_gray(70)), StrokeKind::Outside);
    }
}

/// A 0..1 setting as a 0-100% slider, as Electron's range inputs.
fn percent_slider(ui: &mut egui::Ui, value: &mut f64) {
    let mut pct = (*value * 100.0).round();
    if ui.add(egui::Slider::new(&mut pct, 0.0..=100.0).step_by(1.0).suffix("%")).changed() {
        *value = pct / 100.0;
    }
}

/// What the last request was: a swatch of the colour, where it went, when.
fn last_send(ui: &mut egui::Ui, s: &crate::artnet::client::Status) {
    let Some((at, url, ok)) = &s.last_request else { return };
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        if let Some(c) = s.last_colour {
            let (r, _) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
            let k = c.brightness as f32;
            let scale = |v: u8| (f32::from(v) * k).round() as u8;
            ui.painter().rect_filled(r, 3.0, Color32::from_rgb(scale(c.rgb.r), scale(c.rgb.g), scale(c.rgb.b)));
        }
        let path = url.find("://").and_then(|i| url[i + 3..].find('/').map(|j| &url[i + 3 + j..])).unwrap_or(url);
        let what = match (&s.last_colour, s.active_mode) {
            (Some(c), "reactive" | "effect" | "spatial") if s.last_spot.is_none() && s.last_spatial.is_none() =>
                format!("rgb({}, {}, {}) at {:.0}%", c.rgb.r, c.rgb.g, c.rgb.b, c.brightness * 100.0),
            _ => s.active_saver.clone().map_or_else(String::new, |n| format!("for {n} ({})", s.active_mode)),
        };
        let verb = if *ok { "Sent" } else { "Failed" };
        ui.label(egui::RichText::new(format!("{verb} {what} to {path} at {}", at.format("%H:%M:%S"))).small().weak());
    });
}

fn nav_item(ui: &mut egui::Ui, current: &mut Pane, pane: Pane, label: &str, state: &str, tone: Tone) {
    let selected = *current == pane;
    let resp = ui
        .horizontal(|ui| {
            let r = ui.selectable_label(selected, label);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new(state).small().weak());
                dot(ui, tone);
            });
            r
        })
        .inner;
    if resp.clicked() {
        *current = pane;
    }
}
