// @vitest-environment jsdom
// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
/**
 * The rebuilt Settings panel, the dropdown's open/close, Multi-view and the key
 * handling fixes that came with them.
 *
 * Unlike the other renderer tests this loads the REAL index.html body rather
 * than the id-derived fixture: the panel's behaviour lives in its markup (nav
 * items, sections, switch roles), and a fixture would only test itself.
 */
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { readFileSync } from 'node:fs'
import path from 'node:path'
import { device, projectRoot, requiredElementIds } from './helpers/renderer-dom.js'

const INDEX_HTML = readFileSync(path.resolve(projectRoot, 'src/renderer/index.html'), 'utf8')
const body = INDEX_HTML.slice(INDEX_HTML.indexOf('<body>') + 6, INDEX_HTML.indexOf('</body>'))
  .replace(/<script[\s\S]*?<\/script>/g, '')
document.body.innerHTML = body
globalThis.__INPUT_VIEWER_NO_AUTOSTART__ = true
globalThis.HTMLCanvasElement.prototype.getContext = () => null

const fakeTrack = () => ({
  stop: vi.fn(),
  addEventListener: vi.fn(),
  removeEventListener: vi.fn(),
  readyState: 'live',
  getSettings: () => ({ width: 1920, height: 1080, frameRate: 60 }),
  getCapabilities: () => ({ width: { max: 1920 }, height: { max: 1080 }, frameRate: { max: 60 } })
})
Object.defineProperty(globalThis.navigator, 'mediaDevices', {
  value: {
    getUserMedia: vi.fn(async () => ({
      getTracks: () => [fakeTrack()],
      getVideoTracks: () => [fakeTrack()],
      getAudioTracks: () => [],
    })),
    enumerateDevices: vi.fn(async () => []),
    addEventListener: vi.fn(),
  },
  configurable: true,
})

const R = await import('../src/renderer/renderer.js')
const { state, elements, getDefaultSettings, handleKeyDown } = R
// Wire the real listeners once, so clicks in these tests go through the same
// handlers the app uses.
R.setupEventListeners()

function reset(devices = []) {
  R.hideSettingsModal()
  R.closeDropdown()
  state.settings = { ...getDefaultSettings(), inputs: {} }
  state.devices = devices
  state.leftDeviceId = devices[0]?.deviceId ?? null
  state.rightDeviceId = devices[1]?.deviceId ?? null
  state.layoutMode = 'dual'
  state.defaultInputId = null
  state.frozen = false
  state.legendOpen = false
  state.settingsSection = 'inputs'
  state.expandedRefPanels.clear()
  state.captureResults.clear()
  state.testFlags = { mock: false, mockInputs: 0, noSignal: false, screensaverDelayMs: null }
}

const press = (key, target = document.body) => {
  handleKeyDown({ key, target, preventDefault: vi.fn() })
}

beforeEach(() => {
  vi.clearAllMocks()
  delete globalThis.window.electronAPI
  reset()
})

describe('index.html', () => {
  it('has every element renderer.js looks up by id', () => {
    // The other renderer tests build these from renderer.js itself, so they
    // would not notice the real page missing one.
    const missing = requiredElementIds().filter(id => !document.getElementById(id))
    expect(missing).toEqual([])
  })

  it('no longer has the removed sections', () => {
    for (const id of ['shortcuts-table', 'capture-left-btn', 'capture-right-btn',
      'left-input-list', 'right-input-list', 'single-input-list']) {
      expect(document.getElementById(id), id).toBeNull()
    }
  })

  it('gives every Settings switch role="switch"', () => {
    const switches = [...elements.settingsModal.querySelectorAll('.toggle-switch')]
    expect(switches.length).toBeGreaterThan(0)
    for (const el of switches) expect(el.getAttribute('role'), el.id).toBe('switch')
  })
})

describe('the Settings side nav', () => {
  it('shows one section at a time', () => {
    R.showSettingsModal()
    R.showSettingsSection('artnet')
    const visible = [...elements.settingsModal.querySelectorAll('.settings-section')]
      .filter(s => !s.classList.contains('hidden')).map(s => s.dataset.section)
    expect(visible).toEqual(['artnet'])
    const current = elements.settingsModal.querySelector('[aria-current="page"]')
    expect(current.dataset.section).toBe('artnet')
  })

  it('counts enabled inputs with no no-signal reference', () => {
    reset([device('a', 'Cam A'), device('b', 'Cam B'), device('c', 'Cam C')])
    state.settings.inputs = { c: { name: null, enabled: false } }
    R.showSettingsModal()
    expect(elements.settingsNavInputs.textContent).toBe('2')
  })

  it('says On/Off for Remote keyboard, orange while incomplete', () => {
    R.showSettingsModal()
    expect(elements.settingsNavRemote.textContent).toBe('Off')
    state.remoteKeyboardEnabled = true
    state.remoteKeyboardHost = ''
    R.showSettingsModal()
    expect(elements.settingsNavRemote.textContent).toBe('On')
    expect(elements.settingsNavRemote.querySelector('.status-dot.warn')).not.toBeNull()
    state.remoteKeyboardEnabled = false
  })
})

describe('the Inputs table', () => {
  const rows = () => [...elements.settingsInputList.querySelectorAll('.input-name-row')]

  it('numbers by the dropdown/key order: enabled only, dot past four, dash if off', () => {
    reset(['a', 'b', 'c', 'd', 'e', 'f'].map(id => device(id, `Cam ${id}`)))
    state.settings.inputs = { b: { name: null, enabled: false } }
    R.showSettingsModal()
    expect(rows().map(r => r.querySelector('.input-key').textContent))
      .toEqual(['1', '–', '2', '3', '4', '·'])
  })

  it('shows the hardware label as the placeholder, and an empty name restores it', () => {
    reset([device('a', 'Elgato 4K60 Pro')])
    R.showSettingsModal()
    const field = rows()[0].querySelector('.input-name-field')
    expect(field.placeholder).toBe('Elgato 4K60 Pro')
    expect(field.value).toBe('')
    R.setInputName('a', 'Presenter')
    expect(R.getInputName('a', 'x')).toBe('Presenter')
    R.setInputName('a', '   ')
    expect(state.settings.inputs.a.name).toBeNull()
    expect(R.getInputName('a', 'Elgato 4K60 Pro')).toBe('Elgato 4K60 Pro')
  })

  it('renders names as text, never as markup', () => {
    reset([device('a', '"><img src=x onerror=alert(1)>')])
    R.showSettingsModal()
    expect(elements.settingsInputList.querySelector('img')).toBeNull()
  })

  it('lets the Startup pill be cleared by clicking it again', () => {
    reset([device('a', 'Cam A'), device('b', 'Cam B')])
    R.showSettingsModal()
    const pill = () => rows()[1].querySelector('.startup-pill')
    pill().click()
    expect(state.defaultInputId).toBe('b')
    expect(pill().getAttribute('aria-checked')).toBe('true')
    pill().click()
    expect(state.defaultInputId).toBeNull()
    expect(pill().getAttribute('aria-checked')).toBe('false')
  })

  it('flags an input switched off while it is still on the wall', () => {
    reset([device('a', 'Cam A'), device('b', 'Cam B')])
    state.settings.inputs = { a: { name: null, enabled: false } }
    R.showSettingsModal()
    expect(rows()[0].textContent).toContain('Still on the wall until you switch')
    expect(rows()[1].textContent).not.toContain('Still on the wall')
  })

  it('offers capture only for an input that is on the wall', () => {
    reset([device('a', 'Cam A'), device('b', 'Cam B'), device('c', 'Cam C')])
    R.showSettingsModal()
    const btn = (id) => elements.settingsInputList
      .querySelector(`.ref-panel[data-device-id="${id}"] .ref-capture-btn`)
    expect(btn('a').textContent).toBe('Capture from left half')
    expect(btn('a').disabled).toBe(false)
    expect(btn('b').textContent).toBe('Capture from right half')
    expect(btn('c').disabled).toBe(true)
    expect(btn('c').parentElement.textContent).toContain('Put this input on the wall first')
  })

  it('treats the right half as off the wall in single view', () => {
    reset([device('a', 'Cam A'), device('b', 'Cam B')])
    state.layoutMode = 'single'
    R.showSettingsModal()
    const b = elements.settingsInputList
      .querySelector('.ref-panel[data-device-id="b"] .ref-capture-btn')
    expect(b.disabled).toBe(true)
  })

  it('says so when a capture fails, instead of failing silently', async () => {
    reset([device('a', 'Cam A')])
    R.showSettingsModal()
    const result = await R.captureNoSignalForSide('left')
    expect(result.ok).toBe(false)
    expect(result.message).toMatch(/not showing a picture/)
  })

  it('keeps an expanded no-signal panel open across a re-render', () => {
    reset([device('a', 'Cam A')])
    R.showSettingsModal()
    elements.settingsInputList.querySelector('.ref-toggle-btn').click()
    R.renderSettingsInputList()
    const panel = elements.settingsInputList.querySelector('.ref-panel')
    expect(panel.classList.contains('hidden')).toBe(false)
    expect(elements.settingsInputList.querySelector('.ref-toggle-btn')
      .getAttribute('aria-expanded')).toBe('true')
  })
})

describe('Multi-view', () => {
  it('is on by default and saved', async () => {
    expect(R.isMultiView()).toBe(true)
    const saveSettings = vi.fn(async () => {})
    globalThis.window.electronAPI = { saveSettings }
    state.settings.multiView = false
    await R.saveSettings()
    expect(saveSettings.mock.calls[0][0].multiView).toBe(false)
  })

  it('brings the right half in line with the left when switched off', async () => {
    reset([device('a', 'Cam A'), device('b', 'Cam B')])
    R.setMultiView(false)
    await vi.waitFor(() => expect(state.rightDeviceId).toBe('a'))
  })

  it('makes a tap on the single strip set both halves', async () => {
    reset([device('a', 'Cam A'), device('b', 'Cam B')])
    state.rightDeviceId = 'a'
    state.settings.multiView = false
    R.renderDropdownInputLists()
    expect(R.pickerPlan()).toEqual([{ side: 'both', label: 'Both halves', showKeys: true }])
    elements.wallPickers.querySelector('[data-device-id="b"]').click()
    await vi.waitFor(() => {
      expect(state.leftDeviceId).toBe('b')
      expect(state.rightDeviceId).toBe('b')
    })
  })

  it('sets one half from a per-half picker when on', async () => {
    reset([device('a', 'Cam A'), device('b', 'Cam B')])
    R.renderDropdownInputLists()
    const right = elements.wallPickers.querySelectorAll('.wall-picker')[1]
    right.querySelector('[data-device-id="a"]').click()
    await vi.waitFor(() => expect(state.rightDeviceId).toBe('a'))
    expect(state.leftDeviceId).toBe('a')
  })
})

describe('the dropdown over the wall', () => {
  it('opens and closes with matching ARIA state', () => {
    R.openDropdown()
    expect(elements.dropdownPanel.classList.contains('touch-open')).toBe(true)
    expect(elements.dropdownPanel.getAttribute('aria-hidden')).toBe('false')
    expect(elements.dropdownTrigger.getAttribute('aria-expanded')).toBe('true')
    elements.wallCloseBtn.click()
    expect(state.dropdownOpen).toBe(false)
    expect(elements.dropdownPanel.getAttribute('aria-hidden')).toBe('true')
  })

  it('closes itself when Settings opens from the capsule', () => {
    R.openDropdown()
    elements.openSettingsBtn.click()
    expect(state.dropdownOpen).toBe(false)
    expect(R.isSettingsOpen()).toBe(true)
  })

  it('closes after a quiet stretch, so it cannot cover an unattended wall', () => {
    vi.useFakeTimers()
    try {
      R.openDropdown()
      vi.advanceTimersByTime(29_000)
      expect(state.dropdownOpen).toBe(true)
      vi.advanceTimersByTime(2_000)
      expect(state.dropdownOpen).toBe(false)
    } finally {
      vi.useRealTimers()
    }
  })
})

describe('Esc and the keys', () => {
  const fullscreenApi = () => {
    const api = {
      isFullscreen: vi.fn(async () => true),
      toggleFullscreen: vi.fn(),
      quitApp: vi.fn(),
    }
    globalThis.window.electronAPI = api
    return api
  }

  it('closes Settings without also leaving fullscreen', async () => {
    const api = fullscreenApi()
    R.showSettingsModal()
    press('Escape')
    await Promise.resolve()
    expect(R.isSettingsOpen()).toBe(false)
    expect(api.isFullscreen).not.toHaveBeenCalled()
    expect(api.toggleFullscreen).not.toHaveBeenCalled()
  })

  it('closes the open dropdown first, too', () => {
    const api = fullscreenApi()
    R.openDropdown()
    press('Escape')
    expect(state.dropdownOpen).toBe(false)
    expect(api.isFullscreen).not.toHaveBeenCalled()
  })

  it('leaves fullscreen when nothing is open', async () => {
    const api = fullscreenApi()
    press('Escape')
    await vi.waitFor(() => expect(api.toggleFullscreen).toHaveBeenCalled())
  })

  it('does not quit while a select has focus', () => {
    const api = fullscreenApi()
    press('q', document.getElementById('artnet-target'))
    expect(api.quitApp).not.toHaveBeenCalled()
  })

  it('does not switch layout while typing in a Settings field', () => {
    press('s', document.getElementById('remote-keyboard-host'))
    expect(state.layoutMode).toBe('dual')
  })

  it('still lets Esc through from inside a field', () => {
    R.showSettingsModal()
    press('Escape', document.getElementById('remote-keyboard-host'))
    expect(R.isSettingsOpen()).toBe(false)
  })
})

describe('remote keyboard', () => {
  it('sends through the main process and records the result', async () => {
    const remoteKeySend = vi.fn(async () => ({ ok: true, status: 200 }))
    globalThis.window.electronAPI = { remoteKeySend }
    state.remoteKeyboardEnabled = true
    state.remoteKeyboardHost = 'space_keyboard'
    state.remoteKeyboardApiKey = 'secret'
    await R.sendRemoteKeypress('right')
    expect(remoteKeySend).toHaveBeenCalledWith({
      url: 'http://space_keyboard.local/right', apiKey: 'secret'
    })
    expect(state.remoteKeyboardLast).toMatchObject({ ok: true, status: 200, direction: 'right' })
    state.remoteKeyboardEnabled = false
  })
})
