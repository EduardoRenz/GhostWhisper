# 02 — `FfmpegScreenRecorder`: binário, argumentos e segmentos

- **Status:** CONCLUDED
- **Depende de:** 01
- **Habilita:** 03, 04, 05
- **Issues:** ISS-049, ISS-050 · **BR:** BR-066, BR-067, BR-069, BR-070
- **Refs:** [`../architecture/recording.md`](../architecture/recording.md) §5.1, §5.2, §5.6

## Objetivo

Implementar a captura real: resolver o `ffmpeg`, montar os argumentos (função
**pura**) e gravar **segmentos MKV** com vídeo da janela/tela e áudio mixado
(mic + sistema), sem tocar no pipeline do ASR.

## Entregáveis

- `src-tauri/src/adapters/recording/ffmpeg_path.rs`:
  `resolve_ffmpeg()` em três degraus — `GHOSTWHISPER_FFMPEG`, sidecar ao lado do
  executável, `PATH`. Devolve `Result<PathBuf>` com erro explicável.
- `src-tauri/src/adapters/recording/ffmpeg_args.rs` (**puro**):
  - `build_segment_args(spec, target, segment_path) -> Vec<String>`;
  - `build_video_args` / `build_audio_args` (só áudio → FLAC);
  - casos: só microfone, só sistema, ambos; VAAPI vs x264;
  - `-threads 2` sempre no x264 (RNF-01).
- `src-tauri/src/adapters/recording/ffmpeg_screen_recorder.rs`:
  - `start` → cria `parts/`, abre o processo, `-progress pipe:1`;
  - `pause` → fecha o segmento (SIGINT/`q` no stdin, espera o filho);
  - `resume` → novo segmento `part-NNNN.mkv`;
  - `stop` → fecha o último segmento e delega a finalização à task 04;
  - parse de `out_time_us` para o progresso;
  - `Drop` mata o filho (nunca deixa processo órfão).
- `src-tauri/src/adapters/recording/mod.rs`.

## Critérios de aceite

- [ ] Testes puros dos args cobrindo: vídeo 1080p/8 fps/CRF 27, somente áudio
      (FLAC), sem microfone, VAAPI, `-window_id` vs tela inteira.
- [ ] `resolve_ffmpeg` com `PATH` fake encontra o binário e, sem nenhum degrau,
      devolve `Err` com mensagem — nunca *panic*.
- [ ] Gravar 5 s gera `part-0001.mkv` reproduzível (`ffprobe` lê duração > 0).
- [ ] `pause` + `resume` gera `part-0002.mkv`; o primeiro continua íntegro.
- [ ] Nenhum teste unitário executa `ffmpeg` (RNF-10) — os de processo são
      `#[ignore]` como `captures_the_real_screen`.
- [ ] Sem `DISPLAY`, `capabilities().available == false` com o motivo de Wayland
      (BR-007/ISS-051).

## Verificação

- `cargo test --lib` · `cargo clippy` · teste manual `#[ignore]` de 5 s

## Notas

- A gravação abre um **segundo** cliente no microfone (ISS-050); se o
  `ffmpeg` falhar com dispositivo ocupado, tentar de novo só com o áudio do
  sistema e marcar `warnings` no artefato.
- O áudio é o cru: **sem** AEC, ganho ou gate (BR-069). Não reusar
  `MicrophoneFrontEnd`.
