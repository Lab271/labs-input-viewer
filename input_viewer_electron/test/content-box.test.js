// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
import { describe, it, expect } from 'vitest'
import {
  CROP, SHAPES, lumaGrid, detectContentBox, viewBoxCss, createCropTracker,
} from '../src/renderer/content-box.js'

const W = CROP.SAMPLE_W
const H = CROP.SAMPLE_H
// The wall's cards always deliver this (measured 2026-10-01).
const FW = 3840
const FH = 2160

/**
 * A thumbnail of a source of `ratio` fitted, shape kept, into the 16:9 frame,
 * the way the Elgato driver does it: pure black around it.
 * `fill(x, y)` gives the picture's own luma (default: mid grey).
 */
function frame(ratio, fill = () => 128) {
  const luma = new Uint8Array(W * H)
  const frameRatio = FW / FH
  let cw = W, ch = H
  if (ratio > frameRatio) ch = Math.round(H * frameRatio / ratio)
  else cw = Math.round(W * ratio / frameRatio)
  const x0 = Math.round((W - cw) / 2), y0 = Math.round((H - ch) / 2)
  for (let y = y0; y < y0 + ch; y++) {
    for (let x = x0; x < x0 + cw; x++) luma[y * W + x] = fill(x - x0, y - y0, cw, ch)
  }
  return luma
}

const detect = (luma) => detectContentBox(luma, W, H, FW, FH)

describe('detectContentBox', () => {
  it('finds a 5:1 source letterboxed in the 16:9 frame, and crops exactly', () => {
    const r = detect(frame(5))
    expect(r.shape).toBe('5:1')
    // (1 - (16/9)/5) / 2 = 32.22% off the top and the bottom.
    expect(r.inset.top).toBeCloseTo(32.22, 1)
    expect(r.inset.bottom).toBeCloseTo(32.22, 1)
    expect(r.inset.left).toBe(0)
    expect(viewBoxCss(r)).toBe('inset(32.22% 0% 32.22% 0%)')
  })

  it('finds a 16:10 laptop pillarboxed in the 16:9 frame', () => {
    const r = detect(frame(1.6))
    expect(r.shape).toBe('16:10')
    expect(r.inset.left).toBeCloseTo(5, 1)
    expect(r.inset.right).toBeCloseTo(5, 1)
    expect(r.inset.top).toBe(0)
  })

  it('crops a mirrored MacBook screen, for every MacBook model', () => {
    // Default "looks like" sizes; the EDID offers each so mirroring is 1:1.
    for (const [w, h] of [[1512, 982], [1728, 1117], [1470, 956], [1710, 1107]]) {
      const r = detect(frame(w / h))
      expect(r?.shape, `${w}x${h}`).toBe('MacBook')
      // Pillarboxed in the 16:9 frame: (1 - 1.542/(16/9)) / 2 = 6.6% each side.
      expect(r.inset.left).toBeCloseTo(6.63, 1)
      expect(r.inset.top).toBe(0)
    }
  })

  it('crops the 2920x1200 dual-half mode, and tells it from 21:9', () => {
    // 2.433:1 against 2.370:1, 2.6% apart: what the 384x216 thumbnail is for.
    const half = detect(frame(2920 / 1200))
    expect(half?.shape).toBe('dual half')
    expect(half.inset.top).toBeCloseTo(13.47, 1)
    expect(detect(frame(2918 / 1200))?.shape).toBe('dual half')   // (6000 - 164) / 2
    expect(detect(frame(2560 / 1080))?.shape).toBe('21:9')
    expect(detect(frame(3440 / 1440))?.shape).toBe('21:9')
  })

  it('still tells 3:2 and 16:10 apart from a MacBook', () => {
    expect(detect(frame(3 / 2))?.shape).toBe('3:2')
    expect(detect(frame(1.6))?.shape).toBe('16:10')
  })

  it('reports a native 16:9 source (the Apple TV) as filling the frame', () => {
    expect(detect(frame(16 / 9))).toBe('none')
  })

  it('snaps every known shape to itself', () => {
    for (const s of SHAPES) {
      const r = detect(frame(s.ratio))
      if (Math.abs(s.ratio - FW / FH) / (FW / FH) <= CROP.SHAPE_TOLERANCE) {
        expect(r).toBe('none')
      } else {
        expect(r?.shape, s.name).toBe(s.name)
      }
    }
  })

  it('gives up on an all-black frame instead of guessing', () => {
    expect(detect(new Uint8Array(W * H))).toBeNull()
  })

  it('does not mistake a dark picture for bars', () => {
    // A 16:9 slide that is black except a title near the top: the bottom is
    // "black" but the top is not, so the bars would be lopsided.
    const luma = frame(16 / 9, (x, y) => (y < 8 && x > 20 && x < 70 ? 200 : 4))
    expect(detect(luma)).toBeNull()
  })

  it('keeps the crop to the bars when the picture itself is dark at the edges', () => {
    // 5:1 source with a dark band along its own top and bottom -- still the
    // driver's bars that decide, because those rows are pure black and the
    // picture's dark rows are not.
    const luma = frame(5, (x, y, cw, ch) => (y === 0 || y === ch - 1 ? 18 : 120))
    expect(detect(luma)?.shape).toBe('5:1')
  })

  it('rejects a shape it does not know rather than cropping to it', () => {
    expect(detect(frame(2.8))).toBeNull()
  })

  it('reads real pixels through lumaGrid', () => {
    const rgba = new Uint8ClampedArray([0, 0, 0, 255, 255, 255, 255, 255])
    expect(Array.from(lumaGrid(rgba))).toEqual([0, 255])
  })
})

describe('createCropTracker', () => {
  const five = detect(frame(5))

  it('changes only after the same result repeats', () => {
    const t = createCropTracker()
    for (let i = 1; i < CROP.STABLE_SAMPLES; i++) expect(t.update(five)).toBe(false)
    expect(t.current()).toBe('none')
    expect(t.update(five)).toBe(true)
    expect(t.current().shape).toBe('5:1')
  })

  it('is not reset by an undecidable sample in the middle', () => {
    const t = createCropTracker()
    t.update(five)
    t.update(null) // one black frame
    for (let i = 1; i < CROP.STABLE_SAMPLES - 1; i++) t.update(five)
    expect(t.update(five)).toBe(true)
  })

  it('goes back to uncropped when the source fills the frame again', () => {
    const t = createCropTracker()
    for (let i = 0; i < CROP.STABLE_SAMPLES; i++) t.update(five)
    for (let i = 0; i < CROP.STABLE_SAMPLES - 1; i++) expect(t.update('none')).toBe(false)
    expect(t.update('none')).toBe(true)
    expect(t.current()).toBe('none')
    expect(viewBoxCss(t.current())).toBe('')
  })

  it('starts over when an alternating result never settles', () => {
    const t = createCropTracker()
    const tenth = detect(frame(1.6))
    for (let i = 0; i < 10; i++) expect(t.update(i % 2 ? five : tenth)).toBe(false)
    expect(t.current()).toBe('none')
  })

  it('reset() drops the crop for a new stream', () => {
    const t = createCropTracker()
    for (let i = 0; i < CROP.STABLE_SAMPLES; i++) t.update(five)
    t.reset()
    expect(t.current()).toBe('none')
  })
})
