# 🫟 Comet

Sua comunidade na internet, de player pra player — e seu downloader de confiança.

Comet baixa:

- **Vídeo do YouTube** como `.mp4` (até 1080p)
- **Áudio do YouTube** como `.mp3` (192 kbps)
- **GIF por link** — URL direta (`media.tenor.com/.../algo.gif`) ou página do [Tenor](https://tenor.com) / [Giphy](https://giphy.com)

## Como rodar

Requisitos: Windows + Node.js.

```powershell
npm install
powershell -ExecutionPolicy Bypass -File scripts\fetch-binaries.ps1
npm start
```

`fetch-binaries.ps1` baixa `yt-dlp.exe` e `ffmpeg.exe` para `vendor/` (git-ignored — nunca commitados).

## Atualizar yt-dlp

YouTube muda toda hora; yt-dlp acompanha. Dentro do app use o botão **Atualizar yt-dlp** (roda `yt-dlp.exe -U`), ou rode o script `fetch-binaries.ps1` de novo (ele também atualiza o ffmpeg).

## Uso

1. Cole o link.
2. Clique no formato: `MP4`, `MP3` ou `GIF`.
3. Veja o progresso; **Parar** cancela e apaga o parcial (`.part`).

Escolha **Pasta de destino** para trocar onde os arquivos caem (por padrão, `Downloads`). Arquivo existente nunca é sobrescrito: o Comet salva com sufixo numérico.

## Estrutura

```
main.js               Electron main (IPC, config, orquestração)
preload.js            ponte segura renderer ↔ main
renderer/             janela (HTML/CSS/JS puro, sem bundler)
lib/                  lógica testável de node
test/                 node --test (suite completa)
scripts/              scripts de manutenção
vendor/               binários baixados (git-ignored)
```

## Testes

```
node --test
```

## Licença

MIT — de Axolotl BR, para a internet.

Logo: o cometa é o Twemoji ☄ (`renderer/assets/comet.svg`), de
[jdecked/twemoji](https://github.com/jdecked/twemoji) —
créditos: © Twitter, X e colaboradores, licença CC-BY 4.0.
