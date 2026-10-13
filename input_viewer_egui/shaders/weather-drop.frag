// GENERATED from input_viewer_electron/.../weather.js DROP_FRAG by extract.mjs -- do not edit.
#version 330 core
in float vAlpha;
in vec2 vQuad;
in float vDepth;
uniform float uSnow;
uniform float uLum;
uniform float uNight;   // 1 at night: precipitation is lit differently
out vec4 outColor;

void main() {
  float shape;
  if (uSnow > 0.5) {
    // Soft round flake.
    shape = smoothstep(0.5, 0.06, length(vQuad));
  } else {
    // Capsule: soft across the streak, tapered along it.
    float across = smoothstep(0.5, 0.12, abs(vQuad.x));
    float along = smoothstep(0.5, 0.34, abs(vQuad.y));
    shape = across * along;
  }
  if (shape <= 0.001) discard;

  // Rain is a highlight on the sky rather than a colour of its own, so it is
  // near-white by day and a cooler grey at night.
  // Rain against bright overcast has to be able to read DARKER than the sky, not
  // just brighter -- a white streak on a white sky is invisible. With
  // premultiplied blending a tint below the background darkens it, so this is a
  // blue-grey by day and a pale highlight at night against the dark.
  vec3 tint = mix(vec3(0.42, 0.50, 0.66), vec3(0.97, 0.98, 1.00), uSnow);
  tint = mix(tint, mix(vec3(0.62, 0.70, 0.95), vec3(0.90, 0.93, 1.00), uSnow), uNight);
  // Rain needs contrast against a bright overcast sky, which is exactly when it
  // is most likely to be falling.
  float energy = mix(0.95, 1.15, uSnow);
  outColor = vec4(tint * shape * vAlpha * energy * uLum, shape * vAlpha);
}
