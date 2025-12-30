const { contextBridge, ipcRenderer } = require('electron');

// Expose protected methods to the renderer process
contextBridge.exposeInMainWorld('electronAPI', {
  // Window controls
  toggleFullscreen: () => ipcRenderer.invoke('toggle-fullscreen'),
  isFullscreen: () => ipcRenderer.invoke('is-fullscreen'),
  quitApp: () => ipcRenderer.invoke('quit-app'),
  
  // Updater messages
  onUpdaterMessage: (callback) => {
    ipcRenderer.on('updater-message', (event, message) => callback(message));
  }
});
