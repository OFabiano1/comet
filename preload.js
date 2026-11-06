// Comet — preload bridge. The renderer talks to the main process only
// through this surfaced `window.comet` API, never through raw IPC.
const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('comet', {
  // Requests
  download: (url, kind, opts = {}) => ipcRenderer.invoke('comet:download', { url, kind, opts }),
  cancel: () => ipcRenderer.invoke('comet:cancel'),
  getVersions: () => ipcRenderer.invoke('comet:versions'),
  getFolder: () => ipcRenderer.invoke('comet:folder-get'),
  setFolder: () => ipcRenderer.invoke('comet:folder-set'),
  updateytDlp: () => ipcRenderer.invoke('comet:ytdlp-update'),
  // Event stream: {type:'progress'|'status'|'error'|'done', ...}
  onEvent: (cb) => {
    ipcRenderer.on('comet:event', (_e, payload) => cb(payload));
  },
});
