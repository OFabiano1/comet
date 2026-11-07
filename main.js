// Comet — Electron main process.
// Owns child processes, config, and all IPC. Logic lives in lib/ modules.
const { app, BrowserWindow, ipcMain, dialog } = require('electron');
const path = require('path');
const fsp = require('node:fs/promises');
const fs = require('node:fs');

const { classifyLink } = require('./lib/linkclassify');
const { sanitizeName, dedupeName } = require('./lib/naming');
const { buildYtDlpArgs } = require('./lib/ytdlp-args');
const { parseYtDlpError, detectSkip, detectLoginWall } = require('./lib/progress-parser');
const { extractMediaUrl, gifRoute } = require('./lib/gif-extract');
const { isVideoKindUrl } = require('./lib/platforms');
const { ensureBinaries, missingBinaries } = require('./lib/binaries-fetch');
const runner = require('./lib/runner');

const VENDOR = path.join(__dirname, 'vendor');
const YT_DLP = path.join(VENDOR, 'yt-dlp.exe');
const DOWNLOAD_TIMEOUT_MS = 120_000;

function clearTimer(timer) {
  clearTimeout(timer);
}

async function sendDoneEvent(text) {
  sendEvent({ type: 'done', text });
}

function humanizeNetworkError(err) {
  const code = err?.cause?.code || err?.code;
  if (code === 'ENOTFOUND' || code === 'EAI_AGAIN') {
    return 'Sem internet (ou esse link não existe).';
  }
  if (code === 'ECONNRESET' || code === 'ECONNREFUSED') {
    return 'A conexão caiu no meio. Tenta de novo.';
  }
  if (code === 'CERT_HAS_EXPIRED' || code === 'UNABLE_TO_VERIFY_LEAF_SIGNATURE') {
    return 'Certificado do site inválido, o Comet desistiu.';
  }
  return `Não consegui baixar (${err?.name || 'erro'})`;
}

async function downloadDirectGif(url, outDir, ext, signal) {
  const res = await fetch(url, { signal });
  if (!res.ok || !String(res.headers.get('content-type') || '').startsWith('image/')) {
    throw new Error('Isso não abriu uma imagem — não achei GIF nesse link.');
  }
  await fs.mkdir(outDir, { recursive: true });
  return saveGifFile(await res.arrayBuffer(), outDir, url, ext);
}

async function downloadPageGif(url, outDir, hostType, signal) {
  const page = await fetch(url, { signal });
  if (!page.ok) {
    throw new Error('Não consegui abrir essa página do GIF.');
  }
  const media = extractMediaUrl(await page.text(), hostType);
  if (!media) {
    throw new Error('Não encontrei GIF nesse link.');
  }
  const img = await fetch(media.url, { signal });
  const contentType = String(img.headers.get('content-type') || '');
  if (!img.ok || !contentType.startsWith('image/')) {
    throw new Error('Não encontrei GIF nesse link.');
  }
  // Trust the real content type over a guessed extension when they collide.
  const guessedExt = media.ext;
  const realExt = contentType.split('image/')[1]?.split(';')[0] || guessedExt || '';
  const fileExt = realExt || 'gif';
  await fs.mkdir(outDir, { recursive: true });
  return saveGifFile(await img.arrayBuffer(), outDir, media.url, fileExt);
}

// Tracked child processes live in lib/runner.js.
const killAllTracked = runner.killAll;

// --- Config (user's save folder persists here) ---
const CONFIG_FILE = path.join(app.getPath('userData'), 'config.json');
let config = { saveDir: null };

function readConfig() {
  try {
    const parsed = JSON.parse(fs.readFileSync(CONFIG_FILE, 'utf8'));
    if (parsed && typeof parsed === 'object') config = { ...config, ...parsed };
  } catch { /* missing or unreadable: keep defaults */ }
}

async function writeConfig() {
  await fsp.mkdir(path.dirname(CONFIG_FILE), { recursive: true });
  await fsp.writeFile(CONFIG_FILE, JSON.stringify(config, null, 2));
}

readConfig();

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

// config saveDir (validated) wins; Downloads is the initial default.
async function getOutDir() {
  const fallback = app.getPath('downloads');
  if (config.saveDir) {
    try {
      const stat = await fsp.stat(config.saveDir);
      if (stat.isDirectory()) return config.saveDir;
    } catch { /* stale path in config: fall back */ }
  }
  return fallback;
}

// The #1 expected failure mode: YouTube extractor drift / outdated yt-dlp.
// When the error smells like that, point the user at the update button.
function humanizeError(text, info) {
  const message = parseYtDlpError(text);
  if (detectLoginWall(message)) {
    const site = info?.type === 'platform' ? 'esse site' : 'esse post';
    return `${message} — ${site} pediu login e o Comet não tem login (dados seus, nenhum). Tenta outro post.`;
  }
  return /sign in|confirm|unable to extract|nsig|extractor/i.test(message)
    ? `${message.replace(/\s+— tenta o botão.*$/, '')} — tenta o botão 'Atualizar yt-dlp'.`
    : message;
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
    if (!isVideoKindUrl(info)) {
      return { ok: false, error: 'Isso não parece um link de vídeo (o Comet aceita YouTube, TikTok, Instagram, X, Twitch, Pinterest e GIFs).' };
    }

    // Fresh clone or first run: fetch the vendor binaries on the spot.
    if (missingBinaries(VENDOR).length > 0) {
      const controller0 = new AbortController();
      const token0 = runner.beginCustom({ onAbort: () => controller0.abort() });
      try {
        const fetched = await ensureBinaries({
          vendorDir: VENDOR,
          signal: controller0.signal,
          onStatus: (t) => sendEvent({ type: 'status', text: t }),
        });
        if (!fetched.ok) {
          const err = 'Não consegui baixar os binários do yt-dlp/ffmpeg — confere a internet e tenta de novo.';
          sendEvent({ type: 'error', text: err });
          return { ok: false, error: err };
        }
      } catch (err) {
        if (err?.name === 'AbortError') {
          return { ok: false, cancelled: true };
        }
        throw err;
      } finally {
        runner.endCustom(token0);
      }
    }

    const template = path.join(outDir, '%(title)s.%(ext)s');
    const args = buildYtDlpArgs({
      kind, url, outputTemplate: template,
      ffmpegLocation: path.join(VENDOR, 'ffmpeg.exe'),
    });

    const result = await runner.runBinary({
      cmd: YT_DLP,
      args,
      outDir,
      onProgress: (p) => sendEvent({ type: 'progress', percent: p.percent }),
    });

    if (result.cancelled) return { ok: false, cancelled: true };
    if (!result.ok) {
      const err = humanizeError(result.stderr, info);
      sendEvent({ type: 'error', text: err });
      return { ok: false, error: err };
    }
    if (detectSkip(result.stdout)) {
      await sendDoneEvent('Esse já estava salvo nessa pasta.');
      return { ok: true, file: 'already-existed' };
    }
    await sendDoneEvent('Pronto, arquivo que entrou na pasta.');
    return { ok: true };
  }

  if (kind === 'gif') {
    const route = GIF_KINDS.has(info.type) ? gifRoute(info) : null;
    if (!route) {
      return { ok: false, error: 'Isso não parece um link de GIF.' };
    }

    const controller = new AbortController();
    const token = runner.beginCustom({
      onAbort: () => controller.abort(),
      outDir,
    });
    // Hung server never leaves the UI stuck: hard ceiling.
    const timer = setTimeout(() => controller.abort(), DOWNLOAD_TIMEOUT_MS);

    try {
      const file = route.kind === 'direct'
        ? await downloadDirectGif(url, outDir, route.ext, controller.signal)
        : await downloadPageGif(url, outDir, info.type, controller.signal);
      clearTimer(timer);
      await sendDoneEvent(`Pronto: ${path.basename(file)}`);
      return { ok: true, file };
    } catch (err) {
      clearTimer(timer);
      if (err?.name === 'AbortError' || controller.signal.aborted) {
        sendEvent({ type: 'error', text: 'Download parado.' });
        return { ok: false, cancelled: true };
      }
      const errText = humanizeNetworkError(err);
      sendEvent({ type: 'error', text: errText });
      return { ok: false, error: errText };
    } finally {
      runner.endCustom(token);
    }
  }

  return { ok: false, error: 'Formato desconhecido.' };
});

ipcMain.handle('comet:cancel', () => {
  const killed = runner.activeKill();
  if (killed) sendEvent({ type: 'status', text: 'Download parado.' });
  return { ok: killed };
});

ipcMain.handle('comet:folder-get', async () => ({
  ok: true,
  path: await getOutDir(),
}));

ipcMain.handle('comet:folder-set', async () => {
  const result = await dialog.showOpenDialog(win, {
    title: 'Onde o Comet salva?',
    properties: ['openDirectory', 'createDirectory'],
  });
  if (result.canceled || result.filePaths.length === 0) {
    return { ok: false, error: 'Sem pasta escolhida.' };
  }
  config.saveDir = result.filePaths[0];
  await writeConfig();
  return { ok: true };
});

ipcMain.handle('comet:ytdlp-update', async () => {
  if (!fs.existsSync(YT_DLP)) {
    return { ok: false, error: 'yt-dlp não encontrado — rode o script de instalação (scripts/fetch-binaries.ps1).' };
  }
  const result = await runner.runBinary({ cmd: YT_DLP, args: ['-U'] });
  if (result.cancelled) return { ok: false, error: 'Atualização parada.' };
  if (!result.ok) {
    return { ok: false, error: humanizeError(result.stderr || result.stdout) };
  }
  const lines = (result.stderr || result.stdout || '').split(/\r?\n/).filter(Boolean);
  sendEvent({ type: 'status', text: lines[lines.length - 1] || 'yt-dlp atualizado.' });
  return { ok: true, text: lines[lines.length - 1] || 'yt-dlp atualizado.' };
});
