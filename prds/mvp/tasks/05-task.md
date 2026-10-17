# 05 — Persistência SQLite

- **Status:** CONCLUDED
- **Depende de:** 04 (produtores de dados)
- **Habilita:** 06, 07

## Objetivo
Persistir sessões e transcrições em SQLite com busca full-text (FTS5), usando o
schema recuperado do Perssua como base e migrações versionadas.

## Entregáveis (implementados)
- `src-tauri/src/domain/session.rs` — `Session`, `SessionStatus`.
- `src-tauri/src/ports/repositories.rs` — `SessionRepository`,
  `TranscriptionsRepository`; `src-tauri/src/ports/clock.rs` — `Clock`.
- `src-tauri/src/adapters/system/system_clock.rs` — `SystemClock` (+ `FixedClock` de teste).
- `src-tauri/src/adapters/persistence/schema.sql` — tabelas
  `sessions`/`messages`/`transcriptions` + FTS5 (`messages_fts`,
  `transcriptions_fts`) + triggers + índices + `schema_version`.
- `src-tauri/src/adapters/persistence/sqlite/connection.rs` — abertura,
  schema e conexão compartilhada.
- `src-tauri/src/adapters/persistence/sqlite/local_session_repository.rs` e
  `local_transcriptions_repository.rs` — implementações em SQLite.
- `src-tauri/src/adapters/persistence/memory/*` — dobras de teste em memória.
- `src-tauri/src/application/{session_service,transcription_pipeline}.rs` —
  persistem sessão (start/end) e cada segmento.
- `src-tauri/src/ipc/database_commands.rs` — `db_list_sessions`,
  `db_get_session`, `db_session_transcriptions`, `db_search_transcriptions`.
- `src-tauri/src/error.rs` — variante `Database` + `From<rusqlite::Error>`.
- `src-tauri/Cargo.toml` — `rusqlite` (`bundled`, com FTS5).

## Critérios de aceite
- [x] DB criado no primeiro uso; schema idempotente (`schema_version`).
- [x] Transcrições associadas à sessão correta.
- [x] Busca FTS5 retorna resultados.
- [x] Sessão encerrada é marcada como `ended` com `ended_at`.

## Evidências
- `cargo test` → **33 testes** (inclui CRUD de sessões e insert/list/search de
  transcrições com FTS5 em SQLite in-memory).
- `cargo clippy`/`cargo fmt` limpos.
- Smoke run: app criou `~/.local/share/com.ghostwhisper.desktop/ghostwhisper.db` com todas
  as tabelas + tabelas-sombra do FTS5 e `schema_version = 1`.
- `pnpm build` ok.

## Notas / decisões
- A tabela `messages` faz parte do schema, mas o repositório de mensagens foi
  **adiado** (sem chat no MVP seria dead code). Reintroduzido junto do chat.
- O SQLite bundled **aplica FKs**: os testes inserem sessões antes de
  transcrições.
- Colunas extras em `transcriptions` (`end_ms`, `is_final`) e uma tabela FTS
  dedicada (`transcriptions_fts`) foram adicionadas ao schema original.
