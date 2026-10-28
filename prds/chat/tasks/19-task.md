# 19 — Uma fala finalizada é uma bolha (remove o split de 180)

- **Status:** CONCLUDED
- **Depende de:** 02, 06 (substitui a 06)
- **Habilita:** —
- **Issues:** ISS-073 · **BR:** BR-102 (revoga a BR-031)
- **Refs:** [`../../business-rules.md`](../../business-rules.md) BR-102,
  [`../issues.md`](../issues.md) ISS-073,
  [`../architecture/chat-ui.md`](../architecture/chat-ui.md) §2,
  [`../architecture/transcription-panel.md`](../architecture/transcription-panel.md) §3

## Objetivo

Remover a divisão de exibição em 180 caracteres. Cada fala **finalizada** é uma
única bolha com o texto inteiro, e cada fala **em andamento** é uma única bolha
por falante. O único corte que permanece é o de **sobreposição** dos canais
(BR-043), que existe para intercalar quem falou no meio.

## Por que o split saiu

O commit em blocos ("ouvir um pedaço → processar → commitar → continuar") não é
possível hoje durante fala contínua: o pipeline só fecha em 1 s de silêncio, no
teto de 25 s (corte forçado) ou no experimento de pontuação forte + 0,7 s, que foi
rejeitado por degradar a transcrição (ISS-058). Sem esse commit, dividir a
exibição só criava bolhas que pareciam mensagens fechadas enquanto o turno ainda
estava em processamento. O caminho para bolhas menores **reais** ficou registrado
como experimento futuro (ISS-074, PRD `precision-and-performance`).

## Entregáveis

- `src/core/domain/chat.ts`: removidos `MAX_SPEECH_BUBBLE_CHARS`,
  `chopAtWords` e `splitSpeechText`; `SpeechBubble` sem `continues`/`source_text`;
  `buildChatTimeline`/`bubblesFromTimeline` emitem uma bolha por parte de
  sobreposição, sem corte por tamanho.
- `src/features/chat/ChatView.tsx`: prévia única (Streaming/digitação nela); fala
  finalizada única; IA, tradução e cópia recebem o texto da bolha.
- `src/features/chat/ChatBubble.tsx`: removida a prop `continues` e os guardas;
  header sempre visível; ações na própria bolha.
- `src/styles/globals.css`: removidos `.chat__row--continues` e o `margin-top`
  negativo.
- Docs: BR-031 revogada, BR-102 criada, PRD/arquitetura e `memory.md` alinhados.

## Critérios de aceite

- [x] Nenhuma fala é dividida por tamanho (ao vivo, prévia e histórico).
- [x] Fala longa finalizada continua em **uma** bolha, com o texto inteiro.
- [x] A prévia longa continua em **uma** bolha por falante, com Streaming.
- [x] IA, tradução e copiar operam sobre o texto da bolha; prévia não tem ação de
      IA.
- [x] A sobreposição dos canais continua intercalando (BR-043), sem `continues`.
- [x] `splitSpeechText`/`MAX_SPEECH_BUBBLE_CHARS`/`continues`/`source_text` não
      existem mais no código nem na documentação.
- [x] `pnpm test`, `tsc --noEmit` e `pnpm build` verdes.

## Resultado (2026-10-27)

- Split de 180 removido; a fala inteira (até ~25 s) é uma bolha. O `max_turn_ms`
  e o áudio não mudaram.
- Testes anti-regressão que **falhariam** no código anterior: "a fala finalizada
  continua em uma bolha só" e "a prévia longa fica em uma única linha em
  andamento" (`chat.test.ts`); "mantém a fala longa em uma bolha só" e "mantém a
  prévia longa em uma bolha só" (`ChatView.test.tsx`).
- `pnpm test` **517 ✓** (82 arquivos) · `tsc --noEmit` ✓ · `pnpm build` ✓.
