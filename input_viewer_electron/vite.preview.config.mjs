// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
import { defineConfig } from 'vite'
import path from 'path'
import { fileURLToPath } from 'url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))

// Standalone (non-Electron) Vite config for the screensaver preview harness.
// Serves src/renderer/screensavers/preview.html in a plain browser with a
// real WebGL2 context. Used by `npm run screensaver`.
export default defineConfig({
  root: path.resolve(__dirname, 'src/renderer/screensavers'),
  server: {
    open: '/preview.html',
    port: 5180
  }
})
