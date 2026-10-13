// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! Spatial mode: the wall's colours on the fixtures nearest them. Ported
//! from artnet-sync.js with artnet-spatial.test.js.
//!
//! The reactive modes reduce the frame to one colour, which throws away
//! what makes several savers worth watching -- Matrix Rain's columns, a
//! dive's hot centre against cold edges. The wall runs along one edge of the
//! room, so a strip at room-x 4 m sits under the part of the picture at the
//! same horizontal position. Only that axis is mapped (see
//! `DEFAULT_SPOT_DEPTH`).
//!
//! Cost is the constraint. There is no bulk write -- 40 strips is 40 POSTs
//! -- and this app has already cost the wall 40x once by being casual about
//! per-frame work. So each tick sends only the strips whose colour moved,
//! most-changed first, up to a hard cap.

use std::collections::HashMap;

use serde_json::Value;

use super::colour::{Rgb, SampleGrid, round_u8};

/// Strips written per update, at most. 40 POSTs a second is not an option.
pub const SPATIAL_MAX_WRITES: usize = 8;
/// Per-channel change below which a strip is not rewritten: slightly above
/// what is visible on a diffuse fixture, so a near-static saver settles and
/// stops consuming the budget.
pub const SPATIAL_CHANGE_THRESHOLD: f64 = 6.0;
/// Fade asked of each strip. Longer than the refresh so motion stays smooth.
pub const SPATIAL_TRANSITION_MS: u64 = 1200;

/// One `GET /layout` entry, read leniently: a non-numeric coordinate is
/// absent rather than a parse failure.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Strip {
    pub name: Option<String>,
    pub pixels: usize,
    pub x: Option<f64>,
    pub x1: Option<f64>,
    pub x2: Option<f64>,
}

impl Strip {
    pub fn from_value(v: &Value) -> Option<Self> {
        let o = v.as_object()?;
        let num = |k: &str| o.get(k).and_then(Value::as_f64).filter(|x| x.is_finite());
        Some(Self {
            name: o.get("name").and_then(Value::as_str).map(str::to_owned),
            pixels: num("pixels").map_or(1, |p| p.max(1.0) as usize),
            x: num("x"),
            x1: num("x1"),
            x2: num("x2"),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Extent {
    pub min: f64,
    pub max: f64,
}

pub type Pixels = Vec<[u8; 3]>;

#[derive(Debug, Clone, PartialEq)]
pub struct PlanItem {
    pub name: String,
    pub pixels: Pixels,
    /// Largest per-channel change since last written; infinite if never.
    pub change: f64,
}

/// Each column's colour, luminance-weighted: most savers are mostly black,
/// and a flat mean would pull every column toward black and leave the room
/// a uniform dim wash -- the failure this mode exists to avoid.
pub fn column_profile(rgba: &[u8], grid: SampleGrid) -> Vec<Rgb> {
    let per_tile = grid.bytes_per_tile();
    let n = (grid.tile * grid.tile) as f64;
    (0..grid.tiles_x)
        .map(|tx| {
            let (mut r, mut g, mut b, mut wsum) = (0.0, 0.0, 0.0, 0.0);
            for ty in 0..grid.tiles_y {
                let base = (ty * grid.tiles_x + tx) * per_tile;
                let Some(tile) = rgba.get(base..base + per_tile) else { continue };
                let (mut tr, mut tg, mut tb) = (0.0, 0.0, 0.0);
                for p in tile.as_chunks::<4>().0 {
                    tr += f64::from(p[0]);
                    tg += f64::from(p[1]);
                    tb += f64::from(p[2]);
                }
                let (tr, tg, tb) = (tr / n, tg / n, tb / n);
                let lum = (0.2126 * tr + 0.7152 * tg + 0.0722 * tb) / 255.0;
                let w = lum + 0.02; // a floor, so an all-black column still averages
                r += tr * w;
                g += tg * w;
                b += tb * w;
                wsum += w;
            }
            if wsum > 0.0 {
                Rgb { r: round_u8(r / wsum), g: round_u8(g / wsum), b: round_u8(b / wsum) }
            } else {
                Rgb::default()
            }
        })
        .collect()
}

/// The profile at a fractional horizontal position, interpolated: eight
/// columns across a 12 m room is a sample every 1.5 m, and nearest-column
/// would step visibly where the picture is smooth. Outside the outer
/// column centres it holds the end colour.
pub fn sample_profile(profile: &[Rgb], fx: f64) -> Rgb {
    let Some(last) = profile.len().checked_sub(1) else { return Rgb::default() };
    let pos = fx * profile.len() as f64 - 0.5;
    let i0 = pos.floor();
    let t = pos - i0;
    let at = |i: f64| profile[(i.max(0.0) as usize).min(last)];
    let (a, b) = (at(i0), at(i0 + 1.0));
    let lerp = |a: u8, b: u8| round_u8(f64::from(a) + (f64::from(b) - f64::from(a)) * t);
    Rgb { r: lerp(a.r, b.r), g: lerp(a.g, b.g), b: lerp(a.b, b.b) }
}

/// The horizontal span the fixtures occupy -- not the room's: this rig
/// spans x 1.5..10.5 of a 12 m room, and normalising against the room would
/// map the outer eighth of the picture onto bare floor.
pub fn strip_extent(strips: &[Strip]) -> Option<Extent> {
    let xs = strips.iter().flat_map(|s| [s.x1, s.x2, s.x]).flatten();
    let (min, max) = xs.fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), x| (lo.min(x), hi.max(x)));
    (min.is_finite() && max.is_finite() && max - min >= 1e-6).then_some(Extent { min, max })
}

/// One strip's pixel colours along its own length, x1 to x2 -- not min to
/// max, so the wiring direction is respected: `angle: 180` gives x1 > x2 and
/// pixel 1 belongs at x1. Reversed, half the rig would look wired backwards.
pub fn strip_gradient(strip: &Strip, profile: &[Rgb], extent: Extent) -> Pixels {
    let n = strip.pixels.max(1);
    let span = extent.max - extent.min;
    let x1 = strip.x1.or(strip.x).unwrap_or(extent.min);
    let x2 = strip.x2.or(strip.x).unwrap_or(extent.min);
    (0..n)
        .map(|i| {
            // Pixel centres, so a strip's ends are not pinned to its extremes.
            let along = if n == 1 { 0.5 } else { (i as f64 + 0.5) / n as f64 };
            let x = x1 + (x2 - x1) * along;
            let c = sample_profile(profile, (x - extent.min) / span);
            [c.r, c.g, c.b]
        })
        .collect()
}

/// Which strips to write this tick: the most-changed first, capped, having
/// dropped everything that has not moved perceptibly. A saver with all its
/// action in one place spends the whole budget there. Ties break by name so
/// a rig on a fixed picture does not shuffle what it refreshes.
pub fn spatial_plan(strips: &[Strip], profile: &[Rgb], extent: Extent, last_sent: &HashMap<String, Pixels>,
    max: usize, threshold: f64) -> Vec<PlanItem> {
    let mut candidates: Vec<PlanItem> = strips.iter()
        .filter_map(|st| {
            let name = st.name.clone()?;
            let pixels = strip_gradient(st, profile, extent);
            let change = match last_sent.get(&name) {
                Some(prev) if prev.len() == pixels.len() => pixels.iter().zip(prev)
                    .flat_map(|(a, b)| (0..3).map(move |c| f64::from(a[c].abs_diff(b[c]))))
                    .fold(0.0, f64::max),
                _ => f64::INFINITY,
            };
            (change >= threshold).then_some(PlanItem { name, pixels, change })
        })
        .collect();
    candidates.sort_by(|a, b| b.change.total_cmp(&a.change).then_with(|| a.name.cmp(&b.name)));
    candidates.truncate(max);
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::colour::GRID;
    use super::super::colour::tests::grid;
    use serde_json::json;

    /// Real entries from the rig's GET /layout, both orientations.
    fn layout() -> Vec<Strip> {
        [
            json!({ "name": "u0_01", "pixels": 8, "angle": 180, "x": 6.0, "y": 4.5, "x1": 6.5, "y1": 4.5, "x2": 5.5, "y2": 4.5 }),
            json!({ "name": "u0_02", "pixels": 8, "angle": 90, "x": 6.5, "y": 5.0, "x1": 6.5, "y1": 4.5, "x2": 6.5, "y2": 5.5 }),
            json!({ "name": "u0_05", "pixels": 8, "angle": 0, "x": 5.0, "y": 6.5, "x1": 4.5, "y1": 6.5, "x2": 5.5, "y2": 6.5 }),
            json!({ "name": "u1_01", "pixels": 8, "angle": 0, "x": 1.5, "y": 3.0, "x1": 1.5, "y1": 3.0, "x2": 2.5, "y2": 3.0 }),
            json!({ "name": "u4_08", "pixels": 8, "angle": 180, "x": 10.5, "y": 5.5, "x1": 10.5, "y1": 5.5, "x2": 9.5, "y2": 5.5 }),
        ].iter().filter_map(Strip::from_value).collect()
    }
    const EXTENT: Extent = Extent { min: 1.5, max: 10.5 };

    /// Green on the left half of the frame, red on the right.
    fn split_frame() -> Vec<u8> {
        grid(|tx, _| if tx < GRID.tiles_x / 2 { [0, 200, 0] } else { [200, 0, 0] })
    }
    fn profile() -> Vec<Rgb> {
        column_profile(&split_frame(), GRID)
    }
    fn plan(strips: &[Strip], p: &[Rgb], seen: &HashMap<String, Pixels>) -> Vec<PlanItem> {
        spatial_plan(strips, p, EXTENT, seen, SPATIAL_MAX_WRITES, SPATIAL_CHANGE_THRESHOLD)
    }
    fn written(items: &[PlanItem]) -> HashMap<String, Pixels> {
        items.iter().map(|i| (i.name.clone(), i.pixels.clone())).collect()
    }
    const fn rgb(r: u8, g: u8, b: u8) -> Rgb {
        Rgb { r, g, b }
    }

    // ---- columnProfile

    #[test]
    fn keeps_left_and_right_colours_apart() {
        let p = profile();
        assert_eq!(p.len(), GRID.tiles_x);
        assert!(p[0].g > 150 && p[0].r < 40);
        assert!(p[GRID.tiles_x - 1].r > 150 && p[GRID.tiles_x - 1].g < 40);
    }

    #[test]
    fn one_bright_tile_is_not_lost_down_a_dark_column() {
        let p = column_profile(&grid(|tx, ty| if (tx, ty) == (2, 1) { [0, 255, 0] } else { [0, 0, 0] }), GRID);
        assert!(p[2].g > 200, "{:?}", p[2]);
    }

    #[test]
    fn black_frame_gives_black_columns() {
        assert!(column_profile(&grid(|_, _| [0, 0, 0]), GRID).iter().all(|c| *c == Rgb::default()));
    }

    #[test]
    fn survives_a_short_buffer() {
        assert_eq!(column_profile(&vec![200; GRID.bytes_per_tile() * 3], GRID).len(), GRID.tiles_x);
    }

    // ---- sampleProfile

    const RAMP: [Rgb; 3] = [rgb(0, 0, 0), rgb(100, 100, 100), rgb(200, 200, 200)];

    #[test]
    fn interpolates_between_columns() {
        assert_eq!(sample_profile(&RAMP, 2.0 / 6.0).r, 50);
    }

    #[test]
    fn lands_exactly_on_a_column_at_its_centre() {
        assert_eq!(sample_profile(&RAMP, 1.0 / 6.0).r, 0);
        assert_eq!(sample_profile(&RAMP, 3.0 / 6.0).r, 100);
        assert_eq!(sample_profile(&RAMP, 5.0 / 6.0).r, 200);
    }

    #[test]
    fn holds_the_end_colour_outside_the_outer_centres() {
        assert_eq!(sample_profile(&RAMP, 0.0).r, 0);
        assert_eq!(sample_profile(&RAMP, 1.0).r, 200);
        assert_eq!(sample_profile(&RAMP, -3.0).r, 0);
        assert_eq!(sample_profile(&RAMP, 9.0).r, 200);
    }

    #[test]
    fn empty_profile_is_black() {
        assert_eq!(sample_profile(&[], 0.5), Rgb::default());
    }

    // ---- stripExtent

    #[test]
    fn measures_the_span_the_fixtures_occupy() {
        assert_eq!(strip_extent(&layout()), Some(EXTENT));
    }

    #[test]
    fn no_extent_when_nothing_is_usable() {
        assert_eq!(strip_extent(&[]), None);
        assert_eq!(strip_extent(&[Strip { name: Some("a".into()), ..Default::default() }]), None);
        let at4 = || Strip::from_value(&json!({ "x1": 4, "x2": 4 })).unwrap();
        assert_eq!(strip_extent(&[at4(), at4()]), None);
    }

    #[test]
    fn ignores_non_numeric_coordinates() {
        let s: Vec<Strip> = [json!({ "x1": "nope", "x2": 3 }), json!({ "x1": 7, "x2": null })]
            .iter().filter_map(Strip::from_value).collect();
        assert_eq!(strip_extent(&s), Some(Extent { min: 3.0, max: 7.0 }));
    }

    // ---- stripGradient

    #[test]
    fn respects_the_wiring_direction_so_pixel_1_sits_at_x1() {
        let px = strip_gradient(&layout()[0], &profile(), EXTENT);
        assert_eq!(px.len(), 8);
        assert!(px[0][0] > px[7][0], "pixel 1 redder");
        assert!(px[7][1] > px[0][1], "pixel 8 greener");
    }

    #[test]
    fn runs_the_other_way_for_a_strip_wired_the_other_way() {
        let mirror = Strip { name: Some("mirror".into()), pixels: 8, x1: Some(5.5), x2: Some(6.5), x: None };
        let px = strip_gradient(&mirror, &profile(), EXTENT);
        assert!(px[7][0] > px[0][0] && px[0][1] > px[7][1]);
        let mut forward = strip_gradient(&layout()[0], &profile(), EXTENT);
        forward.reverse();
        assert_eq!(px, forward);
    }

    #[test]
    fn gives_a_vertical_strip_one_flat_colour() {
        let px = strip_gradient(&layout()[1], &profile(), EXTENT);
        assert!(px.iter().all(|p| *p == px[0]));
    }

    #[test]
    fn picks_up_the_colour_above_where_the_strip_is() {
        let p = profile();
        assert!(strip_gradient(&layout()[3], &p, EXTENT).iter().all(|c| c[1] > c[0]), "green end");
        assert!(strip_gradient(&layout()[4], &p, EXTENT).iter().all(|c| c[0] > c[1]), "red end");
    }

    #[test]
    fn one_entry_per_pixel() {
        for st in layout() {
            assert_eq!(strip_gradient(&st, &profile(), EXTENT).len(), st.pixels);
        }
        let single = Strip { name: Some("x".into()), pixels: 1, x1: Some(6.0), x2: Some(6.0), x: None };
        assert_eq!(strip_gradient(&single, &profile(), EXTENT).len(), 1);
    }

    // ---- spatialPlan

    #[test]
    fn writes_everything_the_first_time_round() {
        assert_eq!(plan(&layout(), &profile(), &HashMap::new()).len(), layout().len());
    }

    #[test]
    fn skips_strips_whose_colour_has_not_moved() {
        let seen = written(&plan(&layout(), &profile(), &HashMap::new()));
        assert!(plan(&layout(), &profile(), &seen).is_empty());
    }

    #[test]
    fn never_exceeds_the_write_cap() {
        let many: Vec<Strip> = (0..40).map(|i| Strip {
            name: Some(format!("s{i:02}")), pixels: 8,
            x1: Some(1.5 + i as f64 * 0.22), x2: Some(1.5 + i as f64 * 0.22 + 0.5), x: None,
        }).collect();
        assert_eq!(plan(&many, &profile(), &HashMap::new()).len(), SPATIAL_MAX_WRITES);
    }

    #[test]
    fn spends_the_budget_on_the_strips_that_moved_most() {
        let seen = written(&plan(&layout(), &profile(), &HashMap::new()));
        let flipped = column_profile(&grid(|tx, _| if tx < GRID.tiles_x / 2 { [200, 0, 0] } else { [0, 200, 0] }), GRID);
        let all = spatial_plan(&layout(), &flipped, EXTENT, &seen, 99, SPATIAL_CHANGE_THRESHOLD);
        let capped = spatial_plan(&layout(), &flipped, EXTENT, &seen, 2, SPATIAL_CHANGE_THRESHOLD);
        assert_eq!(capped.len(), 2);
        let chosen = capped.iter().map(|p| p.change).fold(f64::INFINITY, f64::min);
        assert!(all[2..].iter().all(|p| chosen >= p.change));
        assert!(all.windows(2).all(|w| w[0].change >= w[1].change));
    }

    #[test]
    fn honours_the_change_threshold() {
        let seen = written(&plan(&layout(), &profile(), &HashMap::new()));
        let shift = |d: u8| profile().iter().map(|c| Rgb { r: c.r.saturating_add(d), ..*c }).collect::<Vec<_>>();
        assert!(plan(&layout(), &shift(SPATIAL_CHANGE_THRESHOLD as u8 - 2), &seen).is_empty());
        assert!(!plan(&layout(), &shift(SPATIAL_CHANGE_THRESHOLD as u8 + 20), &seen).is_empty());
    }

    #[test]
    fn is_deterministic_when_changes_tie() {
        let names = || plan(&layout(), &profile(), &HashMap::new()).into_iter().map(|p| p.name).collect::<Vec<_>>();
        assert_eq!(names(), names());
    }

    #[test]
    fn ignores_entries_without_a_name() {
        let mut strips = layout();
        strips.extend(Strip::from_value(&json!({ "pixels": 8, "x1": 3, "x2": 4 })));
        strips.extend(Strip::from_value(&Value::Null));
        assert_eq!(plan(&strips, &profile(), &HashMap::new()).len(), layout().len());
    }
}
