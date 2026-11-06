const test = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const path = require('node:path');
const { extractMediaUrl } = require('../lib/gif-extract');

const read = (name) => fs.readFileSync(path.join('test', 'fixtures', name), 'utf8');

test('tenor page html yields the gif media url', () => {
  assert.deepStrictEqual(
    extractMediaUrl(read('tenor.html'), 'tenor'),
    { url: 'https://media.tenor.com/KfDnnGGctXcAAAAi/hamster-dance.gif', ext: 'gif' }
  );
});

test('giphy page html yields the media image url', () => {
  assert.deepStrictEqual(
    extractMediaUrl(read('giphy.html'), 'giphy'),
    { url: 'https://media3.giphy.com/media/l0HUiR0s2Y7XllNAk/giphy.webp', ext: 'webp' }
  );
});

test('page without any media url parses to null', () => {
  assert.strictEqual(extractMediaUrl('<html><head></head><body>oi</body></html>', 'tenor'), null);
  assert.strictEqual(extractMediaUrl('', 'giphy'), null);
});
