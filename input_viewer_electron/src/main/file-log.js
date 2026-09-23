// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
/**
 * Persistent app log: one file per day, kept for a week.
 *
 * Before this, nothing the app printed survived it. The wall came up with a dark
 * capture card, someone restarted the app ten minutes later, and there was no way
 * to tell afterwards what the renderer had seen. This is the record.
 *
 * Bounded three ways, because the wall runs for months:
 *   - files older than `retentionDays` are deleted (on start and at each day change)
 *   - a day's file stops growing at `maxBytesPerDay`, with one line saying so
 *   - more than `maxLinesPerMinute` lines in a minute are dropped and counted,
 *     so a log loop cannot fill the disk before the day cap even matters
 * Worst case on disk is retentionDays x maxBytesPerDay; the default is ~140 MB and
 * a normal day is a few hundred KB.
 *
 * Synchronous appends on purpose: a crash or a hard power-off is exactly when
 * the last lines matter, and at the rate limit the cost is nothing.
 */
const fs = require('fs')
const path = require('path')

const FILE_RE = /^input-viewer-(\d{4})-(\d{2})-(\d{2})\.log$/

function pad(n) {
  return String(n).padStart(2, '0')
}

/** Local-date stamp, so a file is one wall-clock day where the wall is. */
function dayStamp(d) {
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`
}

function timeStamp(d) {
  return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}.` +
    String(d.getMilliseconds()).padStart(3, '0')
}

/**
 * @param {{dir: string, retentionDays?: number, maxBytesPerDay?: number,
 *          maxLinesPerMinute?: number, maxLineLength?: number, now?: () => Date}} opts
 */
function createFileLog({
  dir,
  retentionDays = 7,
  maxBytesPerDay = 20 * 1024 * 1024,
  maxLinesPerMinute = 600,
  maxLineLength = 2000,
  now = () => new Date(),
}) {
  let day = null
  let file = null
  let bytes = 0
  let capped = false
  let minute = null
  let linesThisMinute = 0
  let dropped = 0
  let broken = false

  function prune(today) {
    let names
    try {
      names = fs.readdirSync(dir)
    } catch {
      return []
    }
    // Compare calendar days, not mtimes: a file touched late is still that day.
    const cutoff = new Date(today.getFullYear(), today.getMonth(), today.getDate() - (retentionDays - 1))
    const removed = []
    for (const name of names) {
      const m = FILE_RE.exec(name)
      if (!m) continue
      const fileDay = new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3]))
      if (fileDay < cutoff) {
        try {
          fs.unlinkSync(path.join(dir, name))
          removed.push(name)
        } catch { /* in use or gone; try again tomorrow */ }
      }
    }
    return removed
  }

  function rollIfNeeded(d) {
    const stamp = dayStamp(d)
    if (stamp === day) return
    day = stamp
    file = path.join(dir, `input-viewer-${stamp}.log`)
    capped = false
    try {
      fs.mkdirSync(dir, { recursive: true })
      bytes = fs.existsSync(file) ? fs.statSync(file).size : 0
    } catch {
      bytes = 0
    }
    prune(d)
  }

  function append(text) {
    if (broken) return
    try {
      fs.appendFileSync(file, text)
      bytes += Buffer.byteLength(text)
    } catch {
      // A log that cannot be written must never take the app down with it.
      // Stop trying rather than throw on every console.log.
      broken = true
    }
  }

  return {
    /**
     * @param {string} level  info | warn | error | debug
     * @param {string} source main | renderer | ...
     * @param {string} message
     */
    write(level, source, message) {
      const d = now()
      rollIfNeeded(d)

      const m = Math.floor(d.getTime() / 60_000)
      if (m !== minute) {
        if (dropped > 0 && !capped) {
          append(`${timeStamp(d)} WARN  [log] dropped ${dropped} line(s) over ` +
            `${maxLinesPerMinute}/min in the previous minute\n`)
        }
        minute = m
        linesThisMinute = 0
        dropped = 0
      }
      if (linesThisMinute >= maxLinesPerMinute) {
        dropped += 1
        return
      }
      linesThisMinute += 1

      if (capped) return
      if (bytes >= maxBytesPerDay) {
        capped = true
        append(`${timeStamp(d)} WARN  [log] ${maxBytesPerDay} byte daily cap reached; ` +
          'nothing more is written until tomorrow\n')
        return
      }

      let text = String(message)
      if (text.length > maxLineLength) text = text.slice(0, maxLineLength) + '...(truncated)'
      // One record per line, so a multi-line stack does not interleave with others.
      text = text.replace(/\r?\n/g, '\n    ')
      append(`${timeStamp(d)} ${level.toUpperCase().padEnd(5)} [${source}] ${text}\n`)
    },

    /** Delete files past retention. Returns the names removed. */
    prune() {
      return prune(now())
    },

    /** Today's file. */
    path() {
      rollIfNeeded(now())
      return file
    },

    dir,
  }
}

module.exports = { createFileLog, dayStamp }
