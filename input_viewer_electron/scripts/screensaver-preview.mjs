#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
/**
 * Launches the screensaver preview harness (Vite dev server + browser).
 *
 *   npm run screensaver            -> random screensaver + picker
 *   npm run screensaver -- 1       -> open screensaver index 1 (1-based)
 *   npm run screensaver -- plasma  -> open the "plasma" screensaver by name
 *
 * The selector is forwarded as the page URL hash, which preview.js reads.
 */
import { createServer } from 'vite'
import { spawn } from 'node:child_process'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const root = path.resolve(__dirname, '..')

const selector = process.argv[2] ? `#${encodeURIComponent(process.argv[2])}` : ''

function openBrowser(url) {
  const cmd =
    process.platform === 'darwin' ? 'open'
      : process.platform === 'win32' ? 'cmd'
        : 'xdg-open'
  const args = process.platform === 'win32' ? ['/c', 'start', '""', url] : [url]
  spawn(cmd, args, { stdio: 'ignore', detached: true }).unref()
}

const server = await createServer({
  configFile: path.resolve(root, 'vite.preview.config.mjs'),
  // We open the browser ourselves so we can append the selector hash.
  server: { open: false }
})
await server.listen()

const { port } = server.config.server
const base = `http://localhost:${port}/preview.html${selector}`
server.printUrls()
console.log(`\n  Screensaver preview: ${base}`)
console.log('  Keys in window: ←/→ cycle · R restart · H hide HUD · F fullscreen')
console.log('  Press Ctrl+C to stop.\n')
openBrowser(base)
