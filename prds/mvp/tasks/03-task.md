# 03 — Captura de áudio do sistema

- **Status:** CONCLUDED
- **Depende de:** 00, 02 (compartilha normalização)
- **Habilita:** 04

## Objetivo
Capturar o **áudio do sistema** (monitor de saída PipeWire/PulseAudio) e fornecer
um fallback de **sink virtual** quando não houver monitor.

## Entregáveis (implementados)
- `src-tauri/src/ports/virtual_sink.rs` — porta `VirtualSinkManager`.
- `src-tauri/src/adapters/audio/pactl.rs` — execução do `pactl` + parse do índice
  de módulo (testado).
- `src-tauri/src/adapters/audio/pactl_sink_helper.rs` — `PactlVirtualSinkManager`
  (cria/remove `module-null-sink`).
- `src-tauri/src/adapters/audio/pulse_system_audio_source.rs` —
  `PulseSystemAudioSource`: lista monitores (`pactl list short sources`) e grava
  do monitor via `libpulse-simple` (servidor remixa para mono 16 kHz).
- `src-tauri/src/ipc/audio_commands.rs` — `system_audio_list_devices`,
  `system_audio_start`, `system_audio_stop`, `system_audio_is_running`,
  `system_sink_create`, `system_sink_remove`.
- `src-tauri/src/{lib.rs,state.rs}` — `AppState` ganhou `system_audio` e
  `virtual_sink`.
- `src-tauri/Cargo.toml` — `libpulse-binding`, `libpulse-simple-binding`.

## Critérios de aceite
- [x] Captura do monitor (áudio do sistema) funcional em PipeWire/PulseAudio.
- [x] Sink virtual criado/removido via `pactl` (`module-null-sink`).
- [x] Nenhum driver de kernel instalado (BR-008).
- [~] Seleção do monitor na UI — entra nas Tarefas 06/07 (settings).

## Evidências
- `cargo test` → **21 testes** unitários passam; `clippy`/`fmt` limpos.
- **Integração real** (`cargo test -- --ignored`), servidor PipeWire ativo:
  - `lists_monitors_on_real_server` → ok (monitores listados + padrão marcado).
  - `captures_default_monitor_when_audio_plays` → ok (tocou tom via `paplay` e
    capturou amostras do monitor). **Verificação ponta a ponta de captura.**
- `pnpm test` / `pnpm build` ok.

## Notas
- A gravação usa a API *simple* do Pulse; o servidor faz remix para mono 16 kHz
  (sem custo de CPU no cliente). Se um ambiente recusar esse spec, cair para
  reamostragem no cliente (`pcm_resampler`) é melhoria futura.
- `parec`/`pw-record` não são necessários (usamos libpulse).
