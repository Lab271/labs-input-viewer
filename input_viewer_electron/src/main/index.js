const { app, BrowserWindow, ipcMain, systemPreferences, dialog } = require('electron')
const path = require('path')
const fs = require('fs')
const { exec } = require('child_process')

// Hardware acceleration for video decode/rendering
app.commandLine.appendSwitch('ignore-gpu-blocklist')
app.commandLine.appendSwitch('enable-gpu-rasterization')

// Keep a global reference of the window object
let mainWindow

// Check if running in development mode
// electron-vite sets ELECTRON_RENDERER_URL only during `dev` command
const isDev = !!process.env.ELECTRON_RENDERER_URL

// Auto-updater (lazy-loaded to avoid crash in dev mode)
let autoUpdater = null

function getAutoUpdater() {
  if (!autoUpdater && !isDev) {
    autoUpdater = require('electron-updater').autoUpdater

    // Configure auto-updater to use this repository
    autoUpdater.setFeedURL({
      provider: 'github',
      owner: 'LAB271',
      repo: 'labs-input-viewer'
    })

    // Don't auto-download - prompt user first
    autoUpdater.autoDownload = false
    autoUpdater.autoInstallOnAppQuit = true

    // Setup auto-updater events
    setupAutoUpdaterEvents()
  }
  return autoUpdater
}

// Get app version for display in renderer
function getAppVersion() {
  return app.getVersion()
}

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
        const updater = getAutoUpdater()
        if (updater) {
          updater.checkForUpdates()
        }
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

// Get app version for display in UI
ipcMain.handle('get-app-version', () => {
  return getAppVersion()
})

// Get system volume (0-100)
ipcMain.handle('get-system-volume', async () => {
  return new Promise((resolve) => {
    if (process.platform === 'darwin') {
      // macOS: Use AppleScript to get volume
      exec('osascript -e "output volume of (get volume settings)"', (error, stdout) => {
        if (error) {
          console.error('[Volume] Error getting system volume:', error)
          resolve(50) // Default fallback
        } else {
          resolve(parseInt(stdout.trim(), 10) || 50)
        }
      })
    } else if (process.platform === 'win32') {
      // Windows: Use PowerShell with Windows Audio API
      const psScript = `
        Add-Type -TypeDefinition @"
using System.Runtime.InteropServices;
[Guid("5CDF2C82-841E-4546-9722-0CF74078229A"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IAudioEndpointVolume {
  int _0(); int _1(); int _2(); int _3();
  int SetMasterVolumeLevelScalar(float fLevel, System.Guid pguidEventContext);
  int _5();
  int GetMasterVolumeLevelScalar(out float pfLevel);
  int SetMute([MarshalAs(UnmanagedType.Bool)] bool bMute, System.Guid pguidEventContext);
  int GetMute(out bool pbMute);
}
[Guid("D666063F-1587-4E43-81F1-B948E807363F"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IMMDevice { int Activate(ref System.Guid id, int clsCtx, int activationParams, out IAudioEndpointVolume aev); }
[Guid("A95664D2-9614-4F35-A746-DE8DB63617E6"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IMMDeviceEnumerator { int GetDefaultAudioEndpoint(int dataFlow, int role, out IMMDevice endpoint); }
[ComImport, Guid("BCDE0395-E52F-467C-8E3D-C4579291692E")] class MMDeviceEnumeratorComObject { }
public class Audio {
  static IAudioEndpointVolume Vol() {
    var enumerator = new MMDeviceEnumeratorComObject() as IMMDeviceEnumerator;
    IMMDevice dev; enumerator.GetDefaultAudioEndpoint(0, 1, out dev);
    IAudioEndpointVolume epv; var epvid = typeof(IAudioEndpointVolume).GUID;
    dev.Activate(ref epvid, 23, 0, out epv); return epv;
  }
  public static float Volume { get { float v; Vol().GetMasterVolumeLevelScalar(out v); return v; } set { Vol().SetMasterVolumeLevelScalar(value, System.Guid.Empty); } }
  public static bool Mute { get { bool m; Vol().GetMute(out m); return m; } set { Vol().SetMute(value, System.Guid.Empty); } }
}
"@
[Math]::Round([Audio]::Volume * 100)
      `.replace(/\n/g, ' ')
      exec(`powershell -command "${psScript}"`, { timeout: 5000 }, (error, stdout) => {
        if (error) {
          resolve(50)
        } else {
          const vol = parseInt(stdout.trim(), 10)
          resolve(isNaN(vol) ? 50 : vol)
        }
      })
    } else {
      resolve(50) // Default for unsupported platforms
    }
  })
})

// Set system volume (0-100)
ipcMain.handle('set-system-volume', async (event, volume) => {
  const clampedVolume = Math.max(0, Math.min(100, Math.round(volume)))

  return new Promise((resolve) => {
    if (process.platform === 'darwin') {
      // macOS: Use AppleScript to set volume
      exec(`osascript -e "set volume output volume ${clampedVolume}"`, (error) => {
        if (error) {
          console.error('[Volume] Error setting system volume:', error)
          resolve(false)
        } else {
          resolve(true)
        }
      })
    } else if (process.platform === 'win32') {
      // Windows: Use PowerShell with Windows Audio API
      const volumeFloat = clampedVolume / 100
      const psScript = `
        Add-Type -TypeDefinition @"
using System.Runtime.InteropServices;
[Guid("5CDF2C82-841E-4546-9722-0CF74078229A"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IAudioEndpointVolume {
  int _0(); int _1(); int _2(); int _3();
  int SetMasterVolumeLevelScalar(float fLevel, System.Guid pguidEventContext);
  int _5();
  int GetMasterVolumeLevelScalar(out float pfLevel);
  int SetMute([MarshalAs(UnmanagedType.Bool)] bool bMute, System.Guid pguidEventContext);
  int GetMute(out bool pbMute);
}
[Guid("D666063F-1587-4E43-81F1-B948E807363F"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IMMDevice { int Activate(ref System.Guid id, int clsCtx, int activationParams, out IAudioEndpointVolume aev); }
[Guid("A95664D2-9614-4F35-A746-DE8DB63617E6"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IMMDeviceEnumerator { int GetDefaultAudioEndpoint(int dataFlow, int role, out IMMDevice endpoint); }
[ComImport, Guid("BCDE0395-E52F-467C-8E3D-C4579291692E")] class MMDeviceEnumeratorComObject { }
public class Audio {
  static IAudioEndpointVolume Vol() {
    var enumerator = new MMDeviceEnumeratorComObject() as IMMDeviceEnumerator;
    IMMDevice dev; enumerator.GetDefaultAudioEndpoint(0, 1, out dev);
    IAudioEndpointVolume epv; var epvid = typeof(IAudioEndpointVolume).GUID;
    dev.Activate(ref epvid, 23, 0, out epv); return epv;
  }
  public static float Volume { get { float v; Vol().GetMasterVolumeLevelScalar(out v); return v; } set { Vol().SetMasterVolumeLevelScalar(value, System.Guid.Empty); } }
}
"@
[Audio]::Volume = ${volumeFloat}
      `.replace(/\n/g, ' ')
      exec(`powershell -command "${psScript}"`, { timeout: 5000 }, (error) => {
        resolve(!error)
      })
    } else {
      resolve(false)
    }
  })
})

// =============================================================================
// Auto-Updater Events
// =============================================================================

function setupAutoUpdaterEvents() {
  if (!autoUpdater) return

  autoUpdater.on('checking-for-update', () => {
    log('[AutoUpdater] Checking for update...')
  })

  autoUpdater.on('update-available', (info) => {
    log(`[AutoUpdater] Update available: ${info.version}`)

    // Ensure window is visible and focused for the dialog
    if (mainWindow) {
      if (mainWindow.isMinimized()) mainWindow.restore()
      mainWindow.focus()
    }

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
    }).catch((err) => {
      log(`[AutoUpdater] Dialog error: ${err}`)
    })
  })

  autoUpdater.on('update-not-available', () => {
    log('[AutoUpdater] App is up to date')
  })

  autoUpdater.on('error', (err) => {
    log(`[AutoUpdater] Error: ${err}`)

    // Show error to user
    if (mainWindow) {
      dialog.showMessageBox(mainWindow, {
        type: 'error',
        title: 'Update Error',
        message: 'Failed to download update',
        detail: err.message || String(err),
        buttons: ['OK']
      }).catch(() => {})
    }
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

    // Ensure window is visible and focused for the dialog
    if (mainWindow) {
      if (mainWindow.isMinimized()) mainWindow.restore()
      mainWindow.focus()
    }

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
    }).catch((err) => {
      log(`[AutoUpdater] Dialog error: ${err}`)
      // If dialog fails, install on quit anyway
    })
  })
}

function log(text) {
  console.log(text)
}
