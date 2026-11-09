# Comet Terminal — Design

Data: 2026-11-09

## Intent

Refazer o Comet do zero como aplicativo de **terminal com UI bonita** (TUI).
O usuário cola uma URL (YouTube, vídeo do Twitter/X, GIF do Twitter/X, TikTok)
e o Comet baixa o arquivo. Substitui por completo a versão Electron atual do
repositório (o histórico do git preserva a versão antiga).

## Por quê

- A versão Electron depende de node_modules e janela de browser; o usuário
  quer algo leve, nativo e direto: cola link → baixa.
- TUI roda em qualquer terminal Windows, abre instantâneo.

## Goals

- Colar URL no campo de entrada e apertar Enter para baixar.
- Detectar automaticamente o site: YouTube, Twitter/X (vídeo **e** GIF), TikTok.
- Fila de downloads com progresso ao vivo (barra, %, tamanho).
- GIF do Twitter vira `.gif` real via ffmpeg (paleta + filters).
- UI bonita: tema escuro minimalista, logo `☄ COMET` (ver *Visual Design Guidelines*).
- Exe standalone leve; yt-dlp e ffmpeg são obtidos no primeiro uso
  (padrão já usado pela versão anterior — commit "automatic vendor binary fetch").
- Respeitar copyright e permissões do conteúdo baixado (memória de projeto).

## Non-goals

- Login/cookies (conteúdo atrás de wall não é suportado).
- Playlists/álbums em lote (uma URL = um item; múltiplas URLs podem ser
  coladas em sequência na fila).
- Watch de clipboard em segundo plano — o usuário cola conscientemente.
- Busca/navegação dentro do app.

## Anti-goals

- Nada de Electron, browser embutido ou node_modules.
- Nada de interface gráfica de janela (WPF etc.) — a partir de agora o Comet
  é um app de terminal (pivô registrado no supermemory).

## Constraints

- Windows 10/11, terminal padrão (Windows Terminal / conhost).
- Toolchain disponível: Rust 1.99.1 (sem .NET SDK, sem Go, sem Bun).
- Binários vendor na máquina: `vendor/yt-dlp.exe` (~23 MB),
  `vendor/ffmpeg.exe` (~105 MB), `vendor/ffprobe.exe` (~105 MB) — usados como
  referência de versão; o exe final os busca/baixa no primeiro uso.

## Abordagens consideradas

1. **Rust + ratatui (escolhida)** — exe único ~8 MB, abre instantâneo, TUI
   mais rica do terminal (dashboards, barras ao vivo, paleta própria).
   Rust já instalado. Contra: build mais lento.
2. C# + Spectre.Console — exigia instalar .NET SDK (~200 MB); Spectre é forte
   em output estático, fraco em app interativo; single-file ~70 MB.
3. Python + Textual — dev rápido, mas PyInstaller gera exe pesado, lento e
   com falso-positivo comum de antivírus.

## Decisões tomadas (minhas chamadas, corrigíveis)

- **Stack**: Rust stable + `ratatui` + `crossterm` + `tokio` + `serde` +
  `dirs` (pasta de downloads) + `reqwest` (fetch de vendor) — sem clipboard
  crate: o paste do terminal (Ctrl+V) chega como input bracketed-paste e
  basta tratar o evento.
- **Downloads**: salvam em `%USERPROFILE%\Downloads\Comet\`. Tecla `o` abre a pasta.
- **Fila**: múltiplas URLs podem ser adicionadas antes de baixar; downloads
  rodam em sequência (um por vez, para não brigarem por banda), progresso ao vivo.
- **Detecção de site**: por padrão de URL (`youtube.com/youtu.be`,
  `x.com/twitter.com/.../status/...`, `tiktok.com/@.../video/...`). GIF do
  Twitter: pós-conversão com ffmpeg (`palettegen`+`paletteuse`) quando o
  yt-dlp indicar formato gif/motion photo.
- **Vendor no primeiro uso**: app verifica `yt-dlp.exe`/`ffmpeg.exe` ao lado
  do exe (ou em `%LOCALAPPDATA%\comet\bin`); se faltarem, baixa das fontes
  oficiais (yt-dlp GitHub releases; ffmpeg build gyan.dev essentials) com
  barra de progresso na própria TUI.
- **Erros**: URL inválida/privada/login-wall → mensagem inline na TUI, sem crash.

## Visual Design Guidelines

O Comet já tem identidade visual definida. O design faz a experiência de
"abrir, colar o link e baixar" parecer excelente — sem adicionar etapas.

### Paleta

| Cor       | Uso                                    |
|-----------|----------------------------------------|
| `#111114` | background                             |
| `#19191F` | superfície (áreas de input/fila)       |
| `#292630` | superfície elevada, bordas discretas   |
| `#8F00FF` | acento — estados e ações importantes   |
| `#B78AFF` | acento claro — hover, foco, destaques  |

Cores apenas para indicar estados e ações importantes. Nada de arco-íris.

### Hierarquia e layout (referência visual v1.0.0)

```
   ☄ COMET
   READY
   VIDEO DOWNLOADER · v1.0.0


   DOWNLOAD URL

   [ https://youtube.com/watch?v=…        ]

   YOUTUBE DETECTED · VALID URL


   DOWNLOAD QUEUE

   Video title goes here
   youtube · mp4 · 1080p
   DONE

   Another video                          64% · 12.4 MB / 19.3 MB
   youtube · mp4 · 720p
   DOWNLOADING


   2 ITEMS · 1 COMPLETED

   o OPEN FOLDER      ENTER Add URL      D Download      Q Quit
```

### Regras

1. Não parecer um terminal tradicional cheio de texto.
2. Não parecer um dashboard web dentro do terminal.
3. Usar cores apenas para indicar estados e ações importantes.
4. Animações discretas (spinner e progresso suave), sem efeitos exagerados.
5. Interface adaptável à largura do terminal.
6. Usar Unicode com fallback para terminais que não suportem determinados
   caracteres.
7. Nunca sacrificar legibilidade pela estética.

### Aplicação

- **Hierarquia**: logo, input, fila, status — nessa ordem visual.
- **Espaçamento**: bastante respiro entre seções (linhas em branco generosas).
- **Bordas**: discretas; sem caixas em tudo. Bordas só onde separam dados reais.
- **Feedback**: mensagens pequenas e claras (ex.: `YOUTUBE DETECTED`,
  `DOWNLOADING`, `DONE`).
- O Comet não é um projeto visualmente complexo: abrir, colar, baixar.

## O que fica para o builder

- Estrutura de módulos: `ui.rs` (telas/widgets), `app.rs` (estado/fila),
  `downloader.rs` (yt-dlp/ffmpeg, parse de progresso do stdout),
  `vendor.rs` (garantia de binários), `sites.rs` (detecção/normalização),
  `main.rs` (loop de eventos).
- Parse do progresso do yt-dlp via `--newline --progress-template` para
  alimentar a barra.
- Nome do arquivo: título saneado pelo yt-dlp (`-o` template), dedup `-1`, `-2`.
- Build de release: `cargo build --release` + script para zipar exe + README.
