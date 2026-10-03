// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
/**
 * What the Settings panel says about itself (settings-status.js).
 */
import { describe, it, expect } from 'vitest'
import {
  remoteKeyUrl,
  inputKeyLabel,
  remoteKeyboardStatus,
  artnetStatus,
  isRelayUrl,
  orphanBannerText
} from '../src/renderer/settings-status.js'

describe('remoteKeyUrl', () => {
  it('adds .local to a bare name, as USER_GUIDE says it does', () => {
    // The bug: http:// was prepended first, so the "no dot, no colon" check
    // always saw the colon and the suffix was never added.
    expect(remoteKeyUrl('space_keyboard', 'left')).toBe('http://space_keyboard.local/left')
  })

  it('leaves a name with a dot, a port, or localhost alone', () => {
    expect(remoteKeyUrl('space_keyboard.local', 'right')).toBe('http://space_keyboard.local/right')
    expect(remoteKeyUrl('10.0.0.7', 'left')).toBe('http://10.0.0.7/left')
    expect(remoteKeyUrl('kbd:8080', 'left')).toBe('http://kbd:8080/left')
    expect(remoteKeyUrl('localhost', 'left')).toBe('http://localhost/left')
  })

  it('keeps a scheme that was typed, and still suffixes a bare host after it', () => {
    expect(remoteKeyUrl('https://kbd.example', 'left')).toBe('https://kbd.example/left')
    expect(remoteKeyUrl('http://kbd', 'left')).toBe('http://kbd.local/left')
  })

  it('trims whitespace and a trailing slash', () => {
    expect(remoteKeyUrl('  kbd.lan/  ', 'right')).toBe('http://kbd.lan/right')
  })

  it('returns null with no host', () => {
    expect(remoteKeyUrl('', 'left')).toBeNull()
    expect(remoteKeyUrl('   ', 'left')).toBeNull()
  })
})

describe('inputKeyLabel', () => {
  it('numbers the first four enabled inputs', () => {
    expect([0, 1, 2, 3].map(inputKeyLabel)).toEqual(['1', '2', '3', '4'])
  })

  it('shows a dot past the fourth, where there is no key', () => {
    expect(inputKeyLabel(4)).toBe('·')
  })

  it('shows a dash for a disabled input', () => {
    expect(inputKeyLabel(-1)).toBe('–')
  })
})

describe('remoteKeyboardStatus', () => {
  const base = { enabled: true, host: 'kbd', apiKey: 'k', last: null }

  it('is Off with no line while disabled', () => {
    expect(remoteKeyboardStatus({ ...base, enabled: false }))
      .toEqual({ nav: 'Off', tone: 'off', line: '' })
  })

  it('warns while on but missing a host or key', () => {
    expect(remoteKeyboardStatus({ ...base, host: ' ' }).tone).toBe('warn')
    expect(remoteKeyboardStatus({ ...base, apiKey: '' }).tone).toBe('warn')
  })

  it('reports the last press, good or bad', () => {
    const at = new Date(2026, 9, 3, 14, 2, 3).getTime()
    expect(remoteKeyboardStatus({ ...base, last: { ok: true, direction: 'left', status: 200, at } }))
      .toEqual({ nav: 'On', tone: 'ok', line: 'Sent left at 14:02:03 (HTTP 200).' })
    expect(remoteKeyboardStatus({ ...base, last: { ok: false, direction: 'left', status: 401, at } }).line)
      .toBe('Last press failed at 14:02:03: HTTP 401.')
    expect(remoteKeyboardStatus({ ...base, last: { ok: false, direction: 'left', error: 'timeout', at } }).line)
      .toBe('Last press failed at 14:02:03: timeout.')
  })
})

describe('artnetStatus', () => {
  const on = { artnetEnabled: true, artnetUrl: 'https://relay.lan' }

  it('is Off while disabled', () => {
    expect(artnetStatus({ artnetEnabled: false }, null).nav).toBe('Off')
  })

  it('warns when on with no URL or a URL that is not http(s)', () => {
    expect(artnetStatus({ artnetEnabled: true, artnetUrl: '' }, null).tone).toBe('warn')
    expect(artnetStatus({ artnetEnabled: true, artnetUrl: 'ftp://x' }, null).tone).toBe('warn')
    expect(artnetStatus({ artnetEnabled: true, artnetUrl: 'relay' }, null).tone).toBe('warn')
  })

  it('surfaces the client\'s last error, which used to reach only the log', () => {
    const s = artnetStatus(on, { sent: 3, failures: 2, lastError: 'HTTP 502' })
    expect(s.tone).toBe('warn')
    expect(s.line).toContain('HTTP 502')
  })

  it('is green once sending, and ready before that', () => {
    expect(artnetStatus(on, { sent: 1, failures: 0, lastError: null }).line).toBe('Sending. 1 update so far.')
    expect(artnetStatus(on, { sent: 0, failures: 0, lastError: null }).tone).toBe('ok')
  })
})

describe('isRelayUrl', () => {
  it('accepts http and https with a host only', () => {
    expect(isRelayUrl('http://relay.lan:8080')).toBe(true)
    expect(isRelayUrl('https://spacelights.lab271.io')).toBe(true)
    expect(isRelayUrl('file:///etc/passwd')).toBe(false)
    expect(isRelayUrl('not a url')).toBe(false)
  })
})

describe('orphanBannerText', () => {
  it('agrees with one reference and one device', () => {
    // Was "1 reference belong to 1 device that is not connected."
    expect(orphanBannerText(1, 1)).toMatch(/^1 reference belongs to 1 device that is not connected\./)
  })

  it('agrees with several', () => {
    expect(orphanBannerText(3, 2)).toMatch(/^3 references belong to 2 devices that are not connected\./)
  })
})
