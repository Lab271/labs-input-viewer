// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! Weather (#101), from `weather.js`: precipitation falls at the observed
//! rate, wind pushes it and leans the streaks, cloud cover sets the sky, the
//! palette follows day and night, and Schiphol sits on the horizon (#203).
//!
//! No networking here: `weather_source` polls for the app's lifetime and
//! this only reads the cache. With nothing cached the random rotation skips
//! the saver, but explicit selection still shows it, so it draws FALLBACK --
//! a calm half-clouded day, labelled NO DATA -- rather than a black frame.
//!
//! Two GPU passes into the shared HDR post chain (`shader_saver::render`):
//! the sky as one fullscreen shader, then instanced precipitation with
//! premultiplied blending. Both shaders are generated from `weather.js` by
//! `shaders/extract.mjs`. The readout line is drawn with egui's painter on
//! top rather than through a glyph atlas in the scene (ADR 0029, Adapt): it
//! is one short line, so it skips bloom and ACES, which only dimmed it a
//! little in Electron.

use std::sync::{Arc, Mutex};

use eframe::egui::{self, Align2, Color32, FontId, Rect, pos2};
use eframe::glow::{self, HasContext};

use crate::filelog::{log_error, log_info};
use crate::screensaver::Rng;
use crate::shader_saver::{self, Bloom};
use crate::weather_source::{self, Reading};

const SKY: &str = include_str!("../shaders/weather-sky.frag");
const DROP_VERT: &str = include_str!("../shaders/weather-drop.vert");
const DROP_FRAG: &str = include_str!("../shaders/weather-drop.frag");

/// What to animate with no reading: deliberately unremarkable, and not
/// passed off as an observation.
pub const FALLBACK: Reading = Reading {
    observed_at: None,
    temperature_c: 12.0,
    precipitation_mm_h: 0.0,
    wind_speed_kmh: 6.0,
    wind_direction_deg: 240.0,
    cloud_cover_pct: 45.0,
    weather_code: 1.0,
    is_day: true,
    age: std::time::Duration::ZERO,
    stale: false,
    synthetic: true,
};

/// Precipitation particles at 1080p, scaled by area and capped. Measured in
/// Electron: 900 read as a few scratches rather than rain.
const DROPS_1080P: f32 = 5200.0;
const MAX_DROPS: f32 = 26000.0;
/// mm/h drawn as "as heavy as it gets"; 8 is already a downpour.
const HEAVY_MM_H: f32 = 8.0;
/// Below this air temperature precipitation is snow.
const SNOW_BELOW_C: f64 = 1.0;
/// A new reading eases in over about this long rather than cutting.
const EASE_SECONDS: f32 = 25.0;
/// weather.js MAX_CHARS, the shader's uniform array; kept as the line cap.
const MAX_CHARS: usize = 56;

/// Bloom on the sun, the brightest cloud edges and the lightning; the sky
/// emits real HDR (the sun core is 5.0), so there is something above 1.0.
const BLOOM: Bloom = Bloom { threshold: 1.0, knee: 0.4, intensity: 0.5, radius: 0.9 };

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Clear,
    Cloud,
    Fog,
    Drizzle,
    Rain,
    Snow,
    Storm,
}

/// WMO weather code to a scene kind and a readout label. Grouped: the scene
/// only needs to know what to draw.
pub fn classify(code: f64) -> (Kind, &'static str) {
    // Math.round: halves go up.
    let c = (code + 0.5).floor() as i64;
    match c {
        0 => (Kind::Clear, "CLEAR"),
        1 => (Kind::Clear, "MAINLY CLEAR"),
        2 => (Kind::Cloud, "PARTLY CLOUDY"),
        3 => (Kind::Cloud, "OVERCAST"),
        45 | 48 => (Kind::Fog, "FOG"),
        51..=57 => (Kind::Drizzle, "DRIZZLE"),
        61..=67 => (Kind::Rain, "RAIN"),
        71..=77 => (Kind::Snow, "SNOW"),
        80..=82 => (Kind::Rain, "RAIN SHOWERS"),
        85 | 86 => (Kind::Snow, "SNOW SHOWERS"),
        95.. => (Kind::Storm, "THUNDERSTORM"),
        _ => (Kind::Cloud, "CLOUDY"),
    }
}

/// JS `toFixed(1)`, which rounds halves away from zero where Rust's `{:.1}`
/// rounds them to even.
fn fixed1(v: f64) -> String {
    let r = (v.abs() * 10.0 + 0.5).floor() / 10.0;
    format!("{}{r:.1}", if v < 0.0 && r != 0.0 { "-" } else { "" })
}

/// The readout line. The numbers are what make the scene self-evidently
/// live; the age is what tells stale data from calm weather. Kept to the
/// atlas's character set in Electron, so "21.7C" and "KMH", no degree sign.
pub fn readout_text(r: &Reading) -> String {
    let mut parts = vec![format!("{}C", fixed1(r.temperature_c)), classify(r.weather_code).1.to_owned()];
    if r.wind_speed_kmh >= 1.0 {
        parts.push(format!("WIND {} KMH", (r.wind_speed_kmh + 0.5).floor()));
    }
    if r.precipitation_mm_h >= 0.05 {
        parts.push(format!("{} MM-H", fixed1(r.precipitation_mm_h)));
    }
    if r.synthetic {
        parts.push("NO DATA".to_owned());
    } else if r.stale {
        let mins = (r.age.as_secs_f64() / 60.0 + 0.5).floor();
        parts.push(if mins >= 120.0 { format!("{} HOURS AGO", (mins / 60.0 + 0.5).floor()) } else { format!("{mins} MIN AGO") });
    }
    parts.join("   ")
}

/// Scene parameters derived from a reading; what the easing moves.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Targets {
    cloud: f32,
    wind: f32,
    day: f32,
    fog: f32,
    storm: f32,
    snow: f32,
    /// Normalised precipitation, 0..1.
    rate: f32,
    snow_ground: f32,
}

impl Targets {
    pub fn from(r: &Reading) -> Self {
        let (kind, _) = classify(r.weather_code);
        let is_snow = r.temperature_c < SNOW_BELOW_C || kind == Kind::Snow;
        // Drizzle and rain read as a rate even when the gauge says nearly
        // nothing; without a floor "DRIZZLE" renders as a clear sky.
        let wet_floor = match kind {
            Kind::Drizzle => 0.25,
            Kind::Rain => 0.4,
            _ => 0.0,
        };
        let rate = (r.precipitation_mm_h as f32).max(wet_floor);
        let flag = |b: bool| if b { 1.0 } else { 0.0 };
        Self {
            cloud: r.cloud_cover_pct as f32 / 100.0,
            wind: r.wind_speed_kmh as f32,
            day: flag(r.is_day),
            fog: if kind == Kind::Fog { 0.75 } else { 0.0 },
            storm: flag(kind == Kind::Storm),
            snow: flag(is_snow),
            // Snow is water equivalent, about a tenth the density of rain:
            // 4x, short of physical, because 10x whites out the frame.
            rate: (rate / HEAVY_MM_H * if is_snow { 4.0 } else { 1.0 }).min(1.0),
            // The ground goes white while snow falls or lies.
            snow_ground: if is_snow { (0.35 + rate).min(1.0) } else { 0.0 },
        }
    }

    /// Exponential, frame-rate independent ease toward `to`.
    fn ease(&mut self, to: &Self, dt: f32) {
        let k = 1.0 - (-dt / (EASE_SECONDS / 4.0)).exp();
        for (a, b) in [
            (&mut self.cloud, to.cloud), (&mut self.wind, to.wind), (&mut self.day, to.day),
            (&mut self.fog, to.fog), (&mut self.storm, to.storm), (&mut self.snow, to.snow),
            (&mut self.rate, to.rate), (&mut self.snow_ground, to.snow_ground),
        ] {
            *a += (b - *a) * k;
        }
    }
}

/// gl-base.js `pointScale`: sizes grow with the square root of area, and on
/// a large display nothing is drawn under 4 px.
pub fn point_scale(w: i32, h: i32, base_px: f32) -> f32 {
    let area = w.max(1) as f32 * h.max(1) as f32;
    let scale = (area / (1920.0 * 1080.0)).sqrt().clamp(1.0, 4.0);
    if area <= 1920.0 * 1080.0 * 1.5 {
        return scale;
    }
    scale.max((4.0 / base_px).min(4.0))
}

/// gl-base.js `luminanceScale`: 1.6 on a projector-sized canvas, where room
/// light lifts the black floor; 1 on anything desk-sized.
pub fn luminance_scale(w: i32, h: i32) -> f32 {
    if w.max(1) as f32 * h.max(1) as f32 > 1920.0 * 1080.0 * 3.0 { 1.6 } else { 1.0 }
}

/// One activation.
pub struct Weather {
    rng: Rng,
    /// Per-activation variation; everything meteorological comes from the
    /// reading, never the RNG.
    cloud_offset: f32,
    drop_seed: f32,
    eased: Option<Targets>,
    flash: f32,
    next_flash_at: f32,
}

/// What one frame's GPU passes need, copied into the paint callback.
#[derive(Clone, Copy)]
struct Frame {
    time: f32,
    sky_time: f32,
    eased: Targets,
    flash: f32,
    runway_dir: f32,
    wind_direction_deg: f32,
    drop_seed: f32,
}

impl Weather {
    pub fn new(seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let cloud_offset = rng.range(0.0, 1000.0);
        let drop_seed = rng.f32();
        Self { rng, cloud_offset, drop_seed, eased: None, flash: 0.0, next_flash_at: 0.0 }
    }

    pub fn paint(&mut self, painter: &egui::Painter, rect: Rect, time: f32, dt: f32) {
        let r = weather_source::current_reading().unwrap_or(FALLBACK);
        let target = Targets::from(&r);
        let eased = self.eased.get_or_insert(target);
        eased.ease(&target, dt);
        let eased = *eased;

        // Lightning only in a storm, at irregular intervals from the seeded
        // RNG, so a logged seed replays the same strikes.
        if eased.storm > 0.35 && time > self.next_flash_at {
            self.flash = 1.0;
            self.next_flash_at = time + self.rng.range(2.5, 9.5);
        }
        self.flash *= (-dt * 7.0).exp();

        let wd = r.wind_direction_deg.rem_euclid(360.0);
        let frame = Frame {
            time,
            sky_time: time + self.cloud_offset,
            eased,
            flash: self.flash,
            // Schiphol swaps runway ends with the wind.
            runway_dir: if wd > 90.0 && wd < 270.0 { 1.0 } else { -1.0 },
            wind_direction_deg: r.wind_direction_deg as f32,
            drop_seed: self.drop_seed,
        };
        let cb = eframe::egui_glow::CallbackFn::new(move |info, p| {
            let v = info.viewport_in_pixels();
            draw(p.gl(), [v.left_px, v.from_bottom_px, v.width_px, v.height_px], frame);
        });
        painter.add(egui::PaintCallback { rect, callback: Arc::new(cb) });
        paint_readout(painter, rect, &readout_text(&r));
    }
}

/// The readout, bottom-left, inset by a cell: instrumentation rather than a
/// caption. One square cell per character, as the atlas laid them out, on a
/// dark plate so it stays legible over a night sky and over lying snow.
fn paint_readout(painter: &egui::Painter, rect: Rect, line: &str) {
    let ppp = painter.ctx().pixels_per_point();
    let cell = (rect.height() * ppp * 0.030).round().max(14.0) / ppp;
    let chars: Vec<char> = line.chars().take(MAX_CHARS).collect();
    let bottom = rect.bottom() - cell * 0.9;
    let left = rect.left() + cell * 1.2;
    let plate = Rect::from_min_max(pos2(left, bottom - cell * 0.94), pos2(left + chars.len() as f32 * cell, bottom - cell * 0.06));
    painter.rect_filled(plate, 0.0, Color32::from_black_alpha(119));
    let font = FontId::monospace(cell * 0.82);
    let ink = Color32::from_rgba_unmultiplied(240, 245, 255, 217);
    for (i, ch) in chars.iter().enumerate() {
        let at = pos2(left + (i as f32 + 0.5) * cell, bottom - cell + cell * 0.54);
        painter.text(at, Align2::CENTER_CENTER, ch, font.clone(), ink);
    }
}

struct Gpu {
    sky: glow::Program,
    drop: glow::Program,
    quads: glow::VertexArray,
}

enum State {
    Uninit,
    Ready(Gpu),
    Failed,
}

static GPU: Mutex<State> = Mutex::new(State::Uninit);

/// Whether the saver cannot run here, so the caller falls back to the DVD
/// logo as registry.js does.
pub fn failed() -> bool {
    matches!(*GPU.lock().unwrap(), State::Failed) || shader_saver::post_failed()
}

unsafe fn setup(gl: &glow::Context) -> Result<Gpu, String> {
    unsafe {
        let started = std::time::Instant::now();
        let sky = shader_saver::compile(gl, shader_saver::VERTEX, SKY).map_err(|e| format!("sky: {e}"))?;
        let drop = shader_saver::compile(gl, DROP_VERT, DROP_FRAG).map_err(|e| format!("precipitation: {e}"))?;
        // glsl-lib.js createInstancedQuads: a unit quad as two triangles,
        // `aCorner` per vertex, instanced by gl_InstanceID.
        let corners: [f32; 12] = [-0.5, -0.5, 0.5, -0.5, -0.5, 0.5, 0.5, -0.5, 0.5, 0.5, -0.5, 0.5];
        let quads = gl.create_vertex_array()?;
        let vbo = gl.create_buffer()?;
        gl.bind_vertex_array(Some(quads));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytemuck::cast_slice(&corners), glow::STATIC_DRAW);
        if let Some(loc) = gl.get_attrib_location(drop, "aCorner") {
            gl.enable_vertex_attrib_array(loc);
            gl.vertex_attrib_pointer_f32(loc, 2, glow::FLOAT, false, 0, 0);
        }
        gl.bind_vertex_array(None);
        gl.bind_buffer(glow::ARRAY_BUFFER, None);
        log_info!("[Screensaver] Weather compiled in {} ms", started.elapsed().as_millis());
        Ok(Gpu { sky, drop, quads })
    }
}

fn draw(gl: &glow::Context, viewport: [i32; 4], f: Frame) {
    shader_saver::render(gl, viewport, &BLOOM, 1.0, |gl, w, h| {
        let mut state = GPU.lock().unwrap();
        if let State::Uninit = *state {
            *state = match unsafe { setup(gl) } {
                Ok(g) => State::Ready(g),
                Err(e) => {
                    log_error!("[Screensaver] Weather failed to compile, falling back: {e}");
                    State::Failed
                }
            };
        }
        let State::Ready(gpu) = &*state else { return false };
        unsafe {
            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
            draw_sky(gl, gpu.sky, w, h, &f);
            draw_precipitation(gl, gpu, w, h, &f);
        }
        true
    });
}

unsafe fn draw_sky(gl: &glow::Context, prog: glow::Program, w: i32, h: i32, f: &Frame) {
    unsafe {
        gl.use_program(Some(prog));
        let u = |name: &str| gl.get_uniform_location(prog, name);
        let e = &f.eased;
        gl.uniform_3_f32(u("iResolution").as_ref(), w as f32, h as f32, 1.0);
        gl.uniform_1_f32(u("iTime").as_ref(), f.sky_time);
        for (name, v) in [
            ("uCloud", e.cloud), ("uWind", e.wind), ("uDay", e.day), ("uFog", e.fog),
            ("uStorm", e.storm), ("uSnowGround", e.snow_ground), ("uFlash", f.flash),
            ("uRunwayDir", f.runway_dir),
            // Already normalised by Targets::from; dividing again would
            // zero it for anything short of a downpour.
            ("uPrecip", e.rate),
            ("uLum", luminance_scale(w, h)),
        ] {
            gl.uniform_1_f32(u(name).as_ref(), v);
        }
        gl.draw_arrays(glow::TRIANGLES, 0, 3);
    }
}

/// The drop count for a canvas at a normalised rate: more drops on the wall,
/// not bigger ones (#88).
pub fn drop_count(w: i32, h: i32, rate: f32) -> i32 {
    let area = w as f32 * h as f32 / (1920.0 * 1080.0);
    let max = MAX_DROPS.min((DROPS_1080P * area).round());
    (max * rate).round() as i32
}

unsafe fn draw_precipitation(gl: &glow::Context, gpu: &Gpu, w: i32, h: i32, f: &Frame) {
    let e = &f.eased;
    let count = drop_count(w, h, e.rate);
    if count <= 0 {
        return;
    }
    // The lean is defined in screen space (dx/dy in px) and the normalised
    // drift derived from it, so a wind leans the rain the same on the 5:1
    // wall as in a window. Direction is meteorological "from".
    let slope = -f.wind_direction_deg.to_radians().sin() * (e.wind.min(60.0) / 60.0) * 1.5;
    let shear = slope / (w as f32 / h.max(1) as f32);
    let px = point_scale(w, h, 1.0);
    unsafe {
        gl.enable(glow::BLEND);
        // Premultiplied: the shader already scales colour by coverage.
        gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);
        gl.use_program(Some(gpu.drop));
        let u = |name: &str| gl.get_uniform_location(gpu.drop, name);
        gl.uniform_2_f32(u("uQuadScale").as_ref(), 2.0 / w as f32, 2.0 / h as f32);
        for (name, v) in [
            ("uTime", f.time),
            ("uCount", count as f32),
            ("uFall", 0.5 + 0.9 * e.rate),
            ("uShear", shear),
            ("uSlope", slope),
            ("uLength", px * (14.0 + 26.0 * e.rate)),
            ("uWidth", px * 1.7),
            ("uSnow", if e.snow > 0.5 { 1.0 } else { 0.0 }),
            ("uSeed", f.drop_seed),
            ("uLum", luminance_scale(w, h)),
            ("uNight", 1.0 - e.day),
        ] {
            gl.uniform_1_f32(u(name).as_ref(), v);
        }
        gl.bind_vertex_array(Some(gpu.quads));
        gl.draw_arrays_instanced(glow::TRIANGLES, 0, 6, count);
        gl.disable(glow::BLEND);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::weather_states::STATES;

    const READING: Reading = Reading {
        temperature_c: 12.0,
        precipitation_mm_h: 0.0,
        wind_speed_kmh: 5.0,
        wind_direction_deg: 0.0,
        cloud_cover_pct: 50.0,
        weather_code: 1.0,
        is_day: true,
        synthetic: false,
        ..FALLBACK
    };

    #[test]
    fn every_wmo_code_maps_to_a_kind_with_a_label() {
        for code in 0..=99 {
            assert!(!classify(f64::from(code)).1.is_empty(), "code {code}");
        }
    }

    #[test]
    fn the_codes_that_change_the_scene() {
        assert_eq!(classify(0.0).0, Kind::Clear);
        assert_eq!(classify(3.0).0, Kind::Cloud);
        assert_eq!(classify(45.0).0, Kind::Fog);
        assert_eq!(classify(65.0).0, Kind::Rain);
        assert_eq!(classify(73.0).0, Kind::Snow);
        assert_eq!(classify(95.0).0, Kind::Storm);
        assert_eq!(classify(99.0).0, Kind::Storm);
    }

    #[test]
    fn the_readout_stays_in_the_atlas_set_and_the_uniform_array() {
        const CHARSET: &str = " ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.-:";
        for state in STATES.iter().map(|s| &s.reading).chain([&FALLBACK]) {
            let line = readout_text(state);
            assert!(line.chars().all(|c| CHARSET.contains(c)), "{line:?}");
            assert!(line.len() <= MAX_CHARS, "{line:?}");
        }
    }

    #[test]
    fn shows_the_age_when_stale() {
        let r = Reading { age: std::time::Duration::from_secs(42 * 60), stale: true, ..READING };
        assert!(readout_text(&r).contains("42 MIN AGO"));
        let r = Reading { age: std::time::Duration::from_secs(3 * 3600), stale: true, ..READING };
        assert!(readout_text(&r).contains("3 HOURS AGO"));
    }

    #[test]
    fn says_no_data_plainly() {
        assert!(readout_text(&Reading { synthetic: true, ..READING }).contains("NO DATA"));
    }

    #[test]
    fn reports_temperature_and_condition() {
        let line = readout_text(&Reading { temperature_c: 21.7, weather_code: 61.0, ..READING });
        assert!(line.contains("21.7C") && line.contains("RAIN"), "{line}");
        assert!(!line.contains("AGO") && !line.contains("NO DATA"), "{line}");
        assert_eq!(readout_text(&STATES[5].reading), "-2.5C   SNOW   WIND 5 KMH   1.4 MM-H");
    }

    #[test]
    fn omits_wind_and_rain_when_there_is_none() {
        let line = readout_text(&Reading { wind_speed_kmh: 0.0, precipitation_mm_h: 0.0, weather_code: 0.0, ..READING });
        assert!(!line.contains("WIND") && !line.contains("MM-H"), "{line}");
    }

    #[test]
    fn rounds_halves_up_like_to_fixed() {
        assert_eq!(fixed1(0.25), "0.3");
        assert_eq!(fixed1(-2.5), "-2.5");
    }

    #[test]
    fn drizzle_gets_a_floor_and_snow_counts_four_times() {
        let drizzle = Targets::from(&Reading { weather_code: 53.0, ..READING });
        assert!((drizzle.rate - 0.25 / HEAVY_MM_H).abs() < 1e-6);
        let snow = Targets::from(&STATES[5].reading);
        assert!((snow.rate - 1.4 / HEAVY_MM_H * 4.0).abs() < 1e-6);
        assert_eq!(snow.snow, 1.0);
        assert!(snow.snow_ground > 0.35);
    }

    #[test]
    fn easing_closes_most_of_the_gap_in_25s() {
        let mut eased = Targets::from(&STATES[0].reading);
        let to = Targets::from(&STATES[3].reading);
        for _ in 0..(25 * 60) {
            eased.ease(&to, 1.0 / 60.0);
        }
        assert!((eased.cloud - to.cloud).abs() < 0.02 * (to.cloud - 0.04).abs());
    }

    #[test]
    fn scales_like_gl_base() {
        assert_eq!(point_scale(1920, 1080, 1.0), 1.0);
        assert_eq!(point_scale(6000, 1200, 1.0), 4.0);
        assert!((point_scale(6000, 1200, 4.0) - (6000.0 * 1200.0 / (1920.0 * 1080.0f32)).sqrt()).abs() < 1e-5);
        assert_eq!(luminance_scale(2560, 1440), 1.0);
        assert_eq!(luminance_scale(6000, 1200), 1.6);
    }

    #[test]
    fn more_drops_on_the_wall_up_to_the_cap() {
        assert_eq!(drop_count(1920, 1080, 1.0), 5200);
        assert_eq!(drop_count(6000, 1200, 1.0), 18056);
        assert_eq!(drop_count(7680, 4320, 1.0), 26000);
        assert_eq!(drop_count(1920, 1080, 0.0), 0);
    }

    #[test]
    fn shaders_are_complete_desktop_programs() {
        for (name, src) in [("sky", SKY), ("drop.vert", DROP_VERT), ("drop.frag", DROP_FRAG)] {
            assert!(src.lines().nth(1) == Some("#version 330 core"), "{name}");
            assert!(src.contains("void main()"), "{name}");
        }
    }
}
