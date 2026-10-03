// @vitest-environment jsdom
// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
/**
 * The rendered shortcut hints (#258).
 *
 * shortcuts.test.js checks the list and its agreement with the renderer's action
 * map. This drives the two consumers that put it on screen -- the Settings table
 * and the dropdown -- so a change to the list is visible in both, and so the
 * table can never again be a hand-maintained copy that drifts.
 */
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { readFileSync } from 'node:fs'
import path from 'node:path'
import { installRendererDom, device, projectRoot } from './helpers/renderer-dom.js'

installRendererDom()

const fakeTrack = () => ({
  stop: vi.fn(),
  getSettings: () => ({ width: 1920, height: 1080, frameRate: 60 }),
  getCapabilities: () => ({
    width: { max: 1920 }, height: { max: 1080 }, frameRate: { max: 60 }
  })
})
Object.defineProperty(globalThis.navigator, 'mediaDevices', {
  value: {
    getUserMedia: vi.fn(async () => ({
      getTracks: () => [fakeTrack()],
      getVideoTracks: () => [fakeTrack()],
      getAudioTracks: () => [],
    })),
    enumerateDevices: vi.fn(async () => []),
  },
  configurable: true,
})

const { SHORTCUTS, shortcutById } =
  await import('../src/renderer/shortcuts.js')
const {
  state, elements, getDefaultSettings, handleKeyDown, setLayout,
  renderShortcutHints, renderDropdownInputLists,
  renderShortcutLegend, toggleLegend, closeLegend,
} = await import('../src/renderer/renderer.js')

/** The pickers currently rendered over the wall, and helpers to read them. */
const pickers = () => [...elements.wallPickers.querySelectorAll('.wall-picker')]
const tiles = (picker) => [...picker.querySelectorAll('.input-option')]
const label = (picker) => picker.querySelector('.picker-label').textContent

function reset(devices = []) {
  state.settings = { ...getDefaultSettings(), inputs: {} }
  state.devices = devices
  state.leftDeviceId = devices[0]?.deviceId ?? null
  state.rightDeviceId = devices[1]?.deviceId ?? null
  state.layoutMode = 'dual'
  state.frozen = false
  state.testFlags = {
    mock: false, mockInputs: 0, noSignal: false, screensaverDelayMs: null,
  }
}

beforeEach(() => {
  vi.clearAllMocks()
  reset()
})

describe('the view-mode buttons', () => {
  it('label Dual and Single with their keys', () => {
    renderShortcutHints()
    expect(elements.viewModeDual.textContent).toContain('Dual')
    expect(elements.viewModeDual.querySelector('kbd').textContent).toBe('D')
    expect(elements.viewModeSingle.textContent).toContain('Single')
    expect(elements.viewModeSingle.querySelector('kbd').textContent).toBe('S')
  })

  it('keeps the active class that setLayout toggles', () => {
    // renderShortcutHints rewrites these buttons' contents, so it must not
    // clobber the class setLayout uses to show which mode is current.
    renderShortcutHints()
    setLayout('single')
    expect(elements.viewModeSingle.classList.contains('active')).toBe(true)
    expect(elements.viewModeDual.classList.contains('active')).toBe(false)
  })
})

describe('the pickers over the wall (dropdown 2b)', () => {
  // Chips go only where a tap does what the key does.
  //
  // `1`-`4` call selectInput() with the default side='both' and set BOTH halves.
  // A tap on a per-half picker sets one, so a chip there would document a
  // different action. In single view, and in dual view without Multi-view, a tap
  // sets the whole wall, so the chip is honest.

  const four = () => [device('a', 'Cam A'), device('b', 'Cam B'), device('c', 'Cam C'),
    device('d', 'Cam D')]

  it('puts one picker on each half in dual view, without chips', () => {
    reset(four())
    renderDropdownInputLists()
    const ps = pickers()
    expect(ps.map(label)).toEqual(['Left half', 'Right half'])
    expect(elements.wallPickers.querySelectorAll('kbd')).toHaveLength(0)
    // The rows themselves are still there and still named.
    expect(tiles(ps[0]).map(t => t.textContent)).toEqual(['Cam A', 'Cam B', 'Cam C', 'Cam D'])
  })

  it('labels the first four tiles 1-4 in single view', () => {
    reset(four())
    state.layoutMode = 'single'
    renderDropdownInputLists()
    const ps = pickers()
    expect(ps.map(label)).toEqual(['Whole wall'])
    expect([...ps[0].querySelectorAll('kbd')].map(k => k.textContent))
      .toEqual(['1', '2', '3', '4'])
  })

  it('has one picker for both halves, with chips, when Multi-view is off', () => {
    reset(four())
    state.settings.multiView = false
    renderDropdownInputLists()
    const ps = pickers()
    expect(ps.map(label)).toEqual(['Both halves'])
    expect(ps[0].querySelectorAll('kbd')).toHaveLength(4)
  })

  it('leaves a fifth tile unlabelled rather than promising a key', () => {
    reset(['a', 'b', 'c', 'd', 'e'].map(id => device(id, `Cam ${id}`)))
    state.layoutMode = 'single'
    renderDropdownInputLists()
    const ts = tiles(pickers()[0])
    expect(ts).toHaveLength(5)
    expect(ts[4].querySelector('kbd')).toBeNull()
    expect(ts[3].querySelector('kbd').textContent).toBe('4')
  })

  it('skips disabled inputs, so the numbering matches what is shown', () => {
    // selectInput indexes the enabled list, so a hidden disabled device must not
    // consume a number.
    reset([device('a', 'Cam A'), device('b', 'Cam B'), device('c', 'Cam C')])
    state.settings.inputs = { b: { name: null, enabled: false } }
    state.layoutMode = 'single'
    renderDropdownInputLists()
    const ts = tiles(pickers()[0])
    expect(ts).toHaveLength(2)
    expect(ts.map(t => t.querySelector('kbd').textContent)).toEqual(['1', '2'])
    expect(ts[1].textContent).toContain('Cam C')
  })

  it('marks the input each half is showing', () => {
    reset([device('a', 'Cam A'), device('b', 'Cam B')])
    renderDropdownInputLists()
    const [left, right] = pickers()
    const pressed = (p) => tiles(p).filter(t => t.getAttribute('aria-pressed') === 'true')
      .map(t => t.dataset.deviceId)
    expect(pressed(left)).toEqual(['a'])
    expect(pressed(right)).toEqual(['b'])
    expect(left.querySelector('.picker-current').textContent).toBe('Cam A')
    expect(right.querySelector('.picker-current').textContent).toBe('Cam B')
  })

  it('tags each tile with its device id, which the sweep finds tiles by', () => {
    // paintThumbnail() queries [data-device-id] fresh rather than holding node
    // references, because tiles are rebuilt on any device or selection change --
    // which can happen while a snapshot sweep is still running (#242).
    reset([device('a', 'Cam A'), device('b', 'Cam B')])
    renderDropdownInputLists()
    for (const p of pickers()) {
      expect(tiles(p).map(t => t.dataset.deviceId)).toEqual(['a', 'b'])
    }
  })

  it('renders the snapshot tile before any snapshot exists', () => {
    reset([device('a', 'Cam A')])
    renderDropdownInputLists()
    const tile = elements.wallPickers.querySelector('.input-thumb')
    expect(tile).not.toBeNull()
    expect(tile.classList.contains('has-thumb')).toBe(false)
  })

  it('renders a device label as text, never as markup', () => {
    // Labels come from capture hardware or a user rename.
    reset([device('a', '<img src=x onerror=alert(1)>')])
    state.rightDeviceId = 'a'
    renderDropdownInputLists()
    for (const p of pickers()) {
      expect(p.querySelector('img')).toBeNull()
      expect(p.querySelector('.picker-current').textContent)
        .toBe('<img src=x onerror=alert(1)>')
      expect(p.querySelector('.input-option-name').textContent)
        .toBe('<img src=x onerror=alert(1)>')
    }
  })

  it('re-renders after a number key, not only after a click', async () => {
    reset([device('a', 'Cam A'), device('b', 'Cam B')])
    renderDropdownInputLists()
    handleKeyDown({ key: '2', target: document.body, preventDefault: vi.fn() })
    // selectInput sets both halves, once the streams have been asked for.
    await vi.waitFor(() => {
      const current = pickers().map(p => p.querySelector('.picker-current').textContent)
      expect(current).toEqual(['Cam B', 'Cam B'])
    })
  })
})

describe('the pickers cannot overflow their half', () => {
  // jsdom does no layout, so this asserts the declarations that matter.
  const CSS = readFileSync(
    path.resolve(projectRoot, 'src/renderer/styles.css'), 'utf8')

  const ruleBody = (selector) => {
    const re = new RegExp(
      `${selector.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\s*\\{([^}]*)\\}`, 'g')
    const bodies = [...CSS.matchAll(re)].map(m => m[1])
    expect(bodies.length, `rule not found: ${selector}`).toBeGreaterThan(0)
    return bodies.join('\n')
  }

  it('scrolls the strip sideways instead of widening the half', () => {
    expect(ruleBody('.picker-strip')).toMatch(/overflow-x:\s*auto/)
    expect(ruleBody('.wall-picker')).toMatch(/min-width:\s*0/)
  })

  it('ellipsises a long name on one line', () => {
    expect(ruleBody('.picker-current')).toMatch(/text-overflow:\s*ellipsis/)
    expect(ruleBody('.input-option-name')).toMatch(/text-overflow:\s*ellipsis/)
  })

  it('splits the halves with minmax(0, 1fr), never a bare 1fr', () => {
    // A bare 1fr is minmax(auto, 1fr), whose auto minimum is the item's
    // min-content: a nowrap name pinned the old dropdown's tracks open.
    const src = readFileSync(
      path.resolve(projectRoot, 'src/renderer/renderer.js'), 'utf8')
    expect(src).toContain("'minmax(0, 1fr) var(--center-gap, 60px) minmax(0, 1fr)'")
  })
})

describe('the keys still do what they did', () => {
  // The switch became a lookup, so these check the dispatch end to end for every
  // key with an effect that is observable without a GL context. The screensaver
  // keys (V, +, -) are left out on purpose: starting a saver needs WebGL2, which
  // jsdom has none of -- the same reason screensaver-stepping.test.js covers the
  // index maths rather than a real activation.
  const press = (key, target = document.body) => {
    const preventDefault = vi.fn()
    handleKeyDown({ key, target, preventDefault })
    return preventDefault
  }

  it('D and S still switch layout', () => {
    reset()
    setLayout('single')
    press('d')
    expect(state.layoutMode).toBe('dual')
    press('s')
    expect(state.layoutMode).toBe('single')
  })

  it('a number key selects that input', () => {
    reset([device('a', 'Cam A'), device('b', 'Cam B')])
    press('2')
    expect(state.leftDeviceId).toBe('b')
  })

  it('Space freezes and unfreezes', () => {
    reset()
    press(' ')
    expect(state.frozen).toBe(true)
    press(' ')
    expect(state.frozen).toBe(false)
  })

  it('F asks the main process for fullscreen', () => {
    reset()
    const toggleFullscreen = vi.fn()
    globalThis.window.electronAPI = { toggleFullscreen }
    press('f')
    expect(toggleFullscreen).toHaveBeenCalled()
    // F11 is an alias, not a separate binding.
    press('f11')
    expect(toggleFullscreen).toHaveBeenCalledTimes(2)
    delete globalThis.window.electronAPI
  })

  it('Q quits', () => {
    reset()
    const quitApp = vi.fn()
    globalThis.window.electronAPI = { quitApp }
    press('q')
    expect(quitApp).toHaveBeenCalled()
    delete globalThis.window.electronAPI
  })

  it('is still case-insensitive', () => {
    reset()
    setLayout('dual')
    press('S')
    expect(state.layoutMode).toBe('single')
  })

  it('still ignores every key while typing in a field', () => {
    reset()
    setLayout('dual')
    press('s', { tagName: 'INPUT' })
    expect(state.layoutMode).toBe('dual')
  })

  it('suppresses the default action for exactly the keys that declare it', () => {
    // Space must not scroll and F must not type; Escape and the arrows must keep
    // their default behaviour. Getting this backwards is invisible until it is
    // not, so it is pinned per key.
    reset()
    globalThis.window.electronAPI = {
      toggleFullscreen: vi.fn(),
      quitApp: vi.fn(),
    }
    for (const { keys, id, preventDefault: expected } of SHORTCUTS) {
      // Skip the GL-backed savers, as above.
      if (id.startsWith('screensaver')) continue
      // Escape reaches electronAPI.isFullscreen, which this stub omits.
      if (id === 'escape') continue
      for (const key of keys) {
        const spy = press(key)
        expect(spy.mock.calls.length > 0, `${id} (${key})`).toBe(expected)
      }
    }
    delete globalThis.window.electronAPI
  })

  it('ignores a key that is in no shortcut', () => {
    reset()
    setLayout('dual')
    press('z')
    press('F5')
    expect(state.layoutMode).toBe('dual')
  })

  it('has an entry for every key it claims to handle', () => {
    // Sanity check on the list itself from the handler's side: every declared
    // key resolves to a shortcut with an id the renderer knows.
    for (const shortcut of SHORTCUTS) {
      expect(shortcutById(shortcut.id)).toBeDefined()
    }
  })
})

describe('the shortcut legend (dropup)', () => {
  // Third consumer of SHORTCUTS, after the keydown handler and the Settings
  // table. Deliberately the same rows as the table rather than a shortened
  // "important ones" set -- that would be a fourth hand-maintained list, which is
  // what #258 existed to remove.

  it('renders one row per shortcut, in list order', () => {
    renderShortcutLegend()
    const rows = [...elements.legendGrid.children]
    expect(rows).toHaveLength(SHORTCUTS.length)
    expect(rows.map(r => r.querySelector('.legend-label').textContent.split(' (')[0]))
      .toEqual(SHORTCUTS.map(s => s.label))
  })

  it('prints each shortcut\'s keys as chips', () => {
    renderShortcutLegend()
    const rows = [...elements.legendGrid.children]
    rows.forEach((row, i) => {
      expect([...row.querySelectorAll('kbd')].map(k => k.textContent), SHORTCUTS[i].id)
        .toEqual(SHORTCUTS[i].chips)
    })
  })

  it('carries the caveat on the remote-keyboard rows', () => {
    renderShortcutLegend()
    const notes = [...elements.legendGrid.querySelectorAll('.legend-note')]
    expect(notes).toHaveLength(SHORTCUTS.filter(s => s.note).length)
    expect(notes[0].textContent).toContain('remote keyboard')
  })

  it('replaces its rows rather than appending on a re-render', () => {
    renderShortcutLegend()
    renderShortcutLegend()
    expect(elements.legendGrid.children).toHaveLength(SHORTCUTS.length)
  })

  it('shows every key in the list, now that it is the only rendered copy', () => {
    renderShortcutLegend()
    const chips = [...elements.legendGrid.querySelectorAll('kbd')].map(k => k.textContent).sort()
    expect(chips).toEqual(SHORTCUTS.flatMap(s => s.chips).sort())
  })
})

describe('opening and closing the legend', () => {
  beforeEach(() => {
    closeLegend()
  })

  it('toggles touch-open on both the trigger and the panel', () => {
    // The CSS opens the panel from either, mirroring the dropdown:
    // `#legend-trigger.touch-open + #legend-panel, #legend-panel.touch-open`.
    toggleLegend()
    expect(elements.legendPanel.classList.contains('touch-open')).toBe(true)
    expect(elements.legendTrigger.classList.contains('touch-open')).toBe(true)

    toggleLegend()
    expect(elements.legendPanel.classList.contains('touch-open')).toBe(false)
    expect(elements.legendTrigger.classList.contains('touch-open')).toBe(false)
  })

  it('tracks state.legendOpen', () => {
    toggleLegend()
    expect(state.legendOpen).toBe(true)
    closeLegend()
    expect(state.legendOpen).toBe(false)
  })

  it('closes on Escape, like the other panels', () => {
    globalThis.window.electronAPI = {
      isFullscreen: async () => false,
      toggleFullscreen: vi.fn(),
    }
    toggleLegend()
    handleKeyDown({ key: 'Escape', target: document.body, preventDefault() {} })
    expect(state.legendOpen).toBe(false)
    delete globalThis.window.electronAPI
  })
})

describe('the legend geometry mirrors the dropdown', () => {
  const CSS = readFileSync(
    path.resolve(projectRoot, 'src/renderer/styles.css'), 'utf8')

  const ruleBody = (selector) => {
    const re = new RegExp(
      `${selector.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\s*\\{([^}]*)\\}`, 'g')
    const bodies = [...CSS.matchAll(re)].map(m => m[1])
    expect(bodies.length, `rule not found: ${selector}`).toBeGreaterThan(0)
    return bodies.join('\n')
  }

  it('fades rather than slides, and is hidden from the pointer while closed', () => {
    // The redesign's motion is fades only. visibility (not just opacity) is what
    // stops an invisible panel from catching clicks meant for the wall.
    const body = ruleBody('#legend-panel')
    expect(body).toMatch(/opacity:\s*0/)
    expect(body).toMatch(/visibility:\s*hidden/)
    expect(body).not.toMatch(/translateY\(100%\)/)
  })

  it('overlaps its tab, so hovering from the tab into the panel never crosses a gap', () => {
    // A gap between the 30px tab and the panel would close the hover-opened
    // panel on the way up. The panel's bottom offset must stay below the tab's
    // height, for the fine and the coarse (44px) tab alike.
    const bottoms = [...CSS.matchAll(/#legend-panel\s*\{[^}]*?bottom:\s*(\d+)px/g)].map(m => +m[1])
    expect(bottoms).toEqual([24, 38])
    expect(bottoms[0]).toBeLessThan(30)
    expect(bottoms[1]).toBeLessThan(44)
  })

  it('sets an explicit width, not only a max', () => {
    // A fixed-position element is shrink-to-fit, so `max-width` caps it but never
    // expands it -- and auto-fit needs a DEFINITE width to work out its column
    // count. With only a max it measured 467px and one column at a 1500px
    // viewport, which is the long list this grid exists to avoid.
    const body = ruleBody('#legend-panel')
    expect(body).toMatch(/(^|\s)width:\s*min\(/m)
  })

  it('wraps its columns to the available width', () => {
    expect(ruleBody('.legend-grid'))
      .toMatch(/grid-template-columns:\s*repeat\(auto-fit,\s*minmax\(/)
  })

  it('lets labels wrap rather than truncate', () => {
    // Measured at 1100px: three of twelve labels ellipsised, including both
    // "(if the remote keyboard is enabled)" caveats -- the part that makes those
    // rows make sense. A legend that hides what a key does defeats itself.
    const body = ruleBody('.legend-label')
    expect(body).not.toMatch(/text-overflow:\s*ellipsis/)
    expect(body).not.toMatch(/white-space:\s*nowrap/)
  })

  it('gives the key column a fixed width so labels line up', () => {
    const body = ruleBody('.legend-keys')
    expect(body).toMatch(/width:\s*\d+px/)
    expect(body).toMatch(/flex-shrink:\s*0/)
  })
})
