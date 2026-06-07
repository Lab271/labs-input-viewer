/**
 * DVD-style Bouncing Logo Animation
 *
 * Classic screensaver effect when video feeds lose signal.
 * Logo bounces around the screen and changes color on each bounce.
 */

// Classic DVD screensaver colors as hue-rotate degrees
// These values rotate the hue to achieve the classic DVD colors
// The logo is blue by default, so we rotate from there
const DVD_COLORS = [
  0,     // Blue (original)
  60,    // Cyan
  120,   // Green
  180,   // Yellow
  240,   // Red
  300,   // Magenta
]

// Animation state
let animationState = {
  running: false,
  animationId: null,
  x: 0,
  y: 0,
  vx: 2, // Velocity X (pixels per frame)
  vy: 2, // Velocity Y (pixels per frame)
  colorIndex: 0,
  logoElement: null,
  containerElement: null,
  logoWidth: 0,
  logoHeight: 0
}

/**
 * Initialize the bouncing logo animation
 * @param {HTMLElement} logoElement - The logo element to animate
 * @param {HTMLElement} containerElement - The container element (bounds)
 */
export function initBouncingLogo(logoElement, containerElement) {
  animationState.logoElement = logoElement
  animationState.containerElement = containerElement

  // Set initial random position
  resetPosition()

  // Set initial color
  applyColor()
}

/**
 * Reset position to a random starting point
 */
function resetPosition() {
  if (!animationState.containerElement || !animationState.logoElement) return

  const container = animationState.containerElement.getBoundingClientRect()
  const logo = animationState.logoElement.getBoundingClientRect()

  animationState.logoWidth = logo.width
  animationState.logoHeight = logo.height

  // Random starting position within bounds
  const maxX = container.width - logo.width
  const maxY = container.height - logo.height

  animationState.x = Math.random() * Math.max(0, maxX)
  animationState.y = Math.random() * Math.max(0, maxY)

  // Random direction
  animationState.vx = (Math.random() > 0.5 ? 1 : -1) * 2
  animationState.vy = (Math.random() > 0.5 ? 1 : -1) * 2

  // Apply initial position
  updateLogoPosition()
}

/**
 * Apply the current color to the logo using hue-rotate
 */
function applyColor() {
  if (!animationState.logoElement) return

  const hueRotation = DVD_COLORS[animationState.colorIndex]
  // Use hue-rotate to change the logo color
  // The logo is blue, so rotating the hue shifts it to other colors
  animationState.logoElement.style.filter = `hue-rotate(${hueRotation}deg)`
}

/**
 * Change to the next color
 */
function nextColor() {
  animationState.colorIndex = (animationState.colorIndex + 1) % DVD_COLORS.length
  applyColor()
}

/**
 * Update the logo position on screen
 */
function updateLogoPosition() {
  if (!animationState.logoElement) return

  animationState.logoElement.style.transform =
    `translate(${animationState.x}px, ${animationState.y}px)`
}

/**
 * Animation frame - move the logo and handle bouncing
 */
function animate() {
  if (!animationState.running) return

  const container = animationState.containerElement.getBoundingClientRect()

  // Update logo dimensions (in case of resize)
  const logo = animationState.logoElement.getBoundingClientRect()
  animationState.logoWidth = logo.width
  animationState.logoHeight = logo.height

  // Calculate bounds
  const maxX = container.width - animationState.logoWidth
  const maxY = container.height - animationState.logoHeight

  // Move
  animationState.x += animationState.vx
  animationState.y += animationState.vy

  // Bounce off edges.
  // Only count a bounce when the logo is actually moving *into* the wall.
  // This prevents repeated bounces (and rapid multi-color changes) when the
  // logo is pinned at or past an edge for several frames — e.g. when the
  // container is re-measured smaller mid-flight, which leaves x/y >= max.
  let bounced = false

  if (animationState.x <= 0 && animationState.vx < 0) {
    animationState.x = 0
    animationState.vx = Math.abs(animationState.vx)
    bounced = true
  } else if (animationState.x >= maxX && animationState.vx > 0) {
    animationState.x = maxX
    animationState.vx = -Math.abs(animationState.vx)
    bounced = true
  }

  if (animationState.y <= 0 && animationState.vy < 0) {
    animationState.y = 0
    animationState.vy = Math.abs(animationState.vy)
    bounced = true
  } else if (animationState.y >= maxY && animationState.vy > 0) {
    animationState.y = maxY
    animationState.vy = -Math.abs(animationState.vy)
    bounced = true
  }

  // Keep the logo within bounds even when no bounce was registered
  // (e.g. it was already past a shrunken edge while moving away from it).
  if (animationState.x < 0) animationState.x = 0
  else if (animationState.x > maxX) animationState.x = maxX
  if (animationState.y < 0) animationState.y = 0
  else if (animationState.y > maxY) animationState.y = maxY

  // Change color on bounce
  if (bounced) {
    nextColor()
  }

  // Update position
  updateLogoPosition()

  // Continue animation
  animationState.animationId = requestAnimationFrame(animate)
}

/**
 * Start the bouncing animation
 */
export function startBouncingLogo() {
  if (animationState.running) return

  animationState.running = true
  resetPosition()
  animate()

  console.log('[DVD] Bouncing logo animation started')
}

/**
 * Stop the bouncing animation
 */
export function stopBouncingLogo() {
  animationState.running = false

  if (animationState.animationId) {
    cancelAnimationFrame(animationState.animationId)
    animationState.animationId = null
  }

  console.log('[DVD] Bouncing logo animation stopped')
}

/**
 * Check if animation is currently running
 * @returns {boolean}
 */
export function isBouncingLogoRunning() {
  return animationState.running
}
