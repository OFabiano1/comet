const test = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { runBinary, activeKill, killAll, beginCustom, endCustom, isRunning } = require('../lib/runner');

test('custom active claims one-slot and isRunning reflects it', () => {
  assert.strictEqual(isRunning(), false);
  const token = beginCustom(() => {});
  assert.strictEqual(isRunning(), true);
  endCustom(token);
  assert.strictEqual(isRunning(), false);
});

test('activeKill aborts the custom active and removes partial files', () => {
  const outDir = fs.mkdtempSync(path.join(os.tmpdir(), 'comet-'));
  const partFile = path.join(outDir, 'stream.part');
  fs.writeFileSync(partFile, 'bytes');
  let aborted = false;
  const token = beginCustom({ onAbort: () => { aborted = true; }, outDir });
  const killed = activeKill();
  endCustom(token);
  assert.strictEqual(killed, true);
  assert.strictEqual(aborted, true);
  assert.strictEqual(fs.existsSync(partFile), false);
  assert.strictEqual(isRunning(), false);
});

test('beginCustom enforces one slot at a time', () => {
  const first = beginCustom(() => {});
  assert.throws(() => beginCustom(() => {}), /slot/);
  endCustom(first);
});
