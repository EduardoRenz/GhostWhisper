# 21 — Fim do segmento = fim real da fala (ordem cronológica)

- **Status:** CONCLUDED
- **Depende de:** 16 (intercalação), 19 (uma fala = uma bolha)
- **Habilita:** —
- **Issues:** ISS-079 · **BR:** BR-043 (revisada)
- **Refs:** [`../issues.md`](../issues.md) ISS-079,
  [`../../business-rules.md`](../../business-rules.md) BR-043,
  [`../architecture/transcription-panel.md`](../architecture/transcription-panel.md) §3.4

## Objetivo

Gravar no segmento o **fim real da fala** (VAD/whisper), não o fim do
turno/buffer. É o que garante a ordem cronológica do chat e evita sobreposição
falsa entre canais.

## Causa (sessão `5984de1d…`)

`publish_turn` usava `end_ms = start_ms + cut_ms`. O `cut_ms` inclui o silêncio
até o commit; o `DecodedTurn.speech_end_ms` existia e era ignorado. Na sessão:

- THEY t7: banco 113,985–**127,937 s**; fala real terminou ~122,5 s.
- ME t10: **122,765**–127,373 s.

O banco dizia que o THEY ainda falava quando o ME começou; o corte por
sobreposição inseria a bolha do ME antes do resto “...sugerir melhorias.”.

## Entregáveis

- `TranscriptionPipeline::publish_turn` recebe `speech_end_ms` e grava
  `end_ms = start + speech_end_ms.clamp(0, cut_ms)` (fallback `cut_ms` quando
  não há timestamp).
- Prévia, final e `begin_final_reusing` (promoção do VAD) passam o
  `speech_end_ms`; o fallback de prévia persistida também.
- O overlap/linha temporal/classificador usam o `end_ms` corrigido.
- Testes:
  - `EarlyEndAsr` (fala 500 ms, cut 2 s) → `end_ms=500` na prévia/final;
  - chat com o caso real: THEY termina antes do ME → uma bolha de THEY, ME
    depois, sem `they#ov1`.

## Critérios de aceite

- [x] `end_ms` é o fim da fala, não o cut do turno.
- [x] A prévia e o final usam o mesmo fim real.
- [x] O caso real deixa de gerar sobreposição falsa e a ordem é THEY → ME.
- [x] Sobreposição real continua intercalando.
- [x] `cargo test`, clippy, fmt e `pnpm test` verdes.

## Resultado (2026-10-27)

- `publish_turn` com `speech_end_ms`; prévia/final/promoção/fallback corrigidos.
- Testes de pipeline e chat; `cargo test --lib` **844 ✓** (11 ignorados) ·
  `pnpm test` **521 ✓** · clippy/fmt/tsc ✓.
- Sessões antigas não são retroativamente corrigidas (sem áudio).
