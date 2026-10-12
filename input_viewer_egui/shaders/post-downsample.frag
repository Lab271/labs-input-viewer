// GENERATED from post-fx.js DOWNSAMPLE by extract.mjs -- do not edit.
#version 330 core
uniform sampler2D uSrc;
uniform vec2 uTexel;
out vec4 outColor;
void main() {
  vec2 uv = gl_FragCoord.xy * uTexel * 2.0;
  vec3 sum = texture(uSrc, uv).rgb * 4.0;
  sum += texture(uSrc, uv - uTexel).rgb;
  sum += texture(uSrc, uv + uTexel).rgb;
  sum += texture(uSrc, uv + vec2(uTexel.x, -uTexel.y)).rgb;
  sum += texture(uSrc, uv - vec2(uTexel.x, -uTexel.y)).rgb;
  outColor = vec4(sum / 8.0, 1.0);
}
