/**
 * Input Viewer - Renderer Process
 * 
 * Handles video capture, UI interactions, and keyboard shortcuts
 */

// =============================================================================
// State Management
// =============================================================================

const state = {
  devices: [],
  leftDeviceId: null,
  rightDeviceId: null,
  leftStream: null,
  rightStream: null,
  layoutMode: 'dual', // 'dual', 'left', 'right'
  cursorTimeout: null,
  cursorHideDelay: 3000,
  layoutGap: 20,
  frozen: false,
  settings: null // Will be loaded from file
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
  inputNameOverlay: document.getElementById('input-name-overlay'),
  inputNameText: document.getElementById('input-name-text'),
  freezeOverlay: document.getElementById('freeze-overlay'),
  freezeIndicator: document.getElementById('freeze-indicator'),
  freezeCanvas: document.getElementById('freeze-canvas'),
  infoIcon: document.getElementById('info-icon'),
  infoPanel: document.getElementById('info-panel'),
  settingsIcon: document.getElementById('settings-icon'),
  settingsPanel: document.getElementById('settings-panel'),
  inputList: document.getElementById('input-list'),
  layoutDual: document.getElementById('layout-dual'),
  layoutSingle: document.getElementById('layout-single'),
  layoutGap: document.getElementById('layout-gap'),
  layoutGapValue: document.getElementById('layout-gap-value'),
  updateNotification: document.getElementById('update-notification'),
  updateMessage: document.getElementById('update-message')
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
        layoutGap: state.layoutGap,
        inputs: state.settings.inputs
      }
      await window.electronAPI.saveSettings(settingsToSave)
    }
  } catch (e) {
    console.error('Error saving settings:', e)
  }
}

function getDefaultSettings() {
  return {
    inputs: {},
    layoutGap: 20,
    leftDeviceId: null,
    rightDeviceId: null,
    layoutMode: null // null means use screen-based detection
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
    
    // Set default devices from settings or first device
    if (state.devices.length > 0) {
      // Try to restore from settings
      const savedLeft = state.devices.find(d => d.deviceId === state.settings.leftDeviceId)
      const savedRight = state.devices.find(d => d.deviceId === state.settings.rightDeviceId)
      
      state.leftDeviceId = savedLeft ? savedLeft.deviceId : state.devices[0].deviceId
      state.rightDeviceId = savedRight ? savedRight.deviceId : state.devices[0].deviceId
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
    
    const constraints = {
      video: {
        deviceId: { exact: deviceId },
        width: { ideal: 1920 },
        height: { ideal: 1080 },
        frameRate: { ideal: 30 }
      }
    }
    
    const stream = await navigator.mediaDevices.getUserMedia(constraints)
    videoElement.srcObject = stream
    
    // Store stream reference
    if (side === 'left') {
      state.leftStream = stream
    } else {
      state.rightStream = stream
    }
    
    hideNoSignal(side)
    
    // Update input label
    const device = state.devices.find(d => d.deviceId === deviceId)
    const label = videoElement.parentElement.querySelector('.input-label')
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
}

function hideNoSignal(side) {
  const feed = side === 'left' ? elements.leftFeed : elements.rightFeed
  const overlay = feed.querySelector('.no-signal-overlay')
  overlay.classList.add('hidden')
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
      elements.leftFeed.classList.remove('hidden', 'single')
      elements.rightFeed.classList.remove('hidden', 'single')
      elements.centerDivider.classList.remove('hidden')
      break
    case 'single':
      elements.leftFeed.classList.remove('hidden')
      elements.leftFeed.classList.add('single')
      elements.rightFeed.classList.add('hidden')
      // Keep logo visible in single view
      elements.centerDivider.classList.remove('hidden')
      break
  }
  
  saveSettings()
}

function setLayoutGap(gap) {
  state.layoutGap = gap
  state.settings.layoutGap = gap
  elements.videoWrapper.style.setProperty('--layout-gap', `${gap}px`)
  elements.layoutGapValue.textContent = `${gap}px`
  saveSettings()
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
// Panel Toggles
// =============================================================================

function togglePanel(panel) {
  panel.classList.toggle('hidden')
}

function closeAllPanels() {
  elements.infoPanel.classList.add('hidden')
  elements.settingsPanel.classList.add('hidden')
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
  
  // Panel toggles
  elements.infoIcon.addEventListener('click', (e) => {
    e.stopPropagation()
    elements.settingsPanel.classList.add('hidden')
    togglePanel(elements.infoPanel)
  })
  
  elements.settingsIcon.addEventListener('click', (e) => {
    e.stopPropagation()
    elements.infoPanel.classList.add('hidden')
    togglePanel(elements.settingsPanel)
  })
  
  // Close panels when clicking outside
  document.addEventListener('click', (e) => {
    if (!e.target.closest('.panel') && !e.target.closest('.icon-button')) {
      closeAllPanels()
    }
  })
  
  // Layout gap slider
  elements.layoutGap.addEventListener('input', (e) => {
    setLayoutGap(parseInt(e.target.value))
  })
  
  // Layout mode buttons
  elements.layoutDual.addEventListener('click', () => setLayout('dual'))
  elements.layoutSingle.addEventListener('click', () => setLayout('single'))
  
  // Device changes (when plugging/unplugging devices)
  navigator.mediaDevices.addEventListener('devicechange', async () => {
    console.log('Device change detected')
    await getVideoDevices()
  })
  
  // Auto-updater messages
  if (window.electronAPI) {
    window.electronAPI.onUpdaterMessage((message) => {
      console.log('Updater:', message)
      if (message.includes('Update available') || message.includes('Update downloaded')) {
        elements.updateMessage.textContent = message
        elements.updateNotification.classList.remove('hidden')
        setTimeout(() => {
          elements.updateNotification.classList.add('hidden')
        }, 5000)
      }
    })
  }
}

// =============================================================================
// Initialization
// =============================================================================

async function init() {
  console.log('Input Viewer initializing...')
  
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
  
  // Initialize layout gap
  setLayoutGap(state.settings.layoutGap || 20)
  elements.layoutGap.value = state.layoutGap
  
  // Get video devices and start streams
  await getVideoDevices()
  
  // Start video streams
  if (state.devices.length > 0) {
    await startVideoStream(state.leftDeviceId, elements.leftVideo, 'left')
    if (state.devices.length > 1) {
      await startVideoStream(state.rightDeviceId, elements.rightVideo, 'right')
    }
  }
  
  // Show cursor initially
  showCursor()
  
  console.log('Input Viewer ready')
}

// Start the app
init()
