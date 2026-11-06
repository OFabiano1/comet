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
    const stdoutPieces = [];
    const pump = (chunk, isErr) => {
      for (const line of String(chunk).split(/\r\n|\r|\n/)) {
        if (!line) continue;
        if (isErr) stderrPieces.push(line);
        else stdoutPieces.push(line);
        if (onProgress) {
          const p = parseProgress(line);
          if (p) onProgress(p);
        }
      }
    };

    child.stdout?.on('data', (d) => pump(d, false));
    child.stderr?.on('data', (d) => pump(d, true));

    child.on('error', (err) => {
      finish({ ok: false, stderr: `ERROR: ${err.message}`, stdout: '' });
    });

    child.on('close', (code) => {
      if (record.killedByUs) {
        if (record.outDir) removePartFiles(record.outDir);
        finish({ ok: false, cancelled: true });
      } else if (code === 0) {
        finish({ ok: true, stderr: stderrPieces.join('\n'), stdout: stdoutPieces.join('\n') });
      } else {
        finish({ ok: false, stderr: stderrPieces.join('\n'), stdout: stdoutPieces.join('\n') });
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
  const { outDir } = active;

  // Non-process activity (in-flight fetch): abort and clean partials.
  if (active.__custom) {
    active.customAbort();
    if (outDir) removePartFiles(outDir);
    return true;
  }

  const { child } = active;
  active.killedByUs = true;
  try { child.kill(); } catch { /* already dead */ }
  if (outDir) removePartFiles(outDir);
  return true;
}

const customRegistry = new Set();

// Claims the download slot for a non-process operation (e.g. an in-flight
// fetch). One slot at a time; onAbort is what "Parar" runs.
function beginCustom({ onAbort, outDir } = {}) {
  if (active || customRegistry.size > 0) {
    throw new Error('download slot already claimed');
  }
  const custom = {
    onAbort: onAbort || (() => {}),
    outDir,
    done: false,
  };
  customRegistry.add(custom);

  active = {
    __custom: custom,
    customAbort: () => {
      if (!custom.done) {
        custom.done = true;
        custom.onAbort();
      }
    },
    outDir,
    killedByUs: false,
    child: null,
    resolve: () => {},
    finished: true,
  };
  return custom;
}

// Releases the custom slot. Only the owner token's own release matters.
function endCustom(token) {
  if (!token || !customRegistry.has(token)) return;
  customRegistry.delete(token);
  token.done = true;
  if (active && active.__custom === token) active = null;
}

// Kills every tracked child (window close). Pending runs resolve not-ok.
function killAll() {
  for (const child of children) {
    try { child.kill(); } catch { /* already dead */ }
  }
  children.clear();
  for (const custom of [...customRegistry]) {
    if (!custom.done) {
      custom.done = true;
      try { custom.onAbort(); } catch { /* already aborted */ }
    }
    if (custom.outDir) removePartFiles(custom.outDir);
  }
  customRegistry.clear();
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

module.exports = { runBinary, activeKill, killAll, isRunning, beginCustom, endCustom };
