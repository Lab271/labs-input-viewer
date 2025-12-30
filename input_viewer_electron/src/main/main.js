const { app, BrowserWindow, ipcMain, systemPreferences } = require('electron');
const { autoUpdater } = require('electron-updater');
const path = require('path');

// Keep a global reference of the window object
let mainWindow;

// Check if running in development mode
const isDev = process.argv.includes('--dev');

// Configure auto-updater
autoUpdater.autoDownload = true;
autoUpdater.autoInstallOnAppQuit = true;

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
      preload: path.join(__dirname, 'preload.js')
    }
  });

  // Load the index.html
  mainWindow.loadFile(path.join(__dirname, '../renderer/index.html'));

  // Open DevTools in dev mode
  if (isDev) {
    mainWindow.webContents.openDevTools();
  }

  // Handle window closed
  mainWindow.on('closed', () => {
    mainWindow = null;
  });

  // Check for updates after window is ready (not in dev mode)
  mainWindow.once('ready-to-show', () => {
    if (!isDev) {
      autoUpdater.checkForUpdatesAndNotify();
    }
  });
}

// Request camera permissions on macOS
async function requestCameraPermission() {
  if (process.platform === 'darwin') {
    const status = systemPreferences.getMediaAccessStatus('camera');
    if (status !== 'granted') {
      const granted = await systemPreferences.askForMediaAccess('camera');
      return granted;
    }
    return true;
  }
  return true;
}

// App ready
app.whenReady().then(async () => {
  await requestCameraPermission();
  createWindow();

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createWindow();
    }
  });
});

// Quit when all windows are closed (except on macOS)
app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

// =============================================================================
// IPC Handlers
// =============================================================================

// Get available video input devices
ipcMain.handle('get-video-devices', async () => {
  // This will be handled in renderer via navigator.mediaDevices
  return [];
});

// Toggle fullscreen
ipcMain.handle('toggle-fullscreen', () => {
  if (mainWindow) {
    mainWindow.setFullScreen(!mainWindow.isFullScreen());
    return mainWindow.isFullScreen();
  }
  return false;
});

// Get fullscreen state
ipcMain.handle('is-fullscreen', () => {
  return mainWindow ? mainWindow.isFullScreen() : false;
});

// Quit app
ipcMain.handle('quit-app', () => {
  app.quit();
});

// =============================================================================
// Auto-Updater Events
// =============================================================================

autoUpdater.on('checking-for-update', () => {
  sendStatusToWindow('Checking for update...');
});

autoUpdater.on('update-available', (info) => {
  sendStatusToWindow('Update available: ' + info.version);
});

autoUpdater.on('update-not-available', (info) => {
  sendStatusToWindow('App is up to date');
});

autoUpdater.on('error', (err) => {
  sendStatusToWindow('Error in auto-updater: ' + err);
});

autoUpdater.on('download-progress', (progressObj) => {
  let message = `Download speed: ${progressObj.bytesPerSecond}`;
  message += ` - Downloaded ${progressObj.percent}%`;
  message += ` (${progressObj.transferred}/${progressObj.total})`;
  sendStatusToWindow(message);
});

autoUpdater.on('update-downloaded', (info) => {
  sendStatusToWindow('Update downloaded. Will install on quit.');
});

function sendStatusToWindow(text) {
  if (mainWindow) {
    mainWindow.webContents.send('updater-message', text);
  }
  console.log('[AutoUpdater]', text);
}
