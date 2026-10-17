# 02 — Captura de microfone

- **Status:** CONCLUDED
- **Depende de:** 00
- **Habilita:** 04

## Objetivo
Capturar o microfone com `cpal` e produzir **PCM f32 mono 16 kHz** contínuo,
com medidor de nível para a UI.

## Entregáveis (implementados)
- `src-tauri/src/domain/audio.rs` — `AudioDevice`, `TARGET_SAMPLE_RATE`.
- `src-tauri/src/ports/audio_source.rs` — porta `AudioSource`.
- `src-tauri/src/ports/audio_consumer.rs` — porta `AudioConsumer`.
- `src-tauri/src/adapters/audio/pcm_resampler.rs` — downmix + reamostragem linear
  para 16 kHz (funções puras, com testes).
- `src-tauri/src/adapters/audio/cpal_microphone_source.rs` — captura via `cpal`
  (thread dedicada; suporta f32/i16/u16).
- `src-tauri/src/adapters/audio/level_meter.rs` — `AudioConsumer` que emite o
  evento `audio_level` (RMS/pico, ~10 Hz).
- `src-tauri/src/ipc/audio_commands.rs` — `audio_list_devices`, `audio_start`,
  `audio_stop`, `audio_is_running`.
- `src-tauri/src/{lib.rs,state.rs}` — `AppState` passa a guardar `Arc<dyn AudioSource>`.
- `src-tauri/Cargo.toml` — dependência `cpal`; `profile.dev.incremental = false`.

## Critérios de aceite
- [x] Captura implementada com seleção de dispositivo (por nome) e padrão.
- [x] Pipeline entrega PCM f32 mono 16 kHz (`to_target_mono`, testado).
- [x] Medidor de nível emitido como evento `audio_level`.
- [~] "Lista de microfones na UI" e "medidor reagindo à fala" — dependem da UI
  (Tarefa 07) e de hardware de áudio (indisponível neste ambiente).
- [~] Persistência da seleção — entra na Tarefa 06 (settings).

## Evidências
- `cargo test` → **16 testes** (geometria, áudio/resampler, state, ping, error).
- `cargo clippy --all-targets` → sem warnings; `cargo fmt --check` limpo.
- `cargo build` + smoke run (`timeout 8 ./target/debug/ghostwhisper`) → `exit=124`.
- `pnpm test` / `pnpm build` ok.

## Limitações / verificação pendente
- Sem dispositivo de áudio neste host: a enumeração e o nível não puderam ser
  exercitados de ponta a ponta (validados por testes unitários e compilação).
- Reamostragem linear por bloco (sem anti-aliasing); qualidade suficiente para
  fala no MVP — melhoria futura (`rubato`).

## Testes
- Unitários: downmix (mono/estéreo), reamostragem (48k→16k, identidade, vazio),
  `to_target_mono` (estéreo 48k → 16k).
- Manual (futuro): captura real + medidor em máquina com microfone.
