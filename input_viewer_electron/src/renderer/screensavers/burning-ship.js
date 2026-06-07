/**
 * Burning Ship — the Burning Ship fractal with an endless auto-zoom tour into
 * the iconic "ship" antenna detail, with smooth coloring.
 *
 * Iteration is like the Mandelbrot but takes the absolute value of the real
 * and imaginary parts before squaring:
 *   z = (|Re z| + i|Im z|)^2 + c
 */
import { createShaderScreensaver } from './gl-base.js'

const SHADER = /* glsl */ `
vec3 palette(float t) {
  // Warm "burning" palette: blacks -> reds -> oranges -> yellow-white.
  return vec3(
    smoothstep(0.0, 0.6, t),
    smoothstep(0.25, 0.9, t) * 0.7,
    smoothstep(0.6, 1.0, t) * 0.5
  ) + vec3(0.0, 0.0, 0.05);
}

void mainImage(out vec4 fragColor, in vec2 fragCoord) {
  vec2 res = iResolution.xy;
  vec2 uv = (fragCoord - 0.5 * res) / res.y;

  // Smooth in/out zoom toward a detailed point on the main ship's antenna.
  float zt = iTime * 0.13;
  float zoom = exp(0.5 + 3.2 * (0.5 - 0.5 * cos(zt)));
  vec2 center = vec2(-1.7549, -0.0286); // antenna detail region

  vec2 c = center + uv / zoom;

  vec2 z = vec2(0.0);
  const float MAX_I = 256.0;
  float i = 0.0;
  for (float n = 0.0; n < MAX_I; n++) {
    // Burning ship: abs the components before squaring.
    z = abs(z);
    z = vec2(z.x * z.x - z.y * z.y, 2.0 * z.x * z.y) + c;
    if (dot(z, z) > 256.0) { i = n; break; }
    i = n;
  }

  vec3 col;
  if (dot(z, z) <= 256.0) {
    col = vec3(0.0);
  } else {
    float sn = i - log2(log2(dot(z, z))) + 4.0;
    float t = fract(sn * 0.025 + iTime * 0.04);
    col = palette(t);
    col *= 0.7 + 0.4 * sin(sn * 0.2);
  }
  fragColor = vec4(col, 1.0);
}
`

export default createShaderScreensaver('Burning Ship', SHADER)
