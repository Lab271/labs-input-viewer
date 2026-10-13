// GENERATED from input_viewer_electron/.../julia-family.js by extract.mjs -- do not edit.

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


vec3 pal(float t, vec3 phase) {
  return palettePerceptual(t, phase);
}

// Boundary waypoints, from #121's offline descent (its DIVE_TARGETS, high words).
//
// Only the high word survives here, which is all float32 carries -- a consistency
// with the precision note in the header rather than a loss: these are used as
// centres for a 0.02-0.18 wide window, so 1e-17 of extra mantissa would round away
// regardless.
//
// Every one sits ON the boundary, which is the point: a window centred here has
// connected sets on one side and dust on the other.
vec2 waypoint(int i) {
  if (i == 0) return vec2(-0.650184794685523,  0.353055535780441);
  if (i == 1) return vec2(-0.698213700924294,  0.258982688816176);
  if (i == 2) return vec2(-1.047061989001585, -0.247016285343538);
  if (i == 3) return vec2(-1.336337945564129, -0.052729299598222);
  if (i == 4) return vec2(-0.294091172818190,  0.645193540195812);
  return              vec2(-0.645509523133210, -0.363403402937936);
}

const int WAYPOINTS = 6;

// Seconds spent travelling between one waypoint and the next.
//
// Long, because the interesting thing is slow: the window crossing the boundary as
// the centre moves. A ten-minute no-signal slot covers about seven legs, so several
// connected-to-dust sweeps happen within one activation (#209 asks for at least one).
const float LEG_SECONDS = 84.0;

void mainImage(out vec4 fragColor, in vec2 fragCoord) {
  vec2 res = iResolution.xy;

  // How many bands fit, as whole squares. 5 on the 6000x1200 wall, 2 on 16:9, 1 on
  // anything squarer -- a single Julia set rather than a stretched pair.
  float bands = clamp(floor(res.x / res.y + 0.5), 1.0, 6.0);
  float bandW = res.x / bands;
  float bandIndex = clamp(floor(fragCoord.x / bandW), 0.0, bands - 1.0);

  // A gutter, so the row reads as a deliberate contact sheet rather than as tearing.
  // #92 is the precedent: an abstract seam on a large display reads as a fault.
  float gutterPx = max(2.0, res.y * 0.004);
  vec2 inBand = vec2(fragCoord.x - bandIndex * bandW, fragCoord.y);
  if (inBand.x < gutterPx || inBand.x > bandW - gutterPx) {
    fragColor = vec4(0.0, 0.0, 0.0, 1.0);
    return;
  }

  // z-plane coordinates within this band's own square.
  float side = min(bandW, res.y);
  float view = 1.45 + iSeed.z * 0.35;
  vec2 uv = (inBand - 0.5 * vec2(bandW, res.y)) / side * view;

  // Where the centre of the window is: a slow walk between waypoints, entered at a
  // random leg and phase so activations differ.
  float t = iTime / LEG_SECONDS + iSeed.x * float(WAYPOINTS);
  int leg = int(mod(floor(t), float(WAYPOINTS)));
  int next = int(mod(floor(t) + 1.0, float(WAYPOINTS)));
  float f = fract(t);
  // Smoothstep rather than linear, so the centre eases in and out of each waypoint
  // and lingers where the structure is richest.
  vec2 centre = mix(waypoint(leg), waypoint(next), smoothstep(0.0, 1.0, f));

  // The window breathes, and rotates, so the slice is not always the same cut.
  float breathe = 0.5 + 0.5 * sin(iTime * 0.031 + iSeed.y * 6.2831);
  float width = mix(0.020, 0.180, breathe);
  float ang = iTime * 0.017 + iSeed.w * 6.2831;
  vec2 dir = vec2(cos(ang), sin(ang));

  // This band's c: centred on the walk, offset along the slice.
  float s = bands > 1.0 ? (bandIndex + 0.5) / bands - 0.5 : 0.0;
  vec2 c = centre + dir * (s * width);

  vec2 z = uv;
  const float MAX_I = 320.0;
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
    vec3 phase = vec3(0.0, 0.40, 0.75) + iSeed.w;
    col = pal(sn * 0.025 + iTime * 0.04, phase);
    // Banding on iteration count only. Adding raw iTime here modulates the whole
    // frame, which reads as a fault on a large display (#122).
    col *= 0.5 + 0.5 * sin(sn * 0.25);
    // Fade the far exterior to black.
    //
    // Without this, points that escape immediately still take a full palette colour,
    // so the background is a flat saturated field -- measured at 6000x1200 the frame
    // was largely bright orange with the sets sitting on top of it. That is the worst
    // case for the 12% washout this has to survive, and it leaves bloom nothing to
    // pick out: everything is bright, so nothing glows.
    //
    // Ramping brightness with escape time puts the light where the structure is --
    // dark exterior, bright filigree along the boundary, which is both the classic
    // rendering and the one that holds up in a lit room.
    col *= smoothstep(0.0, 14.0, sn);
  }

  // Hand LINEAR light to the post chain, which sRGB-encodes what it receives.
  // Carried over from julia.js: without this the background washes out to grey,
  // the same double-encode as #140.
  col = pow(max(col, 0.0), vec3(2.2));

  fragColor = vec4(col, 1.0);
}
