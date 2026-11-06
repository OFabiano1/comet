const test = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { runBinary, activeKill, killAll } = require('../lib/runner');

test('runBinary streams parsed progress percentages', async () => {
  const outDir = fs.mkdtempSync(path.join(os.tmpdir(), 'comet-'));
  const percents = [];
  const result = await runBinary({
    cmd: process.execPath,
    args: ['-e', "process.stdout.write('\\r\\n[download]  42.7% of 5.0MiB');"],
    onProgress: (p) => percents.push(p.percent),
    outDir,
  });
  assert.ok(result.ok);
  assert.deepStrictEqual(percents, [42.7]);
});

test('runBinary collects stderr for error reporting', async () => {
  const outDir = fs.mkdtempSync(path.join(os.tmpdir(), 'comet-'));
  const result = await runBinary({
    cmd: process.execPath,
    args: ['-e', "process.stderr.write('ERROR: [youtube] Video unavailable'); process.exit(1);"],
    onProgress: () => {},
    outDir,
  });
  assert.ok(!result.ok);
  assert.ok(!result.cancelled);
  assert.ok(result.stderr.includes('ERROR: [youtube]'));
});

test('activeKill resolves the run cancelled and removes partial files', async () => {
  const outDir = fs.mkdtempSync(path.join(os.tmpdir(), 'comet-'));
  const partFile = path.join(outDir, 'x.part');
  fs.writeFileSync(partFile, 'partial data');

  const pending = runBinary({
    cmd: process.execPath,
    args: ['-e', "process.stdout.write('[download]   5.0%'); setInterval(() => {}, 1000);"],
    onProgress: () => {},
    outDir,
  });

  await new Promise((r) => setTimeout(r, 300));
  const killed = activeKill();
  const result = await pending;

  assert.strictEqual(killed, true);
  assert.deepStrictEqual(result, { ok: false, cancelled: true });
  assert.strictEqual(fs.existsSync(partFile), false);
  killAll();
});

test('killAll ends long-running children', async () => {
  const outDir = fs.mkdtempSync(path.join(os.tmpdir(), 'comet-'));
  const pending = runBinary({
    cmd: process.execPath,
    args: ['-e', 'setInterval(() => {}, 1000);'],
    onProgress: () => {},
    outDir,
  });
  await new Promise((r) => setTimeout(r, 200));
  killAll();
  const result = await pending;
  assert.ok(!result.ok);
});
