/**
 * Input Viewer - Renderer Process
 * 
 * Handles video capture, UI interactions, and keyboard shortcuts
 */

import {
  checkNoSignal,
  isReady as isDetectionReady,
  saveReferenceScreenshot,
  hasReferenceScreenshot,
  captureScreenshot,
  serializeReferences,
  deserializeReferences
} from './detection-simple.js'

import {
  initBouncingLogo,
  startBouncingLogo,
  stopBouncingLogo,
  isBouncingLogoRunning
} from './bouncing-logo.js'

// =============================================================================
// State Management
// =============================================================================

const state = {
  devices: [],
  leftDeviceId: null,
  rightDeviceId: null,
  leftStream: null,
  rightStream: null,
  layoutMode: 'dual', // 'dual', 'single'
  cursorTimeout: null,
  cursorHideDelay: 3000,
  centerGap: 60,
  borderWidth: 0,
  frozen: false,
  settings: null, // Will be loaded from file
  // No-signal detection state
  detectionCanvas: null,
  detectionRunning: false,
  detectionFrameCount: 0, // Frame counter for detection sampling
  noSignalState: {
    left: false,
    right: false
  },
  // DVD screensaver timer
  dvdScreensaverTimeout: null,
  dvdScreensaverDelay: 5 * 60 * 1000 // 5 minutes in milliseconds
}

// =============================================================================
// DOM Elements
// =============================================================================

const elements = {
  leftFeed: document.getElementById('left-feed'),
  rightFeed: document.getElementById('right-feed'),
  leftVideo: document.getElementById('left-video'),
  rightVideo: document.getElementById('right-video'),
  videoWrapper: document.getElementById('video-wrapper'),
  centerDivider: document.getElementById('center-divider'),
  bottomLogo: document.getElementById('bottom-logo'),
  leftBorder: document.getElementById('left-border'),
  rightBorder: document.getElementById('right-border'),
  inputNameOverlay: document.getElementById('input-name-overlay'),
  inputNameText: document.getElementById('input-name-text'),
  freezeOverlay: document.getElementById('freeze-overlay'),
  freezeIndicator: document.getElementById('freeze-indicator'),
  freezeCanvas: document.getElementById('freeze-canvas'),
  dropdownTrigger: document.getElementById('dropdown-trigger'),
  dropdownPanel: document.getElementById('dropdown-panel'),
  inputList: document.getElementById('input-list'),
  layoutDual: document.getElementById('layout-dual'),
  layoutSingle: document.getElementById('layout-single'),
  centerGap: document.getElementById('center-gap'),
  centerGapValue: document.getElementById('center-gap-value'),
  borderWidth: document.getElementById('border-width'),
  borderWidthValue: document.getElementById('border-width-value'),
  updateNotification: document.getElementById('update-notification'),
  updateMessage: document.getElementById('update-message'),
  // Setup wizard elements
  setupWizard: document.getElementById('setup-wizard'),
  setupDetectionBtn: document.getElementById('setup-detection-btn'),
  wizardCaptureBtn: document.getElementById('wizard-capture-btn'),
  wizardSkipBtn: document.getElementById('wizard-skip-btn'),
  wizardPreviewCanvas: document.getElementById('wizard-preview-canvas'),
  wizardPreviewStatus: document.getElementById('wizard-preview-status'),
  // Cached label references (avoids DOM queries in hot paths)
  leftLabel: document.querySelector('#left-feed .input-label'),
  rightLabel: document.querySelector('#right-feed .input-label'),
  // Version display
  appVersion: document.getElementById('app-version'),
  // DVD screensaver overlay
  dvdOverlay: document.getElementById('dvd-overlay'),
  dvdLogo: document.getElementById('dvd-logo')
}

// =============================================================================
// Settings Persistence
// =============================================================================

async function loadSettings() {
  try {
    if (window.electronAPI) {
      const settings = await window.electronAPI.loadSettings()
      return settings
    }
  } catch (e) {
    console.error('Error loading settings:', e)
  }
  return getDefaultSettings()
}

async function saveSettings() {
  try {
    if (window.electronAPI) {
      const settingsToSave = {
        leftDeviceId: state.leftDeviceId,
        rightDeviceId: state.rightDeviceId,
        layoutMode: state.layoutMode,
        centerGap: state.centerGap,
        borderWidth: state.borderWidth,
        inputs: state.settings.inputs,
        initialSetupComplete: state.settings.initialSetupComplete,
        noSignalReferences: state.settings.noSignalReferences
      }
      await window.electronAPI.saveSettings(settingsToSave)
    }
  } catch (e) {
    console.error('Error saving settings:', e)
  }
}

// Debounced save to reduce IPC calls during rapid changes (e.g., slider drags)
let saveSettingsTimeout = null
function debouncedSaveSettings() {
  clearTimeout(saveSettingsTimeout)
  saveSettingsTimeout = setTimeout(saveSettings, 300)
}

function getDefaultSettings() {
  return {
    inputs: {},
    centerGap: 60,
    borderWidth: 0,
    leftDeviceId: null,
    rightDeviceId: null,
    layoutMode: null, // null means use screen-based detection
    initialSetupComplete: false,
    noSignalReferences: null
  }
}

// Get custom name for input
function getInputName(deviceId, defaultName) {
  const inputSettings = state.settings.inputs[deviceId]
  if (inputSettings && inputSettings.name) {
    return inputSettings.name
  }
  return defaultName
}

// Check if input is enabled
function isInputEnabled(deviceId) {
  const inputSettings = state.settings.inputs[deviceId]
  if (inputSettings && typeof inputSettings.enabled === 'boolean') {
    return inputSettings.enabled
  }
  return true // Default to enabled
}

// Set custom name for input
function setInputName(deviceId, name) {
  if (!state.settings.inputs[deviceId]) {
    state.settings.inputs[deviceId] = { enabled: true }
  }
  state.settings.inputs[deviceId].name = name
  saveSettings()
}

// Toggle input enabled/disabled
function toggleInputEnabled(deviceId) {
  if (!state.settings.inputs[deviceId]) {
    state.settings.inputs[deviceId] = { enabled: true, name: null }
  }
  state.settings.inputs[deviceId].enabled = !state.settings.inputs[deviceId].enabled
  saveSettings()
  renderInputList()
}

// =============================================================================
// Freeze Frame
// =============================================================================

function toggleFreeze() {
  state.frozen = !state.frozen
  
  if (state.frozen) {
    // Capture current frame to canvas
    captureFrame()
    elements.freezeOverlay.classList.remove('hidden')
    elements.freezeIndicator.classList.remove('hidden')
    elements.freezeIndicator.innerHTML = '<span class="freeze-icon">❙❙</span> FROZEN'
    elements.freezeIndicator.classList.add('frozen')
    
    // Hide video feeds
    elements.leftVideo.style.opacity = '0'
    elements.rightVideo.style.opacity = '0'
  } else {
    // Show video feeds
    elements.freezeOverlay.classList.add('hidden')
    elements.freezeIndicator.classList.add('hidden')
    elements.leftVideo.style.opacity = '1'
    elements.rightVideo.style.opacity = '1'
    
    // Show brief LIVE indicator
    elements.freezeIndicator.classList.remove('hidden')
    elements.freezeIndicator.innerHTML = '<span class="freeze-icon">▶</span> LIVE'
    elements.freezeIndicator.classList.remove('frozen')
    setTimeout(() => {
      elements.freezeIndicator.classList.add('hidden')
    }, 1000)
  }
}

function captureFrame() {
  const canvas = elements.freezeCanvas
  const ctx = canvas.getContext('2d')
  
  // Get the video wrapper dimensions
  const wrapper = elements.videoWrapper
  canvas.width = wrapper.clientWidth
  canvas.height = wrapper.clientHeight
  
  // Clear canvas
  ctx.fillStyle = '#000000'
  ctx.fillRect(0, 0, canvas.width, canvas.height)
  
  // Draw based on layout mode
  if (state.layoutMode === 'dual') {
    // Draw both videos side by side
    const gap = state.layoutGap
    const halfWidth = (canvas.width - gap) / 2
    
    // Draw left video
    if (elements.leftVideo.srcObject) {
      ctx.drawImage(elements.leftVideo, 0, 0, halfWidth, canvas.height)
    }
    
    // Draw right video
    if (elements.rightVideo.srcObject) {
      ctx.drawImage(elements.rightVideo, halfWidth + gap, 0, halfWidth, canvas.height)
    }
  } else {
    // Single view - draw the active video
    const video = state.layoutMode === 'right' ? elements.rightVideo : elements.leftVideo
    if (video.srcObject) {
      ctx.drawImage(video, 0, 0, canvas.width, canvas.height)
    }
  }
}

// =============================================================================
// Video Device Management
// =============================================================================

async function getVideoDevices() {
  try {
    // Request permission first
    await navigator.mediaDevices.getUserMedia({ video: true })
    
    const devices = await navigator.mediaDevices.enumerateDevices()
    state.devices = devices.filter(device => device.kind === 'videoinput')
    
    console.log('Available video devices:', state.devices)
    
    // Initialize settings for new devices
    state.devices.forEach((device) => {
      if (!state.settings.inputs[device.deviceId]) {
        state.settings.inputs[device.deviceId] = {
          name: null, // Will use default label
          enabled: true
        }
      }
    })
    
    // Save settings with new devices
    saveSettings()
    
    // Set default devices from settings or auto-assign
    if (state.devices.length > 0) {
      // Try to restore from settings
      const savedLeft = state.devices.find(d => d.deviceId === state.settings.leftDeviceId)
      const savedRight = state.devices.find(d => d.deviceId === state.settings.rightDeviceId)
      
      // Auto-assign devices if not saved
      if (savedLeft) {
        state.leftDeviceId = savedLeft.deviceId
      } else {
        // Use first enabled device
        const firstEnabled = state.devices.find(d => isInputEnabled(d.deviceId))
        state.leftDeviceId = firstEnabled ? firstEnabled.deviceId : state.devices[0].deviceId
      }
      
      if (savedRight) {
        state.rightDeviceId = savedRight.deviceId
      } else {
        // For dual mode: use second device if available, otherwise duplicate first
        if (state.devices.length > 1) {
          const secondEnabled = state.devices.slice(1).find(d => isInputEnabled(d.deviceId))
          state.rightDeviceId = secondEnabled ? secondEnabled.deviceId : state.devices[1].deviceId
        } else {
          // Only one device: duplicate it for dual mode
          state.rightDeviceId = state.leftDeviceId
        }
      }
    }
    
    renderInputList()
    return state.devices
  } catch (error) {
    console.error('Error getting video devices:', error)
    showNoSignal('left')
    showNoSignal('right')
    return []
  }
}

async function startVideoStream(deviceId, videoElement, side) {
  try {
    // Stop existing stream
    if (side === 'left' && state.leftStream) {
      state.leftStream.getTracks().forEach(track => track.stop())
    }
    if (side === 'right' && state.rightStream) {
      state.rightStream.getTracks().forEach(track => track.stop())
    }
    
    if (!deviceId) {
      showNoSignal(side)
      return null
    }
    
    // Check if device is enabled
    if (!isInputEnabled(deviceId)) {
      showNoSignal(side)
      return null
    }
    
    // Request highest resolution the device supports
    const constraints = {
      video: {
        deviceId: { exact: deviceId },
        width: { ideal: 4096 },
        height: { ideal: 2160 }
      }
    }

    let stream = await navigator.mediaDevices.getUserMedia(constraints)
    videoElement.srcObject = stream

    // Some capture devices start at low resolution and need a restart to get high-res
    // Check resolution after stream starts and retry if too low
    const track = stream.getVideoTracks()[0]
    const settings = track.getSettings()
    const caps = track.getCapabilities()

    if (caps.width && settings.width < caps.width.max) {
      console.log(`[Video] Got ${settings.width}x${settings.height}, device supports up to ${caps.width.max}x${caps.height.max}. Restarting for higher res...`)

      // Stop current stream
      stream.getTracks().forEach(t => t.stop())

      // Small delay then request max resolution
      await new Promise(resolve => setTimeout(resolve, 100))

      const retryConstraints = {
        video: {
          deviceId: { exact: deviceId },
          width: { ideal: caps.width.max },
          height: { ideal: caps.height.max }
        }
      }
      stream = await navigator.mediaDevices.getUserMedia(retryConstraints)
      videoElement.srcObject = stream

      const newSettings = stream.getVideoTracks()[0].getSettings()
      console.log(`[Video] Retry got ${newSettings.width}x${newSettings.height}`)
    }
    
    // Store stream reference
    if (side === 'left') {
      state.leftStream = stream
    } else {
      state.rightStream = stream
    }
    
    hideNoSignal(side)

    // Update input label using cached reference
    const device = state.devices.find(d => d.deviceId === deviceId)
    const label = side === 'left' ? elements.leftLabel : elements.rightLabel
    if (label && device) {
      const name = getInputName(deviceId, device.label || 'Unknown Input')
      label.textContent = name
    }

    return stream
  } catch (error) {
    console.error(`Error starting ${side} stream:`, error)
    showNoSignal(side)
    return null
  }
}

function showNoSignal(side) {
  const feed = side === 'left' ? elements.leftFeed : elements.rightFeed
  const overlay = feed.querySelector('.no-signal-overlay')
  overlay.classList.remove('hidden')
  state.noSignalState[side] = true
}

function hideNoSignal(side) {
  const feed = side === 'left' ? elements.leftFeed : elements.rightFeed
  const overlay = feed.querySelector('.no-signal-overlay')
  overlay.classList.add('hidden')
  state.noSignalState[side] = false
}

/**
 * Check if DVD screensaver should be shown and update accordingly
 * Shows when all active feeds have no signal for 5 minutes
 */
function updateDvdScreensaver() {
  // Determine which feeds are active based on layout mode
  let allNoSignal = false

  if (state.layoutMode === 'dual') {
    // In dual mode, show DVD when both feeds have no signal
    allNoSignal = state.noSignalState.left && state.noSignalState.right
  } else {
    // In single mode, show DVD when the left feed (active feed) has no signal
    allNoSignal = state.noSignalState.left
  }

  if (allNoSignal) {
    // Start timer if not already running
    if (!state.dvdScreensaverTimeout && !isBouncingLogoRunning()) {
      console.log('[DVD] No signal detected - starting 5 minute timer')
      state.dvdScreensaverTimeout = setTimeout(() => {
        // Double-check we still have no signal before starting
        const stillNoSignal = state.layoutMode === 'dual'
          ? state.noSignalState.left && state.noSignalState.right
          : state.noSignalState.left

        if (stillNoSignal) {
          showDvdScreensaver()
        }
        state.dvdScreensaverTimeout = null
      }, state.dvdScreensaverDelay)
    }
  } else {
    // Signal restored - cancel timer and hide screensaver
    if (state.dvdScreensaverTimeout) {
      clearTimeout(state.dvdScreensaverTimeout)
      state.dvdScreensaverTimeout = null
      console.log('[DVD] Signal restored - cancelled screensaver timer')
    }
    if (isBouncingLogoRunning()) {
      hideDvdScreensaver()
    }
  }
}

/**
 * Show the DVD screensaver overlay
 */
function showDvdScreensaver() {
  elements.dvdOverlay.classList.remove('hidden')
  startBouncingLogo()
  console.log('[DVD] Screensaver activated')
}

/**
 * Hide the DVD screensaver overlay
 */
function hideDvdScreensaver() {
  stopBouncingLogo()
  elements.dvdOverlay.classList.add('hidden')
  console.log('[DVD] Screensaver deactivated')
}

// =============================================================================
// Layout Management
// =============================================================================

function setLayout(mode) {
  state.layoutMode = mode
  state.settings.layoutMode = mode
  
  // Update layout button states
  elements.layoutDual.classList.toggle('active', mode === 'dual')
  elements.layoutSingle.classList.toggle('active', mode === 'single')
  
  switch (mode) {
    case 'dual':
      document.body.classList.remove('single-view')
      elements.leftFeed.classList.remove('hidden', 'single')
      elements.rightFeed.classList.remove('hidden', 'single')
      elements.centerDivider.classList.remove('hidden', 'overlay')
      elements.bottomLogo.classList.add('hidden')
      break
    case 'single':
      document.body.classList.add('single-view')
      elements.leftFeed.classList.remove('hidden')
      elements.leftFeed.classList.add('single')
      elements.rightFeed.classList.add('hidden')
      // Hide center divider and bottom logo in single view - video fills screen
      elements.centerDivider.classList.add('overlay')
      elements.bottomLogo.classList.add('hidden')
      break
  }
  
  saveSettings()
}

function setCenterGap(gap) {
  state.centerGap = gap
  state.settings.centerGap = gap
  elements.centerDivider.style.width = `${gap}px`
  elements.centerGapValue.textContent = `${gap}px`
  debouncedSaveSettings()
}

function setBorderWidth(width) {
  state.borderWidth = width
  state.settings.borderWidth = width
  document.documentElement.style.setProperty('--border-width', `${width}px`)
  elements.borderWidthValue.textContent = `${width}px`
  debouncedSaveSettings()
}

// =============================================================================
// Input Selection
// =============================================================================

async function selectInput(index, side = 'both') {
  // Filter to only enabled devices
  const enabledDevices = state.devices.filter(d => isInputEnabled(d.deviceId))
  const device = enabledDevices[index]
  if (!device) return
  
  if (side === 'left' || side === 'both') {
    state.leftDeviceId = device.deviceId
    await startVideoStream(device.deviceId, elements.leftVideo, 'left')
  }
  
  if (side === 'right' || side === 'both') {
    state.rightDeviceId = device.deviceId
    await startVideoStream(device.deviceId, elements.rightVideo, 'right')
  }
  
  const name = getInputName(device.deviceId, device.label || `Input ${index + 1}`)
  showInputName(name)
  saveSettings()
  renderInputList()
}

function showInputName(name) {
  elements.inputNameText.textContent = name
  elements.inputNameOverlay.classList.remove('hidden')
  
  // Remove after animation
  setTimeout(() => {
    elements.inputNameOverlay.classList.add('hidden')
  }, 2000)
}

// =============================================================================
// UI Rendering
// =============================================================================

function renderInputList() {
  elements.inputList.innerHTML = ''
  
  state.devices.forEach((device, index) => {
    const item = document.createElement('div')
    const isEnabled = isInputEnabled(device.deviceId)
    item.className = `input-item${isEnabled ? '' : ' disabled'}`
    
    const isLeftActive = device.deviceId === state.leftDeviceId
    const isRightActive = device.deviceId === state.rightDeviceId
    const customName = getInputName(device.deviceId, device.label || `Input ${index + 1}`)
    
    item.innerHTML = `
      <div class="input-number">${index + 1}</div>
      <div class="input-name-wrapper">
        <input type="text" class="input-name-field" value="${customName}" data-device-id="${device.deviceId}" />
      </div>
      <div class="input-actions">
        <div class="toggle-switch ${isEnabled ? 'active' : ''}" data-device-id="${device.deviceId}" title="Enable/Disable"></div>
        <button class="left-btn ${isLeftActive ? 'active' : ''}" data-index="${index}" ${!isEnabled ? 'disabled' : ''}>L</button>
        <button class="right-btn ${isRightActive ? 'active' : ''}" data-index="${index}" ${!isEnabled ? 'disabled' : ''}>R</button>
      </div>
    `
    
    // Add event handlers
    const nameField = item.querySelector('.input-name-field')
    nameField.addEventListener('change', (e) => {
      setInputName(device.deviceId, e.target.value)
    })
    nameField.addEventListener('keydown', (e) => {
      e.stopPropagation() // Prevent global keyboard shortcuts
      if (e.key === 'Enter') {
        e.target.blur()
      }
    })
    
    const toggleSwitch = item.querySelector('.toggle-switch')
    toggleSwitch.addEventListener('click', () => {
      toggleInputEnabled(device.deviceId)
    })
    
    const leftBtn = item.querySelector('.left-btn')
    leftBtn.addEventListener('click', () => {
      if (isEnabled) selectInput(index, 'left')
    })
    
    const rightBtn = item.querySelector('.right-btn')
    rightBtn.addEventListener('click', () => {
      if (isEnabled) selectInput(index, 'right')
    })
    
    elements.inputList.appendChild(item)
  })
}

// =============================================================================
// Cursor Management
// =============================================================================

function showCursor() {
  document.body.classList.add('cursor-visible')
  
  clearTimeout(state.cursorTimeout)
  state.cursorTimeout = setTimeout(() => {
    document.body.classList.remove('cursor-visible')
  }, state.cursorHideDelay)
}

// =============================================================================
// Keyboard Shortcuts
// =============================================================================

function handleKeyDown(event) {
  // Don't handle if typing in an input
  if (event.target.tagName === 'INPUT') return
  
  switch (event.key.toLowerCase()) {
    case ' ': // Space bar - Freeze frame
      event.preventDefault()
      toggleFreeze()
      break
    case '1':
    case '2':
    case '3':
    case '4':
      selectInput(parseInt(event.key) - 1)
      break
    case 'f':
    case 'f11':
      event.preventDefault()
      window.electronAPI.toggleFullscreen()
      break
    case 'escape':
      closeAllPanels()
      if (state.frozen) {
        toggleFreeze() // Unfreeze on escape
      }
      window.electronAPI.isFullscreen().then(isFs => {
        if (isFs) window.electronAPI.toggleFullscreen()
      })
      break
    case 'q':
      window.electronAPI.quitApp()
      break
  }
}

// =============================================================================
// Event Listeners
// =============================================================================

function setupEventListeners() {
  // Mouse movement shows cursor
  document.addEventListener('mousemove', showCursor)
  
  // Keyboard shortcuts
  document.addEventListener('keydown', handleKeyDown)
  
  // Keep cursor visible when hovering dropdown
  elements.dropdownTrigger.addEventListener('mouseenter', () => {
    document.body.classList.add('cursor-visible')
    clearTimeout(state.cursorTimeout)
  })
  
  elements.dropdownPanel.addEventListener('mouseenter', () => {
    document.body.classList.add('cursor-visible')
    clearTimeout(state.cursorTimeout)
  })
  
  elements.dropdownPanel.addEventListener('mouseleave', () => {
    showCursor() // Reset cursor timeout
  })
  
  // Layout mode buttons
  elements.layoutDual.addEventListener('click', () => setLayout('dual'))
  elements.layoutSingle.addEventListener('click', () => setLayout('single'))
  
  // Gap and border sliders
  elements.centerGap.addEventListener('input', (e) => {
    setCenterGap(parseInt(e.target.value))
  })
  
  elements.borderWidth.addEventListener('input', (e) => {
    setBorderWidth(parseInt(e.target.value))
  })
  
  // Setup wizard buttons
  elements.setupDetectionBtn.addEventListener('click', () => {
    console.log('[Setup] Setup button clicked')
    showSetupWizard()
  })
  
  elements.wizardCaptureBtn.addEventListener('click', () => {
    console.log('[Setup] Capture button clicked')
    captureNoSignalReference()
  })
  
  elements.wizardSkipBtn.addEventListener('click', () => {
    console.log('[Setup] Skip button clicked')
    hideSetupWizard()
  })
  
  // Device changes (when plugging/unplugging devices)
  navigator.mediaDevices.addEventListener('devicechange', async () => {
    console.log('Device change detected')
    await getVideoDevices()
  })
  
  // Auto-updater download progress
  if (window.electronAPI && window.electronAPI.onUpdaterProgress) {
    window.electronAPI.onUpdaterProgress((percent) => {
      console.log('Updater progress:', percent + '%')
      elements.updateMessage.textContent = `Downloading update... ${percent}%`
      elements.updateNotification.classList.remove('hidden')
      // Hide notification when download completes (dialog will show)
      if (percent >= 100) {
        setTimeout(() => {
          elements.updateNotification.classList.add('hidden')
        }, 1000)
      }
    })
  }
}

// =============================================================================
// No-Signal Detection
// =============================================================================

/**
 * Initialize the no-signal detection system
 */
async function initNoSignalDetection() {
  // Create offscreen canvas for frame capture
  state.detectionCanvas = document.createElement('canvas')
  
  // Load saved reference screenshots from settings
  if (state.settings.noSignalReferences) {
    await deserializeReferences(state.settings.noSignalReferences)
  }
  
  console.log('[Detection] No-signal detection initialized')
  startDetectionLoop()
}

/**
 * Start the detection loop using requestAnimationFrame
 * Detection runs once every 100 frames to avoid CPU overload
 */
function startDetectionLoop() {
  if (state.detectionRunning) return
  state.detectionRunning = true
  state.detectionFrameCount = 0
  
  function detectFrame() {
    // Stop the loop if detection is disabled
    if (!state.detectionRunning) {
      return
    }

    state.detectionFrameCount++

    // Only run detection every 100 frames (~1.6 seconds at 60fps)
    // Reset counter to avoid potential overflow after long runtime
    if (state.detectionFrameCount >= 100) {
      state.detectionFrameCount = 0
      if (state.frozen) {
        requestAnimationFrame(detectFrame)
        return
      }
      const devicesToCheck = getUniqueActiveDevices()
      
      for (const { deviceId, video, side } of devicesToCheck) {
        if (!video.srcObject || video.readyState < 2) continue
        
        // Only check if device has a reference screenshot
        if (!isDetectionReady(deviceId)) continue
        
        const isNoSignal = checkNoSignal(deviceId, video, state.detectionCanvas)
        
        // Update UI if state changed
        if (isNoSignal && !state.noSignalState[side]) {
          showNoSignal(side)
          console.log(`[Detection] No signal detected on ${side} (${deviceId})`)
        } else if (!isNoSignal && state.noSignalState[side]) {
          hideNoSignal(side)
          console.log(`[Detection] Signal restored on ${side} (${deviceId})`)
        }
        
        // If same device is on both sides, sync the state
        if (state.layoutMode === 'dual' && state.leftDeviceId === state.rightDeviceId) {
          const otherSide = side === 'left' ? 'right' : 'left'
          if (isNoSignal && !state.noSignalState[otherSide]) {
            showNoSignal(otherSide)
            console.log(`[Detection] No signal detected on ${otherSide} (synced from ${side})`)
          } else if (!isNoSignal && state.noSignalState[otherSide]) {
            hideNoSignal(otherSide)
            console.log(`[Detection] Signal restored on ${otherSide} (synced from ${side})`)
          }
        }
      }

      // Update DVD screensaver based on current no-signal state
      updateDvdScreensaver()
    }

    // Schedule next frame
    requestAnimationFrame(detectFrame)
  }
  
  requestAnimationFrame(detectFrame)
}

/**
 * Get unique active devices to check (avoid duplicate checks for same device)
 * @returns {Array<{deviceId: string, video: HTMLVideoElement, side: string}>}
 */
function getUniqueActiveDevices() {
  const devices = []
  const checkedIds = new Set()
  
  // In dual mode, check both feeds if they have different sources
  // In single mode, only check the visible feed
  
  if (state.layoutMode === 'dual') {
    // Left feed
    if (state.leftDeviceId && elements.leftVideo.srcObject) {
      devices.push({ 
        deviceId: state.leftDeviceId, 
        video: elements.leftVideo, 
        side: 'left' 
      })
      checkedIds.add(state.leftDeviceId)
    }
    
    // Right feed - only if different device
    if (state.rightDeviceId && elements.rightVideo.srcObject && !checkedIds.has(state.rightDeviceId)) {
      devices.push({ 
        deviceId: state.rightDeviceId, 
        video: elements.rightVideo, 
        side: 'right' 
      })
    } else if (state.rightDeviceId && checkedIds.has(state.rightDeviceId)) {
      // Same device on both feeds - copy state from left
      // This will be handled in the detection result propagation
    }
  } else {
    // Single mode - only check left feed (which shows the active source)
    if (state.leftDeviceId && elements.leftVideo.srcObject) {
      devices.push({ 
        deviceId: state.leftDeviceId, 
        video: elements.leftVideo, 
        side: 'left' 
      })
    }
  }
  
  return devices
}

/**
 * Stop the detection loop
 */
function stopDetectionLoop() {
  state.detectionRunning = false
}

// =============================================================================
// Setup Wizard
// =============================================================================

/**
 * Show the setup wizard
 */
function showSetupWizard() {
  elements.setupWizard.classList.remove('hidden')
  
  // Start preview update loop
  updateWizardPreview()
}

/**
 * Hide the setup wizard
 */
function hideSetupWizard() {
  elements.setupWizard.classList.add('hidden')
}

/**
 * Update wizard preview canvas with current video feed
 */
function updateWizardPreview() {
  // Early exit if wizard is closed (before any work)
  if (elements.setupWizard.classList.contains('hidden')) {
    return
  }

  const selectedInput = document.querySelector('input[name="wizard-input"]:checked').value
  const video = selectedInput === 'left' ? elements.leftVideo : elements.rightVideo

  if (video && video.srcObject && video.readyState >= 2) {
    const ctx = elements.wizardPreviewCanvas.getContext('2d')
    elements.wizardPreviewCanvas.width = video.videoWidth || 640
    elements.wizardPreviewCanvas.height = video.videoHeight || 480
    ctx.drawImage(video, 0, 0, elements.wizardPreviewCanvas.width, elements.wizardPreviewCanvas.height)
    elements.wizardPreviewStatus.textContent = 'Live preview - ready to capture'
    elements.wizardPreviewStatus.style.color = 'var(--text-muted)'
  } else {
    elements.wizardPreviewStatus.textContent = 'No video feed available'
    elements.wizardPreviewStatus.style.color = '#ff4444'
  }

  // Only schedule next frame if wizard is still open (avoids race condition)
  if (!elements.setupWizard.classList.contains('hidden')) {
    requestAnimationFrame(updateWizardPreview)
  }
}

/**
 * Capture no-signal screenshot from wizard
 */
async function captureNoSignalReference() {
  const selectedInput = document.querySelector('input[name="wizard-input"]:checked').value
  const video = selectedInput === 'left' ? elements.leftVideo : elements.rightVideo
  const deviceId = selectedInput === 'left' ? state.leftDeviceId : state.rightDeviceId
  
  if (!deviceId) {
    elements.wizardPreviewStatus.textContent = 'Error: No device selected'
    elements.wizardPreviewStatus.style.color = '#ff4444'
    return
  }
  
  if (!video || !video.srcObject || video.readyState < 2) {
    elements.wizardPreviewStatus.textContent = 'Error: Video feed not ready'
    elements.wizardPreviewStatus.style.color = '#ff4444'
    return
  }
  
  // Capture screenshot
  const canvas = document.createElement('canvas')
  const imageData = captureScreenshot(video, canvas)
  
  if (!imageData) {
    elements.wizardPreviewStatus.textContent = 'Error: Failed to capture screenshot'
    elements.wizardPreviewStatus.style.color = '#ff4444'
    return
  }
  
  // Save reference
  saveReferenceScreenshot(deviceId, imageData)
  
  // Mark initial setup as complete
  state.settings.initialSetupComplete = true
  
  // Save to settings
  state.settings.noSignalReferences = serializeReferences()
  await saveSettings()
  
  // Show success
  elements.wizardPreviewStatus.textContent = '✓ Captured successfully!'
  elements.wizardPreviewStatus.style.color = '#00ff00'
  
  // Close wizard after a delay
  setTimeout(() => {
    hideSetupWizard()
  }, 1500)
}

// =============================================================================
// Initialization
// =============================================================================

async function init() {
  console.log('Input Viewer initializing...')

  // Display app version from package.json
  if (window.electronAPI && window.electronAPI.getAppVersion) {
    try {
      const version = await window.electronAPI.getAppVersion()
      if (elements.appVersion) {
        elements.appVersion.textContent = `Input Viewer v${version}`
      }
    } catch (e) {
      console.error('Error getting app version:', e)
    }
  }

  // Load settings from file
  state.settings = await loadSettings()
  
  // Setup event listeners
  setupEventListeners()
  
  // Detect screen aspect ratio and set default layout
  // If aspect ratio >= 3:1 (super wide) → dual view
  // If aspect ratio < 3:1 (normal/square) → single view
  const screenAspectRatio = window.screen.width / window.screen.height
  console.log(`Screen width: ${window.screen.width}, height: ${window.screen.height}`)
  console.log(`Calculated screen aspect ratio: ${screenAspectRatio.toFixed(2)}`)
  const defaultLayout = screenAspectRatio >= 3 ? 'dual' : 'single'
  console.log(`Screen aspect ratio: ${screenAspectRatio.toFixed(2)} → default layout: ${defaultLayout}`)
  
  // Use saved layout mode if available, otherwise use screen-based default
  const layoutMode = state.settings.layoutMode || defaultLayout
  setLayout(layoutMode)
  
  // Initialize center gap and border width from settings
  const centerGap = state.settings.centerGap || 60
  setCenterGap(centerGap)
  elements.centerGap.value = centerGap
  
  const borderWidth = state.settings.borderWidth || 0
  setBorderWidth(borderWidth)
  elements.borderWidth.value = borderWidth
  
  // Get video devices and start streams
  await getVideoDevices()
  
  // Start video streams
  if (state.devices.length > 0) {
    // Always start left stream
    await startVideoStream(state.leftDeviceId, elements.leftVideo, 'left')
    
    // Start right stream in dual mode
    if (layoutMode === 'dual' && state.rightDeviceId) {
      await startVideoStream(state.rightDeviceId, elements.rightVideo, 'right')
    }
  }
  
  // Initialize DVD bouncing logo
  initBouncingLogo(elements.dvdLogo, elements.dvdOverlay)

  // Initialize no-signal detection (don't await - let it load in background)
  initNoSignalDetection().catch(err => {
    console.error('[Detection] Initialization error:', err)
  })
  
  // Auto-start setup wizard if this is first launch
  if (!state.settings.initialSetupComplete && state.devices.length > 0) {
    console.log('[Setup] First launch detected, showing setup wizard')
    setTimeout(() => showSetupWizard(), 1000) // Delay to let video feeds initialize
  }
  
  // Show cursor initially
  showCursor()
  
  console.log('Input Viewer ready')
}

// Start the app
init()
