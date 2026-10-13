// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! The wall: one or two halves, the pickers laid over the top edge, the legend
//! on the bottom edge, and the no-signal board / screensaver.

use std::collections::HashMap;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Key, Rect, Sense, Stroke, StrokeKind, TextureHandle,
    TextureOptions, Vec2, pos2, vec2,
};
use crate::capture::{self, Device, Stream};
use crate::gpu_video;
use crate::flags::Flags;
use crate::health::{self, Sample, Status, StreamHealth, limits as health_limits};
use crate::render_stats::RenderStats;
use crate::logo::Logo;
use crate::remote_key;
use crate::review::Review;
use crate::transition::{self, Dip};
use crate::screensaver::{self, Rng, Screensaver};
use crate::settings::{Layout, Settings, Store};
use crate::shortcuts::{self, Action};
use crate::update::{self, Updater};

mod settings_ui;
use settings_ui::Pane;
use crate::filelog::{log_error, log_info, log_warn};

const DROPDOWN_IDLE: Duration = Duration::from_secs(30);
const EDGE_PX: f32 = 6.0;
const CLOSE_MARGIN: f32 = 48.0;
const TOAST_FOR: Duration = Duration::from_secs(2);
/// The cursor hides this long after the pointer last moved (ADR 0038).
const CURSOR_IDLE: Duration = Duration::from_secs(3);
/// The green LIVE badge after unfreezing.
const LIVE_FOR: Duration = Duration::from_secs(1);
const FROZEN_RED: Color32 = Color32::from_rgb(200, 30, 30);
const LIVE_GREEN: Color32 = Color32::from_rgb(20, 150, 60);
const TILE: Vec2 = vec2(128.0, 72.0);
const DEVICE_POLL: Duration = Duration::from_secs(10);
/// Video is almost always drawn smaller than it arrives (a 3840x2160 card on a
/// ~2133x1200 half of the wall is 1.8x down), and plain bilinear filtering
/// samples only 2x2 texels: past 2x it skips source pixels, and 1px detail --
/// slide text, table rules -- turns to moiré and shimmers. Trilinear
/// mipmapping averages instead. Measured with `--test-pattern --hud`: it
/// fixes the moiré and keeps two 4K60 inputs at 60 fps with no skipped frames.
const VIDEO_TEXTURE: TextureOptions = TextureOptions::LINEAR.with_mipmap_mode(Some(egui::TextureFilter::Linear));

/// Thumbnail the health check judges flatness on: the size Electron's crop
/// detection shares with it (#314), so a port of cropping can reuse it.
const HEALTH_THUMB: (usize, usize) = (crate::crop::SAMPLE_W, crate::crop::SAMPLE_H);
/// A `[Health]` line per side every 10 minutes, as renderer.js writes.
const HEALTH_SUMMARY_MS: f64 = 10.0 * 60.0 * 1000.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Left,
    Right,
    Both,
}

/// Which input each half starts on, by Electron's rules (`getVideoDevices` and
/// `openInitialStreams`):
///
/// 1. keep what a half already shows if that device is still there (hot-plug);
/// 2. else the saved left/right device;
/// 3. else the first enabled device on the left, the second on the right
///    (or the first again if there is only one);
/// 4. at startup only, `defaultInputId` overrides both halves if it is
///    present and enabled;
/// 5. with Multi-view off the right half always copies the left.
fn choose_sides(devices: &[Device], s: &Settings, current: &[Option<String>; 2], startup: bool) -> [Option<String>; 2] {
    let present = |id: &Option<String>| id.as_ref().filter(|id| devices.iter().any(|d| &d.id == *id)).cloned();
    let mut enabled = devices.iter().filter(|d| s.is_enabled(&d.id)).map(|d| d.id.clone());
    let first = enabled.next().or_else(|| devices.first().map(|d| d.id.clone()));
    let second = enabled.next().or_else(|| first.clone());

    let mut left = present(&current[0]).or_else(|| present(&s.left_device_id)).or_else(|| first.clone());
    let mut right = present(&current[1]).or_else(|| present(&s.right_device_id)).or(second);
    if startup
        && let Some(default) = present(&s.default_input_id).filter(|id| s.is_enabled(id))
    {
        left = Some(default.clone());
        right = Some(default);
    }
    if !s.multi_view {
        right = left.clone();
    }
    [left, right]
}

/// Dual view: left half, logo divider, right half. The halves share what the
/// gap leaves, like the flex layout around `#center-divider`.
pub(crate) fn dual_rects(rect: Rect, gap: f32) -> (Rect, Rect, Rect) {
    let gap = gap.clamp(0.0, rect.width());
    let half = (rect.width() - gap) / 2.0;
    let left = Rect::from_min_size(rect.min, vec2(half, rect.height()));
    let divider = Rect::from_min_size(pos2(left.right(), rect.top()), vec2(gap, rect.height()));
    let right = Rect::from_min_max(pos2(divider.right(), rect.top()), rect.max);
    (left, divider, right)
}

/// The content area once `borderWidth` is taken off each side edge.
fn wall_rect(rect: Rect, border: f32) -> Rect {
    let b = border.clamp(0.0, rect.width() / 2.0);
    Rect::from_min_max(rect.min + vec2(b, 0.0), rect.max - vec2(b, 0.0))
}

/// `.divider-logo`: contained within 90% of the column's width and 80% of its
/// height, centred. With `width` set (`logoWidth`), that width instead,
/// still capped at 80% of the height.
fn logo_rect(divider: Rect, logo: Vec2, width: Option<f32>) -> Rect {
    let w = width.unwrap_or(divider.width() * 0.9);
    let scale = (w / logo.x).min(divider.height() * 0.8 / logo.y);
    Rect::from_center_size(divider.center(), logo * scale)
}


/// `pickerPlan()` from the Electron app: which pickers the dropdown shows and
/// whether their tiles carry number-key chips.
///
/// Per-half pickers carry no chip on purpose: `1`-`4` set both halves, and a
/// chip beside a control that sets one half would document the wrong thing.
fn picker_plan(layout: Layout, multi_view: bool) -> (Vec<(&'static str, Side)>, bool) {
    match (layout, multi_view) {
        (Layout::Dual, true) => (vec![("Left half", Side::Left), ("Right half", Side::Right)], false),
        (Layout::Dual, false) => (vec![("Both halves", Side::Both)], true),
        (Layout::Single, true) => (vec![("Whole wall", Side::Left)], true),
        (Layout::Single, false) => (vec![("Whole wall", Side::Both)], true),
    }
}

/// The picture a slot draws: an egui texture for RGBA frames, or a GPU
/// texture that `gpu_video` converts YUY2 frames into.
#[derive(Clone)]
enum VideoTexture {
    Egui(TextureHandle),
    Gpu { id: egui::TextureId, size: Vec2 },
}

impl VideoTexture {
    fn id(&self) -> egui::TextureId {
        match self {
            VideoTexture::Egui(t) => t.id(),
            VideoTexture::Gpu { id, .. } => *id,
        }
    }
    fn size_vec2(&self) -> Vec2 {
        match self {
            VideoTexture::Egui(t) => t.size_vec2(),
            VideoTexture::Gpu { size, .. } => *size,
        }
    }
}

/// GPU conversion state: compiled once, on the first YUY2 frame.
enum Gpu {
    NotYet,
    Ready(gpu_video::Converter),
    /// Could not set up; YUY2 is converted on the CPU instead (slowly).
    Failed,
}

/// One open capture device, shared by every half that shows it (ADR 0008).
struct Slot {
    /// None between closing the device for a reopen and opening it again.
    stream: Option<Stream>,
    /// When to open again after a reopen's release gap (app clock, ms).
    reopen_at: Option<f64>,
    texture: Option<VideoTexture>,
    shown_seq: u64,
    /// Outlives the stream across reopens, so the backoff backs off.
    health: StreamHealth,
    next_sample: f64,
    stats: RenderStats,
    /// Black-bar crop for this device's picture (ADR 0015).
    crop: crate::crop::CropTracker,
    /// The status last written to the log, as Electron's healthLastStatus.
    last_logged: Option<Status>,
    /// The "opened" line for this stream has been written.
    announced: bool,
}

impl Slot {
    fn open(device: &Device, ctx: &egui::Context, now: f64) -> Self {
        let mut slot = Self {
            stream: None,
            reopen_at: None,
            texture: None,
            shown_seq: 0,
            health: StreamHealth::new(),
            next_sample: 0.0,
            stats: RenderStats::new(),
            crop: crate::crop::CropTracker::default(),
            last_logged: None,
            announced: false,
        };
        slot.attach(device, ctx, now);
        slot
    }

    fn attach(&mut self, device: &Device, ctx: &egui::Context, now: f64) {
        self.stream = Some(Stream::open(device, ctx.clone()));
        self.reopen_at = None;
        self.shown_seq = 0;
        self.stats = RenderStats::new();
        self.health.opened(now);
        self.next_sample = now + health_limits::SAMPLE_MS;
        self.announced = false;
        // A new stream may be a different source in a different shape.
        self.crop.reset();
    }
}

pub struct InputViewer {
    flags: Flags,
    settings: Settings,
    store: Store,
    /// None while closed; which pane while open.
    settings_pane: Option<Pane>,
    devices: Vec<Device>,
    /// The first device list has been applied (defaultInputId is startup-only).
    started: bool,
    device_rx: Option<mpsc::Receiver<Vec<Device>>>,
    next_device_poll: Instant,
    /// Device id shown on [left, right].
    sides: [Option<String>; 2],
    slots: HashMap<String, Slot>,
    /// Last frame seen per device, for the picker tiles. A snapshot, not a
    /// live preview, so opening the dropdown opens no extra streams.
    stills: HashMap<String, VideoTexture>,
    gpu: Gpu,
    /// Per device, kept across reopens so egui registrations are not leaked.
    gpu_textures: HashMap<String, gpu_video::GpuTexture>,
    /// Per side: the dip to black of an input switch (ADR 0037).
    dips: [Dip; 2],
    /// Where the layout is between single (0) and dual (1); moves toward
    /// the setting, so a change mid-way turns around where it is.
    layout_t: f32,
    /// Started on the first press, so a wall without a remote keyboard
    /// never runs the thread.
    remote: Option<remote_key::Sender>,
    remote_last: Option<remote_key::Last>,
    /// Settings: the API key is shown in clear.
    show_api_key: bool,
    frozen: bool,
    /// Until when the LIVE badge shows, after an unfreeze.
    live_until: Option<Instant>,
    /// The pointer last moved; the cursor hides CURSOR_IDLE after it.
    last_pointer_move: Instant,
    dropdown_open: bool,
    dropdown_bottom: f32,
    legend_open: bool,
    legend_top: f32,
    last_activity: Instant,
    toast: Option<(String, Instant)>,
    board_since: [Option<Instant>; 2],
    wall_dark_since: Option<Instant>,
    saver: Option<Screensaver>,
    saver_on_demand: bool,
    saver_started: Instant,
    last_saver: Option<usize>,
    rng: Rng,
    last_frame: Instant,
    review: Option<Review>,
    updater: Option<Updater>,
    logo: Option<Logo>,
    /// --hud: repaints per second, and when the count started.
    ui_fps: f32,
    ui_frames: (u32, Instant),
    report_count: u64,
    /// Origin of the millisecond clock the health policy runs on.
    epoch: Instant,
    /// When the last 10-minute `[Health]` summary was written (ms).
    last_health_summary: f64,
}

impl InputViewer {
    pub fn new(cc: &eframe::CreationContext<'_>, flags: Flags) -> Self {
        // Always dark: this runs on a video wall, and following a light system
        // theme would put white panels over the picture.
        cc.egui_ctx.set_theme(egui::Theme::Dark);
        let review = Review::from_env();
        // Mock and review runs neither read nor write settings.json: they are
        // for looking at the UI, and must not be able to change the wall's
        // configuration.
        let (settings, store) = Store::open(flags.mock.is_none() && review.is_none());
        let layout_t = if settings.layout() == Layout::Dual { 1.0 } else { 0.0 };
        let mut app = Self {
            settings,
            store,
            settings_pane: None,
            devices: Vec::new(),
            started: false,
            device_rx: None,
            next_device_poll: Instant::now() + DEVICE_POLL,
            sides: [None, None],
            slots: HashMap::new(),
            stills: HashMap::new(),
            gpu: Gpu::NotYet,
            gpu_textures: HashMap::new(),
            dips: Default::default(),
            layout_t,
            remote: None,
            remote_last: None,
            show_api_key: false,
            frozen: false,
            live_until: None,
            last_pointer_move: Instant::now(),
            dropdown_open: false,
            dropdown_bottom: 0.0,
            legend_open: false,
            legend_top: f32::MAX,
            last_activity: Instant::now(),
            toast: None,
            board_since: [None, None],
            wall_dark_since: None,
            saver: None,
            saver_on_demand: false,
            saver_started: Instant::now(),
            last_saver: None,
            rng: Rng::new(Rng::clock_seed()),
            last_frame: Instant::now(),
            review,
            // Never in mock or review runs: those are for looking at the UI.
            updater: None,
            logo: Logo::load(&cc.egui_ctx),
            ui_fps: 0.0,
            ui_frames: (0, Instant::now()),
            report_count: 0,
            epoch: Instant::now(),
            last_health_summary: 0.0,
            flags,
        };
        if app.flags.mock.is_none() && app.review.is_none() {
            app.updater = Updater::start(cc.egui_ctx.clone());
        }
        // Electron wrote gpu-report.txt to answer one recurring question --
        // is the wall rendering in software? (#268, #271) -- this line answers
        // it from the log.
        if let Some(gl) = &cc.gl {
            use eframe::glow::HasContext as _;
            let (vendor, renderer, version) = unsafe {
                (
                    gl.get_parameter_string(eframe::glow::VENDOR),
                    gl.get_parameter_string(eframe::glow::RENDERER),
                    gl.get_parameter_string(eframe::glow::VERSION),
                )
            };
            log_info!("[App] GPU: {vendor} / {renderer} / OpenGL {version}");
        }
        if app.flags.no_signal {
            log_info!("[Health] Not started: --no-signal pins the state");
        } else {
            log_info!("[Health] monitoring every {}ms", health_limits::SAMPLE_MS);
        }
        if let Some(n) = app.flags.mock {
            app.set_devices(capture::mock_devices(n, app.flags.test_pattern, app.flags.mock_fault, app.flags.mock_shape));
        } else {
            let (tx, rx) = mpsc::channel();
            let ctx = cc.egui_ctx.clone();
            app.device_rx = Some(rx);
            capture::camera_initialize(move |granted| {
                if !granted {
                    log_warn!("[Video] camera access not granted");
                }
                let _ = tx.send(capture::camera_devices());
                ctx.request_repaint();
            });
        }
        app
    }

    // ---------------------------------------------------------------- devices

    fn set_devices(&mut self, devices: Vec<Device>) {
        if devices == self.devices {
            return;
        }
        log_info!("[Devices] {}", devices.iter().map(|d| d.label.as_str()).collect::<Vec<_>>().join(", "));
        self.devices = devices;
        self.sides = choose_sides(&self.devices, &self.settings, &self.sides, !self.started);
        self.started |= !self.devices.is_empty();
    }

    fn poll_devices(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.device_rx
            && let Ok(list) = rx.try_recv()
        {
            self.set_devices(list);
        }
        // Hot-plug: re-enumerate off the UI thread now and then.
        if self.flags.mock.is_none() && Instant::now() >= self.next_device_poll {
            self.next_device_poll = Instant::now() + DEVICE_POLL;
            let (tx, rx) = mpsc::channel();
            self.device_rx = Some(rx);
            let ctx = ctx.clone();
            std::thread::spawn(move || {
                let _ = tx.send(capture::camera_devices());
                ctx.request_repaint();
            });
        }
    }

    /// The operator's name for an input, else the device's own label.
    fn label_of(&self, id: Option<&String>) -> String {
        let Some(device) = id.and_then(|id| self.devices.iter().find(|d| &d.id == id)) else {
            return "No input".into();
        };
        self.settings.name_of(&device.id).unwrap_or(&device.label).to_owned()
    }

    /// What the pickers offer and what 1-4 count through.
    fn enabled_devices(&self) -> Vec<Device> {
        self.devices.iter().filter(|d| self.settings.is_enabled(&d.id)).cloned().collect()
    }

    // -------------------------------------------------------------- selection

    fn visible_sides(&self) -> &'static [usize] {
        match self.settings.layout() {
            Layout::Dual => &[0, 1],
            Layout::Single => &[0],
        }
    }

    /// Every user-driven switch goes through here, so every one dips.
    fn set_side(&mut self, i: usize, id: Option<String>) {
        if self.sides[i] != id {
            let from = self.sides[i].take();
            self.dips[i].switch(from);
            self.sides[i] = id;
        }
    }

    fn select(&mut self, id: String, side: Side) {
        let side = if self.settings.multi_view { side } else { Side::Both };
        if matches!(side, Side::Left | Side::Both) {
            self.set_side(0, Some(id.clone()));
        }
        if matches!(side, Side::Right | Side::Both) {
            self.set_side(1, Some(id.clone()));
        }
        if !self.dropdown_open {
            self.toast = Some((self.label_of(Some(&id)), Instant::now()));
        }
    }

    fn set_multi_view(&mut self, on: bool) {
        self.settings.multi_view = on;
        if !on {
            self.set_side(1, self.sides[0].clone());
        }
    }

    fn now_ms(&self) -> f64 {
        self.epoch.elapsed().as_secs_f64() * 1000.0
    }

    /// Open a stream for each input on screen and release the rest. A device
    /// closed for a reopen is opened again here once its release gap is over.
    fn sync_streams(&mut self, ctx: &egui::Context) {
        // A disabled input is never opened; its half shows no signal.
        // Also kept: an input still fading out, and the right half while
        // the layout is collapsing it.
        let mut shown: Vec<usize> = self.visible_sides().to_vec();
        if self.layout_t > 0.0 && !shown.contains(&1) {
            shown.push(1);
        }
        let wanted: Vec<String> = shown.iter()
            .filter_map(|&i| self.sides[i].clone())
            .chain(self.dips.iter().filter_map(|d| d.fading().map(str::to_owned)))
            .filter(|id| self.settings.is_enabled(id))
            .collect();
        self.slots.retain(|id, _| wanted.contains(id));
        let now = self.now_ms();
        for id in wanted {
            let Some(device) = self.devices.iter().find(|d| d.id == id).cloned() else { continue };
            match self.slots.get_mut(&id) {
                None => {
                    self.slots.insert(id, Slot::open(&device, ctx, now));
                }
                Some(slot) if slot.stream.is_none() && slot.reopen_at.is_some_and(|t| now >= t) => {
                    slot.attach(&device, ctx, now);
                }
                Some(_) => {}
            }
        }
    }

    fn update_frames(&mut self, ctx: &egui::Context, eframe_frame: &mut eframe::Frame) {
        let now = Instant::now();
        let forced = self.flags.no_signal;
        for (id, slot) in &mut self.slots {
            let Some(stream) = &slot.stream else { continue };
            let frame = stream.latest();
            let fresh = frame.as_ref().filter(|f| f.seq != slot.shown_seq);
            // While frozen, keep sampling health but stop updating the picture.
            if let Some(f) = fresh.filter(|_| !self.frozen && !forced) {
                let image = match &f.pixels {
                    // The Arc is handed over as is: no copy, no conversion here.
                    capture::Pixels::Rgba(image) => Some(image.clone()),
                    capture::Pixels::Yuyv(bytes) => {
                        let gl = eframe_frame.gl().cloned();
                        if let (Gpu::NotYet, Some(gl)) = (&self.gpu, &gl) {
                            self.gpu = match unsafe { gpu_video::Converter::new(gl) } {
                                Ok(c) => {
                                    log_info!("[Video] YUY2 frames are converted on the GPU");
                                    Gpu::Ready(c)
                                }
                                Err(e) => {
                                    log_error!("[Video] GPU conversion unavailable, converting YUY2 on the CPU: {e}");
                                    Gpu::Failed
                                }
                            };
                        }
                        match (&self.gpu, gl) {
                            (Gpu::Ready(conv), Some(gl)) => {
                                let tex = match self.gpu_textures.entry(id.clone()) {
                                    std::collections::hash_map::Entry::Occupied(e) => Some(e.into_mut()),
                                    std::collections::hash_map::Entry::Vacant(v) => match unsafe { gpu_video::GpuTexture::new(&gl, eframe_frame) } {
                                        Ok(t) => Some(v.insert(t)),
                                        Err(e) => {
                                            log_error!("[Video] {id}: could not create its GPU texture: {e}");
                                            None
                                        }
                                    },
                                };
                                if let Some(tex) = tex {
                                    unsafe { tex.update(&gl, conv, bytes, f.width(), f.height()) };
                                    slot.texture = Some(VideoTexture::Gpu {
                                        id: tex.id,
                                        size: vec2(tex.size[0] as f32, tex.size[1] as f32),
                                    });
                                }
                                None
                            }
                            // The slow path, so a machine without the GPU path
                            // still shows a picture.
                            _ => {
                                let rgba = crate::yuv::yuyv_to_rgba(bytes, f.width(), f.height());
                                Some(std::sync::Arc::new(egui::ColorImage::from_rgba_premultiplied([f.width(), f.height()], &rgba)))
                            }
                        }
                    }
                };
                if let Some(image) = image {
                    match &mut slot.texture {
                        Some(VideoTexture::Egui(t)) => t.set(image, VIDEO_TEXTURE),
                        _ => slot.texture = Some(VideoTexture::Egui(ctx.load_texture(id.clone(), image, VIDEO_TEXTURE))),
                    }
                }
                if let Some(t) = &slot.texture {
                    self.stills.insert(id.clone(), t.clone());
                }
                slot.stats.shown(now, f.seq, slot.shown_seq, f.captured_at);
            }
            if let Some(report) = slot.stats.tick(now) {
                self.report_count += 1;
                // Every 5s in the log, so a run can be judged without
                // watching it.
                if self.flags.hud && self.report_count.is_multiple_of(5) {
                    let size = frame.as_ref().map_or((0, 0), |f| (f.width(), f.height()));
                    log_info!("[Render] {} {}x{}: {} · ui {:.0} fps · {} skipped in total",
                        short_id(id), size.0, size.1, report.line(), self.ui_fps, slot.stats.total_skipped);
                }
            }
            if let Some(f) = fresh {
                slot.shown_seq = f.seq;
            }
        }
    }

    /// Which halves show each device, and its name, for log lines that read
    /// per side as Electron's do: `[Health] left (Presenter): ...`.
    fn sides_by_device(&self) -> HashMap<String, (Vec<&'static str>, String)> {
        let mut map: HashMap<String, (Vec<&'static str>, String)> = HashMap::new();
        for &i in self.visible_sides() {
            if let Some(id) = &self.sides[i] {
                let label = self.label_of(Some(id));
                map.entry(id.clone()).or_insert_with(|| (Vec::new(), label)).0.push(["left", "right"][i]);
            }
        }
        map
    }

    /// Health: every 2 s per device, never under --no-signal (it is not
    /// started there in Electron either, rather than run and ignored).
    fn sample_health(&mut self) {
        if self.flags.no_signal {
            return;
        }
        let now_ms = self.now_ms();
        let who = self.sides_by_device();
        let summary = now_ms - self.last_health_summary >= HEALTH_SUMMARY_MS;
        let crop_on = self.settings.crop_letterbox;
        if summary {
            self.last_health_summary = now_ms;
        }
        for (id, slot) in &mut self.slots {
            let Some((sides, label)) = who.get(id) else { continue };
            let Some(stream) = &slot.stream else { continue };
            let frame = stream.latest();
            if !slot.announced
                && let Some(f) = &frame
            {
                slot.announced = true;
                for side in sides {
                    log_info!("[Health] {side} opened \"{label}\" {}x{} id={}", f.width(), f.height(), short_id(id));
                }
            }
            // A failure is acted on at once, as Electron does on a rejected
            // open or an ended track; everything else waits for the sample.
            let new_failure = stream.error().is_some() && !slot.health.status().is_fault();
            if now_ms < slot.next_sample && !new_failure {
                continue;
            }
            slot.next_sample = now_ms + health_limits::SAMPLE_MS;
            check_health(id, slot, frame.as_deref(), now_ms, (sides, label), summary, crop_on);
        }
    }

    /// Does this half show the NO SIGNAL board?
    ///
    /// As in Electron: forced, no input, a missing or disabled device, or a
    /// device that cannot be opened. **Not** a dark, stalled or frameless
    /// stream -- those are reopened by the health policy and stay black
    /// meanwhile; only reference detection (ADR 0014, not ported) turns a
    /// live-but-dark card into the board.
    fn side_no_signal(&self, i: usize) -> bool {
        if self.flags.no_signal {
            return true;
        }
        let Some(id) = self.sides[i].as_ref() else { return true };
        if !self.devices.iter().any(|d| &d.id == id) || !self.settings.is_enabled(id) {
            return true;
        }
        self.slots.get(id).is_some_and(|slot| slot.health.status() == Status::OpenFailed)
    }

    // ------------------------------------------------------------ screensaver

    fn start_saver(&mut self, index: Option<usize>) {
        // --screensaver pins the first pick only; rotation stays random.
        let pinned = self.flags.screensaver.take().and_then(|want| {
            let i = screensaver::NAMES.iter().position(|n| crate::flags::normalise(n) == want);
            if i.is_none() {
                log_warn!("[TestFlags] no screensaver called {want:?}");
            }
            i
        });
        let Some(index) = index.or(pinned).or_else(|| screensaver::pick(&self.savers_enabled(), self.last_saver, &mut self.rng))
        else {
            // Every saver disabled: the split-flap board stays up.
            self.saver = None;
            self.saver_on_demand = false;
            return;
        };
        self.last_saver = Some(index);
        self.saver = Some(Screensaver::new(index, Rng::clock_seed()));
        self.saver_started = Instant::now();
    }

    /// Per saver, in `NAMES` order: whether Settings has it enabled.
    fn savers_enabled(&self) -> Vec<bool> {
        screensaver::NAMES.iter().map(|n| self.settings.saver_enabled(n)).collect()
    }

    fn step_saver(&mut self, delta: isize) {
        let cur = self.saver.as_ref().map(|s| s.index);
        let Some(next) = screensaver::step(&self.savers_enabled(), cur, delta) else { return };
        if self.saver.is_none() {
            self.saver_on_demand = true;
        }
        self.start_saver(Some(next));
    }

    fn update_saver(&mut self, now: Instant) {
        let all_dark = self.visible_sides().iter().all(|&i| self.side_no_signal(i));
        if !all_dark {
            self.wall_dark_since = None;
            if !self.saver_on_demand {
                self.saver = None;
            }
        } else {
            let since = *self.wall_dark_since.get_or_insert(now);
            if self.saver.is_none() && now.duration_since(since) >= self.flags.screensaver_delay {
                self.start_saver(None);
            }
        }
        // Rotate every ten minutes, and at once if the running saver has just
        // been disabled in Settings.
        let disabled = self.saver.as_ref().is_some_and(|s| !self.settings.saver_enabled(s.name()));
        if self.saver.is_some() && (disabled || now.duration_since(self.saver_started) >= screensaver::ROTATE_EVERY) {
            self.start_saver(None);
        }
    }

    // ------------------------------------------------------------------- keys

    fn handle_keys(&mut self, ctx: &egui::Context) {
        let pressed: Vec<Key> = ctx.input(|i| {
            i.events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::Key { key, pressed: true, repeat: false, modifiers, .. }
                        if !modifiers.command && !modifiers.ctrl && !modifiers.alt => Some(*key),
                    _ => None,
                })
                .collect()
        });
        if !pressed.is_empty() {
            self.last_activity = Instant::now();
        }
        // Typing an input's name must not switch inputs or quit (Electron's
        // shortcuts ignore every form control for the same reason), and with
        // Settings open nothing behind the dialog should react at all. Esc
        // still gets through, to close it.
        let quiet = self.settings_pane.is_some() || ctx.egui_wants_keyboard_input();
        for key in pressed {
            let Some(action) = shortcuts::action_for(key) else { continue };
            if quiet && action != Action::Escape {
                continue;
            }
            self.run(action, ctx);
        }
    }

    /// A presenter's slide change, sent to the remote keyboard if enabled.
    fn remote_press(&mut self, ctx: &egui::Context, direction: remote_key::Direction) {
        let s = &self.settings;
        if !s.remote_keyboard_enabled || s.remote_keyboard_api_key.is_empty() {
            return;
        }
        let Some(url) = remote_key::url(&s.remote_keyboard_host, direction) else { return };
        let ctx = ctx.clone();
        self.remote
            .get_or_insert_with(|| remote_key::Sender::new(move || ctx.request_repaint()))
            .send(url, s.remote_keyboard_api_key.clone(), direction);
    }

    /// Freeze is instant both ways (ADR 0037); unfreezing flashes LIVE.
    fn set_frozen(&mut self, on: bool) {
        if self.frozen && !on {
            self.live_until = Some(Instant::now() + LIVE_FOR);
        }
        if on {
            self.live_until = None;
        }
        self.frozen = on;
    }

    fn run(&mut self, action: Action, ctx: &egui::Context) {
        match action {
            Action::SelectInput(i) => {
                if let Some(d) = self.enabled_devices().get(i) {
                    self.select(d.id.clone(), Side::Both);
                }
            }
            Action::LayoutDual => self.settings.layout_mode = Some(Layout::Dual),
            Action::LayoutSingle => self.settings.layout_mode = Some(Layout::Single),
            Action::Freeze => self.set_frozen(!self.frozen),
            Action::ScreensaverToggle => {
                if self.saver.is_some() {
                    self.saver = None;
                    self.saver_on_demand = false;
                } else {
                    self.saver_on_demand = true;
                    self.start_saver(None);
                }
            }
            Action::ScreensaverNext => self.step_saver(1),
            Action::ScreensaverPrev => self.step_saver(-1),
            Action::Fullscreen => {
                let fs = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!fs));
            }
            // One thing per press: close panels if any are open, and stop.
            Action::Escape => {
                if self.settings_pane.is_some() {
                    self.settings_pane = None;
                } else if self.dropdown_open || self.legend_open {
                    self.dropdown_open = false;
                    self.legend_open = false;
                } else {
                    self.set_frozen(false);
                    if self.saver_on_demand {
                        self.saver_on_demand = false;
                        self.saver = None;
                    }
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                }
            }
            Action::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            Action::RemoteBack => self.remote_press(ctx, remote_key::Direction::Left),
            Action::RemoteForward => self.remote_press(ctx, remote_key::Direction::Right),
        }
    }

    // --------------------------------------------------------------- painting

    fn paint_wall(&mut self, ui: &mut egui::Ui, rect: Rect, now: Instant, dt: f32) {
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, Color32::BLACK);
        // borderWidth: black bars over both side edges, inset from the content.
        let rect = wall_rect(rect, self.settings.border_width);

        if let Some(saver) = &mut self.saver {
            let t = now.duration_since(self.saver_started).as_secs_f32();
            saver.paint(&painter, rect, t, dt);
            if t < 4.0 {
                badge(&painter, rect.left_bottom() + vec2(16.0, -16.0), Align2::LEFT_BOTTOM, BADGE_BG,
                    &format!("{}  ·  seed {}", saver.name(), saver.seed));
            }
            return;
        }

        // The halves at `layout_t`: dual at 1, single at 0, sliding between.
        let (l, divider, r) = transition::layout_rects(rect, dual_rects(rect, self.settings.center_gap), self.layout_t);
        let shown = transition::ease(self.layout_t);
        let settled = self.layout_t == 0.0 || self.layout_t == 1.0;
        for (i, half, opacity) in [(0, l, 1.0), (1, r, shown)] {
            if half.width() < 1.0 {
                continue;
            }
            let clipped = painter.with_clip_rect(half);
            let dip = &self.dips[i];
            if self.side_no_signal(i) && dip.fading().is_none() {
                let since = *self.board_since[i].get_or_insert(now);
                let label = self.label_of(self.sides[i].as_ref());
                screensaver::paint_split_flap(&clipped, half,
                    &["NO SIGNAL", &label], now.duration_since(since).as_secs_f32());
            } else {
                self.board_since[i] = None;
                // While a dip fades, the outgoing input; then the new one.
                let id = dip.fading().map(str::to_owned).or_else(|| self.sides[i].clone());
                self.paint_input(&clipped, half, id.as_deref(), opacity);
            }
            let black = dip.black().max(1.0 - opacity);
            if black > 0.0 {
                clipped.rect_filled(half, 0.0, Color32::BLACK.gamma_multiply(black));
            }
        }
        // After the halves: a logo set wider than the gap lies over them.
        if let Some(logo) = self.logo.as_mut().filter(|_| divider.width() >= 1.0) {
            let r = logo_rect(divider, logo.size(), self.settings.logo_width);
            logo.paint(&painter, r, Color32::WHITE.gamma_multiply(shown), settled);
        }

        if self.frozen {
            badge(&painter, rect.right_top() + vec2(-16.0, 16.0), Align2::RIGHT_TOP, FROZEN_RED, "FROZEN");
        } else if self.live_until.is_some_and(|t| now < t) {
            badge(&painter, rect.right_top() + vec2(-16.0, 16.0), Align2::RIGHT_TOP, LIVE_GREEN, "LIVE");
        } else {
            self.live_until = None;
        }
        if self.flags.hud {
            self.paint_hud(&painter, rect);
        }
    }

    /// One input, cropped of its black bars and fitted inside `half`.
    fn paint_input(&self, painter: &egui::Painter, half: Rect, id: Option<&str>, opacity: f32) {
        let Some(slot) = id.and_then(|id| self.slots.get(id)) else { return };
        let Some(tex) = slot.texture.as_ref() else { return };
        // The black-bar crop as texture coordinates (Electron:
        // object-view-box), then object-fit: contain on what is left.
        let crop = slot.crop.current().filter(|_| self.settings.crop_letterbox);
        let ([u0, v0], [u1, v1]) = crop.map_or(([0.0, 0.0], [1.0, 1.0]), crate::crop::Crop::uv);
        let size = tex.size_vec2() * vec2(u1 - u0, v1 - v0);
        let scale = (half.width() / size.x).min(half.height() / size.y);
        let fit = Rect::from_center_size(half.center(), size * scale);
        painter.image(tex.id(), fit, Rect::from_min_max(pos2(u0, v0), pos2(u1, v1)), Color32::WHITE.gamma_multiply(opacity));
    }

    /// Advance the layout slide and each side's dip, before painting.
    fn step_transitions(&mut self, dt: f32) {
        let reduced = self.settings.reduced_motion;
        let target = if self.settings.layout() == Layout::Dual { 1.0 } else { 0.0 };
        self.layout_t = transition::approach(self.layout_t, target, dt, transition::LAYOUT_SECS, reduced);
        for i in 0..2 {
            let ready = self.side_no_signal(i)
                || self.sides[i].as_ref().and_then(|id| self.slots.get(id)).is_some_and(|s| s.texture.is_some());
            self.dips[i].step(dt, ready, reduced);
        }
    }

    fn transitioning(&self) -> bool {
        let target = if self.settings.layout() == Layout::Dual { 1.0 } else { 0.0 };
        self.layout_t != target || self.dips.iter().any(transition::Dip::active)
    }

    /// `--hud`: one line per visible half, over its top-left corner.
    fn paint_hud(&self, painter: &egui::Painter, rect: Rect) {
        let halves: Vec<(usize, Rect)> = match self.settings.layout() {
            Layout::Single => vec![(0, rect)],
            Layout::Dual => {
                let (l, _, r) = dual_rects(rect, self.settings.center_gap);
                vec![(0, l), (1, r)]
            }
        };
        for (i, half) in halves {
            let Some(slot) = self.sides[i].as_ref().and_then(|id| self.slots.get(id)) else { continue };
            let size = slot.stream.as_ref().and_then(Stream::latest).map_or((0, 0), |f| (f.width(), f.height()));
            let stats = slot.stats.last.map_or_else(|| "measuring…".to_owned(), |r| r.line());
            let text = format!("{}x{} · {stats} · ui {:.0} fps", size.0, size.1, self.ui_fps);
            hud_line(painter, half.left_top() + vec2(12.0, 12.0), &text);
        }
    }

    fn paint_toast(&mut self, ctx: &egui::Context, rect: Rect, now: Instant) {
        let Some((text, at)) = &self.toast else { return };
        if now.duration_since(*at) > TOAST_FOR || self.dropdown_open {
            self.toast = None;
            return;
        }
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("toast")));
        badge(&painter, rect.center_bottom() - vec2(0.0, 80.0), Align2::CENTER_BOTTOM, BADGE_BG, text);
    }

    fn dropdown(&mut self, ctx: &egui::Context, rect: Rect) {
        let (pickers, chips) = picker_plan(self.settings.layout(), self.settings.multi_view);
        let mut clicked: Option<(String, Side)> = None;
        let mut open_settings = false;
        let enabled = self.enabled_devices();
        let mut layout = self.settings.layout();
        let mut multi = self.settings.multi_view;

        let resp = egui::Area::new(egui::Id::new("wall-pickers"))
            .fixed_pos(rect.left_top())
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.set_width(rect.width());
                panel_frame().show(ui, |ui| {
                    ui.set_width(rect.width() - 32.0);
                    ui.vertical_centered(|ui| {
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut layout, Layout::Dual, "Dual  D");
                            ui.selectable_value(&mut layout, Layout::Single, "Single  S");
                            ui.separator();
                            ui.checkbox(&mut multi, "Multi-view");
                            if self.flags.mock.is_some() {
                                ui.separator();
                                ui.weak(format!("mock mode: {} inputs", self.devices.len()));
                            }
                            if self.flags.no_signal {
                                ui.weak("· --no-signal");
                            }
                            ui.separator();
                            open_settings = ui.button("Settings").clicked();
                        });
                    });
                    ui.add_space(8.0);
                    ui.columns(pickers.len(), |cols| {
                        for (col, (title, side)) in cols.iter_mut().zip(&pickers) {
                            let current = match side {
                                Side::Right => self.sides[1].as_ref(),
                                _ => self.sides[0].as_ref(),
                            };
                            col.label(egui::RichText::new(*title).small().weak());
                            col.label(egui::RichText::new(self.label_of(current)).strong());
                            egui::ScrollArea::horizontal().id_salt(title).show(col, |ui| {
                                ui.horizontal(|ui| {
                                    for (idx, d) in enabled.iter().enumerate() {
                                        let selected = current == Some(&d.id);
                                        let chip = chips.then(|| shortcuts::input_key_for(idx)).flatten();
                                        let label = self.label_of(Some(&d.id));
                                        if tile(ui, &label, self.stills.get(&d.id), selected, chip).clicked() {
                                            clicked = Some((d.id.clone(), *side));
                                        }
                                    }
                                    if self.devices.is_empty() {
                                        ui.weak("No capture devices found");
                                    } else if enabled.is_empty() {
                                        ui.weak("Every input is disabled. Enable one in Settings.");
                                    }
                                });
                            });
                        }
                    });
                });
            });
        self.dropdown_bottom = resp.response.rect.bottom();
        self.settings.layout_mode = Some(layout);
        if open_settings {
            self.settings_pane = Some(Pane::Inputs);
            self.dropdown_open = false;
        }
        if multi != self.settings.multi_view {
            self.set_multi_view(multi);
        }
        if let Some((id, side)) = clicked {
            self.select(id, side);
        }
    }

    /// The two electron-updater dialogs, as an egui window.
    fn update_dialog(&mut self, ctx: &egui::Context) {
        let Some(updater) = &self.updater else { return };
        let status = updater.status();
        let (title, body) = match &status {
            update::Status::Idle => return,
            update::Status::Available(v) => ("Update available", format!("A new version ({v}) is available.\nWould you like to download it now?")),
            update::Status::Downloading(p) => ("Downloading update", format!("{p}%")),
            update::Status::Ready(v) => ("Update ready", format!("Version {v} has been downloaded.\nThe application will restart to install it.")),
            update::Status::Failed(e) => ("Update error", format!("Failed to update:\n{e}")),
        };
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .order(egui::Order::Tooltip)
            .show(ctx, |ui| {
                ui.label(body);
                ui.add_space(8.0);
                ui.horizontal(|ui| match &status {
                    update::Status::Available(_) => {
                        if ui.button("Download").clicked() {
                            updater.download();
                        }
                        if ui.button("Later").clicked() {
                            updater.dismiss();
                        }
                    }
                    update::Status::Downloading(p) => {
                        ui.add(egui::ProgressBar::new(*p as f32 / 100.0).desired_width(240.0));
                    }
                    update::Status::Ready(_) => {
                        if ui.button("Restart now").clicked() {
                            updater.restart_now();
                        }
                        // Installs on the next quit instead.
                        if ui.button("Later").clicked() {
                            updater.dismiss();
                        }
                    }
                    _ => {
                        if ui.button("OK").clicked() {
                            updater.dismiss();
                        }
                    }
                });
            });
    }

    fn legend(&mut self, ctx: &egui::Context, rect: Rect) {
        let resp = egui::Area::new(egui::Id::new("legend"))
            .anchor(Align2::CENTER_BOTTOM, vec2(0.0, -8.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                panel_frame().show(ui, |ui| {
                    ui.set_max_width((rect.width() - 48.0).min(900.0));
                    egui::Grid::new("legend-grid").num_columns(4).spacing([16.0, 6.0]).show(ui, |ui| {
                        for (i, s) in shortcuts::SHORTCUTS.iter().enumerate() {
                            ui.label(egui::RichText::new(s.chips).monospace().strong());
                            ui.label(s.label);
                            if i % 2 == 1 {
                                ui.end_row();
                            }
                        }
                    });
                });
            });
        self.legend_top = resp.response.rect.top();
    }
}

/// Electron logs `deviceId.slice(0, 8)` -- eight hex characters of a hash.
/// Camera ids here are the device's own path (`\\?\pci#ven_12ab&dev_0710...`
/// on Windows), whose first characters are the same for every card, so they
/// are hashed (FNV-1a) to eight hex characters the same way. Mock ids are
/// already short and readable.
/// ADR 0038: hidden once the pointer has been still for CURSOR_IDLE, but
/// never while the controls, the legend or Settings are open.
fn cursor_hidden(idle: Duration, panels_open: bool) -> bool {
    !panels_open && idle >= CURSOR_IDLE
}

fn short_id(id: &str) -> String {
    if id.starts_with(capture::MOCK_PREFIX) {
        return id.to_owned();
    }
    let hash = id.bytes().fold(0x811c_9dc5u32, |h, b| (h ^ u32::from(b)).wrapping_mul(0x0100_0193));
    format!("{hash:08x}")
}

/// `describeHealth()` from renderer.js, character for character:
/// `ok 60fps luma=80+/-64.1 1920x1080 reopens=0`. `+/-` rather than the
/// plus-minus sign, because the log is read with Windows tools that print it
/// as two garbage characters.
fn describe_health(info: &health::Info, frame: Option<&capture::Frame>) -> String {
    let (luma, std) = info.luma.map_or(("-".to_owned(), "-".to_owned()), |l| {
        (format!("{}", l.mean.round()), format!("{}", (l.std * 10.0).round() / 10.0))
    });
    let size = frame.map_or_else(|| "-".to_owned(), |f| format!("{}x{}", f.width(), f.height()));
    let error = info.error.as_deref().map(|e| format!(" error={e}")).unwrap_or_default();
    format!("{} {}fps luma={luma}+/-{std} {size} reopens={}{error}", info.status.name(), info.fps, info.attempts)
}

/// One health sample for a device: feed the policy, log it per side, and
/// close the device for a reopen when the policy says so.
fn check_health(
    id: &str,
    slot: &mut Slot,
    frame: Option<&capture::Frame>,
    now: f64,
    // The halves showing this device, and its name: for the per-side lines.
    (sides, label): (&[&str], &str),
    summary: bool,
    crop_on: bool,
) {
    let Some(stream) = &slot.stream else { return };
    match stream.error() {
        // Failed before a single frame: it never opened.
        Some(e) if frame.is_none() => {
            if slot.health.status() != Status::OpenFailed {
                slot.health.open_failed(now, &e);
            }
        }
        Some(_) => {
            slot.health.sample(Sample { frames: frame.map(|f| f.seq), ended: true, stats: None }, now);
        }
        None => {
            // One thumbnail serves both the flatness check and the crop, as
            // in Electron since #314.
            let (tw, th) = HEALTH_THUMB;
            let thumb = frame.map(|f| f.health_thumbnail(tw, th));
            let stats = thumb.as_ref().map(|t| health::luma_stats(t));
            slot.health.sample(Sample { frames: Some(frame.map_or(0, |f| f.seq)), ended: false, stats }, now);
            if let (true, Some(t), Some(f)) = (crop_on, &thumb, frame) {
                let detected = crate::crop::detect(&crate::crop::luma_grid(t), tw, th, f.width(), f.height());
                if slot.crop.update(detected.as_ref()) {
                    let what = slot.crop.current().map_or_else(|| "none".to_owned(), |c| {
                        format!("{} {} of {}x{} (measured {:.3}:1)", c.shape, c.css(), f.width(), f.height(), c.measured)
                    });
                    for side in sides {
                        log_info!("[Crop] {side} ({label}): {what}");
                    }
                }
            }
        }
    }
    let info = slot.health.info();
    let described = describe_health(&info, frame);
    if slot.last_logged != Some(info.status) {
        let from = slot.last_logged.map_or("start", Status::name);
        for side in sides {
            if matches!(info.status, Status::Ok | Status::Opening | Status::Idle) {
                log_info!("[Health] {side} ({label}): {from} -> {described}");
            } else {
                log_warn!("[Health] {side} ({label}): {from} -> {described}");
            }
        }
        slot.last_logged = Some(info.status);
    } else if summary && info.status != Status::Idle {
        for side in sides {
            log_info!("[Health] {side} ({label}): {described}");
        }
    }
    let decision = slot.health.decide(now);
    if decision.reopen {
        slot.health.reopening(now);
        let reason = decision.reason.map_or("?", Status::name);
        log_warn!(
            "[Health] reopening {} ({label}) for {}: {} {reason}, attempt {}",
            short_id(id), sides.join("+"), sides.first().copied().unwrap_or("?"), slot.health.info().attempts
        );
        // Dropping the stream closes the device; it is opened again after
        // the release gap, "so the driver sees the device closed rather than
        // a handover".
        slot.stream = None;
        slot.texture = None;
        slot.reopen_at = Some(now + health_limits::REOPEN_RELEASE_MS);
    }
}

fn panel_frame() -> egui::Frame {
    egui::Frame::NONE
        .fill(Color32::from_rgba_unmultiplied(16, 16, 20, 235))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(16)
        .outer_margin(8)
}

fn hud_line(painter: &egui::Painter, at: egui::Pos2, text: &str) {
    let galley = painter.layout_no_wrap(text.to_owned(), FontId::monospace(14.0), Color32::from_rgb(120, 255, 140));
    let r = Rect::from_min_size(at, galley.size()).expand2(vec2(8.0, 4.0));
    painter.rect_filled(r, 4.0, Color32::from_black_alpha(200));
    painter.galley(at, galley, Color32::WHITE);
}

const BADGE_BG: Color32 = Color32::from_black_alpha(190);

fn badge(painter: &egui::Painter, at: egui::Pos2, align: Align2, fill: Color32, text: &str) {
    let galley = painter.layout_no_wrap(text.to_owned(), FontId::proportional(22.0), Color32::WHITE);
    let r = align.anchor_size(at, galley.size()).expand2(vec2(16.0, 8.0));
    painter.rect_filled(r, 8.0, fill);
    painter.galley(r.min + vec2(16.0, 8.0), galley, Color32::WHITE);
}

/// One input tile: a fixed 128x72 still with the name under it, never the
/// strip width (a first pass at `width: 100%` in the Electron app measured
/// 292x164).
fn tile(ui: &mut egui::Ui, label: &str, still: Option<&VideoTexture>, selected: bool, chip: Option<&str>) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(TILE.x, TILE.y + 22.0), Sense::click());
    let img = Rect::from_min_size(rect.min, TILE);
    let p = ui.painter();
    match still {
        Some(t) => {
            p.image(t.id(), img, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        }
        None => {
            p.rect_filled(img, 6.0, Color32::from_gray(30));
            p.text(img.center(), Align2::CENTER_CENTER, "no preview", FontId::proportional(12.0), Color32::from_gray(110));
        }
    }
    let stroke = if selected {
        Stroke::new(3.0, Color32::from_rgb(90, 160, 255))
    } else if resp.hovered() {
        Stroke::new(2.0, Color32::from_gray(160))
    } else {
        Stroke::new(1.0, Color32::from_gray(60))
    };
    p.rect_stroke(img, 6.0, stroke, StrokeKind::Inside);
    let name_pos = pos2(img.left(), img.bottom() + 4.0);
    let name = p.layout(label.to_owned(), FontId::proportional(12.0), Color32::from_gray(220), TILE.x - 20.0);
    p.galley(name_pos, name, Color32::from_gray(220));
    if let Some(k) = chip {
        let c = Rect::from_min_size(pos2(img.right() - 18.0, img.bottom() + 3.0), vec2(18.0, 16.0));
        p.rect_filled(c, 4.0, Color32::from_gray(70));
        p.text(c.center(), Align2::CENTER_CENTER, k, FontId::monospace(12.0), Color32::WHITE);
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

impl eframe::App for InputViewer {
    /// Everything that must keep running when nobody can see the window.
    ///
    /// eframe skips `ui()` entirely while the window is hidden or covered, and
    /// calls only this. Health, reopening and device polling lived in `ui()`
    /// at first, and a covered window simply stopped recovering its cards --
    /// found by `--mock-fault` with the window behind others. On the wall any
    /// window over the app (an update prompt, say) would have done the same.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_devices(ctx);
        self.sync_streams(ctx);
        self.sample_health();
        self.update_saver(Instant::now());
        if let Some(last) = self.remote.as_ref().and_then(remote_key::Sender::poll) {
            self.remote_last = Some(last);
        }
        self.store.tick(&self.settings);
        // Wake up on our own: with every input stalled or failed no frames
        // arrive to request a repaint, and the reopen timers would never fire.
        ctx.request_repaint_after(Duration::from_millis(500));
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let now = Instant::now();
        // Every change to settings, from any control or key, is caught by
        // comparing against this at the end of the frame -- one place, like
        // Electron's saveSettings(), so no call site can forget to save.
        let before = self.settings.clone();
        self.ui_frames.0 += 1;
        let since = now.duration_since(self.ui_frames.1);
        if since >= Duration::from_secs(1) {
            self.ui_fps = self.ui_frames.0 as f32 / since.as_secs_f32();
            self.ui_frames = (0, now);
        }
        let dt = now.duration_since(self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;

        self.handle_keys(&ctx);
        self.update_frames(&ctx, frame);

        let rect = ui.max_rect();
        self.step_transitions(dt);
        self.paint_wall(ui, rect, now, dt);

        // Hover-to-reveal on the top and bottom edges; a tap there toggles.
        let (pointer, moved, tapped) = ctx.input(|i| (i.pointer.hover_pos(), i.pointer.is_moving(), i.pointer.primary_clicked()));
        if moved || tapped {
            self.last_activity = now;
            self.last_pointer_move = now;
        }
        if let Some(p) = pointer.filter(|_| self.settings_pane.is_none()) {
            if p.y <= rect.top() + EDGE_PX {
                self.dropdown_open = true;
            } else if self.dropdown_open && p.y > self.dropdown_bottom + CLOSE_MARGIN {
                self.dropdown_open = false;
            }
            if p.y >= rect.bottom() - EDGE_PX {
                self.legend_open = true;
            } else if self.legend_open && p.y < self.legend_top - CLOSE_MARGIN {
                self.legend_open = false;
            }
        }
        if let Some(open) = self.review.as_ref().and_then(|r| r.open.clone()) {
            self.dropdown_open = open == "dropdown";
            self.legend_open = open == "legend";
            if let Some(pane) = open.strip_prefix("settings") {
                self.settings_pane.get_or_insert(match pane { ":layout" => Pane::Layout, ":remote" => Pane::RemoteKeyboard, ":screensavers" => Pane::Screensavers, _ => Pane::Inputs });
            }
            self.last_activity = now;
        }
        if self.dropdown_open && now.duration_since(self.last_activity) > DROPDOWN_IDLE {
            self.dropdown_open = false;
        }
        if self.dropdown_open {
            self.dropdown(&ctx, rect);
        }
        if self.legend_open {
            self.legend(&ctx, rect);
        }
        self.paint_toast(&ctx, rect, now);
        self.settings_dialog(&ctx, rect);
        self.update_dialog(&ctx);

        // Last, so it overrides whatever cursor a widget asked for.
        let panels_open = self.dropdown_open || self.legend_open || self.settings_pane.is_some();
        if cursor_hidden(now.duration_since(self.last_pointer_move), panels_open) {
            ctx.set_cursor_icon(egui::CursorIcon::None);
        }

        // The halves' current inputs are part of settings (leftDeviceId /
        // rightDeviceId). Not overwritten with None before a side has an
        // input: that is how Electron once forgot the wall's input on restart.
        for (i, saved) in [&mut self.settings.left_device_id, &mut self.settings.right_device_id].into_iter().enumerate() {
            if self.sides[i].is_some() {
                saved.clone_from(&self.sides[i]);
            }
        }
        if self.settings != before {
            self.store.mark_dirty();
        }
        self.store.tick(&self.settings);
        if let Some(review) = &mut self.review {
            review.tick(&ctx);
        }

        // Animate the board and savers; capture threads request repaints for
        // new frames themselves.
        if self.saver.is_some() || self.board_since.iter().any(Option::is_some) || self.toast.is_some() || self.live_until.is_some() || self.transitioning() {
            ctx.request_repaint();
        } else {
            ctx.request_repaint_after(Duration::from_millis(500));
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.store.flush(&self.settings);
        if let Some(u) = &self.updater {
            u.on_exit();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn devs(ids: &[&str]) -> Vec<Device> {
        ids.iter().map(|id| Device { id: (*id).into(), label: id.to_uppercase(), index: None, test_pattern: None, mock_fault: None, mock_shape: None }).collect()
    }

    fn ids(sides: [Option<String>; 2]) -> [Option<&'static str>; 2] {
        sides.map(|s| s.map(|s| &*s.leak()))
    }

    #[test]
    fn first_launch_takes_the_first_two_enabled_inputs() {
        let mut s = Settings::default();
        s.input_mut("a").enabled = false;
        let d = devs(&["a", "b", "c"]);
        assert_eq!(ids(choose_sides(&d, &s, &[None, None], true)), [Some("b"), Some("c")]);
    }

    #[test]
    fn saved_sides_beat_the_defaults_and_vanished_ones_are_ignored() {
        let s = Settings { left_device_id: Some("c".into()), right_device_id: Some("gone".into()), ..Default::default() };
        assert_eq!(ids(choose_sides(&devs(&["a", "b", "c"]), &s, &[None, None], true)), [Some("c"), Some("b")]);
    }

    #[test]
    fn default_input_wins_at_startup_only_and_only_if_enabled() {
        let mut s = Settings { default_input_id: Some("b".into()), left_device_id: Some("a".into()), ..Default::default() };
        let d = devs(&["a", "b"]);
        assert_eq!(ids(choose_sides(&d, &s, &[None, None], true)), [Some("b"), Some("b")]);
        // A hot-plug later keeps what is on screen.
        let on_screen = [Some("a".to_string()), Some("a".to_string())];
        assert_eq!(ids(choose_sides(&d, &s, &on_screen, false)), [Some("a"), Some("a")]);
        s.input_mut("b").enabled = false;
        assert_eq!(ids(choose_sides(&d, &s, &[None, None], true))[0], Some("a"));
    }

    #[test]
    fn without_multi_view_the_right_half_copies_the_left() {
        let s = Settings { multi_view: false, ..Default::default() };
        assert_eq!(ids(choose_sides(&devs(&["a", "b"]), &s, &[None, None], true)), [Some("a"), Some("a")]);
    }

    #[test]
    fn short_ids_are_eight_hex_for_cameras_and_readable_for_mocks() {
        let a = short_id("camera-\\\\?\\pci#ven_12ab&dev_0710&subsys_000e1cfa&rev_00#4&381bec2&0&0019");
        let b = short_id("camera-\\\\?\\pci#ven_12ab&dev_0710&subsys_000e1cfa&rev_00#4&64a0be6&0&0019");
        assert_eq!(a.len(), 8);
        assert_ne!(a, b, "the wall's two cards must not share an id in the log");
        assert_eq!(short_id("mock-input-2"), "mock-input-2");
    }

    #[test]
    fn side_borders_inset_the_content_and_never_cross() {
        let wall = Rect::from_min_size(pos2(0.0, 0.0), vec2(6000.0, 1200.0));
        let inner = wall_rect(wall, 100.0);
        assert_eq!((inner.left(), inner.right()), (100.0, 5900.0));
        assert_eq!(wall_rect(wall, 9999.0).width(), 0.0);
    }

    #[test]
    fn dual_halves_split_what_the_gap_leaves() {
        let wall = Rect::from_min_size(pos2(0.0, 0.0), vec2(6000.0, 1200.0));
        let (l, d, r) = dual_rects(wall, 60.0);
        assert_eq!((l.width(), d.width(), r.width()), (2970.0, 60.0, 2970.0));
        assert_eq!((l.right(), r.left()), (d.left(), d.right()));
        assert_eq!(dual_rects(wall, 9000.0).1.width(), 6000.0);
    }

    #[test]
    fn logo_fits_the_divider_column() {
        let d = Rect::from_min_size(pos2(2970.0, 0.0), vec2(60.0, 1200.0));
        let r = logo_rect(d, vec2(1014.0, 275.0), None);
        assert!((r.width() - 54.0).abs() < 0.01, "90% of the column: {r:?}");
        assert_eq!(r.center(), d.center());
        // A set width may exceed the gap, but never 80% of the height.
        assert!((logo_rect(d, vec2(1014.0, 275.0), Some(400.0)).width() - 400.0).abs() < 0.01);
        let tall = logo_rect(d, vec2(100.0, 100.0), Some(5000.0));
        assert!((tall.height() - 960.0).abs() < 0.01);
    }

    #[test]
    fn cursor_hides_when_idle_but_never_over_open_panels() {
        assert!(!cursor_hidden(Duration::from_millis(2900), false));
        assert!(cursor_hidden(CURSOR_IDLE, false));
        assert!(!cursor_hidden(Duration::from_secs(60), true));
    }

    #[test]
    fn picker_plan_matches_the_electron_table() {
        let sides = |l, m| picker_plan(l, m).0.into_iter().map(|(_, s)| s).collect::<Vec<_>>();
        assert_eq!(sides(Layout::Dual, true), [Side::Left, Side::Right]);
        assert_eq!(sides(Layout::Dual, false), [Side::Both]);
        assert_eq!(sides(Layout::Single, true), [Side::Left]);
        assert_eq!(sides(Layout::Single, false), [Side::Both]);
        assert!(!picker_plan(Layout::Dual, true).1, "per-half pickers carry no key chips");
        assert!(picker_plan(Layout::Single, true).1);
    }
}
