const { app, BrowserWindow, ipcMain, systemPreferences } = require('electron')
const { autoUpdater } = require('electron-updater')
const path = require('path')
const fs = require('fs')

// Keep a global reference of the window object
let mainWindow

// Check if running in development mode
const isDev = process.env.NODE_ENV === 'development'

// Settings file path
const settingsPath = path.join(app.getPath('userData'), 'settings.json')

// Default settings
const defaultSettings = {
  leftDeviceId: null,
  rightDeviceId: null,
  layoutMode: 'dual',
  layoutGap: 2,
  inputs: {} // { deviceId: { name: string, enabled: boolean } }
}

// Load settings from file
function loadSettings() {
  try {
    if (fs.existsSync(settingsPath)) {
      const data = fs.readFileSync(settingsPath, 'utf8')
      return { ...defaultSettings, ...JSON.parse(data) }
    }
  } catch (err) {
    console.error('Error loading settings:', err)
  }
  return { ...defaultSettings }
}

// Save settings to file
function saveSettings(settings) {
  try {
    fs.writeFileSync(settingsPath, JSON.stringify(settings, null, 2), 'utf8')
    return true
  } catch (err) {
    console.error('Error saving settings:', err)
    return false
  }
}

// Configure auto-updater
autoUpdater.autoDownload = true
autoUpdater.autoInstallOnAppQuit = true

function createWindow() {
  // Create the browser window
  mainWindow = new BrowserWindow({
    width: 1920,
    height: 1080,
    minWidth: 800,
    minHeight: 600,
    backgroundColor: '#000000',
    titleBarStyle: 'hiddenInset',
    frame: process.platform === 'darwin' ? true : false,
    fullscreen: true,
    webPreferences: {
      nodeIntegration: false,
      contextIsolation: true,
      preload: path.join(__dirname, '../preload/index.js')
    }
  })

  // Load the index.html
  if (isDev) {
    mainWindow.loadURL('http://localhost:5173')
    mainWindow.webContents.openDevTools()
  } else {
    mainWindow.loadFile(path.join(__dirname, '../renderer/index.html'))
  }

  // Handle window closed
  mainWindow.on('closed', () => {
    mainWindow = null
  })

  // Check for updates after window is ready (not in dev mode)
  mainWindow.once('ready-to-show', () => {
    if (!isDev) {
      autoUpdater.checkForUpdatesAndNotify()
    }
  })
}

// Request camera permissions on macOS
async function requestCameraPermission() {
  if (process.platform === 'darwin') {
    const status = systemPreferences.getMediaAccessStatus('camera')
    if (status !== 'granted') {
      const granted = await systemPreferences.askForMediaAccess('camera')
      return granted
    }
    return true
  }
  return true
}

// App ready
app.whenReady().then(async () => {
  await requestCameraPermission()
  createWindow()

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createWindow()
    }
  })
})

// Quit when all windows are closed (except on macOS)
app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit()
  }
})

// =============================================================================
// IPC Handlers
// =============================================================================

// Toggle fullscreen
ipcMain.handle('toggle-fullscreen', () => {
  if (mainWindow) {
    mainWindow.setFullScreen(!mainWindow.isFullScreen())
    return mainWindow.isFullScreen()
  }
  return false
})

// Get fullscreen state
ipcMain.handle('is-fullscreen', () => {
  return mainWindow ? mainWindow.isFullScreen() : false
})

// Quit app
ipcMain.handle('quit-app', () => {
  app.quit()
})

// Load settings from file
ipcMain.handle('load-settings', () => {
  return loadSettings()
})

// Save settings to file
ipcMain.handle('save-settings', (event, settings) => {
  return saveSettings(settings)
})

// Get settings file path (for debugging)
ipcMain.handle('get-settings-path', () => {
  return settingsPath
})

// =============================================================================
// Auto-Updater Events
// =============================================================================

autoUpdater.on('checking-for-update', () => {
  sendStatusToWindow('Checking for update...')
})

autoUpdater.on('update-available', (info) => {
  sendStatusToWindow('Update available: ' + info.version)
})

autoUpdater.on('update-not-available', () => {
  sendStatusToWindow('App is up to date')
})

autoUpdater.on('error', (err) => {
  sendStatusToWindow('Error in auto-updater: ' + err)
})

autoUpdater.on('download-progress', (progressObj) => {
  let message = `Download speed: ${progressObj.bytesPerSecond}`
  message += ` - Downloaded ${progressObj.percent}%`
  message += ` (${progressObj.transferred}/${progressObj.total})`
  sendStatusToWindow(message)
})

autoUpdater.on('update-downloaded', () => {
  sendStatusToWindow('Update downloaded. Will install on quit.')
})

function sendStatusToWindow(text) {
  if (mainWindow) {
    mainWindow.webContents.send('updater-message', text)
  }
  console.log('[AutoUpdater]', text)
}
