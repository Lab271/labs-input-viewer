const { app, BrowserWindow, ipcMain, systemPreferences, dialog } = require('electron')
const { autoUpdater } = require('electron-updater')
const path = require('path')
const fs = require('fs')

// Keep a global reference of the window object
let mainWindow

// Check if running in development mode
// electron-vite sets ELECTRON_RENDERER_URL only during `dev` command
const isDev = !!process.env.ELECTRON_RENDERER_URL

// Configure auto-updater to use the public releases repository
autoUpdater.setFeedURL({
  provider: 'github',
  owner: 'LAB271',
  repo: 'input-viewer-releases'
})

// Don't auto-download - prompt user first
autoUpdater.autoDownload = false
autoUpdater.autoInstallOnAppQuit = true

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
    // Ensure userData directory exists
    const userDataPath = app.getPath('userData')
    if (!fs.existsSync(userDataPath)) {
      fs.mkdirSync(userDataPath, { recursive: true })
    }
    
    fs.writeFileSync(settingsPath, JSON.stringify(settings, null, 2), 'utf8')
    console.log('[Settings] Saved to:', settingsPath)
    return true
  } catch (err) {
    console.error('[Settings] Error saving settings:', err)
    console.error('[Settings] Settings path:', settingsPath)
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
  // Add a delay to ensure the app is fully loaded
  mainWindow.once('ready-to-show', () => {
    if (!isDev) {
      setTimeout(() => {
        log('[AutoUpdater] Starting update check...')
        autoUpdater.checkForUpdates()
      }, 3000)
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
  log('[AutoUpdater] Checking for update...')
})

autoUpdater.on('update-available', (info) => {
  log(`[AutoUpdater] Update available: ${info.version}`)
  dialog.showMessageBox(mainWindow, {
    type: 'info',
    title: 'Update Available',
    message: `A new version (${info.version}) is available.`,
    detail: 'Would you like to download it now?',
    buttons: ['Download', 'Later'],
    defaultId: 0,
    cancelId: 1
  }).then((result) => {
    if (result.response === 0) {
      autoUpdater.downloadUpdate()
    }
  })
})

autoUpdater.on('update-not-available', () => {
  log('[AutoUpdater] App is up to date')
})

autoUpdater.on('error', (err) => {
  log(`[AutoUpdater] Error: ${err}`)
})

autoUpdater.on('download-progress', (progressObj) => {
  const percent = Math.round(progressObj.percent)
  log(`[AutoUpdater] Download progress: ${percent}%`)
  
  // Send progress to renderer for display
  if (mainWindow) {
    mainWindow.webContents.send('updater-progress', percent)
  }
})

autoUpdater.on('update-downloaded', (info) => {
  log(`[AutoUpdater] Update downloaded: ${info.version}`)
  dialog.showMessageBox(mainWindow, {
    type: 'info',
    title: 'Update Ready',
    message: 'Update downloaded successfully.',
    detail: 'The application will restart to install the update.',
    buttons: ['Restart Now', 'Later'],
    defaultId: 0,
    cancelId: 1
  }).then((result) => {
    if (result.response === 0) {
      autoUpdater.quitAndInstall()
    }
  })
})

function log(text) {
  console.log(text)
}
