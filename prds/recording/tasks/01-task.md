# 01 — Domínio `Recording`, schema e porta `ScreenRecorder`

- **Status:** PENDING
- **Depende de:** —
- **Habilita:** 02, 04, 05, 09
- **Issues:** — · **BR:** BR-065, BR-067, BR-071
- **Refs:** [`../prd.md`](../prd.md) US-02, US-03;
  [`../architecture/recording.md`](../architecture/recording.md) §2, §3, §4, §9

## Objetivo

Criar o **modelo da gravação** (um artefato por sessão), a **porta** de captura
e a **persistência**, com a tabela `recordings` e a máquina de estados.

## Entregáveis

- `src-tauri/src/domain/recording.rs`:
  - `Recording { id, session_id, kind, status, path, segments_dir, size_bytes,
    duration_ms, width, height, fps, started_at_ms, finished_at_ms, error }`;
  - `RecordingKind` (`Video`/`Audio`);
  - `RecordingStatus` (`Recording`/`Paused`/`Finalizing`/`Ready`/`Failed`/
    `Discarded`/`Interrupted`) com `as_str`/`from_str` (padrão de
    `domain/session.rs`);
  - `RecordingSpec` (config **congelada**: modo, fps, resolução, crf, preset,
    kbps, container, alvo);
  - `RecordingTarget` (`Window(id,título)` / `Screen`).
- `src-tauri/src/ports/screen_recorder.rs`: trait `ScreenRecorder`
  (`capabilities`, `start`, `pause`, `resume`, `stop`, `state`) +
  `RecordingCapabilities`, `RecordingState`, `RecordingProgress` (contrato em
  [`../architecture/recording.md`](../architecture/recording.md) §3).
- `src-tauri/src/ports/recordings_repository.rs`: `insert`, `update_status`,
  `set_artifact`, `get_by_session`, `list_active`, `discard`.
- `src-tauri/src/adapters/persistence/schema.sql`:
  `recordings` + `idx_recordings_status`; `schema_version` → **3**.
- `src-tauri/src/adapters/persistence/sqlite/local_recordings_repository.rs`.
- `src-tauri/src/adapters/persistence/memory/in_memory_recordings_repository.rs`
  (BR-013).
- `src-tauri/src/adapters/audio/…` **não** é tocado: a gravação não usa o PCM
  do ASR (BR-069).

## Critérios de aceite

- [ ] `cargo test --lib` verde com round-trip de `RecordingStatus`/`Kind`,
      serialização do `Recording` e do `RecordingSpec`.
- [ ] Sessão com gravação `ready` → `get_by_session` devolve o caminho e o
      tamanho; sessão sem gravação → `None`.
- [ ] Excluir a sessão apaga a linha (`ON DELETE CASCADE`).
- [ ] `schema_version = 3` aplicado **idempotentemente** num banco já existente.
- [ ] `RecordingStatus::Interrupted` existe e é atribuível no boot (teste da
      task 05).

## Verificação

- `cargo test --lib` · `cargo clippy` limpo

## Notas

- `path` é `None` até a finalização; `segments_dir` permite recuperar depois de
  um crash (BR-067).
- O domínio não conhece `ffmpeg`, `mkv` nem `x11grab`: só o artefato e o estado.
