// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
/**
 * Where the picture actually is inside a capture frame: letterbox and pillarbox
 * detection, so the black bars can be cropped away.
 *
 * The Elgato 4K60 Pro MK.2 driver only offers standard capture formats, and the
 * wall's cards deliver 3840x2160 whatever the source sends. A source in any
 * other shape is fitted inside that 16:9 frame with its shape kept and the rest
 * filled with pure black: a laptop at 3840x768 (5:1, the shape of the wall)
 * arrives as a band a third of the frame tall. Without cropping, single view
 * then shows that 16:9 frame in the middle of the 5:1 wall, black all round.
 * Measured on the wall on 2026-10-01.
 *
 * The driver's own log says what it scaled from, but nothing reaches the
 * browser, so the bars are the only evidence. This module finds them in a
 * small luma thumbnail and decides the crop; renderer.js samples and applies it
 * (as `object-view-box`). Pure, so it is unit tested.
 *
 * Three guards keep a dark picture from being mistaken for bars:
 *   - a bar must be pure black across its WHOLE width (BLACK_MAX), and the
 *     driver's padding is exactly that; a dark slide is not;
 *   - the bars must be symmetric, because the driver centres the picture;
 *   - the result must land on a known shape (SHAPES) and repeat STABLE_SAMPLES
 *     times before the crop changes. A frame that is all black, or does not
 *     match, changes nothing.
 */

export const CROP = {
  /**
   * Thumbnail the detector reads. 216 rows resolves a bar to ~0.5% of the frame.
   * It was 96x54 (~2% a row), which could not tell 'dual half' (2.433:1) from
   * 21:9 (2.370:1): both measured 2.400. Reading 384x216 every 2 s is still
   * negligible work.
   */
  SAMPLE_W: 384,
  SAMPLE_H: 216,
  /** Luma (0-255) at or below which a pixel counts as bar. The padding is pure black. */
  BLACK_MAX: 10,
  /** A row/column is picture if more than this share of its pixels is above BLACK_MAX. */
  ACTIVE_SHARE: 0.02,
  /** Bars on opposite sides may differ by this share of the frame (the driver centres). */
  SYMMETRY_TOLERANCE: 0.04,
  /** Measured shape may differ from a known one by this share before it is rejected. */
  SHAPE_TOLERANCE: 0.06,
  /** Identical results needed in a row before the crop changes (~6 s at 2 s). */
  STABLE_SAMPLES: 3,
  /**
   * Bars of at least this share on BOTH axes make a windowbox: the picture sits
   * in a box inside a box. ~4 thumbnail columns / 2 rows.
   */
  WINDOWBOX_MIN_BAR: 0.01,
  /**
   * Insets within this many percent count as the same crop. A windowbox crop is
   * sized from a measurement, so it moves by a thumbnail pixel between samples;
   * without this the debounce would never see three identical results.
   */
  INSET_JITTER: 1.0,
}

/**
 * Shapes a source is snapped to. 5:1 is the wall; the rest are what laptops,
 * cameras and players commonly send.
 *
 * 'MacBook' is a mirrored MacBook screen. Its default "looks like" sizes are all
 * ~1.54:1 -- Pro 14" 1512x982, Pro 16" 1728x1117, Air 13" 1470x956, Air 15"
 * 1710x1107 -- and the wall's EDID offers each of them so mirroring is 1:1.
 * Without it a mirror matched no shape, so its bars were never cropped. It sits
 * between 3:2 and 16:10, which is why matching takes the NEAREST shape.
 *
 * 'dual half' is one half of the wall in dual view: (6000 - gap) / 2 by 1200,
 * offered by the EDID as 2920x1200 so an extended laptop display can fill a half.
 * It sits 2.6% from 21:9, close enough that the thumbnail's precision matters.
 */
export const SHAPES = [
  { name: '5:1', ratio: 5 },
  { name: '32:9', ratio: 32 / 9 },
  { name: 'dual half', ratio: 2920 / 1200 },
  { name: '21:9', ratio: 64 / 27 },
  { name: '2:1', ratio: 2 },
  { name: '16:9', ratio: 16 / 9 },
  { name: '16:10', ratio: 16 / 10 },
  { name: 'MacBook', ratio: 1.542 },
  { name: '3:2', ratio: 3 / 2 },
  { name: '4:3', ratio: 4 / 3 },
  { name: '5:4', ratio: 5 / 4 },
]

/** Rec. 601 luma per pixel of an RGBA buffer. */
export function lumaGrid(rgba) {
  const n = Math.floor(rgba.length / 4)
  const out = new Uint8Array(n)
  for (let i = 0; i < n; i++) {
    const o = i * 4
    out[i] = 0.299 * rgba[o] + 0.587 * rgba[o + 1] + 0.114 * rgba[o + 2]
  }
  return out
}

/**
 * Detect the picture inside a frame.
 *
 * @param {Uint8Array} luma  grid of w*h luma values
 * @param {number} w  grid width
 * @param {number} h  grid height
 * @param {number} frameW  real frame width (videoWidth), for the shape
 * @param {number} frameH  real frame height (videoHeight)
 * @returns {{shape: string, ratio: number, inset: {top:number,right:number,bottom:number,left:number}} | 'none' | null}
 *   a crop; 'none' when the picture fills the frame; null when undecidable
 *   (all black, asymmetric, or no known shape) -- callers keep what they had.
 */
export function detectContentBox(luma, w, h, frameW, frameH) {
  if (!luma || w <= 0 || h <= 0 || !frameW || !frameH) return null

  const rowActive = (y) => {
    let n = 0
    for (let x = 0; x < w; x++) if (luma[y * w + x] > CROP.BLACK_MAX) n++
    return n > w * CROP.ACTIVE_SHARE
  }
  const colActive = (x) => {
    let n = 0
    for (let y = 0; y < h; y++) if (luma[y * w + x] > CROP.BLACK_MAX) n++
    return n > h * CROP.ACTIVE_SHARE
  }

  let top = 0
  while (top < h && !rowActive(top)) top++
  if (top === h) return null // all black: nothing to judge by
  let bottom = h - 1
  while (bottom > top && !rowActive(bottom)) bottom--
  let left = 0
  while (left < w && !colActive(left)) left++
  let right = w - 1
  while (right > left && !colActive(right)) right--

  // Bar sizes as shares of the frame.
  const barT = top / h
  const barB = (h - 1 - bottom) / h
  const barL = left / w
  const barR = (w - 1 - right) / w
  if (Math.abs(barT - barB) > CROP.SYMMETRY_TOLERANCE) return null
  if (Math.abs(barL - barR) > CROP.SYMMETRY_TOLERANCE) return null

  // Shape of what is left, in real pixels.
  const contentW = (right - left + 1) / w * frameW
  const contentH = (bottom - top + 1) / h * frameH
  const measured = contentW / contentH
  const frameRatio = frameW / frameH

  // No bars worth the name: the picture fills the frame.
  if (Math.abs(measured - frameRatio) / frameRatio <= CROP.SHAPE_TOLERANCE) return 'none'

  // Nearest, not first within tolerance: 16:10 and 3:2 are only 6% apart.
  const off = (s) => Math.abs(measured - s.ratio) / s.ratio
  const shape = SHAPES.reduce((best, s) => (off(s) < off(best) ? s : best))
  if (off(shape) > CROP.SHAPE_TOLERANCE) return null

  // Black on BOTH axes: a windowbox. A Mac mirroring its 1.54:1 screen into a
  // wide mode (3840x768, 2920x1200) gets black at the sides from macOS, then
  // black above and below from the card fitting that wide picture into 16:9.
  // The one-axis box below would remove only one pair of bars and leave the
  // picture small in the middle. Seen in the mock setup on 2026-10-03:
  // content 1220x790 in 1920x1080, bars 13.4% top and bottom, 18.2% each side.
  const windowbox = Math.min(barT, barB) > CROP.WINDOWBOX_MIN_BAR &&
    Math.min(barL, barR) > CROP.WINDOWBOX_MIN_BAR

  let inset
  if (windowbox) {
    // The shape's box, sized to CONTAIN what was measured, so it never cuts
    // picture: at worst it leaves a sliver of black (the thumbnail counts a
    // blended edge row as picture). Not the exact box, because where the
    // picture sits inside the outer box is not something the shape alone says.
    let bw = contentW
    let bh = contentW / shape.ratio
    if (bh < contentH) {
      bh = contentH
      bw = contentH * shape.ratio
    }
    bw = Math.min(bw, frameW)
    bh = Math.min(bh, frameH)
    // Rounded DOWN: a smaller inset is a larger box, so rounding never cuts.
    const r1 = (v) => Math.floor(v * 10) / 10
    const sx = r1((1 - bw / frameW) / 2 * 100)
    const sy = r1((1 - bh / frameH) / 2 * 100)
    inset = { top: sy, right: sx, bottom: sy, left: sx }
  } else if (shape.ratio > frameRatio) {
    // The exact, centred crop for that shape -- not the measured one, which is
    // only as precise as the thumbnail.
    const v = (1 - frameRatio / shape.ratio) / 2 * 100
    inset = { top: v, right: 0, bottom: v, left: 0 }
  } else {
    const v = (1 - shape.ratio / frameRatio) / 2 * 100
    inset = { top: 0, right: v, bottom: 0, left: v }
  }
  // The measured ratio rides along for the log: it is what says why a shape
  // was chosen over its neighbour (3:2, MacBook and 16:10 are close together).
  return { shape: shape.name, ratio: shape.ratio, inset, measured, windowbox }
}

/** CSS for a crop: an `object-view-box` value, or '' for none. */
export function viewBoxCss(crop) {
  if (!crop || crop === 'none') return ''
  const p = (v) => `${Math.round(v * 100) / 100}%`
  const { top, right, bottom, left } = crop.inset
  return `inset(${p(top)} ${p(right)} ${p(bottom)} ${p(left)})`
}

/**
 * Debounces detections for one side, so the crop only moves on a result that
 * has repeated STABLE_SAMPLES times in a row.
 */
export function createCropTracker() {
  let current = 'none'
  let candidate = null
  let count = 0
  // Same shape, same kind, and insets within INSET_JITTER: the same crop.
  const same = (a, b) => {
    if (a === b) return true
    if (!a || !b || a === 'none' || b === 'none') return false
    if (a.shape !== b.shape || !!a.windowbox !== !!b.windowbox) return false
    return ['top', 'right', 'bottom', 'left']
      .every(k => Math.abs(a.inset[k] - b.inset[k]) <= CROP.INSET_JITTER)
  }

  return {
    /**
     * Feed one detection. Undecidable samples (null) neither confirm nor
     * reset a candidate, so one black frame mid-change costs nothing.
     * @returns {boolean} true when the crop changed
     */
    update(detected) {
      if (detected === null) return false
      if (same(detected, current)) {
        candidate = null
        count = 0
        return false
      }
      if (same(detected, candidate)) {
        count += 1
      } else {
        candidate = detected
        count = 1
      }
      if (count >= CROP.STABLE_SAMPLES) {
        current = candidate
        candidate = null
        count = 0
        return true
      }
      return false
    },
    /** Forget everything: a new stream starts uncropped. */
    reset() {
      current = 'none'
      candidate = null
      count = 0
    },
    /** 'none' or a crop. */
    current() {
      return current
    },
  }
}
