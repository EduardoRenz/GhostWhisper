# 10 — Dívida de índices do schema e ancoragem do FTS por `id`

- **Status:** PENDING
- **Depende de:** —
- **Habilita:** —
- **Issues:** ISS-041, ISS-042 · **BR:** BR-058
- **Refs:** [`../issues.md`](../issues.md) ISS-041;
  [`../architecture/session-journal.md`](../architecture/session-journal.md) §2.3

## Objetivo

Corrigir dois problemas do schema atual, descobertos ao desenhar o diário:

1. **Listagens ordenadas fazem *temp b-tree*** — faltam índices **compostos**
   (`(session_id, timestamp)`), e `sessions ORDER BY started_at` faz **full
   scan**.
2. **Três índices mortos** — `idx_messages_timestamp`,
   `idx_transcriptions_timestamp` e `idx_sessions_user` (`user_id` é sempre
   `NULL`, BR-002).

E, junto, **ancorar o FTS por `id`** em vez de `rowid` (ISS-041): o
casamento atual quebra **em silêncio** se um `VACUUM` renumerar os rowids
implícitos.

## Entregáveis

- `src-tauri/src/adapters/persistence/schema.sql`:
  ```sql
  CREATE INDEX IF NOT EXISTS idx_transcriptions_session_time ON transcriptions(session_id, timestamp);
  CREATE INDEX IF NOT EXISTS idx_messages_session_time      ON messages(session_id, timestamp);
  CREATE INDEX IF NOT EXISTS idx_sessions_started_at         ON sessions(started_at DESC);

  DROP INDEX IF EXISTS idx_transcriptions_session;   -- coberto pelo composto
  DROP INDEX IF EXISTS idx_transcriptions_timestamp; -- morto
  DROP INDEX IF EXISTS idx_messages_session;         -- coberto pelo composto
  DROP INDEX IF EXISTS idx_messages_timestamp;       -- morto
  DROP INDEX IF EXISTS idx_sessions_user;             -- morto (user_id é sempre NULL)
  -- idx_sessions_status: MANTER — é usado por end_all_active
  ```
  O `schema.sql` roda em `conn.execute_batch(SCHEMA)` a cada boot, então
  `CREATE INDEX IF NOT EXISTS` + `DROP INDEX IF EXISTS` já são a migração —
  **sem framework de migração** (RNF).
- `src-tauri/src/adapters/persistence/sqlite/local_transcriptions_repository.rs`:
  a busca passa a casar por **`id`**:
  ```sql
  t.id IN (SELECT segment_id FROM transcriptions_fts WHERE transcriptions_fts MATCH ?1)
  ```
  (mesmo plano, imune ao `VACUUM` — ISS-041).
- `src-tauri/src/adapters/persistence/sqlite/local_journal_repository.rs`:
  a busca do diário casa por **`entry_id`** (a tabela nova já nasce ancorada,
  task 01).
- Teste de **conjunto de índices** em `sqlite_master` (não de plano de
  execução, que é frágil entre versões do SQLite).

## Critérios de aceite

- [ ] `EXPLAIN QUERY PLAN` de `transcriptions WHERE session_id=? ORDER BY
      timestamp` **não** mostra `USE TEMP B-TREE FOR ORDER BY`.
- [ ] O mesmo para `messages`.
- [ ] `sessions ORDER BY started_at DESC LIMIT n` usa
      `idx_sessions_started_at` (sem `SCAN sessions`).
- [ ] `idx_sessions_status` **continua existindo** e `end_all_active` ainda
      o usa.
- [ ] `idx_messages_timestamp`, `idx_transcriptions_timestamp` e
      `idx_sessions_user` **não existem** mais.
- [ ] O `DELETE` de uma sessão (cascade) **não** faz scan da tabela de
      imagens (o `idx_journal_images_entry` cobre).
- [ ] O boot num banco **antigo** (sem os índices novos) aplica tudo sem
      erro e sem perder dados.
- [ ] `cargo test --lib` verde.

## Notas

- `idx_sessions_user` foi copiado do Perssua (lá existe porque há login;
  aqui `user_id` é sempre `NULL` — BR-002). A **coluna** continua no schema
  (paridade de formato); só o índice sai.
- Os índices são **verificados com `EXPLAIN QUERY PLAN`**, não com
  asserções de plano no teste (plano muda entre versões do SQLite); o teste
  fixa o **conjunto** de índices.
- Esta task é **independente** da feature: pode entrar antes ou depois das
  demais, mas antes da 09 (que fecha a documentação).
