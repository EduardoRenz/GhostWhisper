# 19 — Simplificar drill-hover para um estágio (sem confirmação "Sim")

- **Status:** CONCLUDED
- **Depende de:** 10, 11, 18 (substitui o fluxo de dois estágios)
- **BR:** BR-036 (revisada) · **Rules:** R16/R17
- **Refs:** [`../prd.md`](../prd.md) §4 (US-11),
  [`../architecture/drill-hover.md`](../architecture/drill-hover.md)

## Objetivo

Remover o popup/janela de confirmação **"Sem clique" + "Sim"** e o segundo timer.
Ao completar o preenchimento no **botão principal**, a ação é executada
diretamente.

## Decisão

O fluxo de dois estágios acumulou estados de hover entre duas janelas nativas e
falhas intermitentes (ISS-035/ISS-048). O preenchimento de `dwell_ms` já fornece
feedback e a saída antes do fim cancela o acionamento; a confirmação
intermediária não compensa a complexidade.

## Entregue

- `DwellButton` executa `onActivate` ao completar o timer; clique continua
  imediato.
- `DwellProvider` ficou só com `enabled`/`durationMs`; sem ação pendente, sem
  estado de hover entre janelas.
- Removidos:
  - janela Tauri `dwell` (`tauri.conf.json`), permissão
    `core:window:allow-cursor-position` e rota `dwell`;
  - `DwellPopover`, `DwellPopoverWindow`, `src/adapters/ipc/dwell.ts`,
    `src/adapters/ipc/cursor.ts` e testes;
  - porta `DwellPopoverController`, adaptador `TauriDwellPopover`, IPC
    `dwell_show/hide/context/result/trigger_hover`, campo em `AppState`;
  - posicionamento `dwell_popover_position` e seus testes;
  - `DwellProvider`/`Overlay` sem `DwellActionBar` e cancelamento de popup.
- `DwellButton`/`NotchTabs` sem `triggerId` e sem `aria-haspopup="dialog"`.
- CSS do popup removido; mantidos fill/linha do drill-hover.
- `TouchIcon` removido; `CheckIcon` mantido para outros usos.

## Critérios de aceite

- [x] Hover em Resumo/Sugestões preenche por `dwell_ms` e executa direto.
- [x] Sair antes do fim não executa.
- [x] Clique executa na hora e cancela o timer.
- [x] Não existe janela/popup "Sim" nem segundo timer.
- [x] `dwell_enabled`/`dwell_ms` continuam nos Ajustes e persistem.
- [x] Desligado/sem sessão não arma.
- [x] Sem som/toast; reduced motion preserva o tempo.

## Verificação

- `pnpm test` **377 ✓** · `tsc --noEmit` ✓.
- `cargo test --lib` **614 ✓** (10 ignorados) · clippy `-D warnings` ✓.
- AppImage (21:41) validado manualmente: hover completo em Resumo abriu o painel
  diretamente, sem janela `dwell`/popup; sair antes do fim não executou; clique
  continua imediato.
- Pacote final em `src-tauri/target/release/bundle` (21:41).
