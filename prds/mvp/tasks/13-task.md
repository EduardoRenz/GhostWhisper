# 13 — Streaming de transcrição em tempo real (decisão desacoplada + ASR robusto)

- **Status:** CONCLUDED
- **Depende de:** 04, 11
- **Habilita:** 14 (front-end de áudio do Perssua, BR-018)

## Objetivo
O MVP não estava bom o suficiente: a transcrição não acompanhava a fala em tempo
real. Fechar a lacuna entre o comportamento observado no Perssua e o nosso,
**medindo antes de mexer**, e registrar o que ficou para os próximos ciclos.

Spec: [`architecture/transcription-streaming.md`](../architecture/transcription-streaming.md).

## Entregáveis
- `prds/mvp/architecture/transcription-streaming.md` — tech spec: extração do
  Perssua, curva de RTF medida, decisões e invariantes.
- `src-tauri/src/application/turn_scheduler.rs` — decisão **pura**
  (`TurnState`/`Decision`/`decide`) + `TurnTimings` com override por env.
- `src-tauri/src/application/transcription_pipeline.rs` — thread de decodificação
  própria (`dispatch`), aplicação do resultado por turno, fallback de persistência
  e telemetria de RTF em `info`.
- `src-tauri/src/adapters/speech/whisper_speech_to_text.rs` — thread de trabalho
  única com fila que coalesce prévias, `deadline`, timeouts por qualidade e
  `min(4, núcleos)` threads.
- `src-tauri/Cargo.toml` — `[profile.dev.package.whisper-rs-sys] opt-level = 3`.
- `application/session_service.rs` — pipeline com `TurnTimings::from_env()`.
- `prds/business-rules.md` — BR-018 (defaults de áudio do Perssua, Fase 2).
- `prds/mvp/issues.md` — ISS-013, ISS-014 (FIXED).

## Critérios de aceite
- [x] `tick()` nunca bloqueia numa decodificação lenta (ASR de 400 ms).
- [x] Fim de turno não espera a prévia em voo; resultados são aplicados por turno.
- [x] Falha transitória do ASR não consome a cadência de prévias nem gera hot loop.
- [x] `Final` nunca é descartado pelo coalescing; `stop()`/`flush()` o aguardam.
- [x] Custo por chamada medido e registrado; teto do turno abaixo do penhasco de 30 s.
- [x] `cargo test` 143 ✓ · 7 `ignored` (benchmarks + E2E) · `clippy`/`fmt` ✓ ·
      `pnpm test` 22 ✓ · `tsc`/`pnpm build` ✓.
- [x] E2E real em release: `end_to_end_transcribes_jfk_speech` e
      `end_to_end_streams_preview_before_final` (prévia durante a fala + final) ✓.

## Verificação
- `cargo test --lib` — 143 testes (10 do scheduler + 2 regressões de streaming).
- `cargo test --release --lib -- --ignored --nocapture benchmark_` — curvas de RTF.
- `cargo test --release --lib -- --ignored end_to_end` — fluxo real com o modelo.

## Pendências (decisões já tomadas, próximo ciclo)
- **Fase 2:** front-end de áudio do Perssua (ganho 8×, noise gate 0.025/600 ms,
  VAD 500/160/1000) — BR-018.
- **Fase 3:** `initial_prompt` (`whisper_prompt`) e feature opcional `vulkan`.
- **REA:** noise gate (`N.Kv`), AEC (`se.eo`) e `whisper_speed = 2`.
