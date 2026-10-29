# 17 — Reordenar fontes de IA no arraste (WebKitGTK)

- **Status:** CONCLUDED
- **Depende de:** 08
- **Issues:** ISS-086 · **BR:** BR-044, BR-045 · **Rules:** R16, R17, R18–R22

## Objetivo

Fazer o arraste da aba **Fontes de IA** realmente reordenar no app Linux
(WebKitGTK). A ordem da lista **é** a regra de execução/fallback: a primeira
fonte é a primária, se ela falhar a próxima assume, com circuit breaker em
memória por sessão (BR-044/BR-045 — backend já pronto em
`application/llm_router.rs` e `application/ai_source_health.rs`).

## Causa raiz

`onDragStart` não chamava `DataTransfer.setData`. No WebKitGTK o
`DropTargetGt4` aborta a sessão quando o arraste não carrega dados (WebKit
265857), então `dragover`/`drop` nunca chegam ao DOM: o card arrasta e não
reordena. Os testes antigos disparavam os eventos sintéticos e não pegavam o
defeito.

## Entregáveis

- `src/features/settings/ProviderCard.tsx`:
  - `setData("text/plain", source.id)`, `effectAllowed`/`dropEffect` = `move`,
    `setDragImage` no card, `preventDefault` no `drop`;
  - `onDragLeave` para limpar o destaque ao sair do card;
  - alça (`GripIcon`) como botão focável com `aria-label` e **↑/↓**.
- `src/features/settings/AiSourcesSettings.tsx`:
  - ignora `dragover`/`drop` sem arraste em andamento;
  - lista idêntica não chama `onChange` (não suja o rascunho);
  - usa as funções de domínio no lugar do `splice` inline.
- `src/core/domain/settings.ts`: `moveSource`, `moveSourceBy`,
  `makePrimarySource` (puros, sem I/O).
- `src/core/l10n/locales/{pt,en}.ts`: `sources_move_keyboard` e a dica de
  teclado em `sources_drag_reorder`.
- `src/styles/globals.css`: reset da alça como botão + `:focus-visible`.

## Critérios de aceite

- [x] Arrastar um card sobre outro **reordena** no app Linux (WebKitGTK).
- [x] A ordem do rascunho é a enviada no "Salvar Alterações"; a primeira é a
      primária (badge "Primário").
- [x] `↑`/`↓` na alça reordenam e param nos limites da lista.
- [x] Soltar no próprio card e soltar sem arraste não alteram a lista.
- [x] Regressão travada: `AiSourcesSettings.test.tsx` exige
      `setData("text/plain", id)` (falha sem a correção).
- [x] Progressão aria: alça é botão com `aria-label` traduzido em pt/en.
- [x] `pnpm test` (537 ✓) e `tsc --noEmit` limpos.

## Verificação manual pendente

1. Três fontes na aba "Fontes de IA"; arrastar a terceira para o topo.
2. "Salvar Alterações" → conferir a ordem em `settings.json` e o badge
   "Primário".
3. Reiniciar o app: a ordem e a primária continuam.
4. Fazer a primária falhar (chave inválida) e ver o card de fallback com
   "Fallback ativo" enquanto o circuito estiver aberto.
