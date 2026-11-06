const test = require('node:test');
const assert = require('node:assert');
const { buildYtDlpArgs } = require('../lib/ytdlp-args');

const MP4_FORMAT = 'bestvideo[height<=1080]+bestaudio/best[height<=1080]/best';
const URL = 'https://youtu.be/dQw4w9WaXwQ';
const TEMPLATE = 'out/v%.%(ext)s';

test('mp4 args pin 1080p ffmpeg-merged mp4 without overwriting', () => {
  const args = buildYtDlpArgs({ kind: 'mp4', url: URL, outputTemplate: TEMPLATE, ffmpegLocation: 'vendor/ffmpeg.exe' });
  assert.ok(args.includes('--ffmpeg-location'));
  assert.ok(args.includes('vendor/ffmpeg.exe'));
  const s = args.join(' ');
  assert.ok(args.includes('--merge-output-format'));
  assert.ok(args.includes('mp4'));
  assert.ok(args.includes('--no-overwrites'));
  assert.ok(args.includes(MP4_FORMAT));
  assert.ok(args.includes(TEMPLATE));
});

test('mp3 args extract 192 kbps audio', () => {
  const args = buildYtDlpArgs({ kind: 'mp3', url: URL, outputTemplate: TEMPLATE, ffmpegLocation: 'vendor/ffmpeg.exe' });
  assert.ok(args.includes('-x'));
  assert.ok(args.includes('--audio-format'));
  assert.ok(args.includes('mp3'));
  assert.ok(args.includes('--audio-quality'));
  assert.ok(args.includes('192K'));
  assert.ok(args.includes('--no-overwrites'));
  assert.ok(args.includes('--ffmpeg-location'));
});

test('both arg sets end with the bare url as last argument', () => {
  for (const kind of ['mp4', 'mp3']) {
    const args = buildYtDlpArgs({ kind, url: URL, outputTemplate: TEMPLATE });
    assert.strictEqual(args[args.length - 1], URL);
  }
});
