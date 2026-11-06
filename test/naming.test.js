const test = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { sanitizeName, dedupeName } = require('../lib/naming');

test('sanitizeName strips forbidden Windows characters', () => {
  assert.strictEqual(sanitizeName('AC/DC <Live> 4K Edition'), 'ACDC Live 4K Edition');
  assert.strictEqual(sanitizeName('Video: "the" movie | part 2?*'), 'Video the movie part 2');
  assert.strictEqual(sanitizeName('back\\slashes\\too'), 'backslashestoo');
});

test('sanitizeName keeps emoji and unicode', () => {
  const r = sanitizeName('🫟 axolote α β vídeo');
  assert.ok(r.includes('axolote'));
  assert.ok(r.includes('🫟'));
  assert.ok(r.includes('α β'));
});

test('sanitizeName collapses whitespace and caps at 120 chars', () => {
  assert.strictEqual(sanitizeName('   A    B   '), 'A B');
  assert.ok(sanitizeName('x'.repeat(300)).length <= 120);
});

test('sanitizeName falls back to comet on empty', () => {
  assert.strictEqual(sanitizeName(''), 'comet');
  assert.strictEqual(sanitizeName('   '), 'comet');
  assert.strictEqual(sanitizeName('///***???'), 'comet');
});

test('dedupeName returns name.ext when free', async () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'comet-'));
  const r = await dedupeName(dir, 'video', 'mp4');
  assert.strictEqual(r, 'video.mp4');
});

test('dedupeName (1) when base exists, never overwrite', async () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'comet-'));
  fs.writeFileSync(path.join(dir, 'video.mp4'), 'x');
  fs.writeFileSync(path.join(dir, 'video (1).mp4'), 'x');
  const r = await dedupeName(dir, 'video', 'mp4');
  assert.strictEqual(r, 'video (2).mp4');
});
