# 23 — Retratação da prévia "em andamento" (bolha pendurada)

- **Status:** CONCLUDED
- **Depende de:** 11, 13
- **Habilita:** —

## Objetivo

Acabar com a bolha escura pendurada na conversa — a prévia "em andamento" que ficava
com o **texto cru** (`antes de refinar`) quando o turno terminava sem texto
(ISS-021).

## Contexto

A bolha em andamento é paridade com o Perssua (`.unified-message-current`): texto do
parcial, opacidade reduzida e `…`. Ela aparece **antes** do texto refinado e é
substituída no lugar quando o turno fecha — isso é o desenho, não o bug.

O bug: quando o passe final **não produzia texto**, nada era publicado para o id do
turno e a prévia ficava pendurada:

- final filtrado como alucinação/loop/silêncio (`clean` → `None`);
- turno descartado pelo gate de fala sustentada (`begin_final`);
- `flush` (parar/pausar) com buffer abaixo do mínimo.

Como a prévia pendurada é a única marca da fala, ela fica com o texto cru — e, se
for a primeira fala da sessão, é a **primeira bolha** da conversa, enquanto o resto
cresce abaixo.

## Entregáveis

- `TranscriptionPipeline::retract_turn(turn, base_ms, cut_ms)`: se **uma prévia foi
  mostrada** para aquele turno (`last_preview`), publica um **final de texto vazio**
  com o mesmo id — a UI substitui a prévia e a bolha sai. Vazio não vira bolha nem é
  persistido (`publish_turn` continua sendo o único caminho de escrita no banco).
- Chamadas nos três caminhos: gate de fala, final filtrado e `flush` sem buffer.
- `segment_id(turn)` extraído (prévia/final/retratação compartilham o id).
- `splitChatTimeline` ignora prévias em branco; `useTranslation` não traduz final
  vazio.
- `ChatBubble`: `title` "transcrevendo — o texto ainda pode mudar" na bolha em
  andamento, para não parecer erro/carregamento.

## Critérios de aceite

- [x] Final descartado ⇒ a prévia sai da conversa (nada de bolha pendurada).
- [x] Nada é persistido no banco quando o turno é descartado.
- [x] Sem prévia mostrada, nenhum item vazio é publicado.
- [x] A prévia **durante** a fala continua aparecendo (texto ao vivo), dimmed e com
      o `title` explicando.
- [x] A retratação não consome tradução nem vira bolha.

## Verificação (2026-10-18)

- Rust: `discarded_final_retracts_the_pending_preview` (prévia "bom dia pessoal" →
  final filtrado "Thanks for watching!" ⇒ prévia + retratação de mesmo id, zero
  linhas no banco) e `discarded_final_without_preview_does_not_publish_anything`;
  suíte **227 testes**, clippy/fmt limpos.
- Frontend: `chat.test.ts` (final vazio substitui a prévia e não gera bolha),
  `ChatView.test.tsx` (bolha em andamento com `title`), `useTranslation.test.ts`
  (não traduz prévia, retratação nem texto em branco); suíte **83 testes**,
  `tsc` limpo.
