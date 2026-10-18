# 08 — Janelas redimensionáveis e responsivas

- **Status:** CONCLUDED
- **Depende de:** 03, 04
- **Habilita:** —

## Objetivo

Permitir redimensionar as janelas de resumo/sugestões (sem moldura) para encaixá-las
na tela sem atrapalhar, com o conteúdo se adaptando e o tamanho lembrado entre
reinícios.

## Entregáveis

- `tauri.conf.json`: `resizable: true`, `minWidth: 320`, `minHeight: 240` nas
  janelas `summary` e `suggestions` (seguem sem moldura, always-on-top,
  skip-taskbar).
- `capabilities/default.json`: permissão
  `core:window:allow-start-resize-dragging`.
- `ResizeHandles` (novo, `features/panels/`) + `PanelFrame` com `.panel-shell`:
  8 alças (4 bordas + 4 cantos) que chamam `getCurrentWindow().startResizeDragging(dir)`;
  invisíveis, com cursor próprio; no-op fora do Tauri.
- CSS: `.panel-shell`/`.panel__resize*`; `@media (max-width: 420px)` compactando
  cabeçalho e escondendo a meta do histórico; `.panel__history-text` com
  `flex: 1; min-width: 0`; tabelas do Markdown com rolagem horizontal.
- `Settings.summary_size`/`suggestions_size` (Rust + TS) com clamp no mínimo; o
  boot aplica o tamanho e o tracker de layout salva tamanho + posição.

## Critérios de aceite

- [x] As duas janelas redimensionam pelas bordas/cantos e o conteúdo acompanha
      (rolagem interna).
- [x] Mínimo de 320×240; tamanho e posição voltam iguais ao reabrir o app.
- [x] Sem moldura preservada; alças invisíveis com cursor.
- [x] Cabeçalho e histórico continuam legíveis em largura pequena.
- [x] Testes de `PanelFrame` (8 alças, direção do resize e fechar) e de `Settings`
      (clamp do tamanho).

## Notas

- **Wayland nativo:** `startResizeDragging` depende do compositor e pode não
  funcionar; em **X11/XWayland funciona** (BR-007). O overlay e a janela de
  configurações não entram no escopo.
