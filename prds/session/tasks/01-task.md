# 01 — Domínio do diário, porta `JournalRepository` e schema

- **Status:** CONCLUDED
- **Depende de:** —
- **Habilita:** 03, 08
- **Issues:** ISS-041 (parcial) · **BR:** BR-052, BR-058
- **Refs:** [`../prd.md`](../prd.md) US-01, US-06;
  [`../architecture/session-journal.md`](../architecture/session-journal.md) §2, §4.1

## Objetivo

Criar o **modelo do diário** (a entrada de uma pergunta e sua resposta) e a
porta de persistência, com as **tabas novas** `journal_entries` +
`journal_images` e o **FTS5** ancorado por `id`.

## Entregáveis

- `src-tauri/src/domain/journal.rs`:
  - `JournalEntry { id, session_id, question, answer, asked_at_ms,
    answered_at_ms, status, request_source, provider, model, error }`;
  - `JournalStatus` (`Pending`/`Answered`/`Error`/`Cancelled`) com
    `as_str`/`from_str` (mesmo padrão de `domain/message.rs`);
  - `JournalRequestSource` (`Search`/`Preset`/`Screenshot`);
  - `JournalImage { id, entry_id, mime, data, width, height }`.
- `src-tauri/src/ports/journal_repository.rs`: trait `JournalRepository`
  (contrato completo em [`../architecture/session-journal.md`](../architecture/session-journal.md) §4.1).
- `src-tauri/src/adapters/persistence/sqlite/local_journal_repository.rs`:
  - `insert_question`, `complete_answer`, `fail`, `get`, `list_by_session`,
    `search`, `attach_image`, `images_of`;
  - a **busca** escapa a query do FTS (ISS-042) — o escape nasce ** aqui**,
    não na correção do legado.
- `src-tauri/src/adapters/persistence/memory/in_memory_journal_repository.rs`:
  dobra de teste (BR-013, R8).
- `src-tauri/src/adapters/persistence/schema.sql`:
  - `journal_entries` (com `rowid_pk INTEGER PRIMARY KEY AUTOINCREMENT` +
    `id TEXT NOT NULL UNIQUE`), `journal_images`, `journal_fts` e os
    triggers `journal_ai`/`journal_ad`/`journal_au`;
  - `CREATE INDEX idx_journal_session_time` e `idx_journal_images_entry`;
  - `schema_version` → **2**.
- `src-tauri/src/lib.rs`: registrar os adaptadores no composition root
  (único lugar — R2).

## Critérios de aceite

- [x] `cargo test --lib` verde, com testes do domínio (round-trip de
      `JournalStatus`, serialização) e do repositório (inserir → listar em
      ordem; pergunta sem resposta; busca acha por pergunta **e** por
      resposta; cascade apaga as imagens ao excluir a sessão).
- [x] A busca com texto que **quebraria** o FTS (`"aspas"`, `a*b`, `x:y`)
      **não** derruba a consulta (ISS-042).
- [x] `EXPLAIN QUERY PLAN` da listagem do diário **não** mostra
      `USE TEMP B-TREE FOR ORDER BY` (índice composto).
- [x] O schema é aplicado **idempotentemente** num banco já existente
      (boot de um app que já rodou) sem erro.

## Verificação

- `cargo test --lib` 492 ✓ · `cargo clippy` limpo · `tsc` ✓

## Notas

- A entrada é **persistida antes** da chamada à IA (RNF-06): um crash no
  meio deixa a entrada `pending` no diário.
- `journal_fts` casa por `entry_id` (chave estável) e não por `rowid` do
  FTS — é a lição do ISS-041, nascida com a tabela.
- A tabela `messages` **não** é tocada: as sugestões da IA continuam lá
  (BR-020) e a linha temporal da aba Transcrição fica intacta (BR-040).
