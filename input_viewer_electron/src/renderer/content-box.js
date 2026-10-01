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
  /** Thumbnail the detector reads. 54 rows resolves a bar to ~2% of the frame. */
  SAMPLE_W: 96,
  SAMPLE_H: 54,
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
}

/**
 * Shapes a source is snapped to. 5:1 is the wall; the rest are what laptops,
 * cameras and players commonly send.
 */
export const SHAPES = [
  { name: '5:1', ratio: 5 },
  { name: '32:9', ratio: 32 / 9 },
  { name: '21:9', ratio: 64 / 27 },
  { name: '2:1', ratio: 2 },
  { name: '16:9', ratio: 16 / 9 },
  { name: '16:10', ratio: 16 / 10 },
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
function rowActive(luma, w, y) {
  let n = 0
  for (let x = 0; x < w; x++) if (luma[y * w + x] > CROP.BLACK_MAX) n++
  return n > w * CROP.ACTIVE_SHARE
}

function colActive(luma, w, h, x, y0 = 0, y1 = h) {
  let n = 0
  for (let y = y0; y < y1; y++) if (luma[y * w + x] > CROP.BLACK_MAX) n++
  return n > (y1 - y0) * CROP.ACTIVE_SHARE
}

/**
 * The pure-black margin on each side of a thumbnail, as shares of the frame.
 * Null when the whole frame is black: there is nothing to measure against.
 */
export function measureBars(luma, w, h) {
  if (!luma || w <= 0 || h <= 0) return null
  let top = 0
  while (top < h && !rowActive(luma, w, top)) top++
  if (top === h) return null
  let bottom = h - 1
  while (bottom > top && !rowActive(luma, w, bottom)) bottom--
  let left = 0
  while (left < w && !colActive(luma, w, h, left)) left++
  let right = w - 1
  while (right > left && !colActive(luma, w, h, right)) right--
  return {
    top: top / h,
    bottom: (h - 1 - bottom) / h,
    left: left / w,
    right: (w - 1 - right) / w,
    // Thumbnail rows/columns, for callers that need the exact edges.
    rows: { top, bottom },
    cols: { left, right },
  }
}

/** The exact, centred crop that leaves a picture of `ratio` in a frame of `frameRatio`. */
export function boxForShape(shape, frameRatio) {
  let inset
  if (shape.ratio > frameRatio) {
    const v = (1 - frameRatio / shape.ratio) / 2 * 100
    inset = { top: v, right: 0, bottom: v, left: 0 }
  } else {
    const v = (1 - shape.ratio / frameRatio) / 2 * 100
    inset = { top: 0, right: v, bottom: 0, left: v }
  }
  return { shape: shape.name, ratio: shape.ratio, inset }
}

export function detectContentBox(luma, w, h, frameW, frameH) {
  if (!luma || w <= 0 || h <= 0 || !frameW || !frameH) return null

  const bars = measureBars(luma, w, h)
  if (!bars) return null // all black: nothing to judge by
  const { top, bottom } = bars.rows
  const { left, right } = bars.cols

  // Bar sizes as shares of the frame.
  const barT = bars.top
  const barB = bars.bottom
  const barL = bars.left
  const barR = bars.right
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

  // The exact, centred crop for that shape -- not the measured one, which is
  // only as precise as the thumbnail.
  return boxForShape(shape, frameRatio)
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
  const key = (c) => (c && c !== 'none' ? c.shape : c)

  return {
    /**
     * Feed one detection. Undecidable samples (null) neither confirm nor
     * reset a candidate, so one black frame mid-change costs nothing.
     * @returns {boolean} true when the crop changed
     */
    update(detected) {
      if (detected === null) return false
      if (key(detected) === key(current)) {
        candidate = null
        count = 0
        return false
      }
      if (key(detected) === key(candidate)) {
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

// =============================================================================
// Saved boxes (#316)
// =============================================================================
//
// Automatic detection re-measures every picture, and pure-black symmetric
// content -- a black slide, a screensaver -- is indistinguishable from the
// card's bars in one frame, so the crop resized with the content. What IS
// reliable: a given source resolution always produces exactly the same bars.
//
// So in "saved" mode a side only ever switches between boxes saved for its
// input (or the full frame), with two rules:
//   enter a box  only when the bars match it on every side, STABLE_SAMPLES times;
//   leave a box  only when picture appears in the area it crops away,
//                STABLE_SAMPLES times. Black inside the box can never move it.

/** Boxes an input gets until someone edits its list: what the wall's laptops send. */
export const DEFAULT_SAVED_SHAPES = ['5:1', '16:10']

/** How far measured bars may be from a saved box, as a share of the frame. One thumbnail row is ~1.9%. */
export const MATCH_TOLERANCE = 0.025

/** Shape by name, or null. */
export function shapeByName(name) {
  return SHAPES.find(s => s.name === name) || null
}

/** Do the measured bars match this box on every side? */
export function barsMatchBox(bars, box) {
  if (!bars || !box) return false
  const { top, right, bottom, left } = box.inset
  return Math.abs(bars.top - top / 100) <= MATCH_TOLERANCE &&
    Math.abs(bars.bottom - bottom / 100) <= MATCH_TOLERANCE &&
    Math.abs(bars.left - left / 100) <= MATCH_TOLERANCE &&
    Math.abs(bars.right - right / 100) <= MATCH_TOLERANCE
}

/**
 * Is there picture in the area this box crops away?
 *
 * One thumbnail row/column inside the box's edge is skipped: the smooth
 * downscale blends the picture's first row into the bar's last one, and that
 * blend is not picture outside the box.
 */
export function pictureOutside(luma, w, h, box) {
  const { top, right, bottom, left } = box.inset
  const yTop = Math.floor(top / 100 * h) - 1        // rows [0, yTop) are outside
  const yBot = h - Math.floor(bottom / 100 * h) + 1 // rows [yBot, h) are outside
  const xL = Math.floor(left / 100 * w) - 1
  const xR = w - Math.floor(right / 100 * w) + 1
  for (let y = 0; y < yTop; y++) if (rowActive(luma, w, y)) return true
  for (let y = Math.max(0, yBot); y < h; y++) if (rowActive(luma, w, y)) return true
  for (let x = 0; x < xL; x++) if (colActive(luma, w, h, x)) return true
  for (let x = Math.max(0, xR); x < w; x++) if (colActive(luma, w, h, x)) return true
  return false
}

/**
 * Debounced crop for one side in "saved" mode.
 *
 * update() takes the raw thumbnail rather than a detection, because the two
 * rules ask different questions of it than detectContentBox does.
 */
export function createSavedBoxTracker() {
  let current = 'none'
  let pending = null   // { kind: 'enter'|'leave', key, count }
  let reason = null

  const keyOf = (b) => (b && b !== 'none' ? b.shape : 'none')
  const bump = (kind, key) => {
    if (pending && pending.kind === kind && pending.key === key) pending.count += 1
    else pending = { kind, key, count: 1 }
    return pending.count >= CROP.STABLE_SAMPLES
  }

  return {
    /**
     * @param {{luma: Uint8Array, w: number, h: number, frameRatio: number, uniform: boolean}} sample
     *   uniform: the frame is one flat colour -- black, or the driver's grey
     *   no-signal image -- and says nothing about bars.
     * @param {string[]} savedShapes
     * @returns {boolean} true when the crop changed; reason() says why
     */
    update(sample, savedShapes) {
      if (!sample || sample.uniform) return false
      const bars = measureBars(sample.luma, sample.w, sample.h)
      if (!bars) return false
      const boxes = savedShapes.map(shapeByName).filter(Boolean)
        .map(s => boxForShape(s, sample.frameRatio))
      const match = boxes.find(b => barsMatchBox(bars, b)) || null

      if (current === 'none') {
        if (!match) { pending = null; return false }
        if (!bump('enter', match.shape)) return false
        current = match
        reason = `matches saved ${match.shape} box`
        pending = null
        return true
      }

      // In a box: stay unless picture shows up where it is cropping.
      if (!pictureOutside(sample.luma, sample.w, sample.h, current)) {
        pending = null
        return false
      }
      const next = match && match.shape !== current.shape ? match : 'none'
      if (!bump('leave', keyOf(next))) return false
      reason = `picture outside the ${current.shape} box` +
        (next === 'none' ? '' : `, matches saved ${next.shape} box`)
      current = next
      pending = null
      return true
    },
    reset() {
      current = 'none'
      pending = null
      reason = null
    },
    current() {
      return current
    },
    reason() {
      return reason
    },
  }
}
