// Comet — Electron main process.
// Owns child processes, config, and all IPC. Logic lives in lib/ modules.
const { app, BrowserWindow, ipcMain } = require('electron');
const path = require('path');
const fs = require('node:fs/promises');

const { classifyLink } = require('./lib/linkclassify');
const { sanitizeName, dedupeName } = require('./lib/naming');
const { buildYtDlpArgs } = require('./lib/ytdlp-args');
const { parseYtDlpError } = require('./lib/progress-parser');
const { extractMediaUrl } = require('./lib/gif-extract');
const runner = require('./lib/runner');

const VENDOR = path.join(__dirname, 'vendor');
const YT_DLP = path.join(VENDOR, 'yt-dlp.exe');

// Tracked child processes live in lib/runner.js.
const killAllTracked = runner.killAll;

function createWindow() {
  win = new BrowserWindow({
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

// --- IPC ---
let win = null;
function sendEvent(payload) {
  if (win && !win.isDestroyed()) win.webContents.send('comet:event', payload);
}

// Task 9 replaces this with the persisted config; Downloads until then.
async function getOutDir() {
  return app.getPath('downloads');
}

const GIF_KINDS = new Set(['tenor', 'giphy', 'direct-image']);

async function saveGifFile(data, outDir, url, fileExt) {
  const { ext, name } = extractNameCandidate(url, fileExt);
  const deduped = await dedupeName(outDir, name, ext);
  const target = path.join(outDir, deduped);
  await fs.writeFile(target, Buffer.from(data));
  return target;
}

function extractNameCandidate(url, fileExt) {
  let name = 'comet';
  try {
    const parsed = new URL(url);
    const base = decodeURIComponent(parsed.pathname.split('/').pop() || '');
    if (base) name = base;
  } catch { /* keep comet */ }
  // Strip known extension from the name so dedupe can re-add the real one.
  name = name.replace(/\.(gif|webp|png|jpe?g)$/i, '') || 'comet';
  return { ext: fileExt, name: sanitizeName(name) };
}

ipcMain.handle('comet:versions', () => ({
  app: app.getVersion(),
  electron: process.versions.electron,
}));

ipcMain.handle('comet:download', async (_e, { url, kind, opts }) => {
  opts = opts ?? {};
  const outDir = await getOutDir();

  if (runner.isRunning()) {
    return { ok: false, error: 'Já tem download rodando.' };
  }

  const info = classifyLink(url);

  if (kind === 'mp4' || kind === 'mp3') {
    if (info.type !== 'youtube') {
      return { ok: false, error: 'Isso não parece um link de vídeo do YouTube.' };
    }
    if (!fs.existsSync(YT_DLP)) {
      return { ok: false, error: 'yt-dlp não encontrado — rode o script de instalação (scripts/fetch-binaries.ps1).' };
    }

    const template = path.join(outDir, '%(title)s.%(ext)s');
    const args = buildYtDlpArgs({ kind, url, outputTemplate: template });

    const result = await runner.runBinary({
      cmd: YT_DLP,
      args,
      outDir,
      onProgress: (p) => sendEvent({ type: 'progress', percent: p.percent }),
    });

    if (result.cancelled) return { ok: false, cancelled: true };
    if (!result.ok) {
      const err = parseYtDlpError(result.stderr);
      sendEvent({ type: 'error', text: err });
      return { ok: false, error: err };
    }
    return { ok: true };
  }

  if (kind === 'gif') {
    if (!GIF_KINDS.has(info.type)) {
      return { ok: false, error: 'Isso não parece um link de GIF.' };
    }
    if (info.type === 'direct-image') {
      const res = await fetch(url);
      if (!res.ok || !String(res.headers.get('content-type') || '').startsWith('image/')) {
        return { ok: false, error: 'Não consegui baixar esse GIF.' };
      }
      await fs.mkdir(outDir, { recursive: true });
      const file = await saveGifFile(await res.arrayBuffer(), outDir, url, info.ext);
      return { ok: true, file };
    }

    // Tenor/Giphy page: extract the real media URL, then download it.
    const page = await fetch(url);
    if (!page.ok) {
      return { ok: false, error: 'Não consegui abrir esse link.' };
    }
    const media = extractMediaUrl(await page.text(), info.type);
    if (!media) {
      return { ok: false, error: 'Não encontrei GIF nesse link.' };
    }
    const img = await fetch(media.url);
    const contentType = String(img.headers.get('content-type') || '');
    const fileExt = media.ext && media.ext !== '.'
      ? media.ext
      : contentType.split('image/')[1]?.split(';')[0] || '';

    if (!img.ok || !contentType.startsWith('image/')) {
      return { ok: false, error: 'Não encontrei GIF nesse link.' };
    }
    await fs.mkdir(outDir, { recursive: true });
    const file = await saveGifFile(await img.arrayBuffer(), outDir, media.url, fileExt);
    return { ok: true, file };
  }

  return { ok: false, error: 'Formato desconhecido.' };
});

ipcMain.handle('comet:cancel', () => {
  const killed = runner.activeKill();
  if (killed) sendEvent({ type: 'status', text: 'Download parado.' });
  return { ok: killed };
});

ipcMain.handle('comet:folder-get', () => ({ ok: false, error: 'Nem tudo instalado ainda.' }));
ipcMain.handle('comet:folder-set', () => ({ ok: false, error: 'Nem tudo instalado ainda.' }));
ipcMain.handle('comet:ytdlp-update', () => ({ ok: false, error: 'Nem tudo instalado ainda.' }));
