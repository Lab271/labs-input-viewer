// @vitest-environment jsdom
// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
/**
 * Switching to dual view must open the right-hand stream if it isn't playing.
 *
 * Startup in single view opens only the left side. On the wall on 2026-10-02,
 * switching to dual then showed a black right panel while the dropdown said
 * Presenter was active, until the input was clicked again.
 */
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { installRendererDom } from './helpers/renderer-dom.js'

installRendererDom()

const fakeTrack = (readyState = 'live') => ({
  readyState,
  stop: vi.fn(),
  addEventListener: vi.fn(),
  getSettings: () => ({ width: 3840, height: 2160, frameRate: 60 }),
  getCapabilities: () => ({ width: { max: 3840 }, height: { max: 2160 }, frameRate: { max: 60 } }),
})
const fakeStream = (readyState) => {
  const t = fakeTrack(readyState)
  return { getTracks: () => [t], getVideoTracks: () => [t], getAudioTracks: () => [] }
}

const getUserMedia = vi.fn(async () => fakeStream())
Object.defineProperty(globalThis.navigator, 'mediaDevices', {
  value: { getUserMedia, enumerateDevices: vi.fn(async () => []), addEventListener: vi.fn() },
  configurable: true,
})
globalThis.window.electronAPI = { saveSettings: vi.fn(async () => {}) }

const { state, elements, setLayout, getDefaultSettings } = await import('../src/renderer/renderer.js')

const PRESENTER = 'presenter-card'
const flush = () => new Promise((r) => setTimeout(r, 0))
const openedFor = (id) => getUserMedia.mock.calls.some(([c]) => c?.video?.deviceId?.exact === id)

beforeEach(() => {
  vi.clearAllMocks()
  state.settings = { ...getDefaultSettings(), inputs: {} }
  state.devices = [{ deviceId: PRESENTER, label: 'Game Capture 4K60 Pro MK.2', kind: 'videoinput' }]
  state.layoutMode = 'single'
  state.leftDeviceId = PRESENTER
  state.rightDeviceId = PRESENTER
  state.rightStream = null
  elements.rightVideo.srcObject = null
})

describe('setLayout("dual")', () => {
  it('opens the right stream when the right side has an input but nothing playing', async () => {
    setLayout('dual')
    await flush()
    expect(openedFor(PRESENTER)).toBe(true)
  })

  it('reopens a right stream whose track has ended', async () => {
    elements.rightVideo.srcObject = fakeStream('ended')
    setLayout('dual')
    await flush()
    expect(openedFor(PRESENTER)).toBe(true)
  })

  it('leaves a live right stream alone', async () => {
    elements.rightVideo.srcObject = fakeStream('live')
    setLayout('dual')
    await flush()
    expect(getUserMedia).not.toHaveBeenCalled()
  })

  it('does nothing when the right side has no input', async () => {
    state.rightDeviceId = null
    setLayout('dual')
    await flush()
    expect(getUserMedia).not.toHaveBeenCalled()
  })
})

describe('setLayout("single")', () => {
  it('does not open the right stream', async () => {
    setLayout('single')
    await flush()
    expect(getUserMedia).not.toHaveBeenCalled()
  })
})
