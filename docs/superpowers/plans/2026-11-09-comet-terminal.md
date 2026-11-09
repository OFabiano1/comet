# Comet Terminal — Plano de Implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refazer o Comet como app de terminal (Rust + ratatui): colar URL de YouTube/Twitter/TikTok → baixar com yt-dlp + ffmpeg, com fila e progresso ao vivo.

**Architecture:** Binário único Rust. Loop de eventos crossterm alimenta `App` (estado central, fila de itens). `Downloader` roda yt-dlp como processo filho (tokio) e faz parse do progresso do stdout; conversão de GIF usa ffmpeg. `Vendor` garante yt-dlp/ffmpeg no primeiro uso. `ui.rs` desenha conforme as Visual Design Guidelines.

**Tech Stack:** Rust stable, ratatui, crossterm, tokio (process/time), reqwest (bloqueante via tokio), dirs, anyhow. Sem crate de clipboard — paste vem como evento `Event::Paste` do crossterm.

**Spec:** `docs/superpowers/specs/2026-11-09-comet-terminal-design.md`

## Global Constraints

- Paleta: bg `#111114`, superfície `#19191F`, superfície elevada/borda `#292630`, acento `#8F00FF`, acento claro `#B78AFF` (constantes em `src/theme.rs`).
- Downloads em `dirs::download_dir()/Comet` (fallback: `exe_dir/downloads`).
- Binários vendor em `%LOCALAPPDATA%\comet\bin`; se exe rodar ao lado de `vendor\`, usar esse primeiro.
- Uma URL = um item da fila; downloads em sequência (um por vez).
- Erros aparecem inline na TUI (linha de status), nunca crash, nunca janela.
- Teclas: Enter adiciona URL, `d` baixa fila, `o` abre pasta, `g` alterna GIF no item selecionado, `q` sai.
- Remover por completo o código Electron legado (histórico do git preserva).

## Review Focus

1. URL colada com espaços em branco/duplicada — `detect` deve fazer trim e a fila rejeita duplicata idêntica (comportamento esperado: trim + dedup silencioso com mensagem inline). Teste no Task 2/3.
2. yt-dlp morre no meio (rede caiu) — item vai para estado `Failed` com mensagem inline, fila continua no próximo (esperado: não trava a fila). Teste no Task 5.
3. Vídeo privado/login-wall — stderr do yt-dlp mapeado para mensagem curta clara (`PRIVATE OR LOGIN REQUIRED`). Teste no Task 5.
4. Terminal estreito (<60 colunas) — layout encolhe, texto truncado com `…`, sem pânico. Teste no Task 4.
5. Falha na conversão de GIF — mantém o `.mp4` baixado e marca aviso inline (não perde o download). Teste no Task 6.

---

### Task 1: Scaffold do crate + loop de eventos + tema

**Files:**
- Create: `Cargo.toml`, `src/main.rs`, `src/theme.rs`, `src/app.rs` (esqueleto)
- Delete: nada ainda (Task 8)

**Interfaces:**
- Produces: `theme::*` (constantes `BG`, `SURFACE`, `SURFACE_ALT`, `ACCENT`, `ACCENT_LIGHT` como `Color`); `App` struct com `run_loop` mínimo; binário `comet` que abre TUI, sai com `q`.

- [ ] **Step 1: Criar `Cargo.toml`** com deps: `ratatui = "0.29"`, `crossterm = "0.28"` (feature `event-stream`), `tokio = { version = "1", features = ["rt-multi-thread", "macros", "process", "time", "fs"] }, `reqwest` (rustls), `dirs`, `anyhow`, `serde_json`. Nome do pacote: `comet`.

- [ ] **Step 2: `src/theme.rs`** — só as 5 constantes de cor da paleta acima.

- [ ] **Step 3: `src/app.rs` esqueleto** — `pub struct App { pub input: String, pub status: String, pub should_quit: bool }` + `Default`.

- [ ] **Step 4: `src/main.rs`** — setup/restauração de terminal (enable raw mode, `EnterAlternateScreen`, `EnableBracketedPaste` no início e reverso no Drop), loop: `event::poll(16ms)` → trata `KeyEvent`/`Paste` (paste substitui conteúdo do input; `q` sai; Enter/limpa input), tick → `terminal.draw(ui::draw)`. UI placeholder: header `☄ COMET` + campo de input com cursor. (O `ui::draw` completo vem no Task 4; aqui pode ser uma função provisória.)

- [ ] **Step 5: Verificar** — Run: `cargo build && cargo run` — abre TUI, paste funciona no input, `q` sai sem deixar terminal quebrado. Expected: terminal restaurado após saída.

- [ ] **Step 6: Commit** — `feat: terminal app scaffold with event loop and theme`

### Task 2: Detecção de sites (`sites.rs`)

**Files:**
- Create: `src/sites.rs`
- Modify: `src/main.rs` (declarar módulo)

**Interfaces:**
- Produces: `pub enum Site { YouTube, Twitter, TikTok }` e `pub fn detect(raw: &str) -> Option<Site>`; trim obrigatório, case-insensitive, aceita `x.com` e `twitter.com`.

- [ ] **Step 1: Testes falhando** em `src/sites.rs` (módulo `#[cfg(test)]`):

```rust
assert_eq!(detect("  https://www.youtube.com/watch?v=abc \n"), Some(Site::YouTube));
assert_eq!(detect("https://youtu.be/abc123"), Some(Site::YouTube));
assert_eq!(detect("https://x.com/user/status/123"), Some(Site::Twitter));
assert_eq!(detect("https://twitter.com/user/status/123"), Some(Site::Twitter));
assert_eq!(detect("https://www.tiktok.com/@user/video/123"), Some(Site::TikTok));
assert_eq!(detect("https://example.com"), None);
assert_eq!(detect(""), None);
```

- [ ] **Step 2: Rodar** `cargo test sites` — Expected: FAIL (função não existe).

- [ ] **Step 3: Implementar `detect`** — trim, lowercase, `host` via parse manual leve (sem crate nova): substring match em `youtube.com`/`youtu.be` → YouTube; `x.com`/`twitter.com` (exige `/status/` no path) → Twitter; `tiktok.com` (exige `/video/` ou `/photo/`) → TikTok.

- [ ] **Step 4: Rodar** `cargo test sites` — Expected: PASS.

- [ ] **Step 5: Commit** — `feat: url site detection for youtube twitter tiktok`

### Task 3: Estado do app e fila (`app.rs` completo)

**Files:**
- Modify: `src/app.rs`

**Interfaces:**
- Consumes: `sites::Site`, `sites::detect`.
- Produces:
  - `pub enum Phase { Queued, Downloading, Converting, Done, Failed }`
  - `pub struct Item { pub url: String, pub title: String, pub site: Site, pub kind: Kind, pub phase: Phase, pub pct: f64, pub size_str: String, pub got_str: String, pub error: Option<String> }`
  - `pub enum Kind { Video, Gif }`
  - `pub struct App { pub input: String, pub status: String, pub items: Vec<Item>, pub selected: usize, pub active: Option<usize>, pub completed: usize, pub should_quit: bool }`
  - `pub fn add_url(&mut self) -> Result<(), String>` — detect via `sites::detect`; inválido → `Err("INVALID URL")`; duplicata idêntica → `Err("ALREADY IN QUEUE")`; ok → push `Item { title: "…", phase: Queued, kind: Video, .. }` e limpa `input`.
  - `pub fn toggle_gif(&mut self)` — alterna `kind` do item selecionado se `phase == Queued`.
  - `pub fn summary(&self) -> String` — `"N ITEMS · M COMPLETED"` (singular correto: `1 ITEM`).

- [ ] **Step 1: Testes falhando** — `add_url` aceita YouTube e limpa input; input inválido → `Err("INVALID URL")` e input preservado; duplicata → `Err("ALREADY IN QUEUE")`; `toggle_gif` muda `Kind` só de `Queued`; `summary` → `"2 ITEMS · 1 COMPLETED"` e `"1 ITEM · 0 COMPLETED"`.

- [ ] **Step 2: Rodar** `cargo test app` — Expected: FAIL.

- [ ] **Step 3: Implementar** structs e métodos conforme assinaturas acima.

- [ ] **Step 4: Rodar** `cargo test app` — Expected: PASS.

- [ ] **Step 5: Commit** — `feat: app state with queue item phases and url intake`

### Task 4: UI final conforme guidelines (`ui.rs`)

**Files:**
- Create: `src/ui.rs`
- Modify: `src/main.rs` (chamar `ui::draw`), `src/app.rs` se necessário

**Interfaces:**
- Consumes: `theme::*`, `App`, `Item`, `Phase`.
- Produces: `pub fn draw(f: &mut Frame, app: &App)` — layout de cima p/ baixo, sem bordas em caixas.

- [ ] **Step 1: Layout** (linhas, de cima para baixo, com respiro de 1–2 linhas em branco entre seções):
  - Header: `☄ COMET` (acento), `READY`/`BUSY` (estado), `VIDEO DOWNLOADER · v1.0.0` (cinza suave `#292630` como texto).
  - `DOWNLOAD URL` (label pequena), input em linha `> …` com foco (acento claro), linha de detecção: `YOUTUBE DETECTED` + `VALID URL`/`INVALID URL` em tempo real (usa `sites::detect`).
  - `DOWNLOAD QUEUE` + itens: título (truncado com `…`), `site · ext · quality` quando houver, e status — `DONE` (verde suave? não: acento claro), barra de progresso `% · got/size` (barra de caracteres `━` fina na cor acento), `DOWNLOADING`, `CONVERTING`, `FAILED: msg`.
  - Seleção: indicador `▎` na margem esquerda do item selecionado.
  - Rodapé: resumo (`summary()`) + teclas `o OPEN FOLDER · ENTER ADD · D DOWNLOAD · G GIF · Q QUIT`.

- [ ] **Step 2: Largura** — truncar título/URL com `…` ao estourar; nada de panic com width 20. Teste unitário de helper `truncate_ellipsis(s, max)`: entrada maior/igual/max=0.

- [ ] **Step 3: Rodar** `cargo test ui` + `cargo run` — Expected: PASS nos testes; visual confere com o mockup da spec (hierarquia logo → input → fila → status; cores só para estado).

- [ ] **Step 4: Commit** — `feat: final tui per visual design guidelines`

### Task 5: Downloader (yt-dlp) com progresso

**Files:**
- Create: `src/downloader.rs`
- Modify: `src/app.rs` (mutable update de `Item`), `src/main.rs`

**Interfaces:**
- Consumes: `App.active`, `Item` fields; `vendor::ensure_binaries() -> PathBuf, PathBuf` (yt-dlp, ffmpeg) do Task 7 — **neste task, stub** `pub fn ensure_binaries() -> (PathBuf, PathBuf)` retornando `vendor/` local se existir, senão erro inline; Task 7 substitui.
- Produces:
  - `pub struct Progress { pub pct: f64, pub size_str: String, pub got_str: String }`
  - `pub fn parse_progress(line: &str) -> Option<Progress>` — parse de `[download]  42.3% of 19.30MiB at 2.10MiB/s`.
  - `pub fn parse_error(stderr: &str) -> String` — mapeia `sign in`, `private`, `login` → `"PRIVATE OR LOGIN REQUIRED"`; `Unsupported URL` → `"UNSUPPORTED URL"`; fallback: primeira linha de erro truncada.
  - `pub async fn download(item_idx: usize, app: &Arc<Mutex<App>>, ytdlp: &Path, ffmpeg: &Path) -> ()` — spawn `yt-dlp -o "%(title).80B.%(ext)s" <url>` com cwd = pasta de downloads; lê stdout linha a linha (`--newline`), atualiza `pct/size/got`; ao final renomeia/ext do arquivo real via retorno do yt-dlp (`--print after_move:filepath`); `kind == Gif` → conversão do Task 6; stderr ≠ ok → `parse_error` → `Phase::Failed`; sucesso → `Phase::Done`.
- Fluxo no `main.rs`: tecla `d` → para cada item `Queued` em sequência (loop await), marca `active`, chama `download`, incrementa `completed`.

- [ ] **Step 1: Testes falhando** para `parse_progress` e `parse_error`:

```rust
assert_eq!(parse_progress("[download]  42.3% of 19.30MiB at 2.10MiB/s ETA 00:07").map(|p| p.pct), Some(42.3));
assert_eq!(parse_progress("[download] 100% of 2.00MiB").map(|p| p.pct), Some(100.0));
assert_eq!(parse_progress("garbage"), None);
assert_eq!(parse_error("ERROR: This video is private. [youtube]"), "PRIVATE OR LOGIN REQUIRED");
assert_eq!(parse_error("ERROR: Sign in to confirm your age"), "PRIVATE OR LOGIN REQUIRED");
```

- [ ] **Step 2: Rodar** `cargo test downloader` — Expected: FAIL.

- [ ] **Step 3: Implementar** parse (regex manual por split, sem crate regex) e a função async de download conforme acima.

- [ ] **Step 4: Rodar** `cargo test downloader` — Expected: PASS.

- [ ] **Step 5: Teste real (rede)** — `cargo run`, colar `https://www.youtube.com/watch?v=jNQXAC9IVRw` (vídeo de teste do YouTube, "Me at the zoo"), `d` → progresso sobe, `DONE`, arquivo em `Downloads\Comet`. Colar URL privada → `PRIVATE OR LOGIN REQUIRED` inline, fila continua.

- [ ] **Step 6: Commit** — `feat: yt-dlp download queue with live progress and error mapping`

### Task 6: GIF real do Twitter via ffmpeg

**Files:**
- Modify: `src/downloader.rs`

**Interfaces:**
- Consumes: `Item.kind == Kind::Gif`, ffmpeg path, arquivo baixado `.mp4`.
- Produces: `pub fn gif_args(input: &Path, out_no_ext: &Path) -> Vec<String>` — gera argumentos `[-y, -i, input, -vf, "palettegen", out.png]` e depois `[-y, -i, input, -i, out.png, -lavfi, "paletteuse", out.gif]` (duas passadas); e `pub async fn to_gif(...)` que executa as duas passadas e apaga o `.mp4` e o `.png` temporário no sucesso.

- [ ] **Step 1: Teste do `gif_args`** — assert nas duas chamadas (cada uma com os flags exatos acima). Expected: FAIL → PASS conforme TDD.

- [ ] **Step 2: Implementar `to_gif`** — se ffmpeg falha: NÃO apagar o `.mp4`, marcar `Item.error = Some("GIF CONVERSION FAILED · MP4 KEPT")`, `phase = Done`. Teste com um mp4 mínimo gerado por ffmpeg local (`ffmpeg -f lavfi -i testsrc=duration=1:size=128x128 test.mp4`) executando `to_gif` e afirmando que o `.gif` existe e o `.mp4` foi removido.

- [ ] **Step 3: Teste real** — colar URL de GIF do Twitter (ex.: qualquer status marcado como GIF), `g` no item para `Kind::Gif`, `d` → resultado `.gif` abrível, animado.

- [ ] **Step 4: Commit** — `feat: twitter gif conversion via two-pass ffmpeg palette`

### Task 7: Vendor — yt-dlp/ffmpeg no primeiro uso

**Files:**
- Create: `src/vendor.rs`
- Modify: `src/main.rs`, remover stub do Task 5

**Interfaces:**
- Produces: `pub fn ensure_binaries() -> anyhow::Result<(PathBuf, PathBuf)>` — ordem: `exe_dir/vendor/yt-dlp.exe` se existir; senão `%LOCALAPPDATA%\comet\bin`; faltando yt-dlp → baixa `https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe`; faltando ffmpeg/ffprobe → baixa `https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip` e extrai com crate `zip`. Seta `App.status` com `%` durante download (via callback ou canal). Valida rodando `--version` uma vez.

- [ ] **Step 1: Testes unitários** — resolução de caminho (função pura `bin_dir() -> PathBuf` e `candidate_paths()` ordenados por prioridade): exe-local > localappdata. Sem rede nos testes.
- [ ] **Step 2: Implementar** download reqwest com streaming para arquivo + extração zip; progresso escrito em `Arc<Mutex<App>>.status`.
- [ ] **Step 3: Teste real** — apagar `%LOCALAPPDATA%\comet\bin`, `cargo run`, colar URL válida, `d` → primeiro uso baixa binários com feedback, depois baixa o vídeo.
- [ ] **Step 4: Commit** — `feat: first-run vendor fetch for yt-dlp and ffmpeg`

### Task 8: Remoção do legado + empacotamento

**Files:**
- Delete: `main.js`, `preload.js`, `package.json`, `package-lock.json`, `Comet.sln`, `build.ps1` (substituído), `renderer/`, `lib/`, `src/` (Electron), `test/`, `scripts/` (legado), `dist/`, `node_modules/` (não versionado)
- Keep: `vendor/` (não versionado, mas local), `docs/`, `LICENSE`, `README.md`
- Create: `build.ps1` (novo, Cargo), atualizar `README.md`, `.gitignore` (Rust: `/target`)

**Interfaces:**
- Consumes: tudo anterior.

- [ ] **Step 1: `git rm` nos arquivos Electron listados** — verificar `git status` limpo de versão antiga; `vendor/` e `node_modules/` já ignorados ou removidos do disco.

- [ ] **Step 2: `.gitignore`** — `/target`, `vendor/`, `dist/`.

- [ ] **Step 3: `build.ps1`** — `cargo build --release`, copia `target/release/comet.exe` para `dist/`, zipa `dist/comet-v1.0.0.zip` (exe + README).

- [ ] **Step 4: `README.md`** — novo: o que é, como rodar (``cargo run``), como buildar (``.\build.ps1``), teclas, aviso de copyright.

- [ ] **Step 5: Verificação final** — `cargo test` (todos passam), `cargo build --release`, rodar `dist/comet.exe` de fora do projeto, download de teste do YouTube.

- [ ] **Step 6: Commit** — `feat: remove electron legacy, package release build`
