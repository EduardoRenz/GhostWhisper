# 21 — Filtro de loop de frase do Whisper

- **Status:** CONCLUDED
- **Depende de:** 11, 13
- **Habilita:** —

## Objetivo

Impedir que a repetição de decodificação do Whisper (mesma frase dezenas de vezes)
chegue à conversa — relatado em uso como *"às vezes um chat fica com algumas frases
repetidas, como 'O que eu quero fazer? O que eu quero fazer? …'"* (ISS-019).

## Contexto (por que o filtro é necessário)

A guarda do `whisper.cpp` descarta a sequência repetitiva só quando ela tem **mais
de 32 tokens E** entropia média abaixo de `entropy_thold`. Nossa configuração
(herdada do Perssua: `temperature = 0`, `temperature_inc = 0`,
`entropy_thold = 2.2`, `suppress_nst = true`) mata a repetição "confiante", mas a
repetição de **frase** com entropia acima do limiar passa inteira. Fechar o
`temperature_inc` (habilitar o fallback) custaria até 5 decodificações extras por
turno — por isso o filtro é **determinístico e pós-decodificação**.

## Entregáveis

- `domain/transcription.rs`:
  - `has_repeated_phrase_loop` — transposição do detector `h` do Perssua
    (`22.75b118cd.chunk.js`): frase de até 5 palavras repetida ≥3× a partir do
    início, cobrindo ≥70% do texto, mínimo 6 palavras.
  - `strip_phrase_loop` — encolhe um loop que termina o texto para **uma**
    ocorrência, preservando a fala anterior; loop de **uma palavra só** é
    descartado inteiro.
  - Constantes com os valores do Perssua (5 palavras, 6 palavras, 3 repetições,
    0.7 de cobertura) — R13; o corte do loop no fim usa um teto maior (12 palavras)
    porque exige ≥3 repetições exatas.
- `is_likely_hallucination` passa a considerar o loop de frase (antes só palavra
  repetida ≥4×).
- `application/transcription_pipeline.rs::clean` aplica `strip_phrase_loop` antes do
  filtro de alucinação (vale para prévia e final).

## Desvios conscientes (R12)

1. O Perssua só aplica o detector de loop de frase ao provider **Gemini**;
   aplicamos a **todos** porque o whisper local repete assim (evidência: este
   relato) e a guarda do `whisper.cpp` não cobre o caso.
2. O Perssua **descarta** o texto que é loop (`null`); nós **encolhemos para uma
   ocorrência** — o turno passou pelo portão de fala sustentada (houve fala real) e
   devolver vazio perderia a fala. Loop de uma palavra continua sendo descartado.

## Critérios de aceite

- [x] `"O que eu quero fazer? "` ×15 vira `"O que eu quero fazer?"` (uma vez).
- [x] Fala real + loop → fala + uma ocorrência (prévia e final).
- [x] Loop de uma palavra ("a a a a a") não vira "a".
- [x] Ênfase natural ("não não não", "muito muito muito bom") intacta.
- [x] Guardas numéricas do Perssua na detecção (≥6 palavras, ≥3 repetições, ≤5
      palavras na frase, ≥70% de cobertura).

## Verificação (2026-10-18)

- Domínio: 6 testes novos — `rejects_a_repeated_phrase_loop` (com a frase exata do
  relato), `phrase_loop_detection_keeps_the_same_guards_as_perssua`,
  `strip_phrase_loop_keeps_one_occurrence_of_the_repeated_phrase`,
  `strip_phrase_loop_drops_a_single_word_loop`,
  `strip_phrase_loop_collapses_a_repeated_short_phrase`,
  `strip_phrase_loop_keeps_natural_speech`.
- Pipeline: `repeated_phrase_loop_is_collapsed_to_one_occurrence` e
  `repeated_phrase_loop_is_cut_keeping_the_speech` (na prévia **e** no final).
- **Fala real:** `end_to_end_transcribes_jfk_speech` (27 s) e
  `end_to_end_streams_preview_before_final` (9,6 s) executados com o modelo real —
  o filtro não corta fala legítima.
- Suíte: **225 testes Rust** (+8), 8 `ignored`; clippy/fmt limpos.
