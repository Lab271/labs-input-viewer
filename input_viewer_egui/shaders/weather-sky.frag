// GENERATED from input_viewer_electron/.../weather.js SKY_FRAG by extract.mjs -- do not edit.
#version 330 core
uniform vec3 iResolution;
uniform float iTime;
uniform float uCloud;      // 0..1 cover
uniform float uWind;       // km/h
uniform float uDay;        // 0 = night, 1 = day, eased across dawn/dusk
uniform float uFog;        // 0..1
uniform float uStorm;      // 0..1, drives the lightning flash
uniform float uSnowGround; // 0..1, whitens the ground band
uniform float uFlash;      // 0..1 current flash energy
uniform float uRunwayDir; // -1 or +1: which way the airport is operating, from the wind
uniform float uPrecip;    // 0..1 normalised precipitation, thins traffic and wets the apron
uniform float uLum;
out vec4 outColor;


uint hashU(uint x) {
  x += (x << 10u); x ^= (x >>  6u);
  x += (x <<  3u); x ^= (x >> 11u);
  x += (x << 15u);
  return x;
}
uint hashU(uvec2 v) { return hashU(v.x ^ hashU(v.y)); }
uint hashU(uvec3 v) { return hashU(v.x ^ hashU(v.y) ^ hashU(v.z)); }

// [0,1) float from the top 23 bits, so the mantissa is fully populated.
float hashF(uint h) { return float(h & 0x007FFFFFu) / float(0x00800000u); }

float rand(float p)  { return hashF(hashU(floatBitsToUint(p))); }
float rand(vec2 p)   { return hashF(hashU(floatBitsToUint(p))); }
float rand(vec3 p)   { return hashF(hashU(floatBitsToUint(p))); }
vec2  rand2(vec2 p)  { uint h = hashU(floatBitsToUint(p));
                       return vec2(hashF(h), hashF(hashU(h))); }


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


float fbm(vec2 p, int octaves) {
  float sum = 0.0, amp = 0.5, norm = 0.0;
  for (int i = 0; i < octaves; i++) {
    sum += amp * snoise(p);
    norm += amp;
    p *= 2.02;   // Slightly off 2.0 so octave lattices do not align.
    amp *= 0.5;
  }
  return sum / max(norm, 1e-5);
}


// Sky gradient. Two palettes crossfaded by uDay rather than one hue-rotated:
// a night sky is not a dark day sky, it is a different colour relationship.
vec3 skyGradient(float h) {
  vec3 dayLow  = vec3(0.62, 0.76, 0.92);
  vec3 dayHigh = vec3(0.16, 0.40, 0.78);
  vec3 nightLow  = vec3(0.05, 0.07, 0.14);
  vec3 nightHigh = vec3(0.01, 0.02, 0.06);
  vec3 day   = mix(dayLow, dayHigh, pow(clamp(h, 0.0, 1.0), 0.7));
  vec3 night = mix(nightLow, nightHigh, pow(clamp(h, 0.0, 1.0), 0.9));
  return mix(night, day, uDay);
}

// ----------------------------------------------------------------- airport
//
// Schiphol as a SILHOUETTE (#203). The office is near it, and a generic sky over
// an anonymous dark band becomes a display of *this* place once the ground has an
// airport on it -- which is the same instinct that made the live data worth
// having.
//
// Honesty constraints from the issue, and they shape the code:
//
//   - Silhouette, not simulation. There is no flight data here. The aircraft are
//     decorative and their timing is invented, which is why nothing in the readout
//     refers to traffic. The WEATHER is the live part and that distinction has to
//     stay obvious.
//   - Recognisable, not branded. No logos, no liveries, no attempt at a specific
//     building. It reads as Schiphol because of the horizontal massing and the
//     light pattern, not because anything is copied.
//   - The sky stays the subject. Everything here lives in the lower third:
//     buildings just above the horizon, lighting below it.

// Axis-aligned slab, antialiased. x0..x1 across the frame, rising h above base.
float slab(vec2 p, float x0, float x1, float base, float h, float aa) {
  float inX = smoothstep(x0 - aa, x0 + aa, p.x) * (1.0 - smoothstep(x1 - aa, x1 + aa, p.x));
  float inY = smoothstep(base - aa, base + aa, p.y) * (1.0 - smoothstep(base + h - aa, base + h + aa, p.y));
  return inX * inY;
}

// A point light with a halo. The halo is what fog acts on, so it is a separate
// term rather than a fattened core.
vec3 lamp(vec2 p, vec2 at, float size, vec3 tint, float gain, float haze) {
  float d = length((p - at) / vec2(1.0, 0.42));   // squashed: wide frame, low band
  float core = exp(-d / max(size, 1e-4) * 2.4);
  float halo = exp(-d / max(size * (3.0 + haze * 9.0), 1e-4));
  return tint * (core * 1.6 + halo * (0.25 + haze * 0.9)) * gain;
}

// uv.x is CENTRED ON ZERO in short-axis units -- (frag.x - 0.5*W)/H -- so it spans
// +/-2.5 on the 5:1 wall and only +/-0.89 at 16:9. Every x below is therefore a
// fraction of halfX rather than an absolute, which is what makes the airport span
// the frame at any aspect instead of bunching near the centre. Getting this wrong
// first time put the whole airport in the right-hand third.
vec3 airport(vec2 uv, float horizon, vec3 col, float aaPx) {
  float aa = aaPx;
  // Early-out above the airport zone. Everything here lives below horizon + 0.18:
  // the tower cab tops out at +0.082, its obstruction light at +0.086, and a
  // departure climbs to +0.135, each plus a halo. Above that there is nothing to
  // draw, and on a 5:1 frame with the horizon at 0.16 that is over two thirds of
  // the pixels.
  //
  // Not a micro-optimisation: without it, seven lamp() calls and five slabs
  // evaluate for every pixel of empty sky, which measured 65.3 fps at 6000x1200
  // against 70.8 for the saver without an airport at all -- under the 69.6 fps
  // #203 sets as the bar. The branch is coherent across whole screen regions, so
  // it costs nothing on a GPU.
  if (uv.y > horizon + 0.18) return col;

  float halfX = 0.5 * iResolution.x / iResolution.y;
  // Vanishing point of the runway, shifted by the direction in use. Hoisted
  // here because the aircraft paths below start and end on it too.
  float vpx = uRunwayDir * 0.10 * halfX;
  float night = 1.0 - uDay;

  // Traffic thins in bad weather, which is real behaviour rather than decoration:
  // low visibility and heavy precipitation both cut movement rates.
  float ops = clamp(1.0 - uFog * 0.75 - uPrecip * 0.5, 0.15, 1.0);

  // ---- massing, just above the horizon -------------------------------------
  //
  // Schiphol reads HORIZONTAL, not vertical, which suits a 6000x1200 frame
  // exactly: long low terminal masses and hangars, with one vertical accent.
  float b = 0.0;
  b = max(b, slab(uv, -0.94 * halfX, -0.34 * halfX, horizon, 0.020, aa));   // terminal, long and low
  b = max(b, slab(uv, -0.34 * halfX, -0.20 * halfX, horizon, 0.030, aa));   // pier head
  b = max(b, slab(uv,  0.00 * halfX,  0.26 * halfX, horizon, 0.016, aa));   // hangar row
  b = max(b, slab(uv,  0.40 * halfX,  0.60 * halfX, horizon, 0.024, aa));   // second terminal
  b = max(b, slab(uv,  0.76 * halfX,  0.96 * halfX, horizon, 0.014, aa));   // freight sheds

  // Control tower, deliberately off centre so the composition stays asymmetric.
  float towerX = 0.70 * halfX;
  float towerTop = horizon + 0.082;
  b = max(b, slab(uv, towerX - 0.006, towerX + 0.006, horizon, 0.082, aa));       // shaft
  b = max(b, slab(uv, towerX - 0.017, towerX + 0.017, towerTop - 0.016, 0.016, aa)); // cab

  // By day the massing is a dark silhouette against the sky; by night it is barely
  // darker than the ground and the lights do the work instead.
  vec3 buildCol = mix(vec3(0.012, 0.014, 0.020), vec3(0.055, 0.058, 0.062), uDay);
  col = mix(col, buildCol, b * (0.75 + 0.25 * uDay) * (1.0 - uFog * 0.55));

  // Lit windows along the terminal at night, sparse and irregular.
  if (night > 0.02) {
    float wx = uv.x * 90.0;
    float win = step(0.62, rand(floor(wx)));
    float row = step(horizon + 0.004, uv.y) * (1.0 - step(horizon + 0.016, uv.y));
    col += vec3(0.95, 0.82, 0.55) * win * row * b * night * 0.22 * (1.0 - uFog * 0.6);
  }

  // ---- runway and approach lighting, below the horizon ---------------------
  //
  // The strongest single cue that this is an airport, and the cheapest to draw.
  // A depth parameter that grows toward the viewer gives the perspective: light
  // spacing compresses toward the horizon because 1/depth does.
  float below = horizon - uv.y;
  if (below > 0.0) {
    float d = below + 0.004;
    float z = 0.055 / d;                      // depth along the runway
    float halfW = below * 3.4;               // runway widens toward the viewer

    // Wet apron: precipitation makes the ground reflect the lights it carries.
    float wet = clamp(uPrecip * 1.2, 0.0, 1.0);

    // Centreline lights, receding.
    float alongC = fract(z * 3.0);
    float onC = exp(-abs(uv.x - vpx) / max(below * 0.24, 0.002)) * exp(-alongC * 7.0);
    col += vec3(0.95, 0.93, 0.80) * onC * (0.35 + 0.8 * night) * ops * (0.6 + 0.6 * wet);

    // Blue taxiway edge lights, both sides, offset in phase from the centreline.
    float alongE = fract(z * 2.0 + 0.35);
    float edge = exp(-alongE * 9.0);
    float onL = exp(-abs(uv.x - (vpx - halfW)) / max(below * 0.14, 0.002));
    float onR = exp(-abs(uv.x - (vpx + halfW)) / max(below * 0.14, 0.002));
    col += vec3(0.30, 0.55, 1.0) * edge * (onL + onR) * (0.25 + 0.9 * night) * ops;

    // Approach bar: a bright row that strobes toward the threshold, the sequenced
    // flasher a pilot follows in. Reads as an airport before any shape does.
    float seq = fract(iTime * 1.4);
    float bar = exp(-abs(z - (0.6 + seq * 3.4)) * 2.2);
    float across = exp(-abs(uv.x - vpx) / max(below * 1.3, 0.004));
    col += vec3(1.0, 0.98, 0.90) * bar * across * (0.5 + 1.2 * night) * ops
           * (0.7 + uFog * 1.4);            // in fog the approach lights glow through
  }

  // ---- beacons -------------------------------------------------------------
  //
  // A rotating airport beacon alternating white and green, plus red obstruction
  // lights on the tower. Slow enough to read as rotation, not as a blink.
  float beaconPhase = fract(iTime * 0.42);
  float sweep = exp(-pow((beaconPhase - 0.5) * 6.0, 2.0));
  vec3 beaconTint = mix(vec3(0.55, 1.0, 0.65), vec3(1.0), step(0.5, fract(iTime * 0.21)));
  col += lamp(uv, vec2(towerX, towerTop - 0.004), 0.0022, beaconTint,
              sweep * (0.6 + 1.6 * night), uFog);

  float obst = 0.55 + 0.45 * sin(iTime * 1.9);
  col += lamp(uv, vec2(towerX, towerTop + 0.004), 0.0016, vec3(1.0, 0.15, 0.10),
              obst * (0.5 + 1.2 * night), uFog);
  col += lamp(uv, vec2(-0.20 * halfX, horizon + 0.032), 0.0014, vec3(1.0, 0.15, 0.10),
              obst * (0.4 + 1.0 * night), uFog);

  // ---- aircraft ------------------------------------------------------------
  //
  // Two movements on their own clocks: one climbing away, one on approach. Which
  // way they operate follows uRunwayDir, and therefore the wind -- Schiphol swaps
  // runway ends with it, so this is real behaviour rather than decoration.
  for (int i = 0; i < 2; i++) {
    float fi = float(i);
    float period = 15.0 + fi * 9.0;
    float t = fract((iTime + fi * 7.3) / period);
    // Only airborne for part of the cycle, so the sky is not permanently busy.
    float live = step(0.05, t) * (1.0 - step(0.85, t));
    if (live < 0.5 || ops < 0.3) continue;

    float u = (t - 0.05) / 0.80;
    bool departing = i == 0;
    // Departures climb away from the threshold; arrivals descend toward it.
    float x = departing ? mix(vpx, vpx + uRunwayDir * 0.88 * halfX, u)
                        : mix(vpx - uRunwayDir * 0.92 * halfX, vpx, u);
    float y = departing ? horizon + 0.020 + u * u * 0.115
                        : horizon + 0.135 - u * u * 0.115;
    vec2 at = vec2(x, y);

    // Body: a small dark dash by day, invisible at night except for its lights.
    float body = exp(-length((uv - at) / vec2(0.010, 0.0016)));
    col = mix(col, vec3(0.02), body * uDay * 0.7 * (1.0 - uFog * 0.7));
    // Landing light forward of the body, and a red beacon on top.
    col += lamp(uv, at + vec2(uRunwayDir * (departing ? 0.006 : -0.006), 0.0), 0.0018,
                vec3(1.0, 0.97, 0.88), (0.5 + 1.5 * night) * (1.0 - uFog * 0.5), uFog);
    col += lamp(uv, at + vec2(0.0, 0.0022), 0.0011, vec3(1.0, 0.2, 0.15),
                (0.3 + 0.9 * night) * abs(sin(iTime * 3.1)), uFog);
  }

  return col;
}

void main() {
  vec2 frag = gl_FragCoord.xy;
  // Aspect-correct horizontally so cloud shapes are not stretched 5:1 on the
  // wall (issue #114): x is divided by the SHORT axis, like the fractals.
  vec2 uv = vec2((frag.x - 0.5 * iResolution.x) / iResolution.y,
                 frag.y / iResolution.y);

  // Horizon low in frame, so the sky gets the space and the ground reads as a
  // band rather than half the picture.
  float horizon = 0.16;
  float aboveH = smoothstep(horizon - 0.006, horizon + 0.006, uv.y);
  float skyH = clamp((uv.y - horizon) / (1.0 - horizon), 0.0, 1.0);

  vec3 col = skyGradient(skyH);
  // Overcast is not blue seen through white: the sky itself goes grey. Pull
  // saturation out with cover so no residual blue survives a heavy sky.
  float grey = dot(col, vec3(0.299, 0.587, 0.114));
  col = mix(col, vec3(grey) * mix(1.0, 0.82, uDay), uCloud * 0.85);

  // Sun or moon. Placed off-centre so the 5:1 frame is not symmetrical.
  vec2 body = vec2(-0.9, horizon + 0.42);
  float bodyD = length(uv - body);
  // HDR core: the sun is genuinely brighter than the page, which is what makes
  // the bloom read as light rather than as a white blob.
  float sunCore = smoothstep(0.055, 0.030, bodyD);
  float sunHalo = exp(-bodyD * 5.5);
  vec3 sunCol = mix(vec3(0.85, 0.90, 1.05), vec3(1.00, 0.93, 0.72), uDay);
  // Cloud cover hides it, and squared so the last of the disc goes early: a
  // linear falloff still left a visible dot at 100% cover.
  float clear = 1.0 - uCloud;
  float bodyVis = clear * clear * aboveH;
  col += sunCol * (sunCore * 5.0 + sunHalo * 0.35) * bodyVis;

  // Stars, night only, thinned by cloud. Hashed per cell so they do not crawl.
  if (uDay < 0.85) {
    vec2 cell = floor(frag / 3.0);
    float s = hashF(hashU(uint(cell.x) * 1973u + uint(cell.y) * 9277u));
    float twinkle = 0.75 + 0.25 * sin(iTime * 1.7 + s * 40.0);
    float starMask = step(0.9985, s) * (1.0 - uDay) * (1.0 - 0.9 * uCloud) * aboveH;
    col += vec3(0.9, 0.95, 1.0) * starMask * twinkle * 2.2;
  }

  // Two cloud layers at different scales and drift rates. Wind moves them, so a
  // still day has near-static cloud and a gale has it streaming.
  float drift = iTime * (0.004 + uWind * 0.0016);
  // fbm() is built on snoise and therefore returns roughly -1..1. Remap to 0..1
  // before thresholding: treating it as 0..1 meant even 100% cover left most of
  // the sky above the threshold, so a fully overcast reading rendered as a
  // pleasant partly-cloudy afternoon.
  float lowN  = fbm(uv * vec2(1.5, 3.0) + vec2(drift, 0.0), 5) * 0.5 + 0.5;
  float highN = fbm(uv * vec2(0.7, 1.8) + vec2(drift * 0.45 + 11.3, 4.7), 4) * 0.5 + 0.5;

  // Cover maps to a threshold rather than an opacity: at 20% cover you want a
  // few discrete clouds, not a uniform 20% grey veil over everything. The top of
  // the range goes NEGATIVE so that 100% really is unbroken -- stopping at 0
  // leaves gaps wherever the noise dips, and "overcast" with blue holes in it
  // reads as a bug.
  float lowT  = mix(0.80, -0.10, uCloud);
  float highT = mix(0.88, 0.02, uCloud);
  float lowMask  = smoothstep(lowT, lowT + 0.16, lowN) * aboveH;
  float highMask = smoothstep(highT, highT + 0.22, highN) * aboveH * 0.85;

  // Lit from the sun side, shaded away from it -- a flat grey cloud is the thing
  // that makes procedural skies look like wallpaper.
  float lit = clamp(0.5 + 0.5 * (uv.x - body.x) * -0.8, 0.0, 1.0);
  vec3 cloudLit  = mix(vec3(0.30, 0.33, 0.40), vec3(1.02, 1.00, 0.98), uDay);
  vec3 cloudDark = mix(vec3(0.06, 0.07, 0.11), vec3(0.42, 0.45, 0.52), uDay);
  vec3 cloudCol  = mix(cloudDark, cloudLit, lit);
  // A storm sky is darker and heavier, not just cloudier.
  cloudCol *= mix(1.0, 0.55, uStorm);
  // Modulate by the noise INDEPENDENTLY of coverage, so a fully overcast sky
  // still has lighter and darker patches. Without this, cover=1 saturates the
  // mask everywhere and the result is a flat grey gradient -- the bare-gradient
  // failure #101 warns about, just in grey instead of blue.
  float texN = fbm(uv * vec2(2.6, 5.2) + vec2(drift * 1.3 + 5.1, 8.2), 4) * 0.5 + 0.5;
  cloudCol *= 0.74 + 0.52 * mix(lowN, texN, 0.45);
  // A heavy sky is DARKER, not just more covered. Without this, full cover plus
  // the lit term plus bloom pushed the frame to near-white, which reads as fog
  // rather than as rain.
  cloudCol *= mix(1.0, 0.66, uCloud);

  col = mix(col, cloudCol, clamp(highMask, 0.0, 1.0));
  col = mix(col, cloudCol * 1.04, clamp(lowMask, 0.0, 1.0));

  // Lightning: a full-sky flash plus a brighter core near the storm cell.
  if (uFlash > 0.001) {
    float cellGlow = exp(-length(uv - vec2(1.1, horizon + 0.55)) * 2.6);
    // Restrained on purpose. Lighting the whole sky at 0.35 plus a 2.5 core blew
    // the frame to white and lost the night entirely -- the flash has to read as
    // a flash, which means the dark has to survive it.
    col += vec3(0.85, 0.90, 1.10) * uFlash * (0.10 + 1.1 * cellGlow);
  }

  // Ground: a dark band with a little texture, so the bottom of a 6000px frame
  // is not flat. Deliberately low-contrast -- it is a stage, not the subject.
  if (uv.y < horizon + 0.01) {
    float g = fbm(vec2(uv.x * 3.0, uv.y * 12.0 + 3.0), 3);
    vec3 groundCol = mix(vec3(0.02, 0.025, 0.04), vec3(0.10, 0.11, 0.09), uDay);
    groundCol *= 0.75 + 0.35 * g;
    // Lying snow: bright, low-contrast, and blue-shifted in shadow.
    vec3 snowCol = mix(vec3(0.16, 0.19, 0.26), vec3(0.80, 0.84, 0.92), uDay);
    groundCol = mix(groundCol, snowCol * (0.88 + 0.18 * g), uSnowGround);
    // Wet ground under precipitation reflects a little of the sky.
    col = mix(col, groundCol, 1.0 - aboveH);
  }

  // The airport goes in before the fog, so fog occludes it -- which is the point:
  // low visibility has to change the airport and not just the sky (#203).
  col = airport(uv, horizon, col, 1.5 / iResolution.y);

  // Fog last: it sits in front of everything, thickest at the horizon.
  if (uFog > 0.001) {
    vec3 fogCol = mix(vec3(0.10, 0.11, 0.14), vec3(0.74, 0.76, 0.79), uDay);
    float band = exp(-max(uv.y - horizon, 0.0) * 3.2);
    col = mix(col, fogCol, clamp(uFog * (0.35 + 0.65 * band), 0.0, 0.96));

    // ...and then the airport lights are added back ON TOP of it, because that is
    // how fog actually works: you see a light in fog precisely because the fog
    // scatters it toward you. Mixing the airport under a 96%-opaque fog layer
    // hides it completely, which is physically wrong and visually dead -- the
    // fog state was a blank white field even before the airport existed.
    //
    // Only the halos come back, not the cores or the massing: in real fog the
    // shapes go and the lights bloom. #203 calls this the case where approach
    // lighting alone says airport before any shape does.
    // Same bound as airport(), for the same reason.
    if (uv.y > horizon + 0.18) { outColor = vec4(col * uLum, 1.0); return; }
    float halfXf = 0.5 * iResolution.x / iResolution.y;
    float vpxf = uRunwayDir * 0.10 * halfXf;
    float belowf = horizon - uv.y;
    if (belowf > 0.0) {
      float zf = 0.055 / (belowf + 0.004);
      float seqf = fract(iTime * 1.4);
      float barf = exp(-abs(zf - (0.6 + seqf * 3.4)) * 1.1);
      float acrossf = exp(-abs(uv.x - vpxf) / max(belowf * 2.2, 0.006));
      col += vec3(1.0, 0.97, 0.88) * barf * acrossf * uFog * 0.85;
    }
    // The tower beacon and the obstruction lights diffuse into a wide bloom.
    float bp = fract(iTime * 0.42);
    float sw = exp(-pow((bp - 0.5) * 6.0, 2.0));
    vec3 bt = mix(vec3(0.55, 1.0, 0.65), vec3(1.0), step(0.5, fract(iTime * 0.21)));
    float tx = 0.70 * halfXf;
    col += bt * exp(-length((uv - vec2(tx, horizon + 0.078)) / vec2(1.0, 0.5)) / 0.055)
           * sw * uFog * 0.55;
    col += vec3(1.0, 0.2, 0.14)
           * exp(-length((uv - vec2(tx, horizon + 0.086)) / vec2(1.0, 0.5)) / 0.035)
           * (0.55 + 0.45 * sin(iTime * 1.9)) * uFog * 0.35;
  }

  outColor = vec4(col * uLum, 1.0);
}
