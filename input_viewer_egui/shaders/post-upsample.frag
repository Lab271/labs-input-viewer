// GENERATED from post-fx.js UPSAMPLE by extract.mjs -- do not edit.
#version 330 core
uniform sampler2D uSrc;
uniform vec2 uTexel;
uniform float uRadius;
out vec4 outColor;
void main() {
  vec2 uv = gl_FragCoord.xy * uTexel * 0.5;
  vec2 d = uTexel * uRadius;
  vec3 sum = texture(uSrc, uv + vec2(-d.x, 0.0)).rgb * 2.0;
  sum += texture(uSrc, uv + vec2(d.x, 0.0)).rgb * 2.0;
  sum += texture(uSrc, uv + vec2(0.0, -d.y)).rgb * 2.0;
  sum += texture(uSrc, uv + vec2(0.0, d.y)).rgb * 2.0;
  sum += texture(uSrc, uv).rgb * 4.0;
  sum += texture(uSrc, uv + vec2(-d.x, -d.y)).rgb;
  sum += texture(uSrc, uv + vec2(d.x, -d.y)).rgb;
  sum += texture(uSrc, uv + vec2(-d.x, d.y)).rgb;
  sum += texture(uSrc, uv + vec2(d.x, d.y)).rgb;
  outColor = vec4(sum / 16.0, 1.0);
}
