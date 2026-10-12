// GENERATED from input_viewer_electron/.../plasma.js by extract.mjs -- do not edit.

// Fragment shaders: pixel coordinate -> world space. Needs iResolution.
vec2 worldFromFrag(vec2 fragCoord, vec2 resolution) {
  return (fragCoord - 0.5 * resolution) / resolution.y;
}

// Vertex shaders: world space -> clip space. Needs the aspect ratio (w/h).
// Dividing x by aspect is what undoes the viewport stretch.
vec4 clipFromWorld(vec2 world, float aspect) {
  return vec4(world.x * 2.0 / aspect, world.y * 2.0, 0.0, 1.0);
}

// Half-extent of the visible world region, for placing and wrapping things.
// y is always 0.5; x grows with the aspect ratio.
vec2 worldExtent(float aspect) {
  return vec2(0.5 * aspect, 0.5);
}

// Shortest separation between two points on a torus of the given half-extent.
//
// A plain (b - a) is wrong once a simulation wraps: two points either side of
// the seam are physically adjacent but numerically maximally distant. In boids
// that made neighbours across the edge read as far away, so the flock visibly
// tore at screen edges rather than flowing through them.
vec2 torusDelta(vec2 a, vec2 b, vec2 halfExtent) {
  vec2 d = b - a;
  vec2 span = 2.0 * halfExtent;
  return d - span * floor(d / span + 0.5);
}

// Wrap a position back into [-halfExtent, halfExtent] on both axes.
vec2 torusWrap(vec2 p, vec2 halfExtent) {
  vec2 span = 2.0 * halfExtent;
  return p - span * floor((p + halfExtent) / span);
}


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


vec4 mod289(vec4 x) { return x - floor(x * (1.0 / 289.0)) * 289.0; }
vec4 permute289(vec4 x) { return mod289(((x * 34.0) + 1.0) * x); }
vec4 taylorInvSqrt(vec4 r) { return 1.79284291400159 - 0.85373472095314 * r; }

vec4 snoised(vec3 v) {
  const vec2 C = vec2(1.0 / 6.0, 1.0 / 3.0);
  const vec4 D = vec4(0.0, 0.5, 1.0, 2.0);

  // Skew into the simplex lattice and find the first corner.
  vec3 i  = floor(v + dot(v, C.yyy));
  vec3 x0 = v - i + dot(i, C.xxx);

  // Rank the components to pick which of the six tetrahedra we are in.
  vec3 gt = step(x0.yzx, x0.xyz);
  vec3 l  = 1.0 - gt;
  vec3 i1 = min(gt.xyz, l.zxy);
  vec3 i2 = max(gt.xyz, l.zxy);

  vec3 x1 = x0 - i1 + C.xxx;
  vec3 x2 = x0 - i2 + C.yyy;
  vec3 x3 = x0 - D.yyy;

  i = mod289(i);
  vec4 p = permute289(permute289(permute289(
             i.z + vec4(0.0, i1.z, i2.z, 1.0))
           + i.y + vec4(0.0, i1.y, i2.y, 1.0))
           + i.x + vec4(0.0, i1.x, i2.x, 1.0));

  // Gradients: 7x7x6 points over a cube, mapped onto an octahedron.
  float n_ = 0.142857142857; // 1/7
  vec3  ns = n_ * D.wyz - D.xzx;

  vec4 j = p - 49.0 * floor(p * ns.z * ns.z);

  vec4 xf = floor(j * ns.z);
  vec4 yf = floor(j - 7.0 * xf);

  vec4 gx = xf * ns.x + ns.yyyy;
  vec4 gy = yf * ns.x + ns.yyyy;
  vec4 gh = 1.0 - abs(gx) - abs(gy);

  vec4 b0 = vec4(gx.xy, gy.xy);
  vec4 b1 = vec4(gx.zw, gy.zw);

  vec4 s0 = floor(b0) * 2.0 + 1.0;
  vec4 s1 = floor(b1) * 2.0 + 1.0;
  vec4 sh = -step(gh, vec4(0.0));

  vec4 a0 = b0.xzyw + s0.xzyw * sh.xxyy;
  vec4 a1 = b1.xzyw + s1.xzyw * sh.zzww;

  vec3 g0 = vec3(a0.xy, gh.x);
  vec3 g1 = vec3(a0.zw, gh.y);
  vec3 g2 = vec3(a1.xy, gh.z);
  vec3 g3 = vec3(a1.zw, gh.w);

  vec4 norm = taylorInvSqrt(vec4(dot(g0, g0), dot(g1, g1), dot(g2, g2), dot(g3, g3)));
  g0 *= norm.x; g1 *= norm.y; g2 *= norm.z; g3 *= norm.w;

  // Radial falloff per corner. w is kept unsquared here (Ashima squares in
  // place) because the derivative needs both w^3 and w^4.
  vec4 w  = max(0.6 - vec4(dot(x0, x0), dot(x1, x1), dot(x2, x2), dot(x3, x3)), 0.0);
  vec4 w2 = w * w;
  vec4 w4 = w2 * w2;
  vec4 gd = vec4(dot(g0, x0), dot(g1, x1), dot(g2, x2), dot(g3, x3));

  vec4 c = w2 * w * gd;
  vec3 grad = -8.0 * (c.x * x0 + c.y * x1 + c.z * x2 + c.w * x3)
            + w4.x * g0 + w4.y * g1 + w4.z * g2 + w4.w * g3;

  return 42.0 * vec4(dot(w4, gd), grad);
}

// Value only, for the places that do not need the gradient.
float snoise(vec3 v) { return snoised(v).x; }


// Quilez's decorrelating rotation and its inverse (orthonormal, so transpose).
const mat3 FBMD3_ROT = mat3( 0.00,  0.80,  0.60,
                            -0.80,  0.36, -0.48,
                            -0.60, -0.48,  0.64);
const mat3 FBMD3_ROT_INV = mat3( 0.00, -0.80, -0.60,
                                 0.80,  0.36, -0.48,
                                 0.60, -0.48,  0.64);

vec4 fbmd3(vec3 p, int octaves, float gain, float erosion) {
  // 1.97 rather than 2.0: an exact octave doubling repeats the lattice
  // alignment at every level, and the near-miss is free.
  const float LACUNARITY = 1.97;
  float amp = 0.5, sum = 0.0, norm = 0.0;
  vec3 grad = vec3(0.0);
  mat3 m = mat3(1.0);
  for (int i = 0; i < octaves; i++) {
    vec4 n = snoised(p);
    sum  += amp * n.x / (1.0 + erosion * dot(grad, grad));
    grad += amp * m * n.yzw;
    norm += amp;
    amp  *= gain;
    p = LACUNARITY * FBMD3_ROT * p;
    m = LACUNARITY * FBMD3_ROT_INV * m;
  }
  float inv = 1.0 / max(norm, 1e-5);
  return vec4(sum * inv, grad * inv);
}


const float TAU = 6.28318530718;

// The analytic gradient of an fbmd3 octave is not a small number: simplex noise
// is normalised so its *value* lands in [-1,1], which leaves the derivative
// running around 5-10 per unit of input. Measured here (rendering
// length(n.yzw) directly) it sits near 8 for the octave counts below. Dividing
// by that lets every warp amplitude and bump strength further down be written
// as "how far, in cells" rather than as an opaque small constant, which is the
// difference between tuning and guessing.
const float GRAD_NORM = 1.0 / 8.0;

// Rotations applied to each warp level's flow vector.
//
// Rotating a gradient by 90 degrees in the xy plane turns it into a
// divergence-free flow: instead of pushing the domain downhill (which piles
// material up and flattens the result) it shears the domain sideways, which is
// what produces the marbled, filament-rich look. The two levels use different
// angles so the second warp does not simply reinforce the first.
mat3 flowRotation(float turns) {
  float a = TAU * turns;
  float c = cos(a), s = sin(a);
  // Rotation about z, with a slight tilt into z so the warp also displaces the
  // time axis -- neighbouring regions then evolve slightly out of step, which
  // stops the whole field pulsing as one.
  return mat3( c,  s, 0.0,
              -s,  c, 0.0,
              0.0, 0.12, 0.99);
}

// Base hues, in turns around the OKLab hue wheel. Curated rather than random:
// the interesting failure of a free hue is that it lands somewhere the sRGB
// gamut is narrow at this lightness, and the ramp flattens exactly where it
// should be most saturated. These four were checked against the L/C used below.
float baseHue(float s) {
  if (s < 0.25) return 0.62;  // deep blue -> cyan
  if (s < 0.50) return 0.08;  // amber -> magma
  if (s < 0.75) return 0.42;  // jade -> teal
  return 0.86;                // violet -> rose
}

void mainImage(out vec4 fragColor, in vec2 fragCoord) {
  // World space: divide by the SHORT axis, so the pattern is not stretched 5:1
  // on the wall (issue #114) and a wider canvas shows more plasma rather than a
  // magnified version of the same plasma. y spans [-0.5, 0.5].
  vec2 uv = worldFromFrag(fragCoord, iResolution.xy);

  // Base feature scale, in cells per screen height. Deliberately close to one:
  // in a domain-warped fBm the apparent complexity comes from the warp, not
  // from the base frequency, so the base stays large and soft. At 6000x1200
  // this is ~900px per base cell, and the wall shows about seven of them
  // across. The old version was effectively at 30+ cells per height with no
  // large-scale variation at all, which is precisely why it turned to mud.
  float scale = 1.05 + iSeed.z * 0.30;

  // Field offset. Also decorrelates the three fbm levels from each other.
  vec3 offset = vec3(iSeed.x, iSeed.y, iSeed.w) * 71.0;

  // Time on the third axis. 0.05 cells/second against a cell of 1.0 means the
  // surface renews itself over roughly twenty seconds -- slow enough to be calm
  // on a ten-minute rotation slot, fast enough that it is visibly alive.
  float t = iTime;
  vec3 p = vec3(uv * scale, t * 0.05) + offset;

  // ---- Multi-level domain warp -------------------------------------------
  // The warp vector at each level is the *derivative* of a low-octave fbm, so a
  // coherent flow direction costs one fbm rather than the two or three separate
  // noise fields the usual (Quilez) formulation evaluates. Few octaves and a
  // low gain on purpose: a warp field with fine detail in it displaces
  // neighbouring pixels in unrelated directions, which shreds the level below
  // into grain instead of stretching it into filaments.
  //
  // Displacement is comparable to the cell size at level 1 and a third of it at
  // level 2. That is the regime where fBm stops looking like clouds and starts
  // looking marbled -- much less and it is just fBm, much more and the domain
  // folds onto itself into pinched knots.
  vec4 n1 = fbmd3(p, 3, 0.42, 0.0);
  vec3 p1 = p + 0.90 * GRAD_NORM * (flowRotation(0.25) * n1.yzw);

  // Level 2 runs on the already-warped domain at a slightly higher frequency
  // and a different rotation, so it shears rather than reinforcing level 1.
  vec4 n2 = fbmd3(p1 * 1.6 + vec3(19.3, 7.1, t * 0.018), 3, 0.42, 0.0);
  vec3 p2 = p1 + 0.32 * GRAD_NORM * (flowRotation(-0.17) * n2.yzw);

  // Level 3: the surface itself. Gain 0.40 rather than 0.5 because the gradient
  // is about to be used for lighting -- see the note on fbmd3. Erosion damps
  // octaves where the accumulated slope is already steep, concentrating fine
  // detail into the flanks of the large forms instead of spreading it evenly;
  // 0.35 is moderate, higher flattens the ridges into plateaus.
  vec4 n3 = fbmd3(p2 + vec3(4.7, 11.9, 0.0), 4, 0.40, 0.35);
  float h = n3.x;
  vec2 grad = n3.yz;

  // ---- Lighting -----------------------------------------------------------
  // Shade the fBm as a heightfield. This is the cheapest depth available: the
  // gradient is already computed, and the difference between a colour-mapped
  // plasma and a lit one is the difference between a pattern and a surface.
  //
  // 0.55 cells of rise per cell of run at a typical slope, which is a fairly
  // dramatic relief -- around 30 degrees. Tuned by eye at 6000x1200: higher
  // looks embossed and metallic, lower goes flat.
  vec3 nrm = normalize(vec3(-grad * (0.55 * GRAD_NORM), 1.0));

  // Key light from the upper left, the default for reading relief as raised
  // rather than sunken.
  vec3 lightDir = normalize(vec3(-0.5, 0.62, 0.6));
  float diff = clamp(dot(nrm, lightDir), 0.0, 1.0);

  // Half-vector specular against a head-on viewer. Narrow and strong: it is
  // what puts a highlight on the crest of each ridge and reads as wet or
  // molten rather than matte.
  vec3 halfV = normalize(lightDir + vec3(0.0, 0.0, 1.0));
  float spec = pow(clamp(dot(nrm, halfV), 0.0, 1.0), 34.0);

  // Cheap ambient occlusion: valleys of the heightfield are self-shadowed.
  float ao = clamp(0.45 + 0.75 * (h + 0.35), 0.25, 1.0);

  // ---- Colour -------------------------------------------------------------
  // Hue varies only at very low frequency (0.4 cells per screen height against
  // the surface's ~4) and over a narrow span. Both bounds are the point: a hue
  // that varies as fast as the detail averages to grey, and a hue that sweeps
  // the whole wheel gives a rainbow rather than a palette. +/-0.07 turns is
  // about 25 degrees each way, an analogous scheme.
  //
  // The drift term is 0.0018 turns/second: one lap of the hue wheel per nine
  // minutes, against a ten-minute rotation slot. Fast enough that a long look
  // is not a still image, slow enough that no single glance sees it move.
  float hueField = snoise(vec3(uv * 0.4, t * 0.017) + offset.zxy);
  float hue = baseHue(iSeed.y) + 0.07 * hueField + 0.0018 * t;

  // Lightness carries the structure. Range 0.05..~1.0 in OKLab L, so there are
  // genuine blacks and genuine highlights rather than the old flat 0.75.
  float lit = 0.05 + 0.74 * diff * ao;
  float crest = smoothstep(0.10, 0.68, h);
  float L = clamp(lit + 0.26 * crest, 0.0, 1.0);

  // Chroma falls off at both ends: near black there is no chroma to see, and
  // hot crests desaturating toward white is what makes them read as *bright*
  // rather than merely as a lighter version of the same colour.
  float chroma = 0.125 * smoothstep(0.0, 0.20, L) * (1.0 - 0.45 * smoothstep(0.72, 1.0, L));
  float a = TAU * hue;
  vec3 col = max(oklabToLinear(vec3(L, chroma * cos(a), chroma * sin(a))), 0.0);

  // ---- Composition --------------------------------------------------------
  // A very low-frequency energy field, drifting slower than everything else.
  // Without it a 5:1 canvas is uniformly busy from end to end and the eye has
  // nowhere to rest; with it, whole stretches of the wall fall to a deep quiet
  // base and the bright regions read as events. This is the single change that
  // most affects how the saver looks at wall aspect.
  float energy = snoise(vec3(uv * 0.20, t * 0.012) + offset.yzx * 0.7);
  // Biased dark: the midpoint sits at +0.2 rather than 0, so rather more than
  // half of the field falls into the quiet base. An unbiased mask leaves the
  // wall evenly busy, which is the failure this exists to fix.
  float mask = smoothstep(-0.25, 0.75, energy);
  // The floor is 0.20 rather than something near zero. A quiet region should be
  // *dim plasma*, not an absence of image: a black patch on a lit-room videowall
  // reads as a dead panel, it disappears entirely under the ambient washout, and
  // it leaves shadercheck's structure probe with nothing to measure if its
  // 512px sample happens to land there.
  col *= 0.20 + 1.25 * mask;

  // Specular and crest glow are added *after* the mask so the brightest ridges
  // in an active region genuinely exceed 1.0 and the bloom in the post chain
  // has something real to find. Feeding pre-tonemapped colour into the chain is
  // issue #140; these are HDR values.
  // Same reasoning as the mask floor: the quiet regions keep a fraction of the
  // highlights, so they still glint rather than going inert.
  float activity = 0.18 + 0.82 * mask;
  float heat = activity * pow(crest, 3.0);
  col += col * 2.2 * heat;
  col += vec3(1.0, 0.92, 0.82) * spec * (0.22 + 1.4 * heat) * activity;

  // Deep ambient floor, tinted the complement of the base hue so the quiet
  // regions are a colour rather than an absence of one.
  float ca = TAU * (baseHue(iSeed.y) + 0.5);
  col += max(oklabToLinear(vec3(0.16, 0.05 * cos(ca), 0.05 * sin(ca))), 0.0) * 0.9;

  fragColor = vec4(col, 1.0);
}
