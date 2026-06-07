/**
 * Plasma — flowing colorful domain-warped fractal noise.
 * Smooth, hypnotic, cheap-to-mid GPU cost.
 */
import { createShaderScreensaver } from './gl-base.js'

const SHADER = /* glsl */ `
// Hash / value-noise building blocks
float hash(vec2 p) {
  p = fract(p * vec2(123.34, 456.21));
  p += dot(p, p + 45.32);
  return fract(p.x * p.y);
}

float noise(vec2 p) {
  vec2 i = floor(p);
  vec2 f = fract(p);
  vec2 u = f * f * (3.0 - 2.0 * f);
  return mix(mix(hash(i + vec2(0.0, 0.0)), hash(i + vec2(1.0, 0.0)), u.x),
             mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), u.x), u.y);
}

float fbm(vec2 p) {
  float v = 0.0;
  float a = 0.5;
  for (int i = 0; i < 6; i++) {
    v += a * noise(p);
    p *= 2.0;
    a *= 0.5;
  }
  return v;
}

void mainImage(out vec4 fragColor, in vec2 fragCoord) {
  vec2 uv = fragCoord / iResolution.xy;
  vec2 p = uv * 3.0;
  float t = iTime * 0.15;

  // Domain warping: feed fbm into fbm.
  vec2 q = vec2(fbm(p + vec2(0.0, 0.0) + t),
                fbm(p + vec2(5.2, 1.3) - t));
  vec2 r = vec2(fbm(p + 4.0 * q + vec2(1.7, 9.2) + 0.15 * t),
                fbm(p + 4.0 * q + vec2(8.3, 2.8) - 0.12 * t));
  float f = fbm(p + 4.0 * r);

  // Color palette (cosine gradient).
  vec3 col = 0.5 + 0.5 * cos(6.2831 * (f + vec3(0.0, 0.33, 0.67) + 0.1 * iTime));
  col = mix(col, vec3(0.1, 0.2, 0.6), clamp(length(q), 0.0, 1.0));
  col = mix(col, vec3(0.9, 0.4, 0.1), clamp(r.x * r.x, 0.0, 1.0));
  col *= 0.7 + 0.6 * f;

  fragColor = vec4(col, 1.0);
}
`

export default createShaderScreensaver('Plasma', SHADER)
