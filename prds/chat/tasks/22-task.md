# 22 — Fala em andamento intercalada no bloco finalizado (ISS-082)

- **Status:** CONCLUDED
- **Depende de:** 16 (intercalação), 19 (uma fala = uma bolha), 21 (fim real)
- **Habilita:** —
- **Issues:** ISS-082 · **BR:** BR-043 (revisada)
- **Refs:** [`../issues.md`](../issues.md) ISS-082,
  [`../../business-rules.md`](../../business-rules.md) BR-043,
  [`../architecture/transcription-panel.md`](../architecture/transcription-panel.md) §3.2/§3.4

## Objetivo

Fazer a fala **em andamento** (pill **Streaming**) aparecer no meio do bloco
finalizado que ela interrompeu, em vez de depois do bloco inteiro.

## Causa (sessão `9e6d8300…`, 2026-10-28)

`splitSpeechSpans` só recebia as falas **finalizadas**. A prévia entrava na
linha temporal pelo `start_ms` (ISS-018), mas não era fronteira para dividir o
bloco finalizado. Como a fala finalizada é uma bolha única ancorada no início,
a prévia caía depois dela.

## Entregáveis

- `splitSpeechSpans(spans, interruptions)` recebe as prévias como
  `interruptions`. Elas **não** são divididas (continuam uma bolha por falante,
  BR-102), mas entram na busca de fronteiras.
- `buildChatTimeline` passa `pending` como interrupções.
- O corte conservador da ISS-075 continua: fala ≥ 80 caracteres/6 s,
  interrupção ≥ 1,5 s e todas as partes ≥ 20 caracteres/3 palavras.
- Testes:
  - prévia intercalada no bloco finalizado (caso genérico);
  - caso real `9e6d8300…` (THEY 101,053–121,013 s; prévia ME 119,462 s) →
    `they6`, `me15`, `they6#ov1`;
  - prévia curta (< 1,5 s) ou posterior ao bloco não divide;
  - posição idêntica antes e depois de a prévia virar final.

## Critérios de aceite

- [x] A prévia que começou dentro do bloco aparece entre as duas partes.
- [x] A prévia continua uma bolha só, com **Streaming**.
- [x] Prévia curta/posterior não força corte.
- [x] A posição não muda ao finalizar.
- [x] `pnpm test`, `tsc`, `cargo test`, clippy e fmt verdes.

## Resultado (2026-10-28)

- `splitSpeechSpans` aceita interrupções e `buildChatTimeline` passa as prévias.
- `chat.test.ts` **25 ✓** (3 novos + caso real) · `pnpm test` **525 ✓** ·
  `tsc` ✓.
- Sem mudança no backend; é corte de exibição.
