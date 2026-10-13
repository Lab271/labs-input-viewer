// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
// Regenerates the screensaver shaders and the post-FX passes from the Electron app's sources, so
// the two cannot drift by hand-copying:  node input_viewer_egui/shaders/extract.mjs
import { readFileSync, writeFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

const here = dirname(fileURLToPath(import.meta.url))
const savers = join(here, '../../input_viewer_electron/src/renderer/screensavers')
const { GLSL } = await import(join(savers, 'glsl-lib.js'))

// The pure fragment-shader savers (ADR 0026). Their render options (bloom,
// tonemap, exposure, antialias) are transcribed in shader_saver.rs.
const SAVERS = ['plasma', 'flow-field', 'julia-family', 'burning-ship', 'moire', 'bicycle-horizon']

// The SHADER template is *evaluated*, not regex-substituted: some interpolate
// module constants (julia-family's WINDOW_RANGE), and evaluating also applies
// the template's own escapes exactly as the browser does. The module body runs
// with its imports removed and everything from `export default` on dropped,
// so only its top-level constants are defined -- no GL, no DOM.
for (const name of SAVERS) {
  const src = readFileSync(join(savers, `${name}.js`), 'utf8')
  const body = src
    .replace(/^import .*$/gm, '')
    .replace(/^export default[\s\S]*$/m, '')
  const glsl = new Function('GLSL', `${body}\nreturn SHADER`)(GLSL)
  if (typeof glsl !== 'string' || !glsl.includes('mainImage')) throw new Error(`${name}: no SHADER with mainImage`)
  writeFileSync(join(here, `${name}.frag`),
    `// GENERATED from input_viewer_electron/.../${name}.js by extract.mjs -- do not edit.\n` + glsl)
  console.log(`${name}.frag: ${glsl.split('\n').length} lines`)
}

// The post-FX passes are module-private constants, so they are pulled out of
// the source text rather than imported.
const post = readFileSync(join(savers, 'post-fx.js'), 'utf8')
for (const name of ['BRIGHT_PASS', 'DOWNSAMPLE', 'UPSAMPLE', 'COMPOSITE']) {
  const m = post.match(new RegExp(`const ${name} = \`([\\s\\S]*?)\``))
  if (!m) throw new Error(`${name} not found in post-fx.js`)
  // GLSL ES 3.00 -> desktop GLSL 3.30, which is what egui's glow backend gets.
  const glsl = m[1].replace('#version 300 es\nprecision highp float;', '#version 330 core')
  const file = `post-${name.toLowerCase().replace('_pass', '')}.frag`
  writeFileSync(join(here, file),
    `// GENERATED from post-fx.js ${name} by extract.mjs -- do not edit.\n` + glsl + '\n')
  console.log(file)
}
