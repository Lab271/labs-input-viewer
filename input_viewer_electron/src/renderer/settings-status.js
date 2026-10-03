// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
/**
 * What the Settings panel says about itself: the nav-item status, the status
 * lines under Remote keyboard and Art-Net, and the Key column of the Inputs
 * table.
 *
 * Pure and DOM-free, so it is unit tested in the node environment. Before this
 * existed neither integration showed anything in the UI: a wrong hostname or a
 * dead relay was only visible in the log file.
 */

/**
 * The URL a remote-keyboard press is sent to.
 *
 * A bare name like `space_keyboard` gets `.local` (mDNS). The check runs on the
 * host as typed, BEFORE the scheme is added: the old code prefixed `http://`
 * first, after which the "no dot, no colon" test could never pass and the suffix
 * was never applied.
 *
 * @param {string} host as entered in Settings
 * @param {'left'|'right'} direction
 * @returns {string|null} null when there is no host
 */
export function remoteKeyUrl (host, direction) {
  const h = String(host ?? '').trim()
  if (!h) return null
  const [, scheme, hostPart, rest] = h.match(/^(https?:\/\/)?([^/]*)(.*)$/i)
  let name = hostPart
  if (!name.includes('.') && !name.includes(':') && name.toLowerCase() !== 'localhost') {
    name = `${name}.local`
  }
  return `${scheme || 'http://'}${name}${rest.replace(/\/+$/, '')}/${direction}`
}

/**
 * Key column: the number key that selects this input, '·' for an enabled input
 * past the fourth (there is no key for it), '–' for a disabled one.
 *
 * @param {number} enabledIndex position among ENABLED devices, or -1 if disabled
 */
export function inputKeyLabel (enabledIndex) {
  if (enabledIndex < 0) return '–'
  return enabledIndex < 4 ? String(enabledIndex + 1) : '·'
}

/**
 * Remote keyboard status, for the nav dot and the line under the fields.
 *
 * @param {{enabled: boolean, host: string, apiKey: string,
 *          last: null|{ok: boolean, direction: string, status?: number,
 *                      error?: string, at: number}}} rk
 * @returns {{nav: string, tone: 'ok'|'warn'|'off', line: string}}
 */
export function remoteKeyboardStatus (rk) {
  if (!rk.enabled) return { nav: 'Off', tone: 'off', line: '' }
  if (!rk.host?.trim() || !rk.apiKey) {
    return { nav: 'On', tone: 'warn', line: 'Set a hostname and an API key to send presses.' }
  }
  const last = rk.last
  if (!last) return { nav: 'On', tone: 'ok', line: 'Ready. Nothing sent yet.' }
  const when = clock(last.at)
  if (last.ok) {
    return { nav: 'On', tone: 'ok', line: `Sent ${last.direction} at ${when} (HTTP ${last.status}).` }
  }
  const why = last.status ? `HTTP ${last.status}` : (last.error || 'no response')
  return { nav: 'On', tone: 'warn', line: `Last press failed at ${when}: ${why}.` }
}

/** True for an http(s) URL with a host. */
export function isRelayUrl (url) {
  try {
    const u = new URL(String(url))
    return (u.protocol === 'http:' || u.protocol === 'https:') && !!u.hostname
  } catch {
    return false
  }
}

/**
 * Art-Net status, from settings and the client's getStatus().
 *
 * @param {{artnetEnabled?: boolean, artnetUrl?: string}} settings
 * @param {null|{sent: number, failures: number, lastError: string|null}} status
 * @returns {{nav: string, tone: 'ok'|'warn'|'off', line: string}}
 */
export function artnetStatus (settings, status) {
  if (!settings.artnetEnabled) return { nav: 'Off', tone: 'off', line: '' }
  const url = settings.artnetUrl?.trim()
  if (!url) return { nav: 'On', tone: 'warn', line: 'Not configured: enter the relay URL. Nothing is sent until you do.' }
  if (!isRelayUrl(url)) return { nav: 'On', tone: 'warn', line: 'That is not an http(s) URL. Nothing is sent.' }
  if (status?.failures > 0 && status.lastError) {
    return { nav: 'On', tone: 'warn', line: `Last send failed: ${status.lastError}.` }
  }
  if (status?.sent > 0) {
    return { nav: 'On', tone: 'ok', line: `Sending. ${status.sent} update${status.sent === 1 ? '' : 's'} so far.` }
  }
  return { nav: 'On', tone: 'ok', line: 'Ready. The lights follow the next screensaver.' }
}

/**
 * The orphaned-reference banner, with the grammar right for one or several.
 *
 * @param {number} total references
 * @param {number} devices devices they belong to
 */
export function orphanBannerText (total, devices) {
  return `${total} reference${total === 1 ? ' belongs' : 's belong'} to ` +
    `${devices} device${devices === 1 ? '' : 's'} that ${devices === 1 ? 'is' : 'are'} ` +
    'not connected. If a device changed its id, re-capture its no-signal screen.'
}

function clock (ms) {
  const d = new Date(ms)
  const p = (n) => String(n).padStart(2, '0')
  return `${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`
}
