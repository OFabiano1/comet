// Comet — runner: spawns external binaries (yt-dlp), streams their output
// as progress events, tracks the active download for cancel, and cleans
// partial files so nothing half-written survives a stop.
const { spawn } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');
const { parseProgress } = require('./progress-parser');

const children = new Set();
let active = null; // { child, outDir, resolve, finished, killedByUs }

function runBinary({ cmd, args, onProgress, outDir }) {
  return new Promise((resolve) => {
    const child = spawn(cmd, args, outDir ? { cwd: outDir } : undefined);
    const record = { child, outDir, resolve, killedByUs: false, finished: false };
    children.add(child);
    active = record;

    const finish = (value) => {
      if (record.finished) return;
      record.finished = true;
      children.delete(child);
      if (active === record) active = null;
      resolve(value);
    };

    const stderrPieces = [];
    const pump = (chunk, isErr) => {
      for (const line of String(chunk).split(/\r\n|\r|\n/)) {
        if (!line) continue;
        if (isErr) stderrPieces.push(line);
        if (onProgress) {
          const p = parseProgress(line);
          if (p) onProgress(p);
        }
      }
    };

    child.stdout?.on('data', (d) => pump(d, false));
    child.stderr?.on('data', (d) => pump(d, true));

    child.on('error', (err) => {
      finish({ ok: false, stderr: `ERROR: ${err.message}` });
    });

    child.on('close', (code) => {
      if (record.killedByUs) {
        finish({ ok: false, cancelled: true });
      } else if (code === 0) {
        finish({ ok: true, stderr: stderrPieces.join('\n') });
      } else {
        finish({ ok: false, stderr: stderrPieces.join('\n') });
      }
    });
  });
}

function removePartFiles(outDir) {
  try {
    for (const name of fs.readdirSync(outDir)) {
      if (name.toLowerCase().endsWith('.part')) {
        fs.rmSync(path.join(outDir, name), { force: true });
      }
    }
  } catch { /* dir unreadable; nothing to clean */ }
}

// Cancels the active download: kills the process, removes partial files,
// and resolves it as cancelled. Returns true when a download was running.
function activeKill() {
  if (!active) return false;
  const { child, outDir } = active;
  active.killedByUs = true;
  try { child.kill(); } catch { /* already dead */ }
  if (outDir) removePartFiles(outDir);
  return true;
}

// Kills every tracked child (window close). Pending runs resolve not-ok.
function killAll() {
  for (const child of children) {
    try { child.kill(); } catch { /* already dead */ }
  }
  children.clear();
  if (active) {
    const { outDir, resolve } = active;
    active.killedByUs = true;
    active = null;
    if (outDir) removePartFiles(outDir);
    resolve({ ok: false, cancelled: true });
  }
}

function isRunning() {
  return active !== null;
}

module.exports = { runBinary, activeKill, killAll, isRunning };
