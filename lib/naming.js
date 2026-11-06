// Comet — filename helpers: sanitize titles into safe file names and find
// a free name on disk without ever overwriting an existing file.
const fs = require('node:fs');
const path = require('node:path');

// Windows-forbidden characters plus control characters.
const FORBIDDEN = /[<>:"/\\|?*\u0000-\u001f]/g;

function sanitizeName(title) {
  let name = String(title ?? '').replace(FORBIDDEN, '');
  name = name.trim().replace(/\s+/g, ' ');
  if (name.length > 120) name = name.slice(0, 120).trim();
  return name.length > 0 ? name : 'comet';
}

async function dedupeName(dir, name, ext) {
  const cleanExt = String(ext || '').replace(/^\./, '');
  let candidate = cleanExt ? `${name}.${cleanExt}` : name;
  for (let i = 1; ; i++) {
    if (!fs.existsSync(path.join(dir, candidate))) return candidate;
    candidate = cleanExt ? `${name} (${i}).${cleanExt}` : `${name} (${i})`;
  }
}

module.exports = { sanitizeName, dedupeName };
