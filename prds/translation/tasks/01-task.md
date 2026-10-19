# 01 — Tradução sob demanda na bolha · Entregável T1

- **Status:** IN PROGRESS
- **Depende de:** PRD `chat` (bolhas) e MVP 08 (tradução via LLM)
- **Habilita:** 02 (gate por idioma detectado)
- **Issues:** ISS-029 · **BR:** BR-030

## Objetivo

Tirar o painel duplicado de tradução e traduzir **a fala escolhida**, dentro da
bolha, com um clique e switch de volta.

## Entregáveis

- `src/core/application/useTranslation.ts` — tradução **sob demanda**
  (`translate({id, text})`), fila serial, cache por id, `reset`.
- `src/features/chat/ChatBubble.tsx` — botão/ícone no canto superior direito,
  estado de toggle e a linha traduzida (`chat__translation`).
- `src/features/chat/ChatView.tsx` — repassa traduções/pendências/`onTranslate`
  para as bolhas de **fala** (não para sugestões nem prévias).
- `src/features/overlay/Overlay.tsx` — liga o hook, reseta por sessão, remove o
  `TranslationView`.
- `src/components/TransportIcons.tsx` — `TranslateIcon` (SVG próprio, BR-008).
- `src/styles/globals.css` — estilos do botão e da tradução (tracejado + itálico
  menor); estilos do antigo painel removidos.
- `TranslationView.tsx`/`TranslationView.test.tsx` — **removidos**.

## Critérios de aceite

- [x] Botão aparece nas bolhas de fala quando a tradução está ligada e o destino
      ≠ auto.
- [x] Primeiro clique pede a tradução; ela aparece abaixo do original, com linha
      tracejada, em itálico menor.
- [x] Novo clique esconde (switch de volta) e o botão indica o estado ligado.
- [x] Tradução cacheada por fala (sem nova chamada ao LLM no toggle).
- [x] Tradução igual ao original não é exibida.
- [x] Sem botão em sugestões de IA e nas falas em andamento.
- [x] Painel de tradução abaixo da conversa removido (código e CSS).
- [x] `pnpm test` (122 ✓), `tsc --noEmit` ✓, `pnpm build` ✓.
- [ ] Teste manual do usuário (visual + toggle).

## Como testar

1. Configurações → Transcrição & Áudio → marque **Tradução ao vivo**, destino
   diferente da fala (ex.: origem Auto, destino **English**).
2. Inicie a sessão e fale (ou rode áudio do sistema).
3. Clique no **ícone de tradução** na bolha: a tradução deve aparecer logo abaixo,
   separada por tracejado, menor e em itálico.
4. Clique de novo: volta ao original.
5. Confirme que **não existe** mais nenhum painel de tradução embaixo da conversa.

Ponto de parada: usuário aprova; só então iniciar a T2.

## Riscos / rollback

- Fila serial pode atrasar cliques rápidos em várias bolhas → aceitável (protege
  rate limit); cache evita repetição.
- Botão atrapalhar o clique do overlay → `data-interactive` só no botão.

## Verificação

- Automática: `pnpm test` 122 ✓ · `tsc --noEmit` ✓ · `pnpm build` ✓.
- Pendente: teste manual. Status segue `IN PROGRESS`.
- Observações do usuário: (aguardando)
