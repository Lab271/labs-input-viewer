// GENERATED from post-fx.js COMPOSITE by extract.mjs -- do not edit.
#version 330 core
uniform sampler2D uScene;
uniform sampler2D uBloom;
uniform vec2 uTexel;
uniform float uBloomIntensity;
uniform float uExposure;
uniform int uTonemap;      // 0 = none, 1 = Reinhard-Jodie, 2 = ACES
uniform int uDither;
out vec4 outColor;

// ACES filmic, Narkowicz's fitted curve. Rolls highlights off smoothly instead
// of clipping, which is what preserves structure in a dense bright core.
vec3 tonemapACES(vec3 x) {
  const float a = 2.51, b = 0.03, c = 2.43, d = 0.59, e = 0.14;
  return clamp((x * (a * x + b)) / (x * (c * x + d) + e), 0.0, 1.0);
}

// Cheaper alternative that keeps saturation better than plain Reinhard.
vec3 tonemapReinhardJodie(vec3 x) {
  float l = dot(x, vec3(0.2126, 0.7152, 0.0722));
  vec3 tv = x / (1.0 + x);
  return mix(x / (1.0 + l), tv, tv);
}

// Interleaved gradient noise: a cheap, well-distributed dither. Breaking the
// final 8-bit quantisation matters most on wide smooth gradients -- a bloom
// falloff across 6000px of projector is a textbook banding case.
float ignNoise(vec2 p) {
  return fract(52.9829189 * fract(dot(p, vec2(0.06711056, 0.00583715))));
}

void main() {
  vec2 uv = gl_FragCoord.xy * uTexel;
  vec3 color = texture(uScene, uv).rgb;
  color += texture(uBloom, uv).rgb * uBloomIntensity;
  color *= uExposure;

  if (uTonemap == 2) color = tonemapACES(color);
  else if (uTonemap == 1) color = tonemapReinhardJodie(color);
  else color = clamp(color, 0.0, 1.0);

  // Linear -> sRGB. Without this, linear values are displayed as though they
  // were already encoded, which reads darker and over-saturated.
  color = pow(max(color, 0.0), vec3(1.0 / 2.2));

  if (uDither == 1) {
    // +/- half a code value, so it dithers the quantisation step itself.
    color += (ignNoise(gl_FragCoord.xy) - 0.5) / 255.0;
  }

  outColor = vec4(color, 1.0);
}
