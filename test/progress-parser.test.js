const test = require('node:test');
const assert = require('node:assert');
const { parseProgress, parseYtDlpError } = require('../lib/progress-parser');

test('parseProgress extracts percent from yt-dlp download lines', () => {
  assert.deepStrictEqual(parseProgress('[download]  42.7% of 5.0MiB'), { percent: 42.7, stage: 'download' });
  assert.deepStrictEqual(parseProgress('\r[download]   0.0% of 100.50MiB'), { percent: 0, stage: 'download' });
  assert.deepStrictEqual(parseProgress('[download] 100% of 2.21MiB'), { percent: 100, stage: 'download' });
});

test('parseProgress returns null for non-progress noise', () => {
  assert.strictEqual(parseProgress('[youtube] Extracting channel'), null);
  assert.strictEqual(parseProgress(''), null);
  assert.strictEqual(parseProgress('[Merger] Merging video'), null);
});

test('parseYtDlpError returns the clean ERROR line', () => {
  const out = [
    '[youtube] dQw: Downloading webpage',
    'ERROR: [youtube] dQw: Video unavailable. Reason: private video. Sign in if you were greeted with a message that it is only available to YouTube members',
    'no longer relevant lines',
  ].join('\n');
  const msg = parseYtDlpError(out);
  assert.ok(msg.startsWith('ERROR: [youtube]'));
  assert.ok(!msg.includes('no longer'));
});

test('parseYtDlpError falls back to a generic message', () => {
  assert.strictEqual(parseYtDlpError('nothing here'), 'Erro no download.');
});
