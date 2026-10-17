# 04 — VAD + ASR local (whisper.cpp)

- **Status:** CONCLUDED
- **Depende de:** 02, 03
- **Habilita:** 05, 07, 08

## Objetivo
Segmentar fala (Silero VAD) e transcrever localmente com whisper.cpp, emitindo
segmentos em streaming com timestamps e rótulo de falante (ME/THEY).

## Entregáveis (implementados)
- `src-tauri/src/domain/transcription.rs` — `Speaker`, `TranscriptSegment`,
  `SpeechSpan`, `TranscribedSpan`.
- Portas: `model_store`, `voice_activity`, `speech_to_text`, `event_publisher`,
  `session_controller`.
- `src-tauri/src/adapters/speech/whisper_model_store.rs` — download + verificação
  SHA-256 (Silero VAD com hash fixo; modelos whisper `tiny/base/small/medium`).
- `src-tauri/src/adapters/speech/silero_voice_activity.rs` —
  `SileroVoiceActivityDetector` (whisper.cpp VAD, carregado sob demanda).
- `src-tauri/src/adapters/speech/whisper_speech_to_text.rs` —
  `WhisperSpeechToText` (whisper-rs, timestamps em centissegundos → ms).
- `src-tauri/src/adapters/events/tauri_event_publisher.rs` — evento
  `transcript_segment`.
- `src-tauri/src/application/transcription_pipeline.rs` — buffer → VAD → ASR →
  eventos, com thread trabalhadora (não bloqueia o áudio).
- `src-tauri/src/application/session_service.rs` — `SessionService`
  (implementa a porta `SessionController`).
- `src-tauri/src/ipc/session_commands.rs` — `session_start`, `session_stop`,
  `session_current`.
- `src-tauri/src/{lib.rs,state.rs}` — fiação completa (model store em
  `app_data_dir/models`).
- `src-tauri/Cargo.toml` — `whisper-rs 0.16`, `ureq`, `sha2`, `uuid`.

## Critérios de aceite
- [x] Fala contínua gera texto (validado em integração real, ver abaixo).
- [x] Segmentos têm timestamps (VAD + ASR em ms).
- [x] `speaker` = ME (mic) / THEY (sistema) — definido pela `SessionService`.
- [x] Funciona offline (modelos locais; apenas o download inicial usa rede).
- [~] Refinamento de parciais → `final=true` sempre no MVP; parcial-streaming é
  melhoria futura.
- [~] Persistência dos segmentos — entra na Tarefa 05.

## Evidências
- `cargo test` → **30 testes** unitários; `clippy`/`fmt` limpos.
- **Integração real** (`cargo test -- --ignored`): `end_to_end_transcribes_jfk_speech`
  → baixou Silero VAD + `ggml-tiny`, rodou VAD + Whisper e transcreveu a fala de
  exemplo (`jfk.wav`), contendo "country". **Verificação ponta a ponta do ASR.**
- Pipeline e `SessionService` cobertos por testes com dobras (`FakeVad`,
  `FakeAsr`, `RecordingPublisher`, `FakeAudioSource`).

## Notas
- Modelo padrão de ASR: `tiny` (download ~75 MB na primeira sessão; cache em
  `app_data_dir/models`). Configurável no composition root.
- A API `whisper-rs 0.16` expõe VAD (`WhisperVadContext`), o que permitiu reusar
  o modelo Silero (`ggml-silero-v5.1.2.bin`, MIT) já previsto no PRD.
