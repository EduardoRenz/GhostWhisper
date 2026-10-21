# 06 — Commit por frase (etapa 3) — CANCELADA

- **Status:** CANCELED (implementada como opt-in, **rejeitada no teste manual** e
  **removida do código**)
- **Depende de:** 03 (reuso do final/VAD) e 04 (métrica de commit)
- **Issues:** ISS-058 · **BR:** BR-029 (sem alteração — feature não adotada)
- **Refs:** BR-015, R12, R16,
  [`../../mvp/architecture/transcription-streaming.md`](../../mvp/architecture/transcription-streaming.md) §3–§4

## O que foi tentado

Quebrar o monólogo (fala contínua sem 1 s de silêncio) em blocos de ~2–5 s: o
turno fecharia só o prefixo que termina numa **frase completa estável entre duas
prévias**, e o resto do áudio ficaria no buffer. O prefixo seria decodificado
com beam 5; a flag `GHOSTWHISPER_SENTENCE_COMMIT=1` ligava o experimento.

## Resultado do teste manual (2026-10-21)

Com a flag ligada, o usuário relatou **“um pouco melhor sem o SENTENCE COMMIT:
erra menos e parece um pouco mais rápido”**.

Motivos prováveis:
- cada commit re-decoda o prefixo e **reinicia a cadência de prévias** do resto
  (o texto rolante fica mais tempo parado — sensação de lentidão);
- mais finais competem com prévias na fila global do ASR (dois canais);
- a pontuação do whisper não é fronteira semântica 100% confiável; cortar ali
  pode custar contexto entre frases.

## Decisão

- **Não adotar.** Default permanece o fechamento por 1 s de silêncio (teto de
  25 s para fala contínua).
- **Código removido** (scheduler, pipeline, domínio, testes, envs) para não
  manter ~400 linhas paradas. O histórico fica neste arquivo e no `memory.md`.
- A meta de latência do monólogo fica para a **etapa 2** (commit otimista:
  publicar a prévia no fechamento e corrigir com o beam em background), que não
  muda a segmentação — ainda não implementada.

## Aprendizado (R16/R17)

- Mudança de **segmentação** precisa de A/B manual antes de virar default; a
  etapa 1 já havia mostrado isso.
- O teste de estabilidade entre prévias e o corte no timestamp da frase
  funcionaram como projetado; o problema é o **custo/benefício** (mais commits e
  cadência reiniciada), não a implementação.
