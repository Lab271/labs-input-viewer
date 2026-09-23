// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import { createRequire } from 'node:module'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'

const require = createRequire(import.meta.url)
const { createFileLog, dayStamp } = require('../src/main/file-log.js')

let dir
beforeEach(() => { dir = fs.mkdtempSync(path.join(os.tmpdir(), 'iv-log-')) })
afterEach(() => { fs.rmSync(dir, { recursive: true, force: true }) })

function clock(start) {
  let t = new Date(start)
  return { now: () => t, set: (d) => { t = new Date(d) }, advance: (ms) => { t = new Date(t.getTime() + ms) } }
}

function readToday(log) {
  return fs.readFileSync(log.path(), 'utf8')
}

describe('createFileLog', () => {
  it('writes one timestamped line per record into a file named for the day', () => {
    const c = clock('2026-09-23T08:40:00')
    const log = createFileLog({ dir, now: c.now })
    log.write('info', 'renderer', '[Health] left ok')
    log.write('warn', 'main', 'two\nlines')
    expect(path.basename(log.path())).toBe('input-viewer-2026-09-23.log')
    const text = readToday(log)
    expect(text).toMatch(/^08:40:00\.000 INFO {2}\[renderer\] \[Health\] left ok\n/)
    expect(text).toContain('WARN  [main] two\n    lines\n')
  })

  it('rolls to a new file at midnight', () => {
    const c = clock('2026-09-23T23:59:59')
    const log = createFileLog({ dir, now: c.now })
    log.write('info', 'main', 'before')
    c.advance(2000)
    log.write('info', 'main', 'after')
    expect(fs.readdirSync(dir).sort()).toEqual([
      'input-viewer-2026-09-23.log', 'input-viewer-2026-09-24.log',
    ])
  })

  it('deletes files older than a week, and leaves other files alone', () => {
    for (const d of ['2026-09-10', '2026-09-16', '2026-09-17', '2026-09-22']) {
      fs.writeFileSync(path.join(dir, `input-viewer-${d}.log`), 'x')
    }
    fs.writeFileSync(path.join(dir, 'settings.json'), '{}')
    const c = clock('2026-09-23T09:00:00')
    const log = createFileLog({ dir, retentionDays: 7, now: c.now })
    log.write('info', 'main', 'start')
    // Seven calendar days kept: 17th..23rd.
    expect(fs.readdirSync(dir).sort()).toEqual([
      'input-viewer-2026-09-17.log', 'input-viewer-2026-09-22.log',
      'input-viewer-2026-09-23.log', 'settings.json',
    ])
  })

  it('prunes again when the day changes, not only at startup', () => {
    const c = clock('2026-09-23T12:00:00')
    const log = createFileLog({ dir, retentionDays: 7, now: c.now })
    log.write('info', 'main', 'day 0')
    c.set('2026-10-01T00:00:01')
    log.write('info', 'main', 'day 8')
    expect(fs.readdirSync(dir)).toEqual(['input-viewer-2026-10-01.log'])
  })

  it('stops growing at the daily cap and says so once', () => {
    const c = clock('2026-09-23T09:00:00')
    const log = createFileLog({ dir, maxBytesPerDay: 200, now: c.now })
    for (let i = 0; i < 50; i++) log.write('info', 'main', `line ${i}`)
    const text = readToday(log)
    expect(text.length).toBeLessThan(400)
    expect(text.match(/daily cap reached/g)).toHaveLength(1)
  })

  it('rate-limits a flood and reports how much was dropped', () => {
    const c = clock('2026-09-23T09:00:00')
    const log = createFileLog({ dir, maxLinesPerMinute: 5, now: c.now })
    for (let i = 0; i < 20; i++) log.write('info', 'renderer', `spam ${i}`)
    c.advance(60_000)
    log.write('info', 'renderer', 'next minute')
    const text = readToday(log)
    expect(text.match(/spam/g)).toHaveLength(5)
    expect(text).toContain('dropped 15 line(s)')
    expect(text).toContain('next minute')
  })

  it('truncates a runaway line', () => {
    const c = clock('2026-09-23T09:00:00')
    const log = createFileLog({ dir, maxLineLength: 10, now: c.now })
    log.write('info', 'main', 'x'.repeat(100))
    expect(readToday(log)).toContain('xxxxxxxxxx...(truncated)')
  })

  it('never throws when the directory cannot be written', () => {
    const blocker = path.join(dir, 'not-a-dir')
    fs.writeFileSync(blocker, '')
    const log = createFileLog({ dir: blocker, now: () => new Date('2026-09-23T09:00:00') })
    expect(() => log.write('error', 'main', 'still fine')).not.toThrow()
  })

  it('stamps days in local time', () => {
    expect(dayStamp(new Date(2026, 0, 5, 23, 30))).toBe('2026-01-05')
  })
})
