// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
/**
 * Capture-card health: is this input actually working, and should it be reopened?
 *
 * The wall runs two Elgato 4K60 Pro MK.2 cards, and sometimes one comes up after
 * boot showing black (or the card's own no-signal picture) with a live source
 * plugged in -- until someone restarts the app. The split-flap board never shows
 * for it, because the stream opened fine; nothing downstream knew it was bad.
 *
 * The driver exposes no signal-lock state outside Elgato's own SDK (checked on
 * the wall: nothing in the registry, no service), so the only evidence is what
 * arrives on the stream. This module turns that evidence into a status and a
 * reopen decision. It is pure -- no DOM, no timers, the clock is passed in -- so
 * the policy is unit tested; renderer.js owns the sampling and the reopen.
 *
 * Statuses:
 *   opening      stream open, no frame yet
 *   ok           frames arriving and the picture is not uniform
 *   no-frames    open for FIRST_FRAME_TIMEOUT_MS and not a single frame
 *   stalled      frames WERE flowing and have stopped for STALL_MS
 *   ended        the track ended (driver dropped it, device went away)
 *   open-failed  getUserMedia rejected
 *   dark         frames arriving, but the picture is one flat colour
 *
 * Why a frame COUNT and not motion: #159 rejected motion detection because a
 * held slide is pixel-identical for minutes. A capture card still delivers that
 * held slide at 60fps, so "frames stopped arriving" is a different and safe
 * question. The "was flowing" condition keeps a virtual camera showing a static
 * image -- which genuinely sends only a frame or two (see the rVFC watchdog note
 * in renderer.js) -- from being reported as stalled.
 *
 * Why reopening a dark feed is safe: a reopen blanks the picture for a moment.
 * On a feed that is already one flat colour, that blank is invisible. The worst
 * case is a presenter deliberately blanking to black (PowerPoint `B`), where a
 * reopen is equally invisible. The dark cadence is slow for exactly this reason:
 * with no source plugged in all night, it is the steady state.
 */

export const HEALTH = {
  /** Open but no frame at all after this long: the card is not delivering. */
  FIRST_FRAME_TIMEOUT_MS: 8000,
  /** Frames stopped for this long after flowing: stalled. */
  STALL_MS: 6000,
  /** A feed must deliver at least this rate once before a stop counts as a stall. */
  FLOWING_MIN_FPS: 5,
  /** Uniform picture for this long before it counts as dark. */
  DARK_MS: 6000,
  /** Healthy this long and the reopen backoff starts over. */
  RECOVERED_RESET_MS: 60_000,
  /** Delays before each reopen of a faulted feed; the last one repeats. */
  FAULT_BACKOFF_MS: [2000, 5000, 15_000, 30_000, 60_000],
  /** Delays before each reopen of a dark feed; the last one repeats. */
  DARK_BACKOFF_MS: [5000, 15_000, 30_000, 60_000, 5 * 60_000, 10 * 60_000],
  /** Standard deviation of luma (0-255) below which a picture is one flat colour. */
  UNIFORM_STD_MAX: 3,
}

const FAULTS = new Set(['no-frames', 'stalled', 'ended', 'open-failed'])

/**
 * Mean and standard deviation of luma over an RGBA pixel buffer.
 *
 * Meant for a tiny downscale (32x18 is 576 pixels), not a full frame.
 *
 * @param {Uint8ClampedArray|Uint8Array} data RGBA
 * @returns {{mean: number, std: number}}
 */
export function lumaStats(data) {
  const n = Math.floor(data.length / 4)
  if (n === 0) return { mean: 0, std: 0 }
  let sum = 0
  let sumSq = 0
  for (let i = 0; i < n; i++) {
    const o = i * 4
    // Rec. 601 weights; exactness is irrelevant, flatness is the question.
    const y = 0.299 * data[o] + 0.587 * data[o + 1] + 0.114 * data[o + 2]
    sum += y
    sumSq += y * y
  }
  const mean = sum / n
  const variance = Math.max(0, sumSq / n - mean * mean)
  return { mean, std: Math.sqrt(variance) }
}

/** Is this picture one flat colour -- black, or a solid no-signal screen? */
export function isUniform(stats) {
  return !!stats && stats.std <= HEALTH.UNIFORM_STD_MAX
}

/** Delay before reopen number `attempt` (0-based) under a backoff schedule. */
export function backoffDelay(schedule, attempt) {
  return schedule[Math.min(Math.max(0, attempt), schedule.length - 1)]
}

/**
 * Health state for one side's stream.
 *
 * Feed it events and samples; ask it what to do. `now` is a millisecond clock
 * (performance.now() in the app, a plain number in tests).
 */
export function createStreamHealth() {
  let status = 'idle'
  let openedAt = 0
  let lastFrames = null      // last frame counter reading
  let lastFrameAt = 0        // when the counter last moved
  let lastSampleAt = 0
  let firstFrameAt = null
  let flowing = false        // has this stream ever delivered at FLOWING_MIN_FPS
  let uniformSince = null
  let lastStats = null
  let fps = 0
  let error = null

  // Survives reopens, so the backoff actually backs off.
  let attempts = 0
  let lastReopenAt = null
  let healthySince = null

  function setStatus(next, now) {
    status = next
    if (next === 'ok') {
      if (healthySince === null) healthySince = now
    } else {
      healthySince = null
    }
  }

  return {
    /** A stream was just attached. */
    opened(now) {
      openedAt = now
      lastFrames = null
      lastFrameAt = now
      lastSampleAt = now
      firstFrameAt = null
      flowing = false
      uniformSince = null
      lastStats = null
      fps = 0
      error = null
      setStatus('opening', now)
    },

    /** getUserMedia rejected. */
    openFailed(now, err) {
      openedAt = now
      error = err ? String(err.name || err.message || err) : 'unknown'
      setStatus('open-failed', now)
    },

    /** Nothing to monitor on this side (no device, disabled, torn down). */
    clear() {
      status = 'idle'
      attempts = 0
      lastReopenAt = null
      healthySince = null
      error = null
    },

    /**
     * One observation.
     *
     * @param {{frames: number|null, ended: boolean, stats: {mean:number,std:number}|null}} s
     *   frames: a monotonically increasing count of frames delivered, or null if
     *   the platform cannot say. stats: luma of a small downscale, or null.
     * @param {number} now
     * @returns {string} the status after this sample
     */
    sample({ frames, ended, stats }, now) {
      if (status === 'idle' || status === 'open-failed') return status

      if (ended) {
        setStatus('ended', now)
        return status
      }

      if (typeof frames === 'number') {
        if (lastFrames === null) {
          lastFrames = frames
          // A counter that already reads > 0 on the first look has delivered.
          if (frames > 0) { lastFrameAt = now; firstFrameAt = now }
        } else if (frames > lastFrames) {
          const dt = now - lastSampleAt
          fps = dt > 0 ? ((frames - lastFrames) * 1000) / dt : fps
          if (fps >= HEALTH.FLOWING_MIN_FPS) flowing = true
          if (firstFrameAt === null) firstFrameAt = now
          lastFrames = frames
          lastFrameAt = now
        } else if (frames < lastFrames) {
          // The counter went backwards: the element's playback counter restarts
          // when its source is replaced. A new baseline, not evidence of a stall.
          lastFrames = frames
          lastFrameAt = now
        } else {
          fps = 0
        }
      }
      lastSampleAt = now

      if (stats) {
        lastStats = stats
        if (isUniform(stats)) {
          if (uniformSince === null) uniformSince = now
        } else {
          uniformSince = null
        }
      }

      if (typeof frames === 'number') {
        if (firstFrameAt === null) {
          if (now - openedAt >= HEALTH.FIRST_FRAME_TIMEOUT_MS) {
            setStatus('no-frames', now)
            return status
          }
          setStatus('opening', now)
          return status
        }
        if (flowing && now - lastFrameAt >= HEALTH.STALL_MS) {
          setStatus('stalled', now)
          return status
        }
      }

      if (uniformSince !== null && now - uniformSince >= HEALTH.DARK_MS) {
        setStatus('dark', now)
        return status
      }

      setStatus('ok', now)
      if (attempts > 0 && now - healthySince >= HEALTH.RECOVERED_RESET_MS) {
        attempts = 0
        lastReopenAt = null
      }
      return status
    },

    /**
     * Should this side be reopened now?
     *
     * @returns {{reopen: boolean, reason: string|null, waitMs: number}}
     */
    decide(now) {
      const schedule = FAULTS.has(status) ? HEALTH.FAULT_BACKOFF_MS
        : status === 'dark' ? HEALTH.DARK_BACKOFF_MS
          : null
      if (!schedule) return { reopen: false, reason: null, waitMs: 0 }

      // Measured from the last reopen, or from when the stream opened for the
      // first attempt -- so a stream that goes bad an hour in is reopened after
      // the first delay, not immediately.
      const since = lastReopenAt ?? openedAt
      const wait = backoffDelay(schedule, attempts)
      const waitMs = Math.max(0, since + wait - now)
      return { reopen: waitMs === 0, reason: status, waitMs }
    },

    /** A reopen was started. */
    reopening(now) {
      attempts += 1
      lastReopenAt = now
    },

    /** Snapshot for logging. */
    info() {
      return {
        status,
        fps: Math.round(fps * 10) / 10,
        flowing,
        attempts,
        luma: lastStats ? Math.round(lastStats.mean) : null,
        lumaStd: lastStats ? Math.round(lastStats.std * 10) / 10 : null,
        error,
      }
    },
  }
}
