// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
import { describe, it, expect } from 'vitest'
import {
  HEALTH, createStreamHealth, lumaStats, isUniform, backoffDelay,
} from '../src/renderer/stream-health.js'

const BUSY = { mean: 90, std: 40 }   // a real picture
const FLAT = { mean: 1, std: 0.2 }   // black, or a solid no-signal screen

/** Drive a tracker at a fixed fps for `ms`, sampling every `step` ms. */
function run(h, { from, ms, fps, stats = BUSY, step = 2000, frames }) {
  let t = from
  let count = frames.value
  let status
  while (t < from + ms) {
    t += step
    count += (fps * step) / 1000
    status = h.sample({ frames: count, ended: false, stats }, t)
  }
  frames.value = count
  return { t, status }
}

describe('lumaStats', () => {
  it('reports a flat picture as uniform and a varied one as not', () => {
    const flat = new Uint8ClampedArray(32 * 18 * 4).fill(10)
    expect(isUniform(lumaStats(flat))).toBe(true)

    const bars = new Uint8ClampedArray(32 * 18 * 4)
    for (let i = 0; i < bars.length; i += 4) {
      const v = (i / 4) % 2 ? 250 : 5
      bars[i] = bars[i + 1] = bars[i + 2] = v
    }
    expect(isUniform(lumaStats(bars))).toBe(false)
  })

  it('treats a solid colour that is not black as uniform too', () => {
    // An Elgato-style solid blue no-signal screen is as dead as black.
    const blue = new Uint8ClampedArray(64 * 4)
    for (let i = 0; i < blue.length; i += 4) { blue[i + 2] = 200; blue[i + 3] = 255 }
    const s = lumaStats(blue)
    expect(s.mean).toBeGreaterThan(20)
    expect(isUniform(s)).toBe(true)
  })

  it('does not divide by zero on an empty buffer', () => {
    expect(lumaStats(new Uint8ClampedArray(0))).toEqual({ mean: 0, std: 0 })
  })
})

describe('backoffDelay', () => {
  it('repeats the last step forever', () => {
    const s = [1, 2, 3]
    expect([0, 1, 2, 3, 99].map(a => backoffDelay(s, a))).toEqual([1, 2, 3, 3, 3])
  })
})

describe('createStreamHealth', () => {
  it('is ok while a card delivers a real picture', () => {
    const h = createStreamHealth()
    h.opened(0)
    const { status } = run(h, { from: 0, ms: 30_000, fps: 60, frames: { value: 0 } })
    expect(status).toBe('ok')
    expect(h.decide(30_000).reopen).toBe(false)
  })

  it('is ok on a held slide: pixels frozen, frames still arriving', () => {
    // The #159 trap. A still picture at 60fps is a working card.
    const h = createStreamHealth()
    h.opened(0)
    const { status } = run(h, { from: 0, ms: 60_000, fps: 60, frames: { value: 0 } })
    expect(status).toBe('ok')
  })

  it('reports no-frames when a card opens and never delivers', () => {
    const h = createStreamHealth()
    h.opened(0)
    expect(h.sample({ frames: 0, ended: false, stats: null }, 4000)).toBe('opening')
    expect(h.sample({ frames: 0, ended: false, stats: null }, HEALTH.FIRST_FRAME_TIMEOUT_MS))
      .toBe('no-frames')
  })

  it('reports stalled when frames stop after flowing', () => {
    const h = createStreamHealth()
    h.opened(0)
    const frames = { value: 0 }
    const { t } = run(h, { from: 0, ms: 10_000, fps: 60, frames })
    // Counter stops moving.
    h.sample({ frames: frames.value, ended: false, stats: BUSY }, t + 2000)
    expect(h.sample({ frames: frames.value, ended: false, stats: BUSY }, t + HEALTH.STALL_MS))
      .toBe('stalled')
  })

  it('treats a counter that goes backwards as a new baseline, not a stall', () => {
    const h = createStreamHealth()
    h.opened(0)
    const frames = { value: 0 }
    const { t } = run(h, { from: 0, ms: 10_000, fps: 60, frames })
    frames.value = 0
    const after = run(h, { from: t, ms: 10_000, fps: 60, frames })
    expect(after.status).toBe('ok')
  })

  it('does not call a static virtual camera stalled', () => {
    // One frame, then nothing: never reached FLOWING_MIN_FPS, so a stop is not
    // a stall. (OBS Virtual Camera behaves exactly like this on a still scene.)
    const h = createStreamHealth()
    h.opened(0)
    h.sample({ frames: 1, ended: false, stats: BUSY }, 500)
    for (let t = 2500; t < 60_000; t += 2000) {
      expect(h.sample({ frames: 1, ended: false, stats: BUSY }, t)).toBe('ok')
    }
  })

  it('reports ended as soon as the track ends', () => {
    const h = createStreamHealth()
    h.opened(0)
    expect(h.sample({ frames: 10, ended: true, stats: BUSY }, 1000)).toBe('ended')
  })

  it('reports dark after a flat picture persists, not on the first flat sample', () => {
    const h = createStreamHealth()
    h.opened(0)
    const frames = { value: 0 }
    const first = run(h, { from: 0, ms: 2000, fps: 60, stats: FLAT, frames })
    expect(first.status).toBe('ok')
    const later = run(h, { from: first.t, ms: HEALTH.DARK_MS, fps: 60, stats: FLAT, frames })
    expect(later.status).toBe('dark')
  })

  it('works without a frame counter, on the picture alone', () => {
    const h = createStreamHealth()
    h.opened(0)
    expect(h.sample({ frames: null, ended: false, stats: BUSY }, 2000)).toBe('ok')
    h.sample({ frames: null, ended: false, stats: FLAT }, 4000)
    expect(h.sample({ frames: null, ended: false, stats: FLAT }, 4000 + HEALTH.DARK_MS))
      .toBe('dark')
  })

  it('backs off between reopens of a faulted feed', () => {
    const h = createStreamHealth()
    h.openFailed(0, new Error('NotReadableError'))
    expect(h.info().error).toBe('Error')

    const delays = []
    let t = 0
    for (let i = 0; i < HEALTH.FAULT_BACKOFF_MS.length + 2; i++) {
      const d = h.decide(t)
      expect(d.reopen).toBe(false)
      t += d.waitMs
      expect(h.decide(t)).toMatchObject({ reopen: true, reason: 'open-failed' })
      delays.push(d.waitMs)
      h.reopening(t)
      h.openFailed(t, 'again')
    }
    const last = HEALTH.FAULT_BACKOFF_MS.at(-1)
    expect(delays).toEqual([...HEALTH.FAULT_BACKOFF_MS, last, last])
  })

  it('uses the slower schedule for a dark feed', () => {
    const h = createStreamHealth()
    h.opened(0)
    const frames = { value: 0 }
    const { t } = run(h, { from: 0, ms: HEALTH.DARK_MS + 2000, fps: 60, stats: FLAT, frames })
    expect(h.info().status).toBe('dark')
    const d = h.decide(t)
    expect(d.reason).toBe('dark')
    // First wait is counted from when the stream opened.
    expect(d.waitMs).toBe(Math.max(0, HEALTH.DARK_BACKOFF_MS[0] - t))
  })

  it('starts the backoff over once a reopened feed has stayed healthy', () => {
    const h = createStreamHealth()
    h.openFailed(0, 'x')
    h.reopening(2000); h.openFailed(2000, 'x')
    h.reopening(7000)
    h.opened(7000)
    expect(h.info().attempts).toBe(2)

    const frames = { value: 0 }
    run(h, { from: 7000, ms: HEALTH.RECOVERED_RESET_MS + 4000, fps: 60, frames })
    expect(h.info().attempts).toBe(0)
  })

  it('does nothing for a side with no stream', () => {
    const h = createStreamHealth()
    expect(h.sample({ frames: 0, ended: true, stats: FLAT }, 99_999)).toBe('idle')
    expect(h.decide(99_999).reopen).toBe(false)
  })
})
