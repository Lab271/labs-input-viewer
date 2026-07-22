// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
/**
 * Reaction-Diffusion (Gray-Scott) — organic patterns that grow, divide and
 * crawl. Two chemicals A/B simulated on a ping-pong float texture; the B
 * concentration is colored for display. Periodically re-seeds so the pattern
 * keeps evolving rather than settling.
 */
import { createGLRuntime, createFullscreenPass, createPingPong } from './gl-base.js'

const SIM_FRAG = `#version 300 es
precision highp float;
uniform sampler2D uState;
uniform vec2 uTexel;
uniform float uFeed;
uniform float uKill;
uniform vec2 uSeed;   // seed location (or <0 for none)
out vec4 outState;

void main() {
  vec2 uv = gl_FragCoord.xy * uTexel;
  vec2 s = texture(uState, uv).xy;
  // Laplacian (3x3 kernel).
  vec2 lap = vec2(0.0);
  lap += texture(uState, uv + vec2(-1.0, 0.0) * uTexel).xy * 0.2;
  lap += texture(uState, uv + vec2( 1.0, 0.0) * uTexel).xy * 0.2;
  lap += texture(uState, uv + vec2( 0.0,-1.0) * uTexel).xy * 0.2;
  lap += texture(uState, uv + vec2( 0.0, 1.0) * uTexel).xy * 0.2;
  lap += texture(uState, uv + vec2(-1.0,-1.0) * uTexel).xy * 0.05;
  lap += texture(uState, uv + vec2( 1.0,-1.0) * uTexel).xy * 0.05;
  lap += texture(uState, uv + vec2(-1.0, 1.0) * uTexel).xy * 0.05;
  lap += texture(uState, uv + vec2( 1.0, 1.0) * uTexel).xy * 0.05;
  lap -= s;

  float A = s.x, B = s.y;
  // Canonical Gray-Scott diffusion rates (Karl Sims). Higher rates diffuse
  // the field to a uniform value within a few steps -> solid color, so keep
  // these modest.
  float dA = 0.2097, dB = 0.105;
  float reaction = A * B * B;
  float na = A + (dA * lap.x - reaction + uFeed * (1.0 - A));
  float nb = B + (dB * lap.y + reaction - (uKill + uFeed) * B);

  // Optional seed splat.
  if (uSeed.x >= 0.0) {
    float d = distance(uv, uSeed);
    if (d < 0.02) nb = 1.0;
  }

  outState = vec4(clamp(na, 0.0, 1.0), clamp(nb, 0.0, 1.0), 0.0, 1.0);
}`

const DISPLAY_FRAG = `#version 300 es
precision highp float;
uniform sampler2D uState;
uniform vec2 uResolution;  // canvas size in pixels
uniform float uTime;
out vec4 outColor;

vec3 palette(float t) {
  return 0.5 + 0.5 * cos(6.2831 * (t + vec3(0.0, 0.33, 0.6)));
}

void main() {
  // Map the full canvas to the [0,1] simulation texture, preserving the
  // square grid's aspect by covering the screen (centered).
  vec2 uv = gl_FragCoord.xy / uResolution;
  float b = texture(uState, uv).y;
  float v = smoothstep(0.1, 0.5, b);
  vec3 col = palette(v * 0.8 + uTime * 0.03) * v;
  col += vec3(0.02, 0.0, 0.04); // dim background
  outColor = vec4(col, 1.0);
}`

export default {
  name: 'Reaction Diffusion',
  create(canvas) {
    let runtime = null, gl = null
    let sim = null, display = null, pp = null
    const SIM = 320 // simulation grid resolution (square)
    let seedTimer = 0

    function makeSeed() {
      // Fill mostly A=1, B=0, with a few random B blobs.
      const data = new Float32Array(SIM * SIM * 4)
      for (let i = 0; i < SIM * SIM; i++) {
        data[i * 4 + 0] = 1.0
        data[i * 4 + 3] = 1.0
      }
      for (let s = 0; s < 20; s++) {
        const cx = Math.floor(Math.random() * SIM)
        const cy = Math.floor(Math.random() * SIM)
        for (let y = -6; y <= 6; y++) {
          for (let x = -6; x <= 6; x++) {
            const px = cx + x, py = cy + y
            if (px < 0 || py < 0 || px >= SIM || py >= SIM) continue
            data[(py * SIM + px) * 4 + 1] = 1.0
          }
        }
      }
      return data
    }

    return {
      start() {
        runtime = createGLRuntime(canvas)
        gl = runtime.gl
        gl.getExtension('EXT_color_buffer_float')
        pp = createPingPong(gl, SIM, SIM, makeSeed())
        sim = createFullscreenPass(gl, SIM_FRAG)
        display = createFullscreenPass(gl, DISPLAY_FRAG)

        runtime.start((time) => {
          // Several simulation substeps per frame for faster evolution.
          const steps = 8
          // Re-seed occasionally to keep it lively.
          let seedX = -1, seedY = -1
          seedTimer += 1
          if (seedTimer > 240) {
            seedTimer = 0
            seedX = Math.random()
            seedY = Math.random()
          }
          for (let i = 0; i < steps; i++) {
            gl.bindFramebuffer(gl.FRAMEBUFFER, pp.write.fbo)
            gl.viewport(0, 0, SIM, SIM)
            sim.draw((g, p) => {
              g.activeTexture(g.TEXTURE0)
              g.bindTexture(g.TEXTURE_2D, pp.read.tex)
              g.uniform1i(g.getUniformLocation(p, 'uState'), 0)
              g.uniform2f(g.getUniformLocation(p, 'uTexel'), 1 / SIM, 1 / SIM)
              g.uniform1f(g.getUniformLocation(p, 'uFeed'), 0.0367)
              g.uniform1f(g.getUniformLocation(p, 'uKill'), 0.0649)
              const sx = i === 0 ? seedX : -1
              g.uniform2f(g.getUniformLocation(p, 'uSeed'), sx, sx >= 0 ? seedY : -1)
            })
            pp.swap()
          }

          // Display pass to the screen.
          gl.bindFramebuffer(gl.FRAMEBUFFER, null)
          gl.viewport(0, 0, canvas.width, canvas.height)
          display.draw((g, p) => {
            g.activeTexture(g.TEXTURE0)
            g.bindTexture(g.TEXTURE_2D, pp.read.tex)
            g.uniform1i(g.getUniformLocation(p, 'uState'), 0)
            g.uniform2f(g.getUniformLocation(p, 'uResolution'), canvas.width, canvas.height)
            g.uniform1f(g.getUniformLocation(p, 'uTime'), time)
          })
        })
      },
      stop() {
        if (sim) { sim.destroy(); sim = null }
        if (display) { display.destroy(); display = null }
        if (pp) { pp.destroy(); pp = null }
        if (runtime) { runtime.destroy(); runtime = null }
        gl = null
      }
    }
  }
}
