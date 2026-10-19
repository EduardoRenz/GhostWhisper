# 08 — Janela de Histórico (sessões anteriores)

- **Status:** CONCLUDED
- **Depende de:** 07
- **Habilita:** —

## Objetivo

Dar um lugar próprio ao histórico de sessões, já que o overlay deixa de ter o
painel expansível (task 07). O histórico vira uma **janela própria** na mesma
família (`chat`/`summary`/`suggestions`), aberta por um botão na ActionBar, sem
mudar o conteúdo: lista de sessões (continuar/renomear/excluir) e o detalhe com a
linha temporal em bolhas.

## Entregáveis

**Backend**

- `src-tauri/tauri.conf.json`: nova janela `history` (mesmo molde; `resizable:
  true`, `minWidth/minHeight 320×240`, `decorations: false`, `transparent`,
  `alwaysOnTop`, `skipTaskbar`, `shadow: false`, `visible: false`, `center: true`).
- `src-tauri/capabilities/default.json`: `history` na lista `windows`.
- `src-tauri/src/ipc/panel_commands.rs`: `PANEL_LABELS` += `"history"`.
- `src-tauri/src/domain/settings.rs`: `history_position` / `history_size` +
  defaults + clamp + testes.
- `src-tauri/src/lib.rs`: boot e tracker incluem `history` na iteração de rótulos.

**Frontend**

- `src/adapters/ipc/panels.ts`: `PanelLabel` ganha `"history"`.
- `src/App.tsx`: rota `history` (+ `?window=history`).
- Novo `src/features/history/HistoryWindow.tsx`: `PanelFrame` + 
  `usePanelOpen("history")` + `HistoryPanel` como corpo (o `history__head` interno
  sai, o `PanelFrame` já dá o título e o fechar).
- `src/features/overlay/ActionBar.tsx`: botão **Histórico** (toggle por
  `usePanelOpen`).
- `onContinue` deixa de chamar `showLive()`: continuar uma sessão volta a capturar
  e a conversa aparece na janela de Chat.

## Critérios de aceite

- [ ] O botão **Histórico** alterna esconder/mostrar a janela de histórico.
- [ ] A lista (continuar/renomear/excluir) e o detalhe em bolhas funcionam como
      antes, agora numa janela redimensionável e arrastável.
- [ ] **Continuar sessão** volta a capturar na sessão escolhida e o resultado
      aparece na janela de Chat, sem reabrir o painel do overlay.
- [ ] Posição e tamanho da janela voltam ao reabrir o app.
- [ ] `cargo test`/`clippy`/`fmt` e `pnpm test`/`tsc`/`build` limpos.

## Verificação

- Frontend: `HistoryWindow.test.tsx` (renderiza a lista e o toggle), `App.test.tsx`
  (rota `history`), `HistoryPanel.test.tsx` ajustado ao cabeçalho do `PanelFrame`.
- Rust: `Settings` (`history_size`/`history_position`).

## Notas

- Sem mudança de schema nem de export (RNF-03); o detalhe continua lendo a linha
  temporal do backend (CA-05.1).

## Resultado (2026-10-18)

- Janela `history` declarada e liberada em `capabilities`; `PanelLabel` ganhou
  `"history"`; rota `history` (e `?window=history`).
- `Settings.history_position`/`history_size` com clamp.
- `HistoryWindow` embrulha o `HistoryPanel` no `PanelFrame`; o cabeçalho interno
  virou só o botão "Atualizar" (o título é do painel). "Continuar sessão" volta a
  capturar e abre a janela de Chat.
- Testes: `HistoryWindow.test.tsx` (2) cobrindo moldura e `onContinue`.
