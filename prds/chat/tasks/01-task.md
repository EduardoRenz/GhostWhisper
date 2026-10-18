# 01 — Conversa em bolhas na linha temporal

- **Status:** CONCLUDED
- **Depende de:** —
- **Habilita:** 02, 03

## Objetivo

Trocar a lista de linhas de texto por **bolhas de conversa** (como o Perssua) e
corrigir o bug **ISS-018**: as falas de fontes de áudio diferentes disputavam a
mesma posição/identidade na linha temporal.

## Entregáveis

- `application/transcription_pipeline.rs`: id do segmento passa a incluir o **canal**
  (`{session_id}-{channel}-{origin_ms}-t{turn}`), eliminando a colisão entre
  microfone e sistema (que substituía o texto na UI e falhava no `INSERT`).
- `core/domain/chat.ts` (novo, puro): `splitChatTimeline` — finalizados na linha
  temporal (ordem por `at_ms`), prévias separadas como "em andamento" (uma por
  falante).
- `core/domain/transcript.ts`: `mergeSegment` nunca deixa uma prévia sobrescrever um
  final do mesmo id.
- `features/chat/ChatBubble.tsx` e `features/chat/ChatView.tsx` (novos);
  `features/transcript/TranscriptView.tsx` removido.
- `styles/globals.css`: classes da conversa com os valores de paridade
  ([`../architecture/chat-ui.md`](../architecture/chat-ui.md) §3).
- Auto-scroll **só** quando o usuário está perto do fim.

## Critérios de aceite

- [x] Mic e sistema, no mesmo turno, geram **dois** segmentos distintos (id, banco
      e UI).
- [x] Prévia (**em andamento**) aparece no fim como bolha própria com `…`, nunca
      no meio da linha temporal.
- [x] Fala finalizada aparece alinhada à direita (Você) / esquerda (Interlocutor),
      com `mm:ss`.
- [x] `mergeSegment` protege o final contra prévia atrasada.
- [x] Testes Rust e frontend verdes; tsc limpo.

## Verificação

- Rust: teste de regressão `the_two_channels_of_a_session_never_share_a_turn_id`
  (dois pipelines, mesmo turno, dois segmentos persistidos) + testes de id/tempo
  atualizados.
- Frontend: `chat.test.ts` (ordem, dedupe, prévias por falante, sugestão no mesmo
  instante) e `ChatView.test.tsx` (alinhamento, rótulos, bolha em andamento,
  auto-scroll).

## Resultado (2026-10-18)

- Rust: `the_two_channels_of_a_session_never_share_a_turn_id` (dois pipelines com o
  mesmo turno → ids `s1-microphone-0-t0` / `s1-system-0-t0` e **dois** segmentos
  persistidos) + testes de id/tempo atualizados; suíte **217 testes**, 8 `ignored`,
  clippy/fmt limpos.
- Frontend: `chat.test.ts` (ordem, dedupe, prévia atrasada não reescreve o final,
  uma prévia por falante, vazio/cap) e `ChatView.test.tsx` (alinhamento por falante,
  rótulos, bolha em andamento no fim, copiar, vazio/cabeçalho).
