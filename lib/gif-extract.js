// Comet — extracts the actual GIF media URL out of a GIF host page.
// Tenor embeds media in its ld+json/og:image; Giphy exposes og:image.
// GIFs the user can share would be .gif; .webp is Tenor's preview but also
// a shareable image — prefer .gif when the page offers both.
const MEDIA_URL_RE = /https:[^"'\\\s<>]+?\.(?:gif|webp)/gi;

function extractMediaUrl(html, host) {
  if (!html) return null;

  const matches = [...String(html).matchAll(MEDIA_URL_RE)].map((m) => stripTrailingPunct(m[0]));
  if (matches.length === 0) return null;

  const gifs = matches.filter((u) => u.endsWith('.gif'));
  const pool = gifs.length > 0 ? gifs : matches;

  if (host === 'tenor') {
    const tenorMedia = pool.find((u) => u.includes('tenor.com/'));
    if (tenorMedia) return { url: tenorMedia, ext: 'gif' };
    return { url: pool[0], ext: 'webp' };
  }

  if (host === 'giphy') {
    const giphyMedia = pool.find((u) => u.includes('giphy.com/'));
    if (giphyMedia) {
      const ext = giphyMedia.endsWith('.gif') ? 'gif' : 'webp';
      return { url: giphyMedia, ext };
    }
  }

  return { url: pool[0], ext: pool[0].endsWith('.gif') ? 'gif' : 'webp' };
}

function stripTrailingPunct(url) {
  return url.replace(/[.,;:]+$/, '');
}

module.exports = { extractMediaUrl };
