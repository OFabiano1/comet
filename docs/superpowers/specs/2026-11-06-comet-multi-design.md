# Comet multi-site — design addendum (small change)

Date: 2026-11-06. Extends `2026-11-06-comet-design.md`. Approved in conversation
("pode fazer"), with one constraint added by the human partner: Comet stays a
paste-link app, never a browser.

## Intent

- MP4/MP3 downloads accept links from all yt-dlp-supported sites the user
  named: X/Twitter, TikTok, Instagram, Pinterest, Twitch — plus YouTube —
  and any other site whose link yt-dlp can handle.
- People who clone/down-load the GitHub repo must get a working app without
  manual PowerShell: if `vendor/yt-dlp.exe` or `ffmpeg.exe` are missing at
  download time, Comet fetches them itself.

## Goals

- Classifier knows the named platforms (video hosts) and keeps the GIF flow
  untouched (direct image links, Tenor, Giphy pages).
- MP4/MP3 path accepts: platform links + unknown hosts (yt-dlp decides; its
  error text surfaces if the site is unsupported).
- On missing binaries: automatic one-time fetch (yt-dlp.exe from the official
  GitHub release; ffmpeg from gyan.dev) with live status events, then the
  download proceeds. Works both in `npm start` and the packaged `Comet.exe`.

## Non-goals / constraints

- No embedded browser, no webviews, no site browsing UI. One URL field.
- No cookies/logins. Instagram (and rarely YouTube) pages that demand login
  fail with a clear message naming the site — no workaround in-app.
- "GIFs" on Twitter/TikTok are mp4s at source; they download as mp4.

## Approaches considered

1. **Widen classifier + accept unknown into yt-dlp** — chosen: tiny, uses the
   already-proven engine for ~1800 sites. Unknown links: yt-dlp error decides,
   app never guesses.
2. **Hard-code per-platform handlers** — rejected: duplicate work yt-dlp
   already does; breaks the moment a site changes.
3. **Browser/webview approach** — rejected by the human partner's constraint:
   Comet is an app, not a browser.

## Decided

- Binary auto-fetch lives in the main process (not the PS script): the
  packaged app does not ship `scripts/`. PowerShell is only invoked for the
  ffmpeg zip extraction; yt-dlp.exe is a plain HTTPS download.
- UI copy/subtitle updated to name the sites; README updated.
- Error mapping gains an Instagram/YouTube "login demanded" message.

## Left to the builder

- Exact classifier additions and their test values.
- Auto-fetch event texts; retry behavior on partial ffmpeg download
  (v1: fail with clear message, user retries the button/click).
