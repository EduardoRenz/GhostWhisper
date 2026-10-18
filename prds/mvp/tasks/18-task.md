# 18 — Exportar a sessão em Markdown (com a sugestão da IA e o tempo)

- **Status:** CONCLUDED
- **Depende de:** 16, 17
- **Habilita:** —

## Objetivo
Exportar uma sessão como um documento **Markdown da linha temporal** — inclusive a
**mensagem especial da sugestão da IA, com o tempo** em que foi pedida. É um
diferencial pedido pelo usuário: o Perssua só oferece "copiar resumo/chat/
transcrição" (REA em `sessionDetailPage`), sem arquivo.

## Entregáveis
- `application/markdown_export.rs` — **função pura** `render_markdown(session,
  entries, now_ms)`:
  - cabeçalho: título (`subject` ou "Sessão sem título"), id curto, início, fim
    (ou "em andamento"), duração (`mm:ss`) e contagem de falas/sugestões;
  - uma linha por fala: `- **[mm:ss] Você:** texto`;
  - sugestão como **bloco destacado**: `> 🤖 **Sugestão [mm:ss]** — texto`;
  - texto multi-linha continua dentro do próprio item (lista/citação);
  - `format_local_datetime` (via `chrono`, hora local) e `format_clock` (mesmo
    formato da UI).
- IPC `session_export_markdown(id)` (devolve o texto, para copiar) e
  `session_save_markdown(id, path)` (grava e devolve o caminho final, ganhando
  `.md` quando faltar; cria o diretório-pai se preciso).
- **Diálogo nativo de "salvar como"**: `tauri-plugin-dialog` 2.8.3 (Cargo) +
  `@tauri-apps/plugin-dialog` (npm) + permissão `dialog:default`. O front escolhe o
  caminho (`save()`), o core escreve.
- UI: no detalhe da sessão — **Exportar .md**, **Copiar Markdown** (reusa
  `copyText`) e confirmação com o caminho salvo; nome sugerido
  `ghostwhisper-AAAAMMDD-HHMM-<titulo>.md`.

## Critérios de aceite
- [x] Export gera um Markdown **da linha temporal** (falas + sugestões na ordem do
      tempo), não um "dump" por tabela.
- [x] Cada item tem o tempo (`mm:ss` desde o início da sessão).
- [x] A sugestão da IA aparece como bloco especial, com o tempo em que foi pedida.
- [x] Sessão em andamento usa "em andamento" e a duração corrente.
- [x] Salvar pelo diálogo nativo; cancelar não grava nada; extensão `.md` garantida.
- [x] Também dá para copiar o Markdown para a área de transferência.
- [x] `cargo test`/`clippy`/`fmt` e `pnpm test`/`tsc`/`build` verdes.

## Verificação (2026-10-18)
- Rust: `format_clock` (0 → `00:00`, 65 s → `01:05`, 1 h → `1:02:05`, negativo),
  cabeçalho + linha temporal + bloco de sugestão, "sem título"/"em andamento",
  multi-linha preservado. (`Local`/`Duration` são do `chrono`, sem novo serviço.)
- Frontend: `HistoryPanel` exporta pelo caminho escolhido no diálogo, nome sugerido
  com data/título, `saveSessionMarkdown(id, path)` chamado, cancelar não salva,
  copiar mostra "Markdown copiado.".
- `pnpm build` ✓ (bundles `.deb`/AppImage reconstruídos na task, ver memory).

## Notas
- O Markdown é montado **no backend** a partir do banco: a UI nunca concatena texto
  (uma única implementação da linha temporal, `session_timeline.rs`).
- `chrono` entrou como dependência **direta** (`default-features = false`,
  `clock`+`std`) só para formatar data/hora local; já estava na árvore como
  transitiva.
