# 04 — Finalização: concat, remux MP4 e progresso

- **Status:** PENDING
- **Depende de:** 02, 03
- **Habilita:** 05, 08
- **Issues:** ISS-053 · **BR:** BR-067
- **Refs:** [`../prd.md`](../prd.md) US-02, US-03 (CA-03.6/CA-03.7);
  [`../architecture/recording.md`](../architecture/recording.md) §5.5, §5.6

## Objetivo

Transformar os segmentos MKV num **único MP4** reproduzível, com `-c copy`
quando possível, progresso honesto e preservação total em caso de falha.

## Entregáveis

- `src-tauri/src/adapters/recording/ffmpeg_finalizer.rs`:
  - escreve `parts.txt` (`file '<abs>'`) e roda
    `ffmpeg -f concat -safe 0 -i parts.txt -c copy -movflags +faststart out.mp4`;
  - `-progress pipe:1` + `-nostats`; percentual = `out_time_us / soma_das_durações`
    (durações medidas por `Instant` no fechamento de cada segmento);
  - publica `RecordingProgress::Percent` e, no fim, `Ready`/`Failed`;
  - se `ffprobe`/o concat acusar **dimensões diferentes** entre segmentos
    (ISS-053): reencoda com
    `-vf "scale=W:H:force_original_aspect_ratio=decrease,pad=W:H:(ow-iw)/2:(oh-ih)/2"`
    e fase `"Otimizando gravação…"` na barra;
  - apaga `parts/` ao final **somente** se `keep_segments == false`.
- `src-tauri/src/adapters/recording/ffmpeg_args.rs`:
  `build_concat_args`, `build_normalize_args` (puros, testados).
- `src-tauri/src/adapters/recording/ffmpeg_path.rs`: `ffprobe` opcional
  (mesma resolução em degraus; ausência não impede o concat).
- Recuperação de órfãos: `recover_interrupted(recordings)` — concatena o que
  existir em `segments_dir` e publica `ready`/`failed`.

## Critérios de aceite

- [ ] 3 segmentos de 5 s → um MP4 de ~15 s, `ffprobe` lê H.264 + AAC.
- [ ] O MP4 toca do início ao fim (sem `moov` ausente) e abre no player do
      sistema.
- [ ] `Percent` sobe de 0 a 100 e o evento `Ready` traz `path`, `size_bytes` e
      `duration_ms`.
- [ ] Segmentos com dimensões diferentes → reencode com `scale`+`pad` e o
      arquivo final toca; a fase é reportada.
- [ ] Falha do `ffmpeg` → `Failed` com a *stderr* resumida (sem caminhos de
      áudio), `parts/` **preservado**.
- [ ] `keep_segments = true` mantém `parts/`; `false` apaga.
- [ ] Gravação de 1 h não bloqueia a UI: a finalização roda em thread própria.

## Verificação

- `cargo test --lib` (args) · teste manual `#[ignore]` com 3 segmentos reais

## Notas

- `-c copy` não reencoda: o custo é I/O (~2–5 s por hora). A barra só aparece
  depois de 800 ms (CA-03.6).
- O nome final vem do título da sessão + data, saneado para filesystem.
