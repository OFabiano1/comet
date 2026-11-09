# ☄ Comet

```
                          _        
  ___ ___  _ __ ___   ___| |_  _   
 / __/ _ \| '_ ` _ \ / _ \ __|| |_ 
| (_| (_) | | | | | |  __/ ||_   _|               -  video downloader
 \___\___/|_| |_| |_|\___|\__||_|  
```

Baixador de vídeos no terminal. Cola a URL, aperta Enter, pronto.

Sua comunidade na internet, de player pra player — e seu downloader de confiança.

## Suporta

- YouTube (`youtube.com/watch`, `youtu.be`)
- Twitter/X — vídeo **e** GIF (`.gif` real via ffmpeg)
- TikTok (`tiktok.com/@user/video/…`)

## Uso

```
comet.exe
```

Cole a URL (Ctrl+V) no campo e aperte **Enter**. Depois:

| Tecla | Ação |
|-------|------|
| `Enter` | Adiciona URL à fila |
| `D` | Baixa a fila |
| `G` | Alterna GIF no item selecionado (Twitter) |
| `O` | Abre a pasta com o arquivo do item selecionado já marcado |
| `↑` / `↓` | Move a seleção |
| `Q` / `Esc` / `Ctrl+C` | Sai |

Teclas de comando só valem com o campo de URL vazio. Para digitar a URL à
mão, cole com Ctrl+V.

GIF do Twitter: selecione o item e aperte `G` antes de `D` — o vídeo vira
`.gif` de verdade (paleta otimizada via ffmpeg), não só um mp4 renomeado.

Twitter/X e TikTok pedem cookies: o Comet usa os cookies do Chrome, Edge ou
Brave instalados na máquina (`--cookies-from-browser`). Sem isso, a
plataforma bloqueia o download.

Downloads vão para `%USERPROFILE%\Downloads\Comet\` (subpasta `Comet`, não a
raiz de Downloads). Tweet e TikTok levam o id no nome, como `Layne [1855…].mp4`,
e link no texto do post é tirado do nome.

No primeiro uso, o Comet baixa `yt-dlp` e `ffmpeg` sozinhos para
`%LOCALAPPDATA%\comet\bin` (mostra o progresso na tela). Se `vendor\`
existir ao lado do exe, ele usa os binários de lá.

## Build

Requer [Rust](https://rustup.rs).

```
.\build.ps1
```

Gera `dist\comet.exe` e `dist\comet-v<versão>.zip` (versão lida do `Cargo.toml`).

## Testes

```
cargo test              # unitários
cargo test -- --ignored # download real de rede (~90 MB de vendor)
```

## Limitações

Posts que exigem login falham com a mensagem `vídeo privado ou pede login`.
O Comet não tem login por design. GIF de Twitter/TikTok vem
como `.mp4` do site; o `G` converte para `.gif` real.

## Licença

MIT — de Axolotl BR, para a internet.

Logo: o cometa é o Twemoji ☄ ([jdecked/twemoji](https://github.com/jdecked/twemoji)) —
créditos: © Twitter, X e colaboradores, licença CC-BY 4.0.
