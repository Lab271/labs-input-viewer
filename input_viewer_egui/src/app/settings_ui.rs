// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! The Settings dialog: a side-nav modal, like the Electron one, with the two
//! panes whose features exist here -- Inputs and Layout. Remote keyboard and
//! Art-Net come with those features; a pane for something that does nothing
//! would suggest it works.
//!
//! Everything edits `self.settings` in place. Saving is not done here: the
//! frame loop notices the change and writes it, debounced.

use super::*;
use crate::settings::{BORDER_WIDTH_RANGE, CENTER_GAP_RANGE};
use crate::settings_status::{self as status, InputState, Tone};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Inputs,
    Layout,
    RemoteKeyboard,
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
                    ui.add_space(16.0);
                    ui.label(egui::RichText::new("Art-Net is not in this prototype yet.").small().weak());
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
                    }
                });
            });
            ui.separator();
            let saved = match self.store.path() {
                Some(p) => format!("Changes save automatically to {}", p.display()),
                None => "Test mode: changes apply now but are not saved.".into(),
            };
            ui.label(egui::RichText::new(saved).small().weak());
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
            egui::Grid::new("settings-inputs").num_columns(5).spacing([12.0, 10.0]).show(ui, |ui| {
                ui.weak("Key");
                ui.weak("");
                ui.weak("Name");
                ui.weak("Enabled");
                ui.weak("Start-up");
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
                    ui.end_row();
                }
            });
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
        let halves: Vec<(usize, Rect)> = match self.settings.layout() {
            Layout::Single => vec![(0, inner)],
            Layout::Dual => {
                let (l, divider, r) = dual_rects(inner, self.settings.center_gap);
                if let Some(logo) = &self.logo {
                    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
                    p.image(logo.mipmapped.id(), map(logo_rect(divider, logo.size())), uv, Color32::WHITE);
                }
                vec![(0, l), (1, r)]
            }
        };
        for (i, half) in halves {
            let r = map(half);
            p.rect_filled(r, 0.0, Color32::from_rgb(34, 38, 48));
            let text = self.label_of(self.sides[i].as_ref());
            p.text(r.center(), Align2::CENTER_CENTER, text, FontId::proportional(12.0), Color32::from_gray(200));
        }
        p.rect_stroke(area, 2.0, Stroke::new(1.0, Color32::from_gray(70)), StrokeKind::Outside);
    }
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
