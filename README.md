# ☄ Comet 
```
                          _        
  ___ ___  _ __ ___   ___| |_  _   
 / __/ _ \| '_ ` _ \ / _ \ __|| |_ 
| (_| (_) | | | | | |  __/ ||_   _|               -  video downloader
 \___\___/|_| |_| |_|\___|\__||_|  
 ```

Sua comunidade na internet, de player pra player — e seu downloader de confiança.

Comet baixa:

- **Vídeo** como `.mp4` (até 1080p) de **YouTube, TikTok, Instagram, X/Twitter, Twitch, Pinterest** e os demais ~1800 sites que o yt-dlp suporta.
- **Áudio** como `.mp3` (192 kbps) das mesmas fontes.
- **GIF por link** — URL direta (`media.tenor.com/.../algo.gif`) ou página do [Tenor](https://tenor.com) / [Giphy](https://giphy.com).

Primeira execução sem `vendor/` (clone do GitHub, primeiro `.exe`): o Comet baixa `yt-dlp.exe` + `ffmpeg.exe` sozinho no primeiro download.

Limitações: posts que exigem login (Instagram restringe bastante) falham com mensagem clara — o Comet não tem login por design. "GIFs" de Twitter/TikTok vêm como `.mp4` (o site entrega assim).

## Como rodar

Requisitos: Windows + Node.js.

```powershell
npm install
powershell -ExecutionPolicy Bypass -File scripts\fetch-binaries.ps1
npm start
```

Ou sem Node: rode o executável portátil em `dist\Comet-win32-x64\Comet.exe`. Gerar de novo:

```powershell
npm run package
```

Se o `.exe` reclamar de `yt-dlp`, rode `scripts\fetch-binaries.ps1` uma vez e execute `npm run package` de novo (os binários vão embutidos no pacote).

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
