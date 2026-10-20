# 12 — UI: aba "Geral & HUD" e acabamento visual

- **Status:** CONCLUDED
- **Depende de:** 10
- **Habilita:** 13
- **Issues:** — · **BR:** BR-062

## Objetivo

Trocar o bloco inline da aba "Ajustes" por uma aba **"Geral & HUD"** com os quatro
cards do mockup e aproximar a casca da tela (abas com sublinhado, cabeçalho de
seção, grade de 2 colunas, segmentados, toggle e badge) da referência.

## Entregáveis

- `src/features/settings/HudSettings.tsx` (novo):
  - cabeçalho "Comportamento do HUD & Janela" + rótulo vivo `HUD Anchor: …`;
  - card **Opacidade do Overlay** (slider 20–100 %, badge `75%`, legendas
    Discreto/Equilibrado/Opaco);
  - card **Posição de Ancoragem do Notch** (3 botões segmentados);
  - card **Expandir Notch por Dwell (Hover)** (toggle + tempo de dwell
    100 ms–1,5 s, badge e legendas);
  - card **Tamanho / Densidade do Notch** (3 botões segmentados);
  - card **Privacidade** (stealth) atravessando as colunas.
- `src/features/settings/SettingsPage.tsx`: aba `general` passa a renderizar
  `<HudSettings settings={draft} update={update} />` e o rótulo vira
  **"Geral & HUD"**.
- `src/styles/globals.css`:
  - abas com sublinhado na ativa (em vez de pílula preenchida);
  - `.settings__section-header`, `.settings__grid` (2 colunas; 1 abaixo de
    760 px), `.settings__card--hud`, `.settings__card-description`,
    `.settings__badge`, `.settings__segmented`, `.settings__toggle`,
    `.settings__range-labels`.
- `HudSettings.test.tsx` e ajuste de `SettingsPage.test.tsx` (rótulo da aba).

## Critérios de aceite

- [x] A tela continua com **5 abas**; "Ajustes" deu lugar a "Geral & HUD".
- [ ] A aba mostra os 4 cards do mockup + privacidade, em 2 colunas no desktop.
- [x] Mover o slider de opacidade atualiza o badge e marca o rascunho.
- [x] Clicar numa âncora/densidade atualiza o segmentado e o rascunho.
- [x] O toggle de dwell desabilita o slider quando desligado.
- [x] O rótulo `HUD Anchor:` acompanha a âncora escolhida.
- [x] `pnpm test`, `tsc --noEmit` e `pnpm build` verdes.

## Como testar

1. Abrir Configurações → "Geral & HUD".
2. Comparar com o mockup: abas, cabeçalho da seção, grade 2×2, badge e rodapé.
3. Alternar âncora/densidade, mexer no slider e conferir o rodapé
   "Alterações não salvas".

## Verificação (2026-10-20)

- Automático: `pnpm test` **339 ✓** (55 arquivos, inclui `HudSettings.test.tsx`)
  · `tsc --noEmit` ✓ · `pnpm build` ✓.
- Coberto por teste: 5 abas e rótulo novo, os 4 cards + privacidade, badge e
  `update()` de opacidade/âncora/densidade/dwell, toggle desabilitando o slider e
  o rótulo vivo `HUD Anchor:`.
- Manual (usuário): comparação visual com o mockup (grade de 2 colunas,
  sublinhado da aba ativa, espaçamentos) — item desmarcado de propósito.
