// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
/**
 * Mandelbrot — endless auto-zoom tour into the set with smooth coloring.
 *
 * The camera continuously zooms toward a fixed interesting point and the
 * palette cycles, so it never needs interaction. Uses smooth (continuous)
 * iteration count to avoid banding.
 */
import { createShaderScreensaver } from './gl-base.js'

const SHADER = /* glsl */ `
vec3 palette(float t) {
  return 0.5 + 0.5 * cos(6.2831 * (t + vec3(0.0, 0.33, 0.67)));
}

void mainImage(out vec4 fragColor, in vec2 fragCoord) {
  vec2 res = iResolution.xy;
  vec2 uv = (fragCoord - 0.5 * res) / res.y;

  // Zoom oscillates in and back out so the tour loops smoothly.
  float zt = iTime * 0.15;
  float zoom = exp(-2.0 + 3.0 * (0.5 - 0.5 * cos(zt))); // smooth in/out
  // A point on the boundary that stays interesting while zooming.
  vec2 center = vec2(-0.74364388703, 0.13182590421);

  vec2 c = center + uv / zoom;

  vec2 z = vec2(0.0);
  const float MAX_I = 256.0;
  float i = 0.0;
  for (float n = 0.0; n < MAX_I; n++) {
    z = vec2(z.x * z.x - z.y * z.y, 2.0 * z.x * z.y) + c;
    if (dot(z, z) > 256.0) { i = n; break; }
    i = n;
  }

  vec3 col;
  if (dot(z, z) <= 256.0) {
    col = vec3(0.0); // inside the set
  } else {
    // Smooth iteration count.
    float sn = i - log2(log2(dot(z, z))) + 4.0;
    col = palette(sn * 0.02 + iTime * 0.05);
    col *= 0.6 + 0.4 * sin(sn * 0.3);
  }
  fragColor = vec4(col, 1.0);
}
`

export default createShaderScreensaver('Mandelbrot', SHADER)
