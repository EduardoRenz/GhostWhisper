# 11 — Streaming de transcrição estilo Perssua (whisper fluindo)

- **Status:** CONCLUDED
- **Depende de:** 04, 10
- **Habilita:** —

## Objetivo
Fazer a transcrição **fluir** em tempo real, resolvendo o ISS-010: turnos longos,
prévias esparsas e VAD caro sobre o buffer inteiro. Inputs: REA do Perssua 0.27.5
(`electron/local-whisper`) e a regra **BR-015 / R11–R13** (Perssua é a fonte de
decisão; transpor só a tecnologia).

## Entregáveis
- `application/transcription_pipeline.rs` — buffer de turno, prévias sobre o turno
  crescente (texto rolante), fechamento por silêncio.
- `adapters/speech/whisper_speech_to_text.rs` — VAD nativo do whisper.cpp +
  parâmetros de decode do Perssua.
- `adapters/speech/whisper_speech_engine_factory.rs` — contexto compartilhado.
- `ports/speech_engine_factory.rs` — `create()` devolve só o ASR.
- Remoção da porta `VoiceActivityDetector` e do adapter `SileroVoiceActivityDetector`.

## Parâmetros transpostos (valores do Perssua)
| Item | Perssua | Rust |
|------|---------|------|
| Janela/áudio mínimo | 0,75 s / 0,35 s | `PREVIEW_STEP_MS` / `MIN_WINDOW_MS` |
| Primeira prévia | 1,2 s | `INITIAL_PREVIEW_MS` |
| Ociosidade | 1 s | `IDLE_FORCE_MS` |
| Silêncio p/ fechar | 1 s | `SILENCE_MS` |
| Turno mín/máx | 0,8 s / 60 s | `MIN_SEGMENT_MS` / `MAX_TURN_MS` (55 s) |
| Teto do buffer | 20 s | `MAX_BUFFER_MS` (60 s) |
| VAD | nativo (`--vad …`) | `enable_vad` + `WhisperVadParams` |
| Decode | temp 0 / inc 0 / entropy 2.2 / suppress_nst | idem |

## Critérios de aceite
- [x] Primeira prévia com ≥ 1,2 s de fala; prévias a cada ~0,75 s.
- [x] Texto do turno **cresce** (mesmo id), não "pisca".
- [x] Turno fecha após 1 s de silêncio (final beam) e é persistido.
- [x] Fechamento também pelo fim de fala do VAD do whisper (ruído de fundo).
- [x] Sem VAD Silero separado sobre o buffer inteiro.
- [x] Um contexto whisper compartilhado entre os canais.
- [x] `cargo test`/`clippy`/`fmt` e `pnpm test` limpos.

## Verificação
- `cargo test` → **114 testes** (10 de streaming reescritos/novos); 4 `ignored`.
- `pnpm test` → **18**; `clippy` 0 warnings; `fmt --check` limpo.
- E2E real `end_to_end_transcribes_jfk_speech` mantido (`--ignored`, requer rede).
- Pacotes reconstruídos via `scripts/package-linux.sh deb,appimage`:
  `.deb` **8,08 MiB** e AppImage **85,95 MiB**.

## Pendências
- Validação manual com áudio real (mic + sistema) e medição de RTF nos logs
  (`RUST_LOG=ghostwhisper=debug`).
