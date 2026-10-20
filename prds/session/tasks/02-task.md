# 02 — Aba "Sessão" no painel e botão "Perguntar" no notch

- **Status:** PENDING
- **Depende de:** —
- **Habilita:** 04, 05
- **Issues:** — · **BR:** BR-033, BR-052, BR-057
- **Refs:** [`../prd.md`](../prd.md) US-01, US-05;
  [`../architecture/session-journal.md`](../architecture/session-journal.md) §8

## Objetivo

Dar a **casca**: a terceira aba do painel (**Transcrição | Resumo | Sessão**)
e o **botão "Perguntar"** (ícone de lupa) no switcher do notch, ao lado de
**"Resumo"**, que só existe com sessão iniciada.

## Entregáveis

- `src/core/domain/settings.ts` e `src-tauri/src/domain/settings.rs`:
  `PanelTab` ganha `"session"`; `sanitize_panel_tab` aceita o valor novo e
  cai em `transcription` para o desconhecido.
- `src/features/panel/PanelTabs.tsx`: terceira aba (**Sessão**) com ícone
  próprio (`JournalIcon`).
- `src/features/panel/PanelWindow.tsx`: monta a `panel-view-session` (a
  visualização entra na task 04; aqui é a casca com estado vazio).
- `src/components/icons.tsx`: `SearchIcon` (lupa) e `JournalIcon`, **SVG do
  projeto** (BR-008), no mesmo estilo dos demais.
- `src/features/overlay/NotchTabs.tsx`: botão **"Perguntar"** com
  `SearchIcon`, **entre "Resumo" e "Sugestões"**, como `DwellButton`
  (`triggerId="notch-ask"`, drill-hover BR-036), com `title` e `aria-label`
  (BR-032).
- `src/features/overlay/Overlay.tsx`: `onOpenAsk` →
  `panelOpenTab("chat", "session")`.
- `src/adapters/ipc/panels.ts`: `panelOpenTab` aceita `"session"`.

## Critérios de aceite

- [ ] O painel tem **três** abas; a ativa é lembrada em `settings.panel_tab`
      e sobrevive ao reinício.
- [ ] `panel_tab` com valor legado/desconhecido → `transcription`.
- [ ] Com sessão **ativa**, o notch mostra a **lupa** ao lado de "Resumo";
      em repouso, o switcher **não** existe (BR-032) → o botão não aparece.
- [ ] Clicar na lupa abre/foca o painel **na aba Sessão**.
- [ ] `pnpm test` verde; teste do `PanelTabs` cobrindo as três abas e a
      navegação por setas (←/→/Home/End).

## Notas

- A lupa entra **com rótulo** ("Perguntar"), como "Transcrição", "Resumo" e
  "Sugestões": o switcher é um conjunto de **modos**, e o BR-032 fala de
  *ícones no lugar de rótulos de ação* nos **botões do grupo final** (stealth,
  histórico, config), não no switcher. O **ícone de lupa** é o que o pedido
  do usuário pediu como sinal visível.
- O overlay continua `actionbar | hidden` (BR-032): **não** há superfície
  expansível nem "Ask" flutuante (desvio consciente, R12).
