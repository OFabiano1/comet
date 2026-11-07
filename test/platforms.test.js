const test = require('node:test');
const assert = require('node:assert');
const { classifyLink } = require('../lib/linkclassify');
const { VIDEO_HOSTS, isVideoKindUrl } = require('../lib/platforms');

test('VIDEO_HOSTS names the partner-requested platforms', () => {
  for (const host of ['x.com', 'twitter.com', 'tiktok.com', 'instagram.com', 'pinterest.com', 'twitch.tv', 'clips.twitch.tv']) {
    assert.ok(VIDEO_HOSTS.includes(host), host);
  }
});

test('classifier flags platform hosts', () => {
  assert.strictEqual(classifyLink('https://x.com/user/status/123').type, 'platform');
  const tik = classifyLink('https://www.tiktok.com/@user/video/999');
  assert.strictEqual(tik.type, 'platform');
  assert.strictEqual(classifyLink('https://www.instagram.com/p/abc123/').type, 'platform');
  assert.strictEqual(classifyLink('https://br.pinterest.com/pin/12345/').type, 'platform');
  assert.strictEqual(classifyLink('https://clips.twitch.tv/FunnyClip').type, 'platform');
  assert.strictEqual(classifyLink('https://www.twitch.tv/videos/1').type, 'platform');
});

test('isVideoKindUrl accepts youtube, platforms and unknown; refuses gif kinds', () => {
  assert.strictEqual(isVideoKindUrl({ type: 'youtube' }), true);
  assert.strictEqual(isVideoKindUrl({ type: 'platform' }), true);
  assert.strictEqual(isVideoKindUrl({ type: 'unknown' }), true);
  assert.strictEqual(isVideoKindUrl({ type: 'direct-image', ext: 'gif' }), false);
  assert.strictEqual(isVideoKindUrl({ type: 'tenor' }), false);
  assert.strictEqual(isVideoKindUrl({ type: 'giphy', ext: 'webp' }), false);
});
