// GENERATED from post-fx.js BRIGHT_PASS by extract.mjs -- do not edit.
#version 330 core
uniform sampler2D uSrc;
uniform vec2 uTexel;
uniform float uThreshold;
uniform float uKnee;
out vec4 outColor;

// Soft-knee bright pass. A hard threshold produces a visible ring where pixels
// cross it; the knee blends across a band instead.
void main() {
  vec3 c = texture(uSrc, gl_FragCoord.xy * uTexel).rgb;
  float lum = max(max(c.r, c.g), c.b);
  float soft = clamp(lum - uThreshold + uKnee, 0.0, 2.0 * uKnee);
  soft = soft * soft / (4.0 * uKnee + 1e-5);
  float contribution = max(soft, lum - uThreshold) / max(lum, 1e-5);
  outColor = vec4(c * contribution, 1.0);
}
