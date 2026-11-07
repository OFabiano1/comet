// Comet — platform registry: which links the video kinds accept, in plain
// data. yt-dlp does the actual extraction for all of them.
const VIDEO_HOSTS = [
  'x.com',
  'twitter.com',
  'tiktok.com',
  'instagram.com',
  'pinterest.com',
  'twitch.tv',
  'clips.twitch.tv',
];

function isVideoKindUrl(info) {
  return (
    info.type === 'youtube' ||
    info.type === 'platform' ||
    info.type === 'unknown'
  );
}

module.exports = { VIDEO_HOSTS, isVideoKindUrl };
