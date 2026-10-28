# 06 — Teto da bolha de fala (continuação em bolha seguinte)

- **Status:** CONCLUDED (retroativa) — **substituída pela 19** (ISS-073/BR-102): o
  split de 180 foi removido e a fala passou a ser uma bolha só.
- **Depende de:** 02
- **Habilita:** —
- **Issues:** ISS-031 · **BR:** BR-031
- **Refs:** [`../../business-rules.md`](../../business-rules.md) BR-031,
  [`../issues.md`](../issues.md) ISS-031, [`../architecture/chat-ui.md`](../architecture/chat-ui.md) §2

> Documento reconstruído na v0.2 (2026-10-19) a partir da memória, da issue e do
> código já entregue; a task 06 não tinha arquivo versionado.

## Objetivo

Limitar a altura da bolha de fala sem cortar o áudio/ASR: falas longas continuam
em **bolhas seguintes** da mesma mensagem, sem repetir rótulo/tempo.

## Entregáveis

- `src/core/domain/chat.ts`:
  - `MAX_SPEECH_BUBBLE_CHARS = 180`;
  - `splitSpeechText(text, limit)` prefere fim de frase (`.!?…`, `\n`) e, se a
    frase sozinha passar do teto, corta no fim da **palavra** (nunca no meio);
  - `SpeechBubble.continues` e ids `#N` para as partes seguintes.
- `ChatView`/`ChatBubble`: aplica o split na fala finalizada, na prévia e no
  histórico; a continuação não repete rótulo/tempo e fica colada.
- `HistoryPanel`: `bubblesFromTimeline` aplica a mesma regra.
- Testes: `chat.test.ts` (frase, palavra, fala curta, prévia, histórico).

## Critérios de aceite

- [x] Fala ≤ 180 caracteres continua em uma bolha só.
- [x] Fala longa vira várias bolhas da mesma mensagem, sem repetir nome/tempo.
- [x] O corte é de **exibição**; o texto do ASR e o áudio não mudam.
- [x] `pnpm test`, `tsc --noEmit` e `pnpm build` verdes.

## Resultado (2026-10-18)

- Teto de 180 caracteres, corte determinístico no fim de frase/palavra,
  continuação visual e aplicação no ao vivo, na prévia e no histórico.
- `pnpm test` **134 ✓** (24 arquivos) · `tsc --noEmit` ✓.
