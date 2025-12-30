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
  settings: loadSettings()
};

// =============================================================================
// DOM Elements
// =============================================================================

const elements = {
  leftFeed: document.getElementById('left-feed'),
  rightFeed: document.getElementById('right-feed'),
  leftVideo: document.getElementById('left-video'),
  rightVideo: document.getElementById('right-video'),
  videoWrapper: document.getElementById('video-wrapper'),
  inputNameOverlay: document.getElementById('input-name-overlay'),
  inputNameText: document.getElementById('input-name-text'),
  infoIcon: document.getElementById('info-icon'),
  infoPanel: document.getElementById('info-panel'),
  settingsIcon: document.getElementById('settings-icon'),
  settingsPanel: document.getElementById('settings-panel'),
  inputList: document.getElementById('input-list'),
  layoutGap: document.getElementById('layout-gap'),
  layoutGapValue: document.getElementById('layout-gap-value'),
  updateNotification: document.getElementById('update-notification'),
  updateMessage: document.getElementById('update-message')
};

// =============================================================================
// Settings Persistence
// =============================================================================

function loadSettings() {
  try {
    const saved = localStorage.getItem('inputViewerSettings');
    return saved ? JSON.parse(saved) : getDefaultSettings();
  } catch (e) {
    return getDefaultSettings();
  }
}

function saveSettings() {
  localStorage.setItem('inputViewerSettings', JSON.stringify(state.settings));
}

function getDefaultSettings() {
  return {
    inputs: [],
    layoutGap: 20,
    leftDeviceId: null,
    rightDeviceId: null
  };
}

// =============================================================================
// Video Device Management
// =============================================================================

async function getVideoDevices() {
  try {
    // Request permission first
    await navigator.mediaDevices.getUserMedia({ video: true });
    
    const devices = await navigator.mediaDevices.enumerateDevices();
    state.devices = devices.filter(device => device.kind === 'videoinput');
    
    console.log('Available video devices:', state.devices);
    
    // Update settings with device info
    state.settings.inputs = state.devices.map((device, index) => ({
      id: device.deviceId,
      name: device.label || `Input ${index + 1}`,
      enabled: true
    }));
    
    // Set default devices - both left and right use first device (index 0)
    if (state.devices.length > 0) {
      state.leftDeviceId = state.devices[0].deviceId;
      state.rightDeviceId = state.devices[0].deviceId;
    }
    
    renderInputList();
    return state.devices;
  } catch (error) {
    console.error('Error getting video devices:', error);
    showNoSignal('left');
    showNoSignal('right');
    return [];
  }
}

async function startVideoStream(deviceId, videoElement, side) {
  try {
    // Stop existing stream
    if (side === 'left' && state.leftStream) {
      state.leftStream.getTracks().forEach(track => track.stop());
    }
    if (side === 'right' && state.rightStream) {
      state.rightStream.getTracks().forEach(track => track.stop());
    }
    
    if (!deviceId) {
      showNoSignal(side);
      return null;
    }
    
    const constraints = {
      video: {
        deviceId: { exact: deviceId },
        width: { ideal: 1920 },
        height: { ideal: 1080 },
        frameRate: { ideal: 30 }
      }
    };
    
    const stream = await navigator.mediaDevices.getUserMedia(constraints);
    videoElement.srcObject = stream;
    
    // Store stream reference
    if (side === 'left') {
      state.leftStream = stream;
    } else {
      state.rightStream = stream;
    }
    
    hideNoSignal(side);
    
    // Update input label
    const device = state.devices.find(d => d.deviceId === deviceId);
    const label = videoElement.parentElement.querySelector('.input-label');
    if (label && device) {
      label.textContent = device.label || 'Unknown Input';
    }
    
    return stream;
  } catch (error) {
    console.error(`Error starting ${side} stream:`, error);
    showNoSignal(side);
    return null;
  }
}

function showNoSignal(side) {
  const feed = side === 'left' ? elements.leftFeed : elements.rightFeed;
  const overlay = feed.querySelector('.no-signal-overlay');
  overlay.classList.remove('hidden');
}

function hideNoSignal(side) {
  const feed = side === 'left' ? elements.leftFeed : elements.rightFeed;
  const overlay = feed.querySelector('.no-signal-overlay');
  overlay.classList.add('hidden');
}

// =============================================================================
// Layout Management
// =============================================================================

function setLayout(mode) {
  state.layoutMode = mode;
  
  switch (mode) {
    case 'dual':
      elements.leftFeed.classList.remove('hidden', 'single');
      elements.rightFeed.classList.remove('hidden', 'single');
      break;
    case 'left':
      elements.leftFeed.classList.remove('hidden');
      elements.leftFeed.classList.add('single');
      elements.rightFeed.classList.add('hidden');
      break;
    case 'right':
      elements.rightFeed.classList.add('hidden');
      elements.leftFeed.classList.remove('hidden');
      elements.leftFeed.classList.add('single');
      // Swap streams for right-only mode
      break;
  }
}

function setLayoutGap(gap) {
  state.layoutGap = gap;
  state.settings.layoutGap = gap;
  elements.videoWrapper.style.setProperty('--layout-gap', `${gap}px`);
  elements.layoutGapValue.textContent = `${gap}px`;
  saveSettings();
}

// =============================================================================
// Input Selection
// =============================================================================

async function selectInput(index, side = 'both') {
  const device = state.devices[index];
  if (!device) return;
  
  if (side === 'left' || side === 'both') {
    state.leftDeviceId = device.deviceId;
    await startVideoStream(device.deviceId, elements.leftVideo, 'left');
  }
  
  if (side === 'right' || side === 'both') {
    state.rightDeviceId = device.deviceId;
    await startVideoStream(device.deviceId, elements.rightVideo, 'right');
  }
  
  showInputName(device.label || `Input ${index + 1}`);
  saveSettings();
}

function showInputName(name) {
  elements.inputNameText.textContent = name;
  elements.inputNameOverlay.classList.remove('hidden');
  
  // Remove after animation
  setTimeout(() => {
    elements.inputNameOverlay.classList.add('hidden');
  }, 2000);
}

// =============================================================================
// UI Rendering
// =============================================================================

function renderInputList() {
  elements.inputList.innerHTML = '';
  
  state.devices.forEach((device, index) => {
    const item = document.createElement('div');
    item.className = 'input-item';
    
    const isLeftActive = device.deviceId === state.leftDeviceId;
    const isRightActive = device.deviceId === state.rightDeviceId;
    
    item.innerHTML = `
      <div class="input-number">${index + 1}</div>
      <div class="input-name">${device.label || `Input ${index + 1}`}</div>
      <div class="input-actions">
        <button class="left-btn ${isLeftActive ? 'active' : ''}" data-index="${index}">L</button>
        <button class="right-btn ${isRightActive ? 'active' : ''}" data-index="${index}">R</button>
      </div>
    `;
    
    // Add click handlers
    item.querySelector('.left-btn').addEventListener('click', () => selectInput(index, 'left'));
    item.querySelector('.right-btn').addEventListener('click', () => selectInput(index, 'right'));
    
    elements.inputList.appendChild(item);
  });
}

// =============================================================================
// Panel Toggles
// =============================================================================

function togglePanel(panel) {
  panel.classList.toggle('hidden');
}

function closeAllPanels() {
  elements.infoPanel.classList.add('hidden');
  elements.settingsPanel.classList.add('hidden');
}

// =============================================================================
// Cursor Management
// =============================================================================

function showCursor() {
  document.body.classList.add('cursor-visible');
  
  clearTimeout(state.cursorTimeout);
  state.cursorTimeout = setTimeout(() => {
    document.body.classList.remove('cursor-visible');
  }, state.cursorHideDelay);
}

// =============================================================================
// Keyboard Shortcuts
// =============================================================================

function handleKeyDown(event) {
  // Don't handle if typing in an input
  if (event.target.tagName === 'INPUT') return;
  
  switch (event.key.toLowerCase()) {
    case 'd':
      setLayout('dual');
      showInputName('Dual View');
      break;
    case 'l':
      setLayout('left');
      showInputName('Left View');
      break;
    case 'r':
      setLayout('right');
      showInputName('Right View');
      break;
    case '1':
    case '2':
    case '3':
    case '4':
      selectInput(parseInt(event.key) - 1);
      break;
    case 'f':
    case 'f11':
      event.preventDefault();
      window.electronAPI.toggleFullscreen();
      break;
    case 'escape':
      closeAllPanels();
      window.electronAPI.isFullscreen().then(isFs => {
        if (isFs) window.electronAPI.toggleFullscreen();
      });
      break;
    case 'q':
      window.electronAPI.quitApp();
      break;
  }
}

// =============================================================================
// Event Listeners
// =============================================================================

function setupEventListeners() {
  // Mouse movement shows cursor
  document.addEventListener('mousemove', showCursor);
  
  // Keyboard shortcuts
  document.addEventListener('keydown', handleKeyDown);
  
  // Panel toggles
  elements.infoIcon.addEventListener('click', (e) => {
    e.stopPropagation();
    elements.settingsPanel.classList.add('hidden');
    togglePanel(elements.infoPanel);
  });
  
  elements.settingsIcon.addEventListener('click', (e) => {
    e.stopPropagation();
    elements.infoPanel.classList.add('hidden');
    togglePanel(elements.settingsPanel);
  });
  
  // Close panels when clicking outside
  document.addEventListener('click', (e) => {
    if (!e.target.closest('.panel') && !e.target.closest('.icon-button')) {
      closeAllPanels();
    }
  });
  
  // Layout gap slider
  elements.layoutGap.addEventListener('input', (e) => {
    setLayoutGap(parseInt(e.target.value));
  });
  
  // Device changes (when plugging/unplugging devices)
  navigator.mediaDevices.addEventListener('devicechange', async () => {
    console.log('Device change detected');
    await getVideoDevices();
  });
  
  // Auto-updater messages
  if (window.electronAPI) {
    window.electronAPI.onUpdaterMessage((message) => {
      console.log('Updater:', message);
      if (message.includes('Update available') || message.includes('Update downloaded')) {
        elements.updateMessage.textContent = message;
        elements.updateNotification.classList.remove('hidden');
        setTimeout(() => {
          elements.updateNotification.classList.add('hidden');
        }, 5000);
      }
    });
  }
}

// =============================================================================
// Initialization
// =============================================================================

async function init() {
  console.log('Input Viewer initializing...');
  
  // Setup event listeners
  setupEventListeners();
  
  // Initialize layout gap
  setLayoutGap(state.settings.layoutGap || 20);
  elements.layoutGap.value = state.layoutGap;
  
  // Get video devices and start streams
  await getVideoDevices();
  
  // Start video streams
  if (state.devices.length > 0) {
    await startVideoStream(state.leftDeviceId, elements.leftVideo, 'left');
    if (state.devices.length > 1) {
      await startVideoStream(state.rightDeviceId, elements.rightVideo, 'right');
    }
  }
  
  // Show cursor initially
  showCursor();
  
  console.log('Input Viewer ready');
}

// Start the app
init();
