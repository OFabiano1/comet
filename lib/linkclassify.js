// Comet — URL classifier: decides what kind of thing a pasted link is.
// Only one download path runs per link; a wrong kind is refused, never guessed.
const MEDIA_EXTS = ['gif', 'webp', 'png', 'jpg', 'jpeg'];
const { VIDEO_HOSTS } = require('./platforms');

function classifyLink(input) {
  let url;
  try {
    url = new URL(String(input ?? '').trim());
  } catch {
    return { type: 'unknown' };
  }

  const host = url.hostname.toLowerCase().replace(/^www\./, '');
  const pathExt = url.pathname.toLowerCase().match(/\.([a-z0-9]+)$/);

  if (host === 'youtu.be' || host === 'youtube.com' || host.endsWith('.youtube.com')) {
    return { type: 'youtube' };
  }
  if (VIDEO_HOSTS.some((h) => host === h || host.endsWith(`.${h}`))) {
    return { type: 'platform' };
  }
  if (host === 'tenor.com' || host.endsWith('.tenor.com')) {
    return pathExt && MEDIA_EXTS.includes(pathExt[1])
      ? { type: 'tenor', ext: pathExt[1] }
      : { type: 'tenor' };
  }
  if (host === 'giphy.com' || host.endsWith('.giphy.com')) {
    return pathExt && MEDIA_EXTS.includes(pathExt[1])
      ? { type: 'giphy', ext: pathExt[1] }
      : { type: 'giphy' };
  }
  if (pathExt && MEDIA_EXTS.includes(pathExt[1])) {
    return { type: 'direct-image', ext: pathExt[1] };
  }
  return { type: 'unknown' };
}

module.exports = { classifyLink };
