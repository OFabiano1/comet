const test = require('node:test');
const assert = require('node:assert');
const { classifyLink } = require('../lib/linkclassify');

test('youtube hosts count as youtube videos', () => {
  assert.strictEqual(classifyLink('https://youtu.be/dQw4w9WaXwQ').type, 'youtube');
  assert.strictEqual(classifyLink('https://www.youtube.com/watch?v=X&t=30s').type, 'youtube');
  assert.strictEqual(classifyLink('https://youtube.com/shorts/X').type, 'youtube');
  assert.strictEqual(classifyLink('https://www.youtube.com/embed/X').type, 'youtube');
  assert.strictEqual(classifyLink('https://music.youtube.com/watch?v=X').type, 'youtube');
});

test('tenor links keep type and extract gif extension from media urls', () => {
  assert.deepStrictEqual(
    classifyLink('https://media.tenor.com/x/Name.gif'),
    { type: 'tenor', ext: 'gif' }
  );
  assert.deepStrictEqual(
    classifyLink('https://c.tenor.com/x/Name.webp'),
    { type: 'tenor', ext: 'webp' }
  );
  assert.deepStrictEqual(
    classifyLink('https://tenor.com/view/clip-name-12345'),
    { type: 'tenor' }
  );
});

test('giphy links count as giphy', () => {
  assert.deepStrictEqual(
    classifyLink('https://giphy.com/gifs/x'),
    { type: 'giphy' }
  );
  assert.deepStrictEqual(
    classifyLink('https://media.giphy.com/media/abc/giphy.gif'),
    { type: 'giphy', ext: 'gif' }
  );
});

test('direct image urls keep their extension', () => {
  assert.deepStrictEqual(
    classifyLink('https://example.com/a/clip.GIF'),
    { type: 'direct-image', ext: 'gif' }
  );
  assert.deepStrictEqual(
    classifyLink('https://example.com/a/pic.png'),
    { type: 'direct-image', ext: 'png' }
  );
});

test('anything else and broken input is unknown', () => {
  assert.deepStrictEqual(classifyLink('https://example.com/page'), { type: 'unknown' });
  assert.deepStrictEqual(classifyLink(''), { type: 'unknown' });
  assert.deepStrictEqual(classifyLink('sem link nenhum'), { type: 'unknown' });
});
