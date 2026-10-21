# 06 — IPC, eventos e `SessionEndDto`

- **Status:** CONCLUDED
- **Depende de:** 05
- **Habilita:** 07, 08
- **Issues:** — · **BR:** BR-065, BR-068
- **Refs:** [`../prd.md`](../prd.md) US-01 (CA-01.8), US-03 (CA-03.8);
  [`../architecture/recording.md`](../architecture/recording.md) §7.1, §7.2

## Objetivo

Expor o estado da gravação para a UI (evento, não polling) e devolver o **resumo
da sessão encerrada** — o payload do modal.

## Entregáveis

- `src-tauri/src/ipc/recording_commands.rs`:
  - `recording_start(session_id?, target?)`, `recording_pause`,
    `recording_resume`, `recording_stop`, `recording_state`,
    `recording_capabilities`, `recording_discard`,
    `recording_open_folder`;
  - `recording_state` devolve `{ status, kind, target_label, elapsed_ms,
    size_bytes }`.
- `src-tauri/src/ipc/session_commands.rs`:
  - `session_stop` passa a devolver `SessionEndDto` (ao invés de `()`);
  - `session_end_context` / `session_end_clear` (padrão `capture_pending`).
- `src-tauri/src/ports/event_publisher.rs`:
  - `RecordingStateEvent { status, kind, session_id, target_label, elapsed_ms }`;
  - `RecordingProgressEvent { phase, percent, message }`;
  - defaults no-op para não quebrar fakes (padrão de
    `emit_transcript_activity`).
- `src-tauri/src/adapters/events/tauri_event_publisher.rs`: eventos
  `recording_state` e `recording_progress`.
- `src/adapters/ipc/recording.ts` + `src/core/domain/recording.ts`.
- `src-tauri/src/lib.rs`: registrar os comandos no `invoke_handler`.

## Critérios de aceite

- [ ] O frontend recebe `recording_state` em cada transição (start/pause/
      resume/finalizing/ready/failed) e **não** precisa de polling.
- [ ] Fechar/abrir o overlay mantém o botão vermelho (estado do backend).
- [ ] `session_stop` devolve `SessionEndDto` com duração, contagem de falas e
      sugestões e `recording: Option<…>`.
- [ ] `session_delete` e a limpeza de boot **não** chamam `session_end_open`
      (CA-03.8).
- [ ] `recording_stop` durante `finalizing` é idempotente.
- [ ] Eventos de sessão antiga são ignorados pela UI (mesmo cuidado do
      `TranscriptActivityEvent`).

## Verificação

- `cargo test --lib` (serialização dos DTOs) · `pnpm test` (hook consumindo
  evento) · `cargo clippy`

## Notas

- O modal é uma janela própria; o payload é guardado em `AppState` e buscado no
  mount (mesmo ciclo do `capture`).
- Nada de áudio ou frame no payload (RNF-08).
