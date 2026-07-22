// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
import { describe, it, expect } from 'vitest'
import { compareFrames, CONFIG } from '../src/renderer/detection-simple.js'

// Build a minimal ImageData-like frame: { width, height, data }.
// `fill` may be a constant byte or a function (byteIndex) => byte.
function frame(width, height, fill) {
  const data = new Uint8ClampedArray(width * height * 4)
  for (let i = 0; i < data.length; i++) {
    data[i] = typeof fill === 'function' ? fill(i) : fill
  }
  return { width, height, data }
}

describe('compareFrames (no-signal detection)', () => {
  it('treats identical frames as a match', () => {
    expect(compareFrames(frame(16, 16, 120), frame(16, 16, 120))).toBe(true)
  })

  it('treats frames differing well beyond the threshold as no match', () => {
    // every channel differs by 200 (>> pixelDifferenceThreshold of 30)
    expect(compareFrames(frame(16, 16, 0), frame(16, 16, 200))).toBe(false)
  })

  it('tolerates small per-channel differences within the threshold', () => {
    // diff of 20 per channel is <= threshold (30), so still a match
    expect(compareFrames(frame(16, 16, 100), frame(16, 16, 120))).toBe(true)
  })

  it('returns false when dimensions differ', () => {
    expect(compareFrames(frame(16, 16, 0), frame(8, 8, 0))).toBe(false)
  })

  it('falls below the match ratio when half the frame changes', () => {
    const half = data => data // readability alias
    const base = frame(16, 16, 100)
    // second half of the buffer differs by 100 (> threshold) -> ~50% match < 95%
    const changed = frame(16, 16, i => (i < base.data.length / 2 ? 100 : 200))
    expect(compareFrames(base, half(changed))).toBe(false)
  })

  it('exposes tunable CONFIG thresholds', () => {
    expect(CONFIG.pixelDifferenceThreshold).toBeTypeOf('number')
    expect(CONFIG.matchThreshold).toBeGreaterThan(0)
    expect(CONFIG.matchThreshold).toBeLessThanOrEqual(1)
  })
})
