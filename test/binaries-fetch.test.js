const test = require('node:test');
const assert = require('node:assert');
const { binariesFetchPlan } = require('../lib/binaries-fetch');

test('binary fetch plan names the official sources verbatim', () => {
  const plan = binariesFetchPlan({ vendorDir: 'vendor' });
  assert.strictEqual(
    plan.ytDlpUrl,
    'https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe',
  );
  assert.strictEqual(
    plan.ffmpegUrl,
    'https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip',
  );
  assert.ok(plan.ytDlpTarget.endsWith('yt-dlp.exe'));
});
