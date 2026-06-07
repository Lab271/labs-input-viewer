/**
 * Julia set — continuously morphing c-parameter traces a path through
 * parameter space, so the fractal shape constantly evolves.
 */
import { createShaderScreensaver } from './gl-base.js'

const SHADER = /* glsl */ `
vec3 palette(float t) {
  return 0.5 + 0.5 * cos(6.2831 * (t + vec3(0.0, 0.40, 0.75)));
}

void mainImage(out vec4 fragColor, in vec2 fragCoord) {
  vec2 res = iResolution.xy;
  vec2 uv = (fragCoord - 0.5 * res) / res.y * 1.6;

  // Animate c along a smooth Lissajous-like path near the boundary.
  float t = iTime * 0.12;
  vec2 c = vec2(0.7885 * cos(t * 1.3), 0.7885 * sin(t * 0.9));
  // Pull it toward classic interesting values.
  c = mix(c, vec2(-0.8, 0.156), 0.3 + 0.3 * sin(t * 0.5));

  vec2 z = uv;
  const float MAX_I = 256.0;
  float i = 0.0;
  for (float n = 0.0; n < MAX_I; n++) {
    z = vec2(z.x * z.x - z.y * z.y, 2.0 * z.x * z.y) + c;
    if (dot(z, z) > 256.0) { i = n; break; }
    i = n;
  }

  vec3 col;
  if (dot(z, z) <= 256.0) {
    col = vec3(0.02, 0.0, 0.05);
  } else {
    float sn = i - log2(log2(dot(z, z))) + 4.0;
    col = palette(sn * 0.025 + iTime * 0.04);
    col *= 0.5 + 0.5 * sin(sn * 0.25 + iTime);
  }
  fragColor = vec4(col, 1.0);
}
`

export default createShaderScreensaver('Julia Set', SHADER)
