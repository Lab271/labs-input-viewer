// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! Where the picture actually is inside a capture frame: letterbox and
//! pillarbox detection, so the black bars can be cropped away. A port of
//! `content-box.js` (ADR 0015), with its tests.
//!
//! The wall's Elgato cards deliver 3840x2160 whatever the source sends. A
//! source in another shape is fitted inside that frame, shape kept, with pure
//! black around it: a laptop at 3840x768 (5:1, the wall's shape) arrives as a
//! band a third of the frame tall.
//!
//! Three guards keep a dark picture from being mistaken for bars:
//! - a bar must be pure black across its WHOLE width (`BLACK_MAX`);
//! - the bars must be symmetric, because the driver centres the picture;
//! - the result must land on a known shape (`SHAPES`) and repeat
//!   `STABLE_SAMPLES` times before the crop changes. An all-black or
//!   unmatched frame changes nothing.
//!
//! The crop is applied as UV coordinates on the textured quad -- the egui
//! equivalent of `object-view-box` -- so freeze, which draws the same texture,
//! gets it for free (ADR 0017).

/// Thumbnail the detector reads; 216 rows resolves a bar to ~0.5% of the
/// frame (96x54 could not tell 'dual half' from 21:9).
pub const SAMPLE_W: usize = 384;
pub const SAMPLE_H: usize = 216;
/// Luma at or below which a pixel counts as bar. The padding is pure black.
pub const BLACK_MAX: u8 = 10;
/// A row or column is picture if more than this share of it is above BLACK_MAX.
pub const ACTIVE_SHARE: f64 = 0.02;
/// Bars on opposite sides may differ by this share of the frame.
pub const SYMMETRY_TOLERANCE: f64 = 0.04;
/// A measured shape may differ from a known one by this share.
pub const SHAPE_TOLERANCE: f64 = 0.06;
/// Identical results in a row before the crop changes (~6 s at 2 s).
pub const STABLE_SAMPLES: u32 = 3;
/// Bars of at least this share on BOTH axes make a windowbox.
pub const WINDOWBOX_MIN_BAR: f64 = 0.01;
/// Insets within this many percent count as the same crop.
pub const INSET_JITTER: f64 = 1.0;

pub struct Shape {
    pub name: &'static str,
    pub ratio: f64,
}

/// 5:1 is the wall; the rest are what laptops, cameras and players send.
/// 'MacBook' is a mirrored MacBook screen (~1.54:1 for every model's default
/// size); 'dual half' is one half of the wall in dual view, 2920x1200.
pub const SHAPES: &[Shape] = &[
    Shape { name: "5:1", ratio: 5.0 },
    Shape { name: "32:9", ratio: 32.0 / 9.0 },
    Shape { name: "dual half", ratio: 2920.0 / 1200.0 },
    Shape { name: "21:9", ratio: 64.0 / 27.0 },
    Shape { name: "2:1", ratio: 2.0 },
    Shape { name: "16:9", ratio: 16.0 / 9.0 },
    Shape { name: "16:10", ratio: 16.0 / 10.0 },
    Shape { name: "MacBook", ratio: 1.542 },
    Shape { name: "3:2", ratio: 3.0 / 2.0 },
    Shape { name: "4:3", ratio: 4.0 / 3.0 },
    Shape { name: "5:4", ratio: 5.0 / 4.0 },
];

/// Percent of the frame cut from each side.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Inset {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Crop {
    pub shape: &'static str,
    pub ratio: f64,
    pub inset: Inset,
    /// What the thumbnail measured, for the log: it says why a shape won
    /// over its neighbour (3:2, MacBook and 16:10 are close together).
    pub measured: f64,
    pub windowbox: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Detection {
    /// The picture fills the frame.
    Full,
    Crop(Crop),
}

impl Crop {
    /// `inset(32.22% 0% 32.22% 0%)`, as Electron's viewBoxCss writes it.
    pub fn css(&self) -> String {
        let p = |v: f64| format!("{}%", (v * 100.0).round() / 100.0);
        format!("inset({} {} {} {})", p(self.inset.top), p(self.inset.right), p(self.inset.bottom), p(self.inset.left))
    }

    /// Texture coordinates of the visible picture: min and max, 0..1.
    pub fn uv(&self) -> ([f32; 2], [f32; 2]) {
        let i = self.inset;
        (
            [(i.left / 100.0) as f32, (i.top / 100.0) as f32],
            [(1.0 - i.right / 100.0) as f32, (1.0 - i.bottom / 100.0) as f32],
        )
    }
}

/// Rec. 601 luma per pixel of an RGBA buffer.
pub fn luma_grid(rgba: &[u8]) -> Vec<u8> {
    rgba.as_chunks::<4>().0.iter()
        .map(|p| (0.299 * f64::from(p[0]) + 0.587 * f64::from(p[1]) + 0.114 * f64::from(p[2])) as u8)
        .collect()
}

/// Detect the picture inside a frame. `None` when undecidable (all black,
/// asymmetric, or no known shape): callers keep what they had.
pub fn detect(luma: &[u8], w: usize, h: usize, frame_w: usize, frame_h: usize) -> Option<Detection> {
    if w == 0 || h == 0 || frame_w == 0 || frame_h == 0 || luma.len() < w * h {
        return None;
    }
    let row_active = |y: usize| (0..w).filter(|&x| luma[y * w + x] > BLACK_MAX).count() as f64 > w as f64 * ACTIVE_SHARE;
    let col_active = |x: usize| (0..h).filter(|&y| luma[y * w + x] > BLACK_MAX).count() as f64 > h as f64 * ACTIVE_SHARE;

    let mut top = 0;
    while top < h && !row_active(top) {
        top += 1;
    }
    if top == h {
        return None; // all black: nothing to judge by
    }
    let mut bottom = h - 1;
    while bottom > top && !row_active(bottom) {
        bottom -= 1;
    }
    let mut left = 0;
    while left < w && !col_active(left) {
        left += 1;
    }
    let mut right = w - 1;
    while right > left && !col_active(right) {
        right -= 1;
    }

    let (wf, hf) = (w as f64, h as f64);
    let bar_t = top as f64 / hf;
    let bar_b = (h - 1 - bottom) as f64 / hf;
    let bar_l = left as f64 / wf;
    let bar_r = (w - 1 - right) as f64 / wf;
    if (bar_t - bar_b).abs() > SYMMETRY_TOLERANCE || (bar_l - bar_r).abs() > SYMMETRY_TOLERANCE {
        return None;
    }

    let (fw, fh) = (frame_w as f64, frame_h as f64);
    let content_w = (right - left + 1) as f64 / wf * fw;
    let content_h = (bottom - top + 1) as f64 / hf * fh;
    let measured = content_w / content_h;
    let frame_ratio = fw / fh;
    if (measured - frame_ratio).abs() / frame_ratio <= SHAPE_TOLERANCE {
        return Some(Detection::Full);
    }

    // Nearest, not first within tolerance: 16:10 and 3:2 are only 6% apart.
    let off = |s: &Shape| (measured - s.ratio).abs() / s.ratio;
    let shape = SHAPES.iter().min_by(|a, b| off(a).total_cmp(&off(b)))?;
    if off(shape) > SHAPE_TOLERANCE {
        return None;
    }

    // Black on BOTH axes: a windowbox -- a Mac mirroring its 1.54:1 screen
    // into a wide mode, black at the sides from macOS and above and below
    // from the card.
    let windowbox = bar_t.min(bar_b) > WINDOWBOX_MIN_BAR && bar_l.min(bar_r) > WINDOWBOX_MIN_BAR;
    let inset = if windowbox {
        // The shape's box sized to CONTAIN what was measured, so it never
        // cuts picture.
        let (mut bw, mut bh) = (content_w, content_w / shape.ratio);
        if bh < content_h {
            bh = content_h;
            bw = content_h * shape.ratio;
        }
        bw = bw.min(fw);
        bh = bh.min(fh);
        // Rounded DOWN: a smaller inset is a larger box, so it never cuts.
        let r1 = |v: f64| (v * 10.0).floor() / 10.0;
        let sx = r1((1.0 - bw / fw) / 2.0 * 100.0);
        let sy = r1((1.0 - bh / fh) / 2.0 * 100.0);
        Inset { top: sy, right: sx, bottom: sy, left: sx }
    } else if shape.ratio > frame_ratio {
        // The exact, centred crop for that shape, not the measured one.
        let v = (1.0 - frame_ratio / shape.ratio) / 2.0 * 100.0;
        Inset { top: v, right: 0.0, bottom: v, left: 0.0 }
    } else {
        let v = (1.0 - shape.ratio / frame_ratio) / 2.0 * 100.0;
        Inset { top: 0.0, right: v, bottom: 0.0, left: v }
    };
    Some(Detection::Crop(Crop { shape: shape.name, ratio: shape.ratio, inset, measured, windowbox }))
}

/// Debounces detections, so the crop only moves on a result that repeated
/// `STABLE_SAMPLES` times in a row.
#[derive(Debug, Default)]
pub struct CropTracker {
    current: Option<Crop>,
    candidate: Option<Option<Crop>>,
    count: u32,
}

fn same(a: Option<&Crop>, b: Option<&Crop>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            a.shape == b.shape
                && a.windowbox == b.windowbox
                && [(a.inset.top, b.inset.top), (a.inset.right, b.inset.right), (a.inset.bottom, b.inset.bottom), (a.inset.left, b.inset.left)]
                    .iter()
                    .all(|(x, y)| (x - y).abs() <= INSET_JITTER)
        }
        _ => false,
    }
}

impl CropTracker {
    /// Feed one detection; `None` (undecidable) neither confirms nor resets a
    /// candidate, so one black frame mid-change costs nothing. Returns true
    /// when the crop changed.
    pub fn update(&mut self, detected: Option<&Detection>) -> bool {
        let Some(detected) = detected else { return false };
        let crop = match detected {
            Detection::Full => None,
            Detection::Crop(c) => Some(c.clone()),
        };
        if same(crop.as_ref(), self.current.as_ref()) {
            self.candidate = None;
            self.count = 0;
            return false;
        }
        if self.candidate.as_ref().is_some_and(|c| same(crop.as_ref(), c.as_ref())) {
            self.count += 1;
        } else {
            self.candidate = Some(crop);
            self.count = 1;
        }
        if self.count >= STABLE_SAMPLES {
            self.current = self.candidate.take().flatten();
            self.count = 0;
            return true;
        }
        false
    }

    /// A new stream starts uncropped.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn current(&self) -> Option<&Crop> {
        self.current.as_ref()
    }
}

#[cfg(test)]
mod tests {
    //! Ported from test/content-box.test.js.
    use super::*;

    const W: usize = SAMPLE_W;
    const H: usize = SAMPLE_H;
    // The wall's cards always deliver this (measured 2026-10-01).
    const FW: usize = 3840;
    const FH: usize = 2160;

    /// A source of `ratio` fitted, shape kept, into the 16:9 frame, as the
    /// Elgato driver does: pure black around it.
    fn frame_with(ratio: f64, fill: impl Fn(usize, usize, usize, usize) -> u8) -> Vec<u8> {
        let mut luma = vec![0u8; W * H];
        let frame_ratio = FW as f64 / FH as f64;
        let (mut cw, mut ch) = (W, H);
        if ratio > frame_ratio {
            ch = (H as f64 * frame_ratio / ratio).round() as usize;
        } else {
            cw = (W as f64 * ratio / frame_ratio).round() as usize;
        }
        let (x0, y0) = (((W - cw) as f64 / 2.0).round() as usize, ((H - ch) as f64 / 2.0).round() as usize);
        for y in y0..y0 + ch {
            for x in x0..x0 + cw {
                luma[y * W + x] = fill(x - x0, y - y0, cw, ch);
            }
        }
        luma
    }

    fn frame(ratio: f64) -> Vec<u8> {
        frame_with(ratio, |_, _, _, _| 128)
    }

    fn detect_luma(luma: &[u8]) -> Option<Detection> {
        detect(luma, W, H, FW, FH)
    }

    fn crop(d: Option<Detection>) -> Crop {
        match d {
            Some(Detection::Crop(c)) => c,
            other => panic!("expected a crop, got {other:?}"),
        }
    }

    fn close(a: f64, b: f64, places: i32) -> bool {
        (a - b).abs() < 10f64.powi(-places) / 2.0
    }

    /// A picture of `ratio` in a screen of `screen_ratio`, that screen in the
    /// 16:9 frame. Returns the thumbnail and the picture's true insets (%).
    fn windowboxed(ratio: f64, screen_ratio: f64) -> (Vec<u8>, f64, f64) {
        let frame_ratio = FW as f64 / FH as f64;
        let (mut sw, mut sh) = (1.0, 1.0);
        if screen_ratio > frame_ratio {
            sh = frame_ratio / screen_ratio;
        } else {
            sw = screen_ratio / frame_ratio;
        }
        let (mut pw, mut ph) = (sw, sh);
        if ratio > screen_ratio {
            ph = sh * screen_ratio / ratio;
        } else {
            pw = sw * ratio / screen_ratio;
        }
        let mut luma = vec![0u8; W * H];
        let (x0, x1) = (((1.0 - pw) / 2.0 * W as f64).round() as usize, ((1.0 + pw) / 2.0 * W as f64).round() as usize);
        let (y0, y1) = (((1.0 - ph) / 2.0 * H as f64).round() as usize, ((1.0 + ph) / 2.0 * H as f64).round() as usize);
        for y in y0..y1 {
            for x in x0..x1 {
                luma[y * W + x] = 128;
            }
        }
        (luma, (1.0 - ph) / 2.0 * 100.0, (1.0 - pw) / 2.0 * 100.0)
    }

    #[test]
    fn five_to_one_letterboxed_crops_exactly() {
        let r = crop(detect_luma(&frame(5.0)));
        assert_eq!(r.shape, "5:1");
        assert!(close(r.inset.top, 32.22, 1) && close(r.inset.bottom, 32.22, 1), "{r:?}");
        assert_eq!(r.inset.left, 0.0);
        assert_eq!(r.css(), "inset(32.22% 0% 32.22% 0%)");
    }

    #[test]
    fn sixteen_ten_pillarboxed() {
        let r = crop(detect_luma(&frame(1.6)));
        assert_eq!(r.shape, "16:10");
        assert!(close(r.inset.left, 5.0, 1) && close(r.inset.right, 5.0, 1));
        assert_eq!(r.inset.top, 0.0);
    }

    #[test]
    fn every_macbook_model_mirrored() {
        for (w, h) in [(1512.0, 982.0), (1728.0, 1117.0), (1470.0, 956.0), (1710.0, 1107.0)] {
            let r = crop(detect_luma(&frame(w / h)));
            assert_eq!(r.shape, "MacBook", "{w}x{h}");
            assert!(close(r.inset.left, 6.63, 1), "{w}x{h}: {r:?}");
            assert_eq!(r.inset.top, 0.0);
        }
    }

    #[test]
    fn dual_half_told_apart_from_21_9() {
        let half = crop(detect_luma(&frame(2920.0 / 1200.0)));
        assert_eq!(half.shape, "dual half");
        assert!(close(half.inset.top, 13.47, 1), "{half:?}");
        assert_eq!(crop(detect_luma(&frame(2918.0 / 1200.0))).shape, "dual half");
        assert_eq!(crop(detect_luma(&frame(2560.0 / 1080.0))).shape, "21:9");
        assert_eq!(crop(detect_luma(&frame(3440.0 / 1440.0))).shape, "21:9");
    }

    #[test]
    fn windowbox_cropped_on_all_sides_never_into_the_picture() {
        for (ratio, screen) in [(1512.0 / 982.0, 2920.0 / 1200.0), (1512.0 / 982.0, 5.0), (1.6, 5.0)] {
            let (luma, top, left) = windowboxed(ratio, screen);
            let r = crop(detect_luma(&luma));
            assert!(r.windowbox, "{ratio:.2} in {screen:.2}");
            assert!(r.inset.top <= top + 50.0 / H as f64, "{r:?} vs top {top}");
            assert!(r.inset.left <= left + 50.0 / W as f64, "{r:?} vs left {left}");
            assert!(top - r.inset.top < 1.0 && left - r.inset.left < 1.0, "{r:?} vs {top},{left}");
        }
    }

    #[test]
    fn one_axis_crop_when_only_one_pair_of_bars() {
        assert!(!crop(detect_luma(&frame(1512.0 / 982.0))).windowbox);
        let five = crop(detect_luma(&frame(5.0)));
        assert!(!five.windowbox);
        assert_eq!(five.inset.left, 0.0);
    }

    #[test]
    fn three_two_and_sixteen_ten_told_apart_from_a_macbook() {
        assert_eq!(crop(detect_luma(&frame(1.5))).shape, "3:2");
        assert_eq!(crop(detect_luma(&frame(1.6))).shape, "16:10");
    }

    #[test]
    fn native_16_9_fills_the_frame() {
        assert_eq!(detect_luma(&frame(16.0 / 9.0)), Some(Detection::Full));
    }

    #[test]
    fn every_known_shape_snaps_to_itself() {
        let frame_ratio = FW as f64 / FH as f64;
        for s in SHAPES {
            let d = detect_luma(&frame(s.ratio));
            if (s.ratio - frame_ratio).abs() / frame_ratio <= SHAPE_TOLERANCE {
                assert_eq!(d, Some(Detection::Full), "{}", s.name);
            } else {
                assert_eq!(crop(d).shape, s.name);
            }
        }
    }

    #[test]
    fn all_black_is_undecidable() {
        assert_eq!(detect_luma(&vec![0u8; W * H]), None);
    }

    #[test]
    fn a_dark_picture_is_not_bars() {
        // A black slide with a title near the top: lopsided "bars".
        let luma = frame_with(16.0 / 9.0, |x, y, _, _| if y < 8 && x > 20 && x < 70 { 200 } else { 4 });
        assert_eq!(detect_luma(&luma), None);
    }

    #[test]
    fn dark_picture_edges_do_not_move_the_crop() {
        let luma = frame_with(5.0, |_, y, _, ch| if y == 0 || y == ch - 1 { 18 } else { 120 });
        assert_eq!(crop(detect_luma(&luma)).shape, "5:1");
    }

    #[test]
    fn an_unknown_shape_is_not_cropped() {
        assert_eq!(detect_luma(&frame(2.8)), None);
    }

    #[test]
    fn luma_grid_reads_real_pixels() {
        assert_eq!(luma_grid(&[0, 0, 0, 255, 255, 255, 255, 255]), [0, 255]);
    }

    fn five() -> Detection {
        detect_luma(&frame(5.0)).unwrap()
    }

    #[test]
    fn tracker_changes_only_after_repeats() {
        let mut t = CropTracker::default();
        for _ in 1..STABLE_SAMPLES {
            assert!(!t.update(Some(&five())));
        }
        assert!(t.current().is_none());
        assert!(t.update(Some(&five())));
        assert_eq!(t.current().unwrap().shape, "5:1");
    }

    #[test]
    fn tracker_not_reset_by_an_undecidable_sample() {
        let mut t = CropTracker::default();
        t.update(Some(&five()));
        t.update(None);
        for _ in 1..STABLE_SAMPLES - 1 {
            t.update(Some(&five()));
        }
        assert!(t.update(Some(&five())));
    }

    #[test]
    fn tracker_goes_back_to_uncropped() {
        let mut t = CropTracker::default();
        for _ in 0..STABLE_SAMPLES {
            t.update(Some(&five()));
        }
        for _ in 0..STABLE_SAMPLES - 1 {
            assert!(!t.update(Some(&Detection::Full)));
        }
        assert!(t.update(Some(&Detection::Full)));
        assert!(t.current().is_none());
    }

    #[test]
    fn tracker_starts_over_when_results_alternate() {
        let mut t = CropTracker::default();
        let (five, tenth) = (five(), detect_luma(&frame(1.6)).unwrap());
        for i in 0..10 {
            assert!(!t.update(Some(if i % 2 == 1 { &five } else { &tenth })));
        }
        assert!(t.current().is_none());
    }

    #[test]
    fn tracker_ignores_windowbox_jitter_but_follows_a_real_move() {
        let mut t = CropTracker::default();
        let mk = |top, side| Detection::Crop(Crop {
            shape: "MacBook", ratio: 1.542, windowbox: true, measured: 1.54,
            inset: Inset { top, right: side, bottom: top, left: side },
        });
        let (a, b) = (mk(13.4, 18.2), mk(13.9, 18.0));
        for _ in 0..STABLE_SAMPLES {
            t.update(Some(&a));
        }
        for i in 0..10 {
            assert!(!t.update(Some(if i % 2 == 1 { &a } else { &b })));
        }
        let moved = mk(20.0, 25.0);
        for _ in 1..STABLE_SAMPLES {
            assert!(!t.update(Some(&moved)));
        }
        assert!(t.update(Some(&moved)));
    }

    #[test]
    fn reset_drops_the_crop() {
        let mut t = CropTracker::default();
        for _ in 0..STABLE_SAMPLES {
            t.update(Some(&five()));
        }
        t.reset();
        assert!(t.current().is_none());
    }

    #[test]
    fn uv_matches_the_inset() {
        let (min, max) = crop(detect_luma(&frame(5.0))).uv();
        assert_eq!(min[0], 0.0);
        assert!((min[1] - 0.3222).abs() < 0.001 && (max[1] - 0.6778).abs() < 0.001);
    }
}
