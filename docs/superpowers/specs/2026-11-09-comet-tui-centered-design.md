# Comet TUI — Layout Centralizado (v1.0.1)

Data: 2026-11-09

## Intent

Trocar o layout ancorado no canto superior esquerdo (largura total do
terminal) por uma composição **centralizada, compacta e com presença
visual** — um pequeno app flutuando no meio do terminal. Sem mudar
arquitetura, download, atalhos ou comportamento.

## Escopo

- Só `src/ui.rs` (e versão em `Cargo.toml`/`build.ps1` se necessário).
- **Não** tocar `downloader.rs`, `vendor.rs`, `sites.rs`, `app.rs`, `main.rs`.

## Layout

Coluna de conteúdo com largura `min(64, largura_terminal - 4)` (mínimo 20),
centralizada horizontal e verticalmente como um bloco.

```
            ☄ COMET
      VIDEO DOWNLOADER · v1.0.1
               READY

        DOWNLOAD URL
     ┌────────────────────────────┐
     │ https://youtu.be/…       ↵  │
     └────────────────────────────┘
      YOUTUBE · TWITTER/X · TIKTOK

        DOWNLOAD QUEUE
            0 ITEMS
        Nothing here yet
   Your downloads will appear here

   ENTER Add · D Download · G GIF · Q Quit
```

## Regras

1. Logo, versão e status centralizados; versão vem de `CARGO_PKG_VERSION`
   (nada hardcoded).
2. Input com borda roxa sutil (`Rounded`, `ACCENT`), placeholder
   `Paste your link here...` e símbolo `↵` na borda direita.
3. Linha auxiliar: com input vazio mostra as plataformas suportadas; com URL
   mostra detecção (`YOUTUBE DETECTED · VALID URL` / `INVALID URL`).
4. Fila: cabeçalho `DOWNLOAD QUEUE` + contador (`N ITEMS`); item selecionado
   marcado com `▎`; 3 linhas por item (título/progresso, meta, chip).
5. Empty state: `Nothing here yet` + `Your downloads will appear here`,
   alturas justas — sem vão gigante.
6. Footer compacto e centralizado: `ENTER Add · D Download · G GIF · Q Quit`.
   A tecla `O` (abrir pasta) continua funcionando; não entra no footer
   (mockup define os 4 atalhos exibidos).
7. Espaçamento: 1 linha em branco entre seções; em terminal alto (`>= 16`
   linhas) mantém respiro, em terminal baixo remove as linhas em branco.
8. Sem overflow horizontal: tudo truncado com `…` dentro da coluna.
9. Cursor fica dentro do campo de input, depois do texto. URL longa mostra o
   **final** (`tail_ellipsis`) — o cursor nunca some.
10. Progresso do download alinhado à direita da linha do título.

## Decisões minhas (corrigíveis)

- **Largura 64**: cabe título + progresso lado a lado sem cortar.
- **Centralização vertical do bloco inteiro**: com a fila cheia o bloco
  cresce para cima e para baixo; quando não cabe, ancora no topo e a fila
  rola.
- **Logo**: `☄ COMET` em accent bold, com respiro de 1 linha. Sem ASCII art
  (pesa demais para 64 colunas).

## Testes

- `content_width` e `centered_rect` (funções puras) — larguras 80/40/20.
- `tail_ellipsis` mantém o final do texto.
- Render 80x24: logo centralizado, placeholder, plataformas, empty state,
  contador, footer; **coluna 0 vazia** (prova da centralização).
- Render com item: título e progresso visíveis.
- Render 20x8 e 10x4: sem panic.
