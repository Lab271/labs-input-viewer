// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
// Regenerates plasma.frag and the post-FX passes from the Electron app's sources, so the two cannot
// drift by hand-copying:  node input_viewer_egui/shaders/extract.mjs
import { readFileSync, writeFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

const here = dirname(fileURLToPath(import.meta.url))
const savers = join(here, '../../input_viewer_electron/src/renderer/screensavers')
const { GLSL } = await import(join(savers, 'glsl-lib.js'))

const src = readFileSync(join(savers, 'plasma.js'), 'utf8')
const body = src.match(/const SHADER = \/\* glsl \*\/ `([\s\S]*?)`\n/)[1]
const glsl = body.replace(/\$\{GLSL\.(\w+)\}/g, (_, k) => {
  if (!(k in GLSL)) throw new Error(`GLSL.${k} not found`)
  return GLSL[k]
})
writeFileSync(join(here, 'plasma.frag'),
  '// GENERATED from input_viewer_electron/.../plasma.js by extract.mjs -- do not edit.\n' + glsl)
console.log(`plasma.frag: ${glsl.split('\n').length} lines`)

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
