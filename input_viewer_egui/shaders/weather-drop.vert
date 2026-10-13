// GENERATED from input_viewer_electron/.../weather.js DROP_VERT by extract.mjs -- do not edit.
#version 330 core
uniform vec2 uQuadScale;    // pixel -> clip, per axis
uniform float uTime;
uniform float uCount;
uniform float uFall;        // fall speed, screens per second
uniform float uShear;       // horizontal drift per unit of fall
uniform float uLength;      // streak length in px
uniform float uWidth;       // streak width in px
uniform float uSnow;        // 0 = rain, 1 = snow
uniform float uSeed;
uniform float uSlope;       // screen-space lean: dx/dy in pixels
out float vAlpha;
out vec2 vQuad;
out float vDepth;


in vec2 aCorner;
uniform sampler2D uState;
uniform float uSide;

// Per-instance state, fetched by instance id.
vec4 fetchInstance(out vec2 uvOut) {
  int id = gl_InstanceID;
  int x = id % int(uSide);
  int y = id / int(uSide);
  uvOut = (vec2(float(x), float(y)) + 0.5) / uSide;
  return texture(uState, uvOut);
}

// Place the quad at the instance position, rotated to face its velocity and
// scaled by size. The stretch factor elongates along the direction of travel:
// 1.0 is round, higher values give motion-blur streaks.
// size MUST be isotropic in whatever space you pass it -- pixels, or world
// units. Passing a size that already has a per-axis conversion folded in (e.g.
// multiplied by a pixel->clip uQuadScale) rotates an anisotropically-scaled
// corner, which shears the quad by the display aspect: 5x on a 6000x1200 wall
// and invisible at 16:9. Convert to clip on the RESULT, not the input.
// See issue #190; boids.js had exactly this bug.
vec2 orientedQuadOffset(vec2 vel, vec2 size, float stretch) {
  float speed = length(vel);
  // Below a threshold the heading is noise, so keep the quad axis-aligned
  // rather than letting it spin wildly when a particle is nearly stationary.
  vec2 dir = speed > 1e-4 ? vel / speed : vec2(1.0, 0.0);
  vec2 perp = vec2(-dir.y, dir.x);
  vec2 c = aCorner * size;
  return dir * (c.x * stretch) + perp * c.y;
}


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


void main() {
  vQuad = aCorner;
  float id = float(gl_InstanceID);
  float h1 = hashF(hashU(uint(id) * 747796405u + uint(uSeed * 4096.0)));
  float h2 = hashF(hashU(uint(id) * 2891336453u + 17u));
  float h3 = hashF(hashU(uint(id) * 1013904223u + 71u));

  // Depth: near drops are bigger, faster and brighter. Three discrete-ish bands
  // read better than a continuous ramp at wall distance.
  vDepth = 0.35 + 0.65 * h3;

  // Fall. Normalised space 0..1 vertically, wrapping. Snow falls far slower and
  // wanders; rain is close to straight.
  float speed = uFall * mix(1.0, 0.18, uSnow) * vDepth;
  float y = fract(h1 + 1.0 - uTime * speed);
  float x = h2 + (1.0 - y) * uShear * mix(1.0, 0.35, uSnow);
  if (uSnow > 0.5) {
    // Wander, at a per-flake frequency so the field does not sway in unison.
    x += 0.035 * sin(uTime * (0.5 + h3 * 0.9) + h1 * 31.0);
  }
  x = fract(x);

  // Fade the top and bottom edges so drops appear and vanish rather than
  // popping at the frame boundary.
  vAlpha = smoothstep(0.0, 0.06, y) * smoothstep(1.0, 0.94, y) * (0.35 + 0.65 * vDepth);

  vec2 centre = vec2(x * 2.0 - 1.0, y * 2.0 - 1.0);

  // Size in PIXELS, then converted to clip on the result -- never fold the
  // per-axis conversion in before shaping, or the quad shears with the display
  // aspect (issue #190).
  vec2 sizePx = uSnow > 0.5
    ? vec2(uWidth * 4.5 * vDepth)
    : vec2(uWidth * vDepth, uLength * vDepth);
  // Shear the streak along its travel so it leans with the wind instead of
  // falling vertically through a slanted field.
  // Lean the streak along its actual screen-space travel. uShear is a drift in
  // NORMALISED width and uSlope the same thing in pixels; using the normalised
  // value here made the streaks near-vertical while the field drifted sideways,
  // so the rain fell straight through slanted weather.
  vec2 offPx = vec2(aCorner.x * sizePx.x - aCorner.y * sizePx.y * uSlope,
                    aCorner.y * sizePx.y);
  gl_Position = vec4(centre + offPx * uQuadScale, 0.0, 1.0);
}
