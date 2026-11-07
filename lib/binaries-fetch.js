// Comet — automatic vend-binary fetch. Plain URLs in, files out of the way
// of the download flow. yt-dlp is a plain exe; ffmpeg is a zip that only
// PowerShell on standard Windows can extract (Expand-Archive).
const { spawn } = require('node:child_process');
const fsp = require('node:fs/promises');
const fs = require('node:fs');
const path = require('node:path');

function binariesFetchPlan({ vendorDir }) {
  return {
    ytDlpUrl: 'https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe',
    ffmpegUrl: 'https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip',
    ytDlpTarget: path.join(vendorDir, 'yt-dlp.exe'),
    ffmpegDir: vendorDir,
  };
}

function missingBinaries(vendorDir) {
  const missing = [];
  for (const name of ['yt-dlp.exe', 'ffmpeg.exe', 'ffprobe.exe']) {
    if (!fs.existsSync(path.join(vendorDir, name))) missing.push(name);
  }
  return missing;
}

async function downloadToFile(url, target, { signal, onStatus } = {}) {
  const res = await fetch(url, { signal });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  const buf = Buffer.from(await res.arrayBuffer());
  if (buf.length === 0) throw new Error('conteúdo vazio');
  await fsp.mkdir(path.dirname(target), { recursive: true });
  await fsp.writeFile(target, buf);
  return buf.length;
}

async function extractFfmpeg(zipBytes, ffmpegUrl, vendorDir, { signal, onStatus } = {}) {
  const tmp = path.join(require('node:os').tmpdir(), 'comet-ffmpeg.zip');
  await fsp.writeFile(tmp, zipBytes);
  const shell = [
    `Expand-Archive -Force -LiteralPath '${tmp}' -DestinationPath '${tmp}.x' | Out-Null`,
    `$v = Get-ChildItem '${tmp}.x' -Filter 'ffmpeg.exe' -Recurse | Select-Object -First 1`,
    `$p = Get-ChildItem '${tmp}.x' -Filter 'ffprobe.exe' -Recurse | Select-Object -First 1`,
    `if(-not $v -or -not $p){ throw 'ffmpeg no zip' }`,
    `Remove-Item '${path.join(vendorDir, 'ffmpeg.exe')}' -Force -ErrorAction SilentlyContinue`,
    `Copy-Item $v.FullName '${path.join(vendorDir, 'ffmpeg.exe')}' -Force`,
    `Copy-Item $p.FullName '${path.join(vendorDir, 'ffprobe.exe')}' -Force`,
  ].join('; ');
  return new Promise((resolve, reject) => {
    const child = spawn('powershell.exe', ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-Command', shell], { signal });
    let err = '';
    child.stderr.on('data', (d) => { err += String(d); });
    child.on('close', (code) => (code === 0 ? resolve() : reject(new Error(err.trim() || `powershell exit ${code}`))));
    child.on('error', reject);
  }).finally(async () => {
    await fsp.rm(`${tmp}.x`, { recursive: true, force: true });
    await fsp.rm(tmp, { force: true });
  });
}

// Fetches whatever vendor binaries are missing. Each download replaces the
// prior one atomically enough for a one-user tool (write to final name).
async function ensureBinaries({ vendorDir, signal, onStatus } = {}) {
  const plan = binariesFetchPlan({ vendorDir });
  const missing = missingBinaries(vendorDir);
  if (missing.length === 0) return { ok: true };

  const report = onStatus || (() => {});
  try {
    if (missing.includes('yt-dlp.exe')) {
      report('Baixando yt-dlp (aprox 24 MB)...');
      await downloadToFile(plan.ytDlpUrl, plan.ytDlpTarget, { signal });
      report('yt-dlp pronto.');
    }
    if (missing.includes('ffmpeg.exe') || missing.includes('ffprobe.exe')) {
      report('Baixando ffmpeg (aprox 90 MB) — demora mais.');
      const res = await fetch(plan.ffmpegUrl, { signal });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const zip = Buffer.from(await res.arrayBuffer());
      report('Desempacotando ffmpeg...');
      await extractFfmpeg(zip, plan.ffmpegUrl, vendorDir, { signal });
      report('ffmpeg pronto.');
    }
    if (missingBinaries(vendorDir).length > 0) {
      return { ok: false, missing: missingBinaries(vendorDir) };
    }
    return { ok: true };
  } catch (err) {
    if (err?.name === 'AbortError') throw err;
    return { ok: false, missing, error: err?.message || String(err) };
  }
}

module.exports = { binariesFetchPlan, missingBinaries, ensureBinaries };
