// @vitest-environment jsdom
// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
/**
 * Which input each side shows must survive a device change.
 *
 * getVideoDevices() runs at startup and again on every `devicechange`. On the
 * wall on 2026-10-01 an EDID write in the Elgato utility fired one, and the
 * left side was silently re-picked as "first enabled device" -- the Apple TV
 * card -- while its stream still showed the presenter's laptop. The label and
 * the dropdown said Apple TV, and the next health reopen would have switched
 * the wall to it.
 */
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { installRendererDom, device } from './helpers/renderer-dom.js'

installRendererDom()

const fakeTrack = () => ({
  stop: vi.fn(),
  getSettings: () => ({ width: 3840, height: 2160, frameRate: 60 }),
  getCapabilities: () => ({ width: { max: 3840 }, height: { max: 2160 }, frameRate: { max: 60 } }),
})
const fakeStream = () => ({
  getTracks: () => [fakeTrack()],
  getVideoTracks: () => [fakeTrack()],
  getAudioTracks: () => [],
})

// The wall's real order: enumerateDevices lists the Apple TV card first.
const APPLE_TV = device('appletv-card', 'Game Capture 4K60 Pro MK.2 (2)')
const PRESENTER = device('presenter-card', 'Game Capture 4K60 Pro MK.2')
const WEBCAM = device('webcam', 'C922 Pro Stream Webcam')

let enumerated = [APPLE_TV, PRESENTER]
Object.defineProperty(globalThis.navigator, 'mediaDevices', {
  value: {
    getUserMedia: vi.fn(async () => fakeStream()),
    enumerateDevices: vi.fn(async () => enumerated),
    addEventListener: vi.fn(),
  },
  configurable: true,
})

const saved = []
globalThis.window.electronAPI = { saveSettings: vi.fn(async (s) => { saved.push(s) }) }

const { state, getVideoDevices, saveSettings, getDefaultSettings } =
  await import('../src/renderer/renderer.js')

function reset({ left = null, right = null, savedLeft = null, savedRight = null } = {}) {
  state.settings = {
    ...getDefaultSettings(),
    inputs: {},
    leftDeviceId: savedLeft,
    rightDeviceId: savedRight,
  }
  state.devices = []
  state.leftDeviceId = left
  state.rightDeviceId = right
  enumerated = [APPLE_TV, PRESENTER]
  saved.length = 0
}

beforeEach(() => reset())

describe('getVideoDevices on a device change', () => {
  it('keeps both sides on the presenter when the saved choice is stale', async () => {
    // Exactly the wall's state: both sides on the presenter, saved ids null.
    reset({ left: PRESENTER.deviceId, right: PRESENTER.deviceId })
    await getVideoDevices()
    expect(state.leftDeviceId).toBe(PRESENTER.deviceId)
    expect(state.rightDeviceId).toBe(PRESENTER.deviceId)
  })

  it('keeps a side even when the saved choice names another card', async () => {
    reset({ left: PRESENTER.deviceId, savedLeft: APPLE_TV.deviceId })
    await getVideoDevices()
    expect(state.leftDeviceId).toBe(PRESENTER.deviceId)
  })

  it('keeps sides when an unrelated device (a webcam) appears', async () => {
    reset({ left: PRESENTER.deviceId, right: APPLE_TV.deviceId })
    enumerated = [WEBCAM, APPLE_TV, PRESENTER]
    await getVideoDevices()
    expect(state.leftDeviceId).toBe(PRESENTER.deviceId)
    expect(state.rightDeviceId).toBe(APPLE_TV.deviceId)
  })

  it('does re-pick a side whose card has actually gone', async () => {
    reset({ left: PRESENTER.deviceId, right: PRESENTER.deviceId })
    enumerated = [APPLE_TV]
    await getVideoDevices()
    expect(state.leftDeviceId).toBe(APPLE_TV.deviceId)
  })
})

describe('getVideoDevices at startup', () => {
  it('restores the saved choice when nothing is chosen yet', async () => {
    reset({ savedLeft: PRESENTER.deviceId, savedRight: APPLE_TV.deviceId })
    await getVideoDevices()
    expect(state.leftDeviceId).toBe(PRESENTER.deviceId)
    expect(state.rightDeviceId).toBe(APPLE_TV.deviceId)
  })
})

describe('saveSettings', () => {
  it('does not overwrite a saved input with null before one is chosen', async () => {
    reset({ savedLeft: PRESENTER.deviceId, savedRight: APPLE_TV.deviceId })
    await saveSettings()
    expect(saved.at(-1).leftDeviceId).toBe(PRESENTER.deviceId)
    expect(saved.at(-1).rightDeviceId).toBe(APPLE_TV.deviceId)
  })

  it('saves the side that is actually chosen, and mirrors it in memory', async () => {
    reset({ left: PRESENTER.deviceId, savedLeft: APPLE_TV.deviceId })
    await saveSettings()
    expect(saved.at(-1).leftDeviceId).toBe(PRESENTER.deviceId)
    expect(state.settings.leftDeviceId).toBe(PRESENTER.deviceId)
  })
})
