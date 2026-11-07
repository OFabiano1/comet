# Comet multi-site Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax.

**Goal:** MP4/MP3 from any yt-dlp-supported site (X/Twitter, TikTok, Instagram, Pinterest, Twitch, YouTube, unknowns) + automatic binary fetch when `vendor/` is empty (repo clone / fresh exe).

**Architecture:** Only the classifier and the main-process acceptance gate change for downloads. Binary auto-fetch: main-process spawn of a small embedded PowerShell step (temp ps1) for ffmpeg (zip) and direct HTTPS for yt-dlp.exe; events stream to the existing status line.

**Tech Stack:** unchanged (Electron CommonJS, node:test).

**Spec:** `docs/superpowers/specs/2026-11-06-comet-multi-design.md`

## Global Constraints

- No webview/browser UI (hard constraint).
- No cookies/login; Instagram login walls surface a clear message.
- GIF flow untouched (`direct-image`, `tenor`, `giphy` and their pages).
- Nothing binary committed to git; `vendor/` fetched at runtime.
- UI copy Portuguese; no fake percentages.

## Review Focus

1. Non-video link clicked as MP4 (e.g. a Google page) → yt-dlp error text, never a silent false success or crash. Test: `multi-site.test.js` acceptance + existing stderr path.
2. Missing binaries on fresh clone → auto-fetch runs, status events flow, download proceeds after. Test: events path covered by unit test of the fetch plan function.
3. Instagram login wall → message names Instagram. Test: `humanizeError` mapping.
4. GIF flow regression → existing tests keep passing unchanged files' behavior.
5. Partial incoming ffmpeg zip (connection drop) → clear error, no corrupt extraction accepted. Test: unit checks fetch step failure propagation.

---

### Task 1: Classifier platforms + video acceptance gate

**Files:**
- Create: `lib/platforms.js`
- Test: `test/platforms.test.js`
- Modify: `lib/linkclassify.js`, `main.js`

**Interfaces:**
- Consumes: existing `classifyLink`.
- Produces: `lib/platforms.js` `VIDEO_HOSTS: string[]` (`x.com`, `twitter.com`, `tiktok.com`, `instagram.com`, `pinterest.com`, `twitch.tv`, `clips.twitch.tv`); `isVideoKindUrl(info) -> bool` true for `{type:'youtube'}`, any of these hosts (type `platform`), and `{type:'unknown'}`.

- [ ] **Step 1:** Failing tests: classifier returns `{type:'platform'}` for x.com/tiktok/instagram/pinterest/twitch hosts mid-path; `isVideoKindUrl` true for platform/unknown/youtube and false for gif kinds.
- [ ] **Step 2:** `node --test test/platforms.test.js` FAIL.
- [ ] **Step 3:** Implement; `main.js` mp4/mp3 guard switches from `info.type !== 'youtube'` to `!isVideoKindUrl(info)`.
- [ ] **Step 4:** suite PASS.
- [ ] **Step 5:** Commit `feat: multi-site video acceptance`.

### Task 2: Auto-fetch binaries

**Files:**
- Modify: `main.js`
- Create: `lib/binaries-fetch.js`
- Test: `test/binaries-fetch.test.js`

**Interfaces:**
- Consumes: runner custom slot (Task 6 prior), `sendEvent`.
- Produces: `binariesFetchPlan({vendorDir}) -> {ytDlpUrl, ffmpegUrl}`; main flow: on missing `/yt-dlp.exe` OR `/ffmpeg.exe` before a run: `ensureBinaries({vendorDir, signal, onStatus}) -> Promise<{ok, missing:[]}>` — yt-dlp.exe direct fetch; ffmpeg.zip fetch → PowerShell `Expand-Archive` pick `bin\ffmpeg.exe`/`ffprobe.exe`; partial/error → `{ok:false}`; slots guarded (custom), progress text events (no fake %: byte count when possible).

- [ ] **Step 1:** Failing test on `binariesFetchPlan` URLs (exact verbatim URLs: `https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe`, `https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip`); slot rejection when fetch in flight.
- [ ] **Step 2:** FAIL check.
- [ ] **Step 3:** Implement `lib/binaries-fetch.js`; wire in `main.js` before the mp4/mp3 `runBinary` (after kind/host checks): missing → await `ensureBinaries` with status events; still missing after → error pointing at retry.
- [ ] **Step 4:** PASS.
- [ ] **Step 5:** Commit `feat: automatic vendor binary fetch`.

### Task 3: Error mapping + copy + README

**Files:**
- Modify: `lib/progress-parser.js` (`humanizeError` moves message logic usable for platform errors), `main.js`, `renderer/index.html`, `README.md`
- Test: `test/progress-parser.test.js` (add cases)

**Interfaces:**
- Consumes: parseYtDlpError plumbing.
- Produces: login-wall detection: error containing `Sign in`/`login` → host-named message ("Esse post do Instagram pediu login. O Comet não tem login — tenta outro post."); subtitle text lists sites: "YouTube, TikTok, Instagram, X, Twitch, Pinterest e mais."; README documents clone behavior (first click fetches binaries) and Instagram caveat.

- [ ] **Step 1:** failing tests: `Sign in to confirm` error → message contains 'login'; smoke unknown-site URL lands in Task 4 CLI step.
- [ ] **Step 2:** FAIL → implement → PASS (suite green).
- [ ] **Step 3:** UI copy + README.
- [ ] **Step 4:** Commit `feat: platform error mapping, copy, docs`.

### Task 4: Verify + package + push

- [ ] **Step 1:** `node --test` all green.
- [ ] **Step 2:** CLI smoke: one ts URL and one pinterest URL through exact built args; one `unknown` URL (example.com) → yt-dlp unsupported error path.
- [ ] **Step 3:** `npm run package`; launch `dist\Comet-win32-x64\Comet.exe` once; kill.
- [ ] **Step 4:** Commit + push.
