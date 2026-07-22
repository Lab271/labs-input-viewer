// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
/**
 * Standalone screensaver preview harness.
 *
 * Run via `npm run screensaver` (opens this page in a browser with a real
 * GPU/WebGL2 context). Pass a selector to jump straight to one:
 *   npm run screensaver -- 1          (by index)
 *   npm run screensaver -- plasma     (by name)
 *
 * The npm script forwards the selector as the URL hash (#1 / #plasma).
 * Reuses the exact registry + screensaver modules the app ships.
 */
import {
  SCREENSAVERS,
  initScreensavers,
  startScreensaver,
  stopScreensaver,
  listScreensavers
} from './registry.js'

const canvas = document.getElementById('screensaver-canvas')
const listEl = document.getElementById('list')
const hud = document.getElementById('hud')
const fpsEl = document.getElementById('fps')

initScreensavers(canvas)

let current = -1

function normalize(s) {
  return s.toLowerCase().replace(/\s+/g, '')
}

// Resolve initial selection from the URL hash (#1 or #plasma), else random.
function initialIndex() {
  const raw = decodeURIComponent(location.hash.replace(/^#/, '')).trim()
  if (!raw) return Math.floor(Math.random() * SCREENSAVERS.length)
  const asNum = Number(raw)
  if (!Number.isNaN(asNum) && raw !== '') {
    return ((asNum % SCREENSAVERS.length) + SCREENSAVERS.length) % SCREENSAVERS.length
  }
  const idx = SCREENSAVERS.findIndex((s) => normalize(s.name) === normalize(raw))
  return idx >= 0 ? idx : 0
}

function renderList() {
  listEl.innerHTML = ''
  listScreensavers().forEach((name, i) => {
    const btn = document.createElement('button')
    btn.textContent = `${i + 1}. ${name}`
    btn.className = i === current ? 'active' : ''
    btn.onclick = () => select(i)
    listEl.appendChild(btn)
  })
}

function select(index) {
  current = ((index % SCREENSAVERS.length) + SCREENSAVERS.length) % SCREENSAVERS.length
  stopScreensaver()
  const name = startScreensaver(current)
  location.hash = String(current + 1)
  document.title = `Screensaver: ${name}`
  renderList()
}

// FPS meter.
let frames = 0
let lastFpsT = performance.now()
function fpsLoop() {
  frames++
  const now = performance.now()
  if (now - lastFpsT >= 500) {
    fpsEl.textContent = `${Math.round((frames * 1000) / (now - lastFpsT))} fps`
    frames = 0
    lastFpsT = now
  }
  requestAnimationFrame(fpsLoop)
}
requestAnimationFrame(fpsLoop)

window.addEventListener('keydown', (e) => {
  switch (e.key) {
    case 'ArrowRight': select(current + 1); break
    case 'ArrowLeft': select(current - 1); break
    case 'r': case 'R': select(current); break
    case 'h': case 'H': hud.classList.toggle('hidden'); break
    case 'f': case 'F':
      if (!document.fullscreenElement) document.documentElement.requestFullscreen()
      else document.exitFullscreen()
      break
  }
})

select(initialIndex())
