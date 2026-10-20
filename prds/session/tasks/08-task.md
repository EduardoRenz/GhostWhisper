# 08 — Pesquisa no diário e perguntas no export Markdown

- **Status:** CONCLUDED
- **Depende de:** 01, 04
- **Habilita:** —
- **Issues:** ISS-042 · **BR:** BR-016, BR-020, BR-052
- **Refs:** [`../prd.md`](../prd.md) US-06, US-07;
  [`../architecture/session-journal.md`](../architecture/session-journal.md) §2.3, §9

## Objetivo

Fechar o ciclo do diário: **pesquisar** o que foi perguntado e **levar as
perguntas para o export** da sessão.

## Entregáveis

- `src-tauri/src/adapters/persistence/sqlite/local_journal_repository.rs`:
  - `search` com o **escape** do FTS (ISS-042): a query do usuário vira
    termo literal (`"` → `""`, embrulhada em aspas) — o Perssua passa a
    string crua e **quebra** (ver [`../issues.md`](../issues.md) → ISS-042).
- `src-tauri/src/ipc/journal_commands.rs`: `journal_search { session_id,
  query, limit }`.
- `src/features/journal/JournalView.tsx`: campo de busca com **debounce de
  180 ms** (paridade com o `AllSessionsPage` do Perssua); a busca cobre
  **pergunta e resposta**, dentro da **sessão atual**; entradas sem resposta
  também são achadas (pela pergunta).
- `src-tauri/src/application/markdown_export.rs`: seção **"Perguntas"** no
  export Markdown da sessão, com pergunta, resposta e instante — montada no
  **core**, nunca na UI (BR-020, BR-016).
- `src-tauri/src/application/session_timeline.rs`: **não** muda — a linha
  temporal da aba Transcrição continua sendo falas + sugestões (BR-040).

## Critérios de aceite

- [x] Buscar por palavra que está **só na resposta** acha a entrada.
- [x] Buscar por palavra que está **só na pergunta** acha a entrada.
- [x] Buscar por texto com **aspas/asterisco/dois-pontos** **não** derruba a
      consulta nem a tela (ISS-042).
- [x] A busca é **limitada à sessão atual**.
- [x] A busca não dispara a cada tecla (debounce 180 ms).
- [x] O export Markdown traz a seção **Perguntas**, na ordem, com o
      instante; e o export **sem** perguntas não cria a seção vazia.
- [x] `cargo test --lib` e `pnpm test` verdes.

## Verificação

- `cargo test --lib` ✓ (export com a seção "Perguntas") · busca com escape (ISS-042)

## Notas

- A busca do Perssua é o **modelo** (`getSessionsPage`), mas com **dois
  defeitos** que não são copiados: o `MATCH` sem escape e a ordenação só por
  recência (sem ranking). Aqui a ordenação é por instante da pergunta.
- O FTS é **por sessão**: o aggregate entre sessões continua sendo o
  Histórico (BR-052).
