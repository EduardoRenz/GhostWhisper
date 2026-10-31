# 25 — Sistema sem `pactl` + aviso de canal indisponível

- **Status:** CONCLUDED
- **Depende de:** 20 (troca de canais), 14 (áudio do sistema)
- **Habilita:** captura de sistema em PipeWire puro (sem `pulseaudio-utils`)
- **Issues:** ISS-087 · **BR:** BR-018, BR-019

## Objetivo

O usuário relatou: *"iniciei o programa e comecou só capturando meu mic. ao
tentar mudar para 'ambos' nao mudava, e nem dava alguma mensagem de feedback em
caso de erro"*. O ambiente tem PipeWire + `pipewire-pulse`, mas **não tem o
binário `pactl`** (pacote `pulseaudio-utils`). O canal do sistema falhava em
silêncio e a sessão ficava só no microfone.

## Causa

- `PulseSystemAudioSource::list_devices`/`default_monitor_name` dependiam de
  `pactl` (`get-default-sink`, `list short sources`) sem fallback.
- `open_channel` devolvia `None` e `set_channels` mantinha o microfone (política
  correta da task 20), mas retornava `Ok`; a UI não tinha como saber.
- `Simple::new` era aberto dentro da thread de captura: um erro de fonte
  aparecia tarde, só no log.

## Entregáveis

- **Descoberta sem `pactl`** em `pulse_system_audio_source.rs`:
  - `default_sink_name`: `pactl` → `wpctl inspect @DEFAULT_AUDIO_SINK@` →
    `pw-metadata -n default`;
  - `list_devices`: `pactl` quando disponível; senão sinks do
    `pw-cli ls Node` → `<sink>.monitor` (ou o monitor padrão).
- **Conexão síncrona:** `Simple::new` em `start`; erro chega ao chamador.
- **Feedback:** `SessionWarningEvent` na porta `EventPublisher`, evento
  `session_warning` no adaptador Tauri, emitido por `SessionService::open_channel`
  quando microfone/sistema não abrem. `useSessionState` traduz (pt/en) e o
  overlay mostra o aviso; os canais que funcionam continuam.
- **Testes:** parsers `wpctl`/`pw-metadata`/`pw-cli`; testes ignorados reais
  (`lists_monitors_on_real_server`, `opens_the_default_monitor_on_real_server`);
  `start_warns_about_the_channel_that_could_not_capture` e o caso parcial de
  `set_channels`; `useSessionState` para o evento.

## Critérios de aceite

- [x] Em PipeWire sem `pactl`, o app descobre e abre o monitor padrão.
- [x] Trocar para "Ambos" com o sistema indisponível mantém o mic e mostra o
  aviso (não fecha em silêncio).
- [x] Iniciar com Ambos e falhar o sistema também avisa.
- [x] Sem regressão na descoberta via `pactl`/PulseAudio.
- [x] `cargo test`, `clippy`, `rustfmt`, `pnpm test`, `tsc`.

## Verificação (2026-10-30)

- No host sem `pactl`: `lists_monitors_on_real_server` ✓ e
  `opens_the_default_monitor_on_real_server` ✓ (libpulse abriu
  `<sink>.monitor`).
- Rust: **882 ✓** (12 ignorados) · clippy `-D warnings` ✓ · rustfmt ✓.
- Frontend: **543 ✓** (82 arquivos) · `tsc --noEmit` ✓.
- O aviso aparece no overlay com a mensagem localizada; o microfone continua
  captando.
