# 23 — Indicador de fala da aba não fica preso

- **Status:** CONCLUDED
- **Depende de:** — · **Issues:** ISS-084 · **BR:** BR-033 (revisada)
- **Refs:** [`../issues.md`](../issues.md) ISS-084

## Objetivo

Apagar o selo **Streaming**/gradiente da aba quando não há fala sendo
processada, inclusive ao pausar/encerrar.

## Entregáveis

- `publish_activity` exige fala recente (`ACTIVITY_IDLE_TIMEOUT = 2 s`),
  decodificação em voo ou final pendente; `tick` reavalia a cada 50 ms.
- `useTranscriptActivity(sessionId, enabled)` zera ao pausar/parar; o
  `PanelWindow` passa `state.listening && !state.paused`.
- Testes:
  - `activity_signal_expires_without_recent_speech`;
  - `transcript_activity_store_tracks_the_channel_until_the_final`;
  - hook “zera o sinal quando a sessão pausa ou para”.

## Critérios de aceite

- [x] Sem fala recente, o sinal apaga.
- [x] Pausar/encerrar zera o sinal na UI.
- [x] Fala em processamento continua acesa.
- [x] `cargo test`, `pnpm test`, clippy/fmt/tsc verdes.

## Resultado (2026-10-28)

- Timeout no pipeline + reset no hook; testes verdes.
