// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! Reducing a sampled frame to what the fixtures need: one colour, or where
//! the light is. A line-for-line port of artnet-sync.js `dominantColour`,
//! `hueToRgb255`, `luminanceFocus` and `spotDiameter`, with its tests.
//!
//! The input is always the sample grid's layout (see [`GRID`]): tiles
//! row-major, `ty` outer and `tx` inner, each tile's 8x8 RGBA bytes
//! contiguous, and **`ty = 0` the BOTTOM row** because GL reads bottom-up.

/// gl-base.js `SAMPLE_GRID`: an 8x4 grid of 8x8 tiles spread across the frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SampleGrid {
    pub tile: usize,
    pub tiles_x: usize,
    pub tiles_y: usize,
}

pub const GRID: SampleGrid = SampleGrid { tile: 8, tiles_x: 8, tiles_y: 4 };

impl SampleGrid {
    pub const fn bytes_per_tile(self) -> usize {
        self.tile * self.tile * 4
    }
    pub const fn bytes(self) -> usize {
        self.bytes_per_tile() * self.tiles_x * self.tiles_y
    }
}

/// Spot diameter range, in the relay's normalised field units. A single
/// bright filament gives a tight spot, a full-frame wash a broad one.
pub const SPOT_DIAMETER_MIN: f64 = 0.18;
pub const SPOT_DIAMETER_MAX: f64 = 0.8;
/// Edge softness asked of the spot. Mid-range: a hard edge reads as a fault.
pub const SPOT_SOFTNESS: f64 = 0.55;
/// Where in the room the spot sits, front to back. A setting, not something
/// taken from the frame: the wall is one edge of the room, so screen-x has a
/// real counterpart and screen-y does not ("why does the spot walk backwards
/// when the animation rises?").
pub const DEFAULT_SPOT_DEPTH: f64 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// A colour for the fixtures: fully saturated, with brightness separate.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Dominant {
    pub rgb: Rgb,
    /// 0..1.
    pub brightness: f64,
}

/// Where the light is: screen space, (0,0) top-left.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Focus {
    pub cx: f64,
    pub cy: f64,
    /// 0 for a point source, 1 for a full-frame wash.
    pub spread: f64,
    pub weight: f64,
}

/// JS `Math.round` for the non-negative values used here.
pub(crate) fn round_u8(v: f64) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

/// Trim float-division noise from a 0..1 field value. Without it a uniform
/// grid's centroid ships as 0.5000000000000001.
pub fn round4(v: f64) -> f64 {
    (v * 10000.0).round() / 10000.0
}

/// One colour for the room.
///
/// Weighted by saturation and then by brightness, both deliberately: a
/// plain mean over anything colourful converges on grey and would drive the
/// room to a dull neutral, while a frame that is 90% black with a bright
/// cyan filament should send cyan. Hue is a circular mean (summing unit
/// vectors), because a numeric mean of hues puts red-and-magenta at green.
/// Near-black pixels have no meaningful hue and do not vote.
///
/// Returned at full saturation with the frame's mean luminance as
/// brightness: the fixtures look best given a saturated colour and a
/// separate level -- a desaturated average makes the room look off rather
/// than dim.
pub fn dominant_colour(rgba: &[u8]) -> Dominant {
    let (mut sx, mut sy, mut sat_weight, mut lum_sum, mut samples) = (0.0, 0.0, 0.0, 0.0, 0usize);
    for px in rgba.as_chunks::<4>().0 {
        let (r, g, b) = (f64::from(px[0]) / 255.0, f64::from(px[1]) / 255.0, f64::from(px[2]) / 255.0);
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        lum_sum += lum;
        samples += 1;

        let chroma = max - min;
        if max <= 0.02 || chroma <= 0.01 {
            continue; // black or grey: no hue to vote
        }
        // Hue in turns, from the standard piecewise definition.
        let mut h = if max == r {
            ((g - b) / chroma) / 6.0
        } else if max == g {
            (2.0 + (b - r) / chroma) / 6.0
        } else {
            (4.0 + (r - g) / chroma) / 6.0
        };
        if h < 0.0 {
            h += 1.0;
        }
        // Saturated AND bright pixels carry the most.
        let sat = chroma / max;
        let w = sat * sat * lum;
        let a = h * std::f64::consts::TAU;
        sx += a.cos() * w;
        sy += a.sin() * w;
        sat_weight += w;
    }

    let mean_lum = if samples > 0 { lum_sum / samples as f64 } else { 0.0 };
    // So a dim scene still lights the room a little rather than reading as
    // off. Nothing subtle: sqrt.
    let brightness = mean_lum.max(0.0).sqrt().min(1.0);

    if sat_weight <= 1e-6 {
        // A monochrome frame -- white particles, the DVD logo on black.
        // Warm white, and let brightness carry it.
        return Dominant { rgb: Rgb { r: 255, g: 214, b: 170 }, brightness };
    }
    let mut hue = sy.atan2(sx) / std::f64::consts::TAU;
    if hue < 0.0 {
        hue += 1.0;
    }
    Dominant { rgb: hue_to_rgb255(hue), brightness }
}

/// Fully saturated, full-value RGB for a hue in turns.
pub fn hue_to_rgb255(h: f64) -> Rgb {
    // HSV->RGB at s=v=1. The `1 -` is not optional: without it every hue
    // comes out as its own complement, which looks like a wiring fault in the
    // fixtures rather than a maths error here.
    let f = |n: f64| {
        let k = (n + h * 6.0) % 6.0;
        let v = 1.0 - k.min(4.0 - k).clamp(0.0, 1.0);
        round_u8(v.clamp(0.0, 1.0) * 255.0)
    };
    Rgb { r: f(5.0), g: f(3.0), b: f(1.0) }
}

/// The luminance-weighted centroid of the grid, in **screen** coordinates,
/// and how spread out the light is horizontally.
///
/// THE Y FLIP IS LOAD-BEARING: the grid's first tile row is the bottom of
/// the picture. Reporting rows as-is would light the room upside-down,
/// which in a room looks like miswired fixtures rather than a sign error.
pub fn luminance_focus(rgba: &[u8], grid: SampleGrid) -> Focus {
    let per_tile = grid.bytes_per_tile();
    let (mut wx, mut wy, mut wsum, mut wxx) = (0.0, 0.0, 0.0, 0.0);
    for ty in 0..grid.tiles_y {
        for tx in 0..grid.tiles_x {
            let base = (ty * grid.tiles_x + tx) * per_tile;
            let Some(tile) = rgba.get(base..base + per_tile) else { continue };
            let lum: f64 = tile.as_chunks::<4>().0.iter()
                .map(|p| 0.2126 * f64::from(p[0]) + 0.7152 * f64::from(p[1]) + 0.0722 * f64::from(p[2]))
                .sum();
            // Mean tile luminance, squared so a bright accent pulls harder
            // than a large dim area -- the same reasoning as the saturation
            // weighting above.
            let mean = lum / (grid.tile * grid.tile) as f64 / 255.0;
            let w = mean * mean;
            if w <= 0.0 {
                continue;
            }
            let fx = (tx as f64 + 0.5) / grid.tiles_x as f64;
            let fy = 1.0 - (ty as f64 + 0.5) / grid.tiles_y as f64; // GL rows are bottom-up
            wx += fx * w;
            wy += fy * w;
            wxx += fx * fx * w;
            wsum += w;
        }
    }
    if wsum <= 1e-9 {
        // Black frame: centre and go wide, rather than parking the spot in a
        // corner because of float noise.
        return Focus { cx: 0.5, cy: 0.5, spread: 1.0, weight: 0.0 };
    }
    let cx = wx / wsum;
    let variance = (wxx / wsum - cx * cx).max(0.0);
    // A uniform frame has variance 1/12; normalised so a wash reads as 1.
    let spread = (variance.sqrt() / (1.0f64 / 12.0).sqrt()).min(1.0);
    Focus { cx, cy: wy / wsum, spread, weight: wsum.sqrt() }
}

/// Horizontal spread onto a spot diameter.
pub fn spot_diameter(spread: f64) -> f64 {
    let t = spread.clamp(0.0, 1.0);
    SPOT_DIAMETER_MIN + (SPOT_DIAMETER_MAX - SPOT_DIAMETER_MIN) * t
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn solid(r: u8, g: u8, b: u8, pixels: usize) -> Vec<u8> {
        [r, g, b, 255].repeat(pixels)
    }

    /// Mostly black, with `lit` pixels of one colour: the accent case.
    fn accent_on_black(r: u8, g: u8, b: u8, lit: usize) -> Vec<u8> {
        let mut buf = vec![0u8; 512 * 4];
        for px in buf.as_chunks_mut::<4>().0.iter_mut().take(lit) {
            *px = [r, g, b, 255];
        }
        buf
    }

    /// A sample-grid buffer, coloured per tile.
    pub fn grid(f: impl Fn(usize, usize) -> [u8; 3]) -> Vec<u8> {
        let mut buf = vec![0u8; GRID.bytes()];
        for ty in 0..GRID.tiles_y {
            for tx in 0..GRID.tiles_x {
                let [r, g, b] = f(tx, ty);
                let base = (ty * GRID.tiles_x + tx) * GRID.bytes_per_tile();
                for px in buf[base..base + GRID.bytes_per_tile()].as_chunks_mut::<4>().0 {
                    *px = [r, g, b, 255];
                }
            }
        }
        buf
    }

    pub fn one_tile(at_x: usize, at_y: usize, rgb: [u8; 3]) -> Vec<u8> {
        grid(|tx, ty| if tx == at_x && ty == at_y { rgb } else { [0, 0, 0] })
    }

    const WHITE: [u8; 3] = [255, 255, 255];

    fn close(a: f64, b: f64, digits: i32) -> bool {
        (a - b).abs() < 10f64.powi(-digits) / 2.0
    }

    // ---- artnet-sync.test.js: dominantColour

    #[test]
    fn returns_the_hue_of_a_solid_frame() {
        let Rgb { r, g, b } = dominant_colour(&solid(0, 200, 255, 512)).rgb;
        assert!(b > 200 && g > 150 && r < 80, "{r} {g} {b}");
    }

    #[test]
    fn sends_the_accent_not_the_average_on_a_mostly_black_frame() {
        let Rgb { r, g, b } = dominant_colour(&accent_on_black(255, 0, 200, 8)).rgb;
        assert!(r > 180 && b > 120 && g < 90, "{r} {g} {b}");
    }

    #[test]
    fn does_not_average_opposing_hues_into_green() {
        let mut buf = solid(255, 0, 0, 256);
        buf.extend(solid(0, 255, 255, 256));
        let Rgb { r, g, b } = dominant_colour(&buf).rgb;
        assert!(!(g > r && g > b), "{r} {g} {b}");
    }

    #[test]
    fn falls_back_to_warm_white_on_a_monochrome_frame() {
        let d = dominant_colour(&solid(180, 180, 180, 512));
        assert!(d.rgb.r > d.rgb.g && d.rgb.g > d.rgb.b);
        assert!(d.brightness > 0.5);
    }

    #[test]
    fn reports_brightness_from_the_whole_frame_not_the_accent() {
        let dim = dominant_colour(&accent_on_black(255, 0, 200, 4));
        let bright = dominant_colour(&solid(255, 0, 200, 512));
        assert!(dim.brightness < bright.brightness);
        assert!(dim.brightness >= 0.0);
    }

    #[test]
    fn black_frame_is_zero_brightness_without_dividing_by_zero() {
        assert_eq!(dominant_colour(&solid(0, 0, 0, 512)).brightness, 0.0);
    }

    #[test]
    fn handles_an_empty_buffer() {
        assert_eq!(dominant_colour(&[]).brightness, 0.0);
    }

    #[test]
    fn brightness_always_in_range() {
        for buf in [solid(0, 0, 0, 512), solid(255, 255, 255, 512), solid(12, 200, 7, 512),
            accent_on_black(3, 250, 190, 8), vec![0; 4]] {
            let b = dominant_colour(&buf).brightness;
            assert!((0.0..=1.0).contains(&b), "{b}");
        }
    }

    // ---- hueToRgb255

    #[test]
    fn hits_the_primaries_at_the_expected_turns() {
        assert_eq!(hue_to_rgb255(0.0), Rgb { r: 255, g: 0, b: 0 });
        assert_eq!(hue_to_rgb255(1.0 / 3.0), Rgb { r: 0, g: 255, b: 0 });
        assert_eq!(hue_to_rgb255(2.0 / 3.0), Rgb { r: 0, g: 0, b: 255 });
    }

    #[test]
    fn wraps_at_one() {
        assert_eq!(hue_to_rgb255(1.0), hue_to_rgb255(0.0));
    }

    // ---- artnet-spot.test.js: luminanceFocus

    #[test]
    fn flips_y_the_first_row_is_the_bottom_of_the_screen() {
        let bottom = luminance_focus(&one_tile(3, 0, WHITE), GRID);
        let top = luminance_focus(&one_tile(3, GRID.tiles_y - 1, WHITE), GRID);
        assert!(bottom.cy > 0.5 && top.cy < 0.5);
        assert!(close(bottom.cy, 1.0 - 0.5 / GRID.tiles_y as f64, 5));
        assert!(close(top.cy, 0.5 / GRID.tiles_y as f64, 5));
    }

    #[test]
    fn puts_cx_at_the_lit_column() {
        let n = GRID.tiles_x as f64;
        assert!(close(luminance_focus(&one_tile(0, 1, WHITE), GRID).cx, 0.5 / n, 5));
        assert!(close(luminance_focus(&one_tile(GRID.tiles_x - 1, 1, WHITE), GRID).cx, 1.0 - 0.5 / n, 5));
        assert!(close(luminance_focus(&one_tile(3, 1, WHITE), GRID).cx, 3.5 / n, 5));
    }

    #[test]
    fn weights_tiles_by_luminance_squared_not_linearly() {
        // White at fx 1/16, mid-grey at 15/16: squared weighting gives
        // 0.2386, linear would give 0.3549.
        let f = luminance_focus(&grid(|tx, ty| match (tx, ty) {
            (0, 1) => WHITE,
            (7, 1) => [128, 128, 128],
            _ => [0, 0, 0],
        }), GRID);
        assert!(close(f.cx, 0.2386, 3), "{}", f.cx);
    }

    #[test]
    fn a_wash_is_spread_and_a_point_is_concentrated() {
        assert!(luminance_focus(&grid(|_, _| WHITE), GRID).spread > 0.9);
        assert!(luminance_focus(&one_tile(4, 2, WHITE), GRID).spread < 0.05);
    }

    #[test]
    fn centres_and_widens_on_a_black_frame() {
        assert_eq!(luminance_focus(&grid(|_, _| [0, 0, 0]), GRID), Focus { cx: 0.5, cy: 0.5, spread: 1.0, weight: 0.0 });
    }

    #[test]
    fn ignores_a_short_buffer_instead_of_reading_past_the_end() {
        let short = vec![255u8; GRID.bytes_per_tile() * 3];
        assert!(luminance_focus(&short, GRID).cx.is_finite());
    }

    // ---- spotDiameter

    #[test]
    fn spot_diameter_maps_spread_onto_the_range_clamped() {
        assert!(close(spot_diameter(0.0), SPOT_DIAMETER_MIN, 6));
        assert!(close(spot_diameter(1.0), SPOT_DIAMETER_MAX, 6));
        assert!(close(spot_diameter(-5.0), SPOT_DIAMETER_MIN, 6));
        assert!(close(spot_diameter(5.0), SPOT_DIAMETER_MAX, 6));
        assert!(spot_diameter(0.5) > spot_diameter(0.25));
    }
}
