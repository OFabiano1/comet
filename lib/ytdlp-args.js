// Comet — builds the exact yt-dlp argument vectors. Pins:
// mp4: best up to 1080p, merged/fallback to progressive, ffmpeg-merged mp4.
// mp3: extract audio, 192 kbps mp3. Both never overwrite existing files.
function buildYtDlpArgs({ kind, url, outputTemplate }) {
  if (!url || !outputTemplate) {
    throw new Error('buildYtDlpArgs requires url and outputTemplate');
  }

  const args = ['--no-warnings', '--no-overwrites', '-o', outputTemplate];

  if (kind === 'mp4') {
    args.push(
      '-f', 'bestvideo[height<=1080]+bestaudio/best[height<=1080]/best',
      '--merge-output-format', 'mp4',
    );
  } else if (kind === 'mp3') {
    args.push('-x', '--audio-format', 'mp3', '--audio-quality', '192K');
  } else {
    throw new Error(`unsupported kind: ${kind}`);
  }

  args.push(String(url));
  return args;
}

module.exports = { buildYtDlpArgs };
