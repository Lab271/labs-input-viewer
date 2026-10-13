// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! YUY2 (packed 4:2:2) colour maths, BT.709 limited range.
//!
//! The wall's Elgato cards offer 3840x2160 only as YUY2. Converting that to
//! RGBA on one CPU core took ~49 ms a frame on the wall and capped capture at
//! ~12 fps, so the conversion moved to the GPU (`gpu_video.rs`). What stays on
//! the CPU is here:
//!
//! - `yuyv_to_rgba`, the shader's formula in Rust, so the colour maths has
//!   unit tests;
//! - `rgba_to_yuyv`, so the `--test-pattern` source can emit what the cards
//!   emit and exercise the same path -- split across cores, since a 4K frame
//!   at 60 fps on one core would be the bottleneck being measured;
//! - `luma_thumbnail`, so the health check reads brightness straight from the
//!   Y bytes, without converting the frame.
//!
//! BT.709 because HD and UHD HDMI video is 709. nokhwa's CPU path used BT.601
//! coefficients; the difference is a slight shift in greens and reds.
//!
//! Layout: every 4 bytes are `Y0 U Y1 V` for two horizontally adjacent pixels.

/// Kr = 0.2126, Kb = 0.0722; limited range: Y 16..235, Cb/Cr 16..240.
pub fn ycbcr_to_rgb(y: u8, cb: u8, cr: u8) -> [u8; 3] {
    let y = (f32::from(y) - 16.0) / 219.0;
    let cb = (f32::from(cb) - 128.0) / 224.0;
    let cr = (f32::from(cr) - 128.0) / 224.0;
    let r = y + 1.5748 * cr;
    let g = y - 0.1873 * cb - 0.4681 * cr;
    let b = y + 1.8556 * cb;
    [r, g, b].map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8)
}

pub fn rgb_to_ycbcr(r: u8, g: u8, b: u8) -> [u8; 3] {
    let (r, g, b) = (f32::from(r) / 255.0, f32::from(g) / 255.0, f32::from(b) / 255.0);
    let y = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    let cb = (b - y) / 1.8556;
    let cr = (r - y) / 1.5748;
    [
        (16.0 + 219.0 * y).round().clamp(0.0, 255.0) as u8,
        (128.0 + 224.0 * cb).round().clamp(0.0, 255.0) as u8,
        (128.0 + 224.0 * cr).round().clamp(0.0, 255.0) as u8,
    ]
}

/// What the GPU shader computes, on the CPU. Width must be even.
pub fn yuyv_to_rgba(yuyv: &[u8], w: usize, h: usize) -> Vec<u8> {
    let mut out = vec![0u8; w * h * 4];
    for (i, q) in yuyv[..w * h * 2].as_chunks::<4>().0.iter().enumerate() {
        let [y0, u, y1, v] = [q[0], q[1], q[2], q[3]];
        let [r0, g0, b0] = ycbcr_to_rgb(y0, u, v);
        let [r1, g1, b1] = ycbcr_to_rgb(y1, u, v);
        out[i * 8..i * 8 + 8].copy_from_slice(&[r0, g0, b0, 255, r1, g1, b1, 255]);
    }
    out
}

/// RGBA to YUY2, chroma averaged over each pixel pair, rows split across
/// threads. Width must be even.
pub fn rgba_to_yuyv(rgba: &[u8], w: usize, h: usize) -> Vec<u8> {
    let mut out = vec![0u8; w * h * 2];
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).min(16);
    let rows_per = h.div_ceil(threads).max(1);
    std::thread::scope(|scope| {
        for (chunk_index, out_rows) in out.chunks_mut(rows_per * w * 2).enumerate() {
            let src = &rgba[chunk_index * rows_per * w * 4..];
            scope.spawn(move || {
                let mut memo = PairMemo::default();
                for (i, q) in out_rows.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                    *q = memo.convert(&src[i * 8..i * 8 + 8]);
                }
            });
        }
    });
    out
}

/// One pixel pair to `Y0 U Y1 V`, remembering the last pair: flat colour
/// (the test pattern, a slide) repeats pairs, and a repeat costs a compare
/// instead of the float maths.
#[derive(Default)]
struct PairMemo {
    last: Option<([u8; 8], [u8; 4])>,
}

impl PairMemo {
    fn convert(&mut self, p: &[u8]) -> [u8; 4] {
        let key: [u8; 8] = p[..8].try_into().expect("8 bytes");
        if let Some((k, out)) = self.last
            && k == key
        {
            return out;
        }
        let [y0, u0, v0] = rgb_to_ycbcr(p[0], p[1], p[2]);
        let [y1, u1, v1] = rgb_to_ycbcr(p[4], p[5], p[6]);
        let u = (u16::from(u0) + u16::from(u1)).div_ceil(2) as u8;
        let v = (u16::from(v0) + u16::from(v1)).div_ceil(2) as u8;
        let out = [y0, u, y1, v];
        self.last = Some((key, out));
        out
    }
}

/// Convert only the rectangle `x0..x1, y0..y1` of an RGBA frame into an
/// existing YUY2 frame of the same size, rows split across threads. `x0` is
/// rounded down and `x1` up to whole pixel pairs.
pub fn rgba_rect_to_yuyv(rgba: &[u8], w: usize, (x0, y0, x1, y1): (usize, usize, usize, usize), out: &mut [u8]) {
    let (x0, x1) = (x0 & !1, (x1 + 1).min(w) & !1);
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    let rows = y1 - y0;
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).min(8);
    let rows_per = rows.div_ceil(threads).max(1);
    let region = &mut out[y0 * w * 2..y1 * w * 2];
    std::thread::scope(|scope| {
        for (chunk, out_rows) in region.chunks_mut(rows_per * w * 2).enumerate() {
            scope.spawn(move || {
                let mut memo = PairMemo::default();
                for (r, row) in out_rows.chunks_mut(w * 2).enumerate() {
                    let y = y0 + chunk * rows_per + r;
                    let src = &rgba[(y * w + x0) * 4..(y * w + x1) * 4];
                    for (i, q) in row[x0 * 2..x1 * 2].as_chunks_mut::<4>().0.iter_mut().enumerate() {
                        *q = memo.convert(&src[i * 8..i * 8 + 8]);
                    }
                }
            });
        }
    });
}

/// A grey RGBA thumbnail from the Y bytes alone, in full range (0..255) so
/// the health check's thresholds mean the same as for an RGBA frame.
/// Box-averaged like `health::thumbnail`, 4x4 samples per cell.
pub fn luma_thumbnail(yuyv: &[u8], w: usize, h: usize, tw: usize, th: usize) -> Vec<u8> {
    let mut out = vec![0u8; tw * th * 4];
    if w < 2 || h == 0 || yuyv.len() < w * h * 2 {
        return out;
    }
    const GRID: usize = 4;
    for ty in 0..th {
        let (y0, y1) = (ty * h / th, ((ty + 1) * h / th).max(ty * h / th + 1));
        for tx in 0..tw {
            let (x0, x1) = (tx * w / tw, ((tx + 1) * w / tw).max(tx * w / tw + 1));
            let mut acc = 0u32;
            for gy in 0..GRID {
                let y = y0 + (y1 - y0) * (2 * gy + 1) / (2 * GRID);
                for gx in 0..GRID {
                    let x = x0 + (x1 - x0) * (2 * gx + 1) / (2 * GRID);
                    // Y sits at byte 0 of each pixel's pair-half: 2 bytes a pixel.
                    acc += u32::from(yuyv[(y * w + x) * 2]);
                }
            }
            let mean = acc as f32 / (GRID * GRID) as f32;
            let full = ((mean - 16.0) * 255.0 / 219.0).round().clamp(0.0, 255.0) as u8;
            let o = (ty * tw + tx) * 4;
            out[o..o + 4].copy_from_slice(&[full, full, full, 255]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_colours_round_trip() {
        for rgb in [[0, 0, 0], [255, 255, 255], [255, 0, 0], [0, 255, 0], [0, 0, 255], [192, 192, 0], [16, 16, 20]] {
            let [y, cb, cr] = rgb_to_ycbcr(rgb[0], rgb[1], rgb[2]);
            let back = ycbcr_to_rgb(y, cb, cr);
            for c in 0..3 {
                assert!((i16::from(back[c]) - i16::from(rgb[c])).abs() <= 2, "{rgb:?} -> {back:?}");
            }
        }
    }

    #[test]
    fn limited_range_black_and_white() {
        assert_eq!(rgb_to_ycbcr(0, 0, 0), [16, 128, 128]);
        assert_eq!(rgb_to_ycbcr(255, 255, 255), [235, 128, 128]);
        assert_eq!(ycbcr_to_rgb(16, 128, 128), [0, 0, 0]);
        assert_eq!(ycbcr_to_rgb(235, 128, 128), [255, 255, 255]);
    }

    #[test]
    fn a_frame_round_trips_through_yuyv() {
        let (w, h) = (64, 36);
        let rgba = crate::capture::draw_mock_frame(1, 0.3, w, h);
        let back = yuyv_to_rgba(&rgba_to_yuyv(&rgba, w, h), w, h);
        // Colour is shared per pixel pair, so a pair straddling a bar edge
        // blends -- as on the real cards; that is 4:2:2. Every pair whose two
        // source pixels agree must come back within rounding.
        for (src, out) in rgba.as_chunks::<8>().0.iter().zip(back.as_chunks::<8>().0) {
            if src[..4] == src[4..] {
                let worst = src.iter().zip(out).map(|(a, b)| (i16::from(*a) - i16::from(*b)).abs()).max().unwrap();
                assert!(worst <= 2, "uniform pair {src:?} came back as {out:?}");
            }
        }
    }

    #[test]
    fn parallel_conversion_matches_single_rows() {
        // Rows split across threads must join seamlessly: compare a frame
        // whose height does not divide evenly.
        let (w, h) = (32, 37);
        let rgba = crate::capture::draw_mock_frame(2, 0.6, w, h);
        let yuyv = rgba_to_yuyv(&rgba, w, h);
        for y in [0, 18, 36] {
            let row = &rgba[y * w * 4..(y + 1) * w * 4];
            assert_eq!(&rgba_to_yuyv(row, w, 1)[..], &yuyv[y * w * 2..(y + 1) * w * 2], "row {y}");
        }
    }

    #[test]
    fn a_rectangle_converts_exactly_as_the_whole_frame_does() {
        let (w, h) = (64, 36);
        let rgba = crate::capture::draw_mock_frame(3, 0.4, w, h);
        let whole = rgba_to_yuyv(&rgba, w, h);
        // Start from a different frame, then convert a rect with odd edges.
        let mut out = rgba_to_yuyv(&crate::capture::draw_mock_frame(1, 0.9, w, h), w, h);
        rgba_rect_to_yuyv(&rgba, w, (5, 3, 41, 30), &mut out);
        for y in 0..h {
            for x in 0..w {
                let inside = (4..42).contains(&x) && (3..30).contains(&y);
                let o = (y * w + x) * 2;
                if inside {
                    assert_eq!(out[o], whole[o], "Y at {x},{y}");
                }
            }
        }
    }

    #[test]
    fn luma_thumbnail_reads_full_range() {
        let (w, h) = (64, 36);
        let black: Vec<u8> = (0..w * h / 2).flat_map(|_| [16, 128, 16, 128]).collect();
        let white: Vec<u8> = (0..w * h / 2).flat_map(|_| [235, 128, 235, 128]).collect();
        assert_eq!(luma_thumbnail(&black, w, h, 8, 4)[0], 0);
        assert_eq!(luma_thumbnail(&white, w, h, 8, 4)[0], 255);
    }
}
