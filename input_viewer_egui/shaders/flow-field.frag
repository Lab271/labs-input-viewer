// GENERATED from input_viewer_electron/.../flow-field.js by extract.mjs -- do not edit.

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



vec3 mod289(vec3 x) { return x - floor(x * (1.0 / 289.0)) * 289.0; }
vec2 mod289(vec2 x) { return x - floor(x * (1.0 / 289.0)) * 289.0; }
vec3 permute289(vec3 x) { return mod289(((x * 34.0) + 1.0) * x); }

float snoise(vec2 v) {
  const vec4 C = vec4(0.211324865405187, 0.366025403784439,
                     -0.577350269189626, 0.024390243902439);
  vec2 i  = floor(v + dot(v, C.yy));
  vec2 x0 = v - i + dot(i, C.xx);
  vec2 i1 = (x0.x > x0.y) ? vec2(1.0, 0.0) : vec2(0.0, 1.0);
  vec4 x12 = x0.xyxy + C.xxzz;
  x12.xy -= i1;
  i = mod289(i);
  vec3 p = permute289(permute289(i.y + vec3(0.0, i1.y, 1.0)) + i.x + vec3(0.0, i1.x, 1.0));
  vec3 m = max(0.5 - vec3(dot(x0, x0), dot(x12.xy, x12.xy), dot(x12.zw, x12.zw)), 0.0);
  m = m * m; m = m * m;
  vec3 x = 2.0 * fract(p * C.www) - 1.0;
  vec3 h = abs(x) - 0.5;
  vec3 ox = floor(x + 0.5);
  vec3 a0 = x - ox;
  m *= 1.79284291400159 - 0.85373472095314 * (a0 * a0 + h * h);
  vec3 g;
  g.x  = a0.x  * x0.x  + h.x  * x0.y;
  g.yz = a0.yz * x12.xz + h.yz * x12.yw;
  return 130.0 * dot(m, g);
}


vec2 curl2d(vec2 p, float t) {
  const float e = 0.008;
  // Decorrelated planes rather than a shared time offset, so the field evolves.
  float n1 = snoise(p + vec2(0.0, e) + vec2(0.0, t * 0.35));
  float n2 = snoise(p - vec2(0.0, e) + vec2(0.0, t * 0.35));
  float n3 = snoise(p + vec2(e, 0.0) + vec2(t * 0.35, 0.0));
  float n4 = snoise(p - vec2(e, 0.0) + vec2(t * 0.35, 0.0));
  return vec2(n1 - n2, -(n3 - n4)) / (2.0 * e);
}


// Divergence-free flow from the shared library (issue #115). Two fixes over the
// hand-rolled version this replaces:
//
//   - it used value noise, which leaves axis-aligned lattice artifacts; on a
//     6000px-wide wall that grid is visible at normal viewing distance
//   - it added time as a scalar offset to *both* curl components, which slides
//     the whole field diagonally instead of evolving it. That is why the flow
//     visibly translated across the screen rather than churning in place.
vec2 flow(vec2 p, float t) {
  return curl2d(p, t);
}

void mainImage(out vec4 fragColor, in vec2 fragCoord) {
  vec2 res = iResolution.xy;
  vec2 uv = (fragCoord - 0.5 * res) / res.y;
  // Time offset alone is not enough: the noise field is sampled at a fixed
  // position, so a different phase gives the same field slid sideways. The
  // spatial offset below is what puts each activation in a different region.
  float t = iTime * 0.08 + iSeed.w * 12.0;

  vec2 fieldOffset = vec2(iSeed.x, iSeed.y) * 48.0;
  // Field scale varies, so streaks are coarser or finer per activation.
  float fscale = 1.6 + iSeed.z * 0.9;
  vec2 p = uv * 2.0 + fieldOffset;
  float bright = 0.0;
  // Integrate the streamline backwards along a fixed ARC of the flow, in STEPS
  // samples.
  //
  // WHY STEPS IS 16 AND NOT 40
  //
  // Each sample costs four snoise calls, because curl2d() takes the curl by
  // central differences. At 40 steps that is 160 noise evaluations per pixel --
  // about 1.2 billion per frame at 6000x1200 -- and this saver measured 15.6 fps
  // there, the slowest of all thirty (#225).
  //
  // ARC is held at the original 40 x 0.012, so the streamline traced is exactly
  // the same length through exactly the same field; it is integrated more
  // coarsely, not shortened. The brightness weight is ARC/STEPS for the same
  // reason, so accumulated brightness is unchanged and the streak curve and bloom
  // threshold below stay valid without retuning.
  //
  // A cheaper flow() was tried first and rejected. snoised() in the shared
  // library returns a value and its exact analytic gradient in one call, and the
  // curl of a scalar potential is that gradient rotated a quarter turn -- one
  // evaluation instead of four. It reached 40 fps, but it is the curl of a single
  // 3D potential, whereas curl2d samples its two components in decorrelated
  // planes slid along different axes. That is not the same field: the streamlines
  // and the speed distribution both change, and the frame lost its blacks (p05
  // 0.078 against 0.008, 8.9% of the frame below 0.15 against 36.6%). Recovering
  // the floor by retuning the speed ramp got p05 and the median back but not the
  // deep shadow, and the image still read as marbling rather than filaments on
  // black. #119 asks for this saver to get cheaper, NOT to be restyled, so the
  // field stays exactly as it was and only the sample count moves.
  const int STEPS = 16;
  const float ARC = 0.48;
  const float STEP = ARC / float(STEPS);
  // Carried out of the loop for the hue below, which used to spend a 17th flow()
  // call -- four more noise evaluations -- on a value the last iteration had
  // already computed.
  vec2 v = vec2(0.0);
  for (int i = 0; i < STEPS; i++) {
    v = flow(p * fscale, t);
    p -= v * STEP;
    // Brightness from how aligned the local flow is + a moving phase.
    float speed = length(v);
    bright += STEP * smoothstep(0.0, 1.5, speed);
  }

  // Streak intensity. The exponent controls how much of the frame is lit and
  // the multiplier controls how hard the lit parts burn.
  //
  // This was pow(bright, 1.3) * 2.2, which was authored before the post chain
  // existed (#112) and went straight to an 8-bit framebuffer with no encode.
  // Feeding those same values through the chain's sRGB encode lifts them a
  // second time -- a linear 0.02 background displays at 0.126, and the dense
  // mid-tones at 0.16 land near 0.52, so the dark gaps between filaments fill
  // in and the whole field reads as washed-out grey rather than glowing
  // filaments on black. Same class of bug as the raymarch double-encode in
  // #140, milder because it is gamma only and not gamma plus tonemap.
  //
  // A steeper exponent darkens everything below the filament cores, which is
  // what restores the black background; the higher multiplier keeps the cores
  // as bright as they were. Measured across a sweep, displayed values after
  // the chain's ACES + sRGB:
  //
  //   pow 1.3 x2.2 (original): p05 0.126, 0% of frame below 0.15
  //   pow 2.4 x6.5           : p05 0.034, 25%
  //   pow 4.0 x26.0 (this)   : p05 0.007, 37%
  //
  // The background is genuinely black again and filament peaks are unchanged
  // (p95 rises slightly). The median is still 0.34, so this is a substantial
  // improvement rather than a complete cure -- a saver authored against the
  // chain from the start would spend fewer pixels in the mid-tones.
  float streak = pow(bright, 4.0) * 26.0;
  // Hue cycles along the flow direction and time, reusing the last step's vector.
  vec2 vdir = v;
  vec3 phase = vec3(0.0, 0.33, 0.67) + iSeed.x;
  float hue = atan(vdir.y, vdir.x) / 6.2831 + 0.5 + 0.05 * iTime;
  vec3 col = palettePerceptual(hue, phase);
  col *= streak;

  // Subtle dark vignette.
  col *= 1.0 - 0.4 * dot(uv, uv);
  fragColor = vec4(col, 1.0);
}
