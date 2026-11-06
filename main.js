// Comet — Electron main process.
// Owns child processes, config, and all IPC. Logic lives in lib/ modules.
const { app, BrowserWindow, ipcMain } = require('electron');
const path = require('path');

// Tracked child processes (downloads, self-updates). Task 6 wires the
// runner into this set; Task 9 wires yt-dlp self-update.
const children = new Set();

function killAllTracked() {
  for (const child of children) {
    try { child.kill(); } catch { /* already dead */ }
  }
  children.clear();
}

function createWindow() {
  const win = new BrowserWindow({
    width: 560,
    height: 560,
    minWidth: 480,
    minHeight: 520,
    title: 'Comet',
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false,
    },
  });

  win.loadFile(path.join(__dirname, 'renderer', 'index.html'));
  return win;
}

app.whenReady().then(() => {
  createWindow();

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow();
  });
});

app.on('window-all-closed', () => {
  killAllTracked();
  app.quit();
});

// --- IPC (Task 6/9 fill in the real handlers) ---
ipcMain.handle('comet:versions', () => ({
  app: app.getVersion(),
  electron: process.versions.electron,
}));
ipcMain.handle('comet:download', () => ({ ok: false, error: 'Nem tudo instalado ainda.' }));
ipcMain.handle('comet:cancel', () => ({ ok: true }));
ipcMain.handle('comet:folder-get', () => ({ ok: false, error: 'Nem tudo instalado ainda.' }));
ipcMain.handle('comet:folder-set', () => ({ ok: false, error: 'Nem tudo instalado ainda.' }));
ipcMain.handle('comet:ytdlp-update', () => ({ ok: false, error: 'Nem tudo instalado ainda.' }));
