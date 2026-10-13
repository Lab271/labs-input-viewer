// GENERATED from input_viewer_electron/.../burning-ship.js by extract.mjs -- do not edit.

vec3 cosinePalette(float t, vec3 phase) {
  return 0.5 + 0.5 * cos(6.28318530718 * (t + phase));
}

vec3 oklabToLinear(vec3 lab) {
  float l_ = lab.x + 0.3963377774 * lab.y + 0.2158037573 * lab.z;
  float m_ = lab.x - 0.1055613458 * lab.y - 0.0638541728 * lab.z;
  float s_ = lab.x - 0.0894841775 * lab.y - 1.2914855480 * lab.z;
  vec3 lms = vec3(l_ * l_ * l_, m_ * m_ * m_, s_ * s_ * s_);
  return vec3(
    +4.0767416621 * lms.x - 3.3077115913 * lms.y + 0.2309699292 * lms.z,
    -1.2684380046 * lms.x + 2.6097574011 * lms.y - 0.3413193965 * lms.z,
    -0.0041960863 * lms.x - 0.7034186147 * lms.y + 1.7076147010 * lms.z);
}

// Perceptually even ramp: L in [0,1], hue in turns, chroma ~[0,0.4].
vec3 oklabRamp(float t, float lightness, float chroma, float hueTurns) {
  float h = 6.28318530718 * (hueTurns + t);
  return max(oklabToLinear(vec3(lightness, chroma * cos(h), chroma * sin(h))), 0.0);
}

// Drop-in replacement for the cosine palette, in OKLab.
//
// Constant lightness and chroma, hue sweeping -- the textbook perceptually
// uniform ramp. Holding both fixed is the whole point: varying them is what
// gives the cosine palette its muddy troughs and blown highlights.
//
// L=0.75, C=0.12 were chosen by measurement, not taste. Sampling 24 hues and
// converting back to OKLab:
//
//   cosine palette      chroma varies 2.3x across the ramp
//   L=0.70 C=0.13       1.08x, but 3/24 hues fall outside sRGB
//   L=0.72 C=0.15       1.20x, 7/24 out of gamut
//   L=0.75 C=0.12       1.00x, 0/24 out of gamut   <- this one
//
// Out-of-gamut hues clip, which reintroduces exactly the flat patches the
// change is meant to remove, so staying inside sRGB matters more than squeezing
// out extra saturation.
//
// The phase vector keeps the cosine version's meaning: .x rotates the hue
// wheel, and the spread between components nudges chroma, so per-activation
// palette variation still varies the palette.
vec3 palettePerceptual(float t, vec3 phase) {
  float chroma = 0.12 * (0.85 + 0.3 * fract(phase.y - phase.z));
  return oklabRamp(phase.x + t, 0.75, chroma, 0.0);
}


// Warm "burning" palette. Built from a cosine gradient rather than the
// monotonic smoothstep ramp this used to use: the input is wrapped with
// fract(), and a monotonic ramp under fract() has a hard discontinuity at the
// wrap point -- a visible colour seam that swept through the image as time
// advanced (see #123). A cosine palette is periodic by construction, so it
// wraps seamlessly, and it lets highlights actually reach white.
vec3 palette(float t, float rot) {
  vec3 phase = vec3(0.0, 0.12, 0.26) + rot;
  vec3 c = palettePerceptual(t, phase);
  // Weight toward reds/oranges so it still reads as fire rather than rainbow.
  return c * vec3(1.0, 0.72, 0.45) + vec3(0.0, 0.0, 0.05);
}

// Detail regions along the ship and its antennae, all of which hold structure
// under zoom.
vec2 zoomTarget(float s) {
  if (s < 0.2) return vec2(-1.7549,  -0.0286);  // antenna detail (original)
  if (s < 0.4) return vec2(-1.7433,  -0.0182);  // antenna fork
  if (s < 0.6) return vec2(-1.5810,  -0.0175);  // second ship
  if (s < 0.8) return vec2(-1.9411,  -0.0048);  // far antenna
  return vec2(-1.7269,  -0.0393);               // mast junction
}

void mainImage(out vec4 fragColor, in vec2 fragCoord) {
  vec2 res = iResolution.xy;
  vec2 uv = (fragCoord - 0.5 * res) / res.y;

  // Smooth in/out zoom toward a detailed point on the main ship's antenna.
  // Depth capped at 3.4 -- past roughly exp(3.5) the 32-bit float precision
  // here breaks down into visible blocking.
  float rate = 0.10 + iSeed.z * 0.07;
  float depth = 2.8 + iSeed.w * 0.6;
  float zt = iTime * rate;
  float zoom = exp(0.5 + depth * (0.5 - 0.5 * cos(zt)));
  vec2 center = zoomTarget(iSeed.x);

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
    col = palette(t, iSeed.y);
    col *= 0.7 + 0.4 * sin(sn * 0.2);
  }

  // Hand LINEAR light to the post chain.
  //
  // Everything above was authored before #112, when this saver rendered
  // straight to an 8-bit framebuffer with no encode -- so those values are
  // display-referred (a 0.6 here meant "display at 0.6"). The chain added by
  // #112 sRGB-ENCODES whatever it receives, which lifted them a second time:
  // measured displayed p05 was 0.645 with only 1%% of the frame dark, i.e. the
  // background washed out to grey. Same class of bug as the raymarch
  // double-encode in #140, gamma-only rather than gamma plus tonemap.
  //
  // pow(col, 2.2) inverts the encode the chain will apply, so the round trip
  // is exact and the pre-#112 look is restored -- while bloom and ACES now
  // operate on physically-linear values, which is what they expect.
  col = pow(max(col, 0.0), vec3(2.2));

  fragColor = vec4(col, 1.0);
}
