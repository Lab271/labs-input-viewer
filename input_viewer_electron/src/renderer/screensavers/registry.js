// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
/**
 * Screensaver registry.
 *
 * Holds the list of available screensavers and drives a single active one
 * against a shared <canvas>. A new random screensaver is chosen on each
 * activation (start), so the no-signal screen varies over time.
 *
 * Each screensaver module is an object:
 *   { name: string, create(canvas) -> { start(), stop() } }
 *
 * To add a screensaver: drop a module in this folder and add it to the
 * SCREENSAVERS array below. Pure fragment-shader ones can use
 * createShaderScreensaver from gl-base.js.
 */
import dvdLogo from './dvd-logo.js'
import plasma from './plasma.js'
import flowField from './flow-field.js'
import raymarch from './raymarch.js'
import mandelbrot from './mandelbrot.js'
import julia from './julia.js'
import burningShip from './burning-ship.js'
import reactionDiffusion from './reaction-diffusion.js'
import particleSwarm from './particle-swarm.js'
import whiteParticles from './white-particles.js'
import boids from './boids.js'
import strangeAttractor from './strange-attractor.js'

export const SCREENSAVERS = [
  dvdLogo,
  plasma,
  flowField,
  raymarch,
  mandelbrot,
  julia,
  burningShip,
  reactionDiffusion,
  particleSwarm,
  whiteParticles,
  boids,
  strangeAttractor
]

let canvasEl = null
let active = null          // the running instance { start, stop }
let activeIndex = -1
let running = false

/**
 * Bind the registry to a canvas element. Call once at init.
 * @param {HTMLCanvasElement} canvas
 */
export function initScreensavers(canvas) {
  canvasEl = canvas
}

/** @returns {string[]} names of all registered screensavers */
export function listScreensavers() {
  return SCREENSAVERS.map((s) => s.name)
}

/**
 * Resolve a selector (index, name, or undefined=random) to an index.
 * @param {number|string|undefined} selector
 * @returns {number}
 */
function resolveIndex(selector) {
  if (selector === undefined || selector === null || selector === 'random') {
    return Math.floor(Math.random() * SCREENSAVERS.length)
  }
  if (typeof selector === 'number') {
    return ((selector % SCREENSAVERS.length) + SCREENSAVERS.length) % SCREENSAVERS.length
  }
  // String name (case-insensitive, ignore spaces).
  const norm = (s) => s.toLowerCase().replace(/\s+/g, '')
  const idx = SCREENSAVERS.findIndex((s) => norm(s.name) === norm(selector))
  return idx >= 0 ? idx : Math.floor(Math.random() * SCREENSAVERS.length)
}

/**
 * Start a screensaver on the bound canvas.
 * @param {number|string} [selector] - index, name, or omit for random
 * @returns {string} the name of the screensaver that started
 */
export function startScreensaver(selector) {
  if (!canvasEl) throw new Error('initScreensavers(canvas) must be called first')
  if (running) stopScreensaver()

  activeIndex = resolveIndex(selector)
  const saver = SCREENSAVERS[activeIndex]
  try {
    active = saver.create(canvasEl)
    active.start()
    running = true
    console.log(`[Screensaver] Started: ${saver.name}`)
  } catch (err) {
    console.error(`[Screensaver] Failed to start "${saver.name}":`, err)
    // Fall back to the DVD logo if a fancy one fails (e.g. shader error).
    if (saver !== SCREENSAVERS[0]) {
      activeIndex = 0
      active = SCREENSAVERS[0].create(canvasEl)
      active.start()
      running = true
      console.log('[Screensaver] Fell back to DVD Logo')
    }
  }
  return SCREENSAVERS[activeIndex]?.name
}

/** Stop the active screensaver. */
export function stopScreensaver() {
  if (active) {
    try { active.stop() } catch (err) { console.error('[Screensaver] stop error:', err) }
    active = null
  }
  running = false
  activeIndex = -1
}

/** @returns {boolean} */
export function isScreensaverRunning() {
  return running
}
