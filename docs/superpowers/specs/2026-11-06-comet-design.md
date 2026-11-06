# Comet — Design Document

Date: 2026-11-06
Status: draft (pending builder check + human approval)

## Intent

Comet is a small desktop app for downloading media from a pasted link:

- **YouTube link** → download the video as `.mp4` or extract the audio as `.mp3`.
- **GIF link** (Tenor, Giphy, or any site) → save the `.gif` file directly.

YouTube does not natively serve GIFs; per user decision, Comet downloads GIFs
**by link** from GIF hosts and does not convert video segments to GIF.

## Why

Personal tool for the user: fetch a video as a file, fetch its audio as an
mp3, and save GIFs they find online. Simple, fast, no accounts.

## Goals

- Paste a YouTube URL, choose `.mp4` or `.mp3`, download, see progress.
- Paste a GIF URL (direct file URL, or a Tenor/Giphy page URL), download the `.gif`.
- Works on Windows (PowerShell environment).
- Every download lands in a folder the user can pick (default: `Downloads`).

## Non-goals

- No playlists / batch downloads (v1: one link at a time).
- No UI to search YouTube or GIF libraries.
- No video trimming, cutting, or GIF conversion.
- No DRM circumvention or account/cookie-based downloading.

## Anti-goals (failure even if it technically works)

- Looks like generic crypto/casino slop or an over-Neon gamer template.
- Downloads that silently fail without a clear error message.
- Binaries (`yt-dlp.exe`, `ffmpeg.exe`) committed to the repository.

## Constraints

- Windows; project path contains `#` (`C:\AxolotlBR\#Projetos\comet`). Known
  quirk: Vite dev/build breaks on paths containing `#`. Therefore **no
  Vite/bundlers**; plain Electron with direct HTML/CSS/JS.
- YouTube frequently changes behavior; downloads come from `yt-dlp`, which
  ships fixes constantly. The app must keep `yt-dlp` updatable, and the
  design must treat "yt-dlp needs updating" as the primary expected failure
  mode.
- No secrets in the repo, no invented data.

## Approaches considered

1. **Electron without a bundler + local `yt-dlp.exe` / `ffmpeg.exe`** — chosen.
   Window is plain HTML/CSS/JS; downloads run as child processes. Avoids the
   `#` path quirk; keeps the stack visible and boring; matches the user's
   pick (desktop app, JS).
2. **Electron + Vite + TypeScript** — same stack as axolotl-hub, but forces a
   `C:\Temp` dev workflow because of the `#` path. More structure for a small
   app. Rejected.
3. **Python + Tkinter + yt-dlp package** — one language, lighter, but the user
   picked a JS desktop app and the UI would be cruder. Rejected.

## Decided parts

- Platform: Electron (plain, no bundler). Main process in one `main.js`,
  renderer in `index.html` + `renderer.js` + CSS. `npm start` to run; no
  installer packaging in v1.
- Binary handling: `yt-dlp.exe` and `ffmpeg.exe` live in `vendor/`, never
  committed (`gitignore`). A small setup step (documented script) fetches
  current binaries; the renderer exposes a manual "update yt-dlp" affordance
  when it is feasible, since outdated yt-dlp is the main expected failure.
- YouTube download: spawn `yt-dlp` and parse its progress output.
  - `.mp4`: best available up to 1080p, progressive format preferred so the
    file stays simple; fall back to merged best-format if needed (ffmpeg).
  - `.mp3`: extract audio from best audio stream, 192 kbps mp3.
- GIF download: HTTP GET on the pasted URL.
  - URL points at an actual image file (`Content-Type` image) → save
    immediately as `.gif`.
  - URL is a Tenor or Giphy page → fetch the page, extract the real GIF URL,
    save it. (Tenor often serves `.webp`/`.mp4` for large media; save the
    actual format and name the file by its real extension.)
- Output naming: sanitized video title / GIF slug + correct extension.
- Platform reporting: progress bar plus status line in simple terms
  ("Baixando…", "Pronto", clear error text). No fake percentages.
- UX language: Portuguese, informal but not childish (Axolotl BR voice).

## Builder-check calls (settled during review; rules the builder must follow)

- **Save folder:** persistent setting (config file), changeable via an
  in-app "choose folder" native dialog. `Downloads`
  (`app.getPath('downloads')`) is the initial value, never a hard-coded ask
  per download.
- **Existing file:** never overwrite silently. Let yt-dlp's numeric-suffix
  deduplication handle collisions; GIF saves auto-rename as
  `name (1).gif` if it would overwrite.
- **yt-dlp update:** the in-app update affordance runs `yt-dlp.exe -U`
  (self-update) as a child process. `ffmpeg` has no `-U`; updating it stays
  a manual re-run of the setup script.
- **Cancel:** the UI has a stop button that kills the running child process
  and removes the partial `.part` file. Closing the window also terminates
  child processes cleanly (no orphaned downloads).

## Left to the builder

- Exact UI layout and visual identity (clean and minimal, not a generic
 neon gamer template look).
- Error surfacing details and edge cases (network drop mid-download,
  private/unavailable video, non-GIF URL).
- The Tenor/Giphy page-parsing implementation.
- Electron packaging tooling decision (installer or `npm start` only) — v1
  default is `npm start` only; changing it needs a reason.
- Subtitle/metadata handling defaults (v1: none, just media files).
