# 02 — Sugestões da IA na mesma conversa + histórico em bolhas

- **Status:** CONCLUDED
- **Depende de:** 01
- **Habilita:** —

## Objetivo

Colocar a sugestão da IA **na mesma linha temporal** das falas (BR-020) e usar o
mesmo componente de bolha no detalhe do histórico — uma conversa só, ao vivo e
depois.

## Entregáveis

- `core/domain/chat.ts`: `buildChatTimeline(segments, suggestions)` — falas e
  sugestões ordenadas por `at_ms`, dedupe por id/`message_id`.
- `ChatBubble`: variante de sugestão (rótulo "Sugestão da IA", glifo de faísca,
  `mm:ss`, botão **Copiar**).
- Cabeçalho do chat com o botão **Sugerir** (substitui o `InsightsPanel`, removido).
- `Overlay`: `useInsight(sessionId)` + `useInsightFeed()` (atalho) com dedupe por
  `message_id`, repassando a lista para o `ChatView`.
- `HistoryPanel`: detalhe renderiza com `ChatView` a partir de `db_session_timeline`.

## Critérios de aceite

- [ ] Sugestão aparece como bolha na ordem do tempo, com tempo e copiar.
- [ ] Sugestão do atalho não duplica a do banco.
- [ ] Detalhe do histórico usa as mesmas bolhas (falas + sugestões).
- [ ] Nenhum IPC/schema novo.

## Verificação

- `chat.test.ts`: ordem fala/sugestão, empate no mesmo `at_ms`, dedupe.
- `ChatView.test.tsx`: bolha de sugestão, copiar, render do histórico.

## Resultado (2026-10-18)

- `buildChatTimeline` ordena falas + sugestões por tempo (falas antes no empate) e
  deduplica por `message_id`; `useInsight` já assina o evento do atalho com dedupe,
  então a lista é única (o `InsightsPanel` foi removido e o botão "Sugerir" vive no
  cabeçalho do chat).
- `HistoryPanel` renderiza o detalhe com `ChatView` a partir de
  `db_session_timeline` (`bubblesFromTimeline`) — testes do histórico atualizados
  para as bolhas; nenhum IPC/schema novo.
