// @vitest-environment jsdom
// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
/**
 * Freezing in dual view must draw both feeds where the live layout has them.
 *
 * captureFrame() read `state.layoutGap`, which nothing in the renderer ever
 * set. The gap was undefined, every destination coordinate NaN, and the frozen
 * dual-view frame came out solid black. The live layout sizes #center-divider
 * from state.centerGap (the Settings "Center Gap" slider), so that is the gap
 * the freeze has to leave too.
 */
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { installRendererDom } from './helpers/renderer-dom.js'

installRendererDom()
globalThis.window.electronAPI = { saveSettings: vi.fn(async () => {}) }

const { state, elements, toggleFreeze, setCenterGap, getDefaultSettings } =
  await import('../src/renderer/renderer.js')

const WRAPPER_W = 1291
const WRAPPER_H = 1130

let drawn
const fakeCtx = () => ({
  fillStyle: '',
  fillRect: vi.fn(),
  drawImage: vi.fn((video, sx, sy, sw, sh, dx, dy, dw, dh) => {
    drawn.push({ video, dx, dy, dw, dh })
  }),
})

const givePicture = (video, w, h) => {
  video.srcObject = {}
  Object.defineProperty(video, 'videoWidth', { value: w, configurable: true })
  Object.defineProperty(video, 'videoHeight', { value: h, configurable: true })
}

beforeEach(() => {
  drawn = []
  state.settings = { ...getDefaultSettings(), inputs: {} }
  state.layoutMode = 'dual'
  state.frozen = false
  Object.defineProperty(elements.videoWrapper, 'clientWidth', { value: WRAPPER_W, configurable: true })
  Object.defineProperty(elements.videoWrapper, 'clientHeight', { value: WRAPPER_H, configurable: true })
  const ctx = fakeCtx()
  elements.freezeCanvas.getContext = () => ctx
  givePicture(elements.leftVideo, 1280, 720)
  givePicture(elements.rightVideo, 1280, 720)
})

/** The draw call that placed one side. */
const sideOf = (video) => drawn.find(d => d.video === video)

describe('freeze in dual view', () => {
  it.each([0, 49, 60, 200])('leaves the %ipx centre gap the live layout has', (gap) => {
    setCenterGap(gap)
    toggleFreeze()

    expect(drawn).toHaveLength(2)
    for (const d of drawn) {
      for (const n of [d.dx, d.dy, d.dw, d.dh]) expect(Number.isFinite(n)).toBe(true)
    }

    // Each side is a 16:9 picture fitted (contain) into a (W - gap)/2 half,
    // centred in it, exactly as two flex: 1 feeds either side of the divider.
    const half = (WRAPPER_W - gap) / 2
    const left = sideOf(elements.leftVideo)
    const right = sideOf(elements.rightVideo)
    expect(left.dw).toBeCloseTo(half)
    expect(right.dw).toBeCloseTo(half)
    expect(left.dx).toBeCloseTo(0)
    expect(right.dx).toBeCloseTo(half + gap)
    expect(right.dx + right.dw).toBeCloseTo(WRAPPER_W)
    expect(right.dx - (left.dx + left.dw)).toBeCloseTo(gap)

    toggleFreeze()
  })
})
