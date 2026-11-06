// Comet — parses yt-dlp output: download progress lines and error lines.
// Progress arrives KDD-style: everything streams to stdout/stderr with \r
// carriage returns instead of full lines.

const PROGRESS_RE = /\[download\]\s+([\d.]+)%/;

function parseProgress(line) {
  const pct = String(line ?? '').match(PROGRESS_RE);
  if (!pct) return null;
  return { percent: Number(pct[1]), stage: 'download' };
}

function parseYtDlpError(output) {
  for (const line of String(output ?? '').split(/[\r\n]+/)) {
    const idx = line.indexOf('ERROR:');
    if (idx !== -1) return line.slice(idx).trim();
  }
  return 'Erro no download.';
}

// yt-dlp exits 0 when the file already exists (--no-overwrites skip).
function detectSkip(output) {
  return /has already been downloaded/i.test(String(output ?? ''));
}

module.exports = { parseProgress, parseYtDlpError, detectSkip };
