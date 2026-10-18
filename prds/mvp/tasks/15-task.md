# 15 — Monitor atrasado do microfone ("ouvir minha voz")

- **Status:** CONCLUDED
- **Depende de:** 14
- **Habilita:** —

## Objetivo
Dar retorno **audível** ao calibrar o microfone: ouvir a própria voz com um atraso
fixo confirma na hora qual dispositivo está captando e se o nível está bom — sem
depender de números (RMS/barra) no painel de teste.

## Desenho final (e por que mudou em relação ao esboço)
| Esboço inicial | Implementado | Motivo |
|---|---|---|
| `ports/audio_player.rs` + adapter `cpal` de saída | `ports/audio_monitor.rs` + `PulseDelayedMonitor` (API simple do Pulse) | o caminho `cpal` de saída exigiria reamostrar 16 kHz → taxa do dispositivo; o servidor Pulse/PipeWire já faz remix/resample, e `libpulse-simple-binding` **já era dependência** (usada na captura do sistema) |
| ring buffer no adapter | fila de atraso **no domínio** (`DelayedAudioQueue`) | política pura → testável sem placa de som nem threads |
| monitor no caminho cru do microfone | monitor no **mesmo ponto que o ASR recebe** (depois do ganho/gate, BR-018) | ouvir exatamente o que o whisper recebe é o que ajuda a decidir a sensibilidade |
| ganho de monitor independente | sem ganho extra (o nível é o do ASR; ajuste pela “Sensibilidade do microfone”) | evita número mágico — o slider é o controle |
| atraso escolhido na UI | **400 ms** fixo, override por ambiente | um botão, não um painel de configuração |

## Entregáveis
- `domain/audio.rs` — `DelayedAudioQueue`: blocos com carimbo `pronto_em = agora +
  atraso`, `drain_ready`, teto de acúmulo (`atraso + 500 ms`, descarta o **mais
  antigo** para a voz não atrasar sem limite) e contador de descartes.
  Constante `DEFAULT_MONITOR_DELAY_MS = 400`.
- `ports/audio_monitor.rs` — porta `AudioMonitor` (`start(delay_ms)`, `push`,
  `stop`, `is_running`).
- `adapters/audio/pulse_delayed_monitor.rs` — `PulseDelayedMonitor`: thread que
  abre a saída padrão (PCM 16 bits mono 16 kHz) e escreve o que venceu,
  **silêncio** quando nada venceu (mantém o stream no ritmo do relógio de áudio).
  O erro de abertura volta **para quem chamou** (a UI avisa em vez de ficar muda);
  override `GHOSTWHISPER_MONITOR_DELAY_MS`.
- `adapters/audio/monitored_audio_source.rs` — decorador `MonitoredAudioSource` +
  `MonitorFanOut`: repassa o áudio ao pipeline e **espelha** no monitor
  (desligado = nada é reproduzido).
- `lib.rs`: microfone = `MonitoredAudioSource(MicrophoneFrontEnd(cpal))`; o
  monitor sobe desligado (nenhum som numa reunião normal).
- IPC: `audio_monitor_start(delay_ms?)`, `audio_monitor_stop`,
  `audio_monitor_is_running`, `audio_monitor_default_delay`.
- UI: toggle **“Ouvir minha voz (com 0,4 s de atraso)”** no teste de áudio, só no
  canal de microfone, habilitado apenas com o teste em execução, com aviso de
  **fone de ouvido**; desliga ao parar o teste, ao trocar para o áudio do sistema
  e ao sair da tela.

## Critérios de aceite
- [x] Ouvir a voz com atraso estável, sem realimentação **em fone**.
- [x] Toggle só no microfone e só durante o teste; saída encerrada ao parar/sair.
- [x] Nenhum ganho/efeito extra no caminho do ASR (decorador só espelha).
- [x] Nenhum log de áudio; nada de rede (BR-002).
- [x] Testes: fila de atraso (unidade), decorador (espelha/não espelha/delega),
  adapter (idle + teste real com servidor Pulse).

## Verificação (2026-10-18)
- Rust: `DelayedAudioQueue` (espera o atraso exato, ordem dos blocos, descarte do
  mais antigo no acúmulo, `clear`, vazio ignorado), `MonitoredAudioSource`
  (espelha para o monitor e repassa igual ao pipeline; desligado não reproduz;
  delegação; erro do monitor reportado), `PulseDelayedMonitor` (idle não enfileira
  nem abre saída; teste real `#[ignore]` contra o servidor Pulse — **executado**:
  liga, toca, para e zera o pendente).
- Frontend: toggle no microfone apenas, desabilitado antes do teste, atraso no
  rótulo, liga/desliga, e o monitor desliga ao parar o teste.

## Pendências
- Teste manual com fone (recomendado) e, opcionalmente, um atraso ajustável na UI.
- Se o dispositivo padrão sair durante o monitor, o stream cai (log + `is_running`
  falso); reconectar é manual (ligar de novo).
