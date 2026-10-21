# 15 — Opacidade global das janelas em tempo real

- **Status:** CONCLUDED
- **Depende de:** 10–13; BR-062/BR-064
- **Issues:** — · **BR:** BR-062
- **Refs:** [`../prd.md`](../prd.md) §10,
  [`../architecture/settings-redesign.md`](../architecture/settings-redesign.md) §9,
  [`../../notch/architecture/design-tokens.md`](../../notch/architecture/design-tokens.md)

## Objetivo

O slider de **"Geral & HUD"** deixa de controlar apenas o fundo do notch: passa
a ser a **opacidade global das superfícies de todas as janelas**. A mudança é
aplicada em tempo real nas janelas abertas e a opacidade é salva
automaticamente, sem depender do botão "Salvar Alterações".

O campo persistido continua `overlay_opacity` (nome legado) para não exigir
migração de settings.

## Entregáveis

- **Hook global** `useHudOpacity` chamado no `App` de cada webview:
  - carrega `overlay_opacity` no boot;
  - aplica `--hud-alpha` no `document.documentElement`;
  - escuta `hud_opacity_preview` (preview) e `settings_changed` (reconciliação).
- **Preview:** `settings_preview_overlay_opacity` valida 20–100 % e emite
  `hud_opacity_preview` para todas as janelas.
- **Autosave:** `settings_set_overlay_opacity` grava apenas a opacidade, sem
  reaplicar atalhos/geometria, e emite preview + `settings_changed`. A UI
  coalesce o arraste com debounce de 200 ms e grava o último valor ao desmontar.
- **Superfícies com `--hud-alpha`** (canais RGB, sem `color-mix`):
  - notch (já existia), painéis/headers;
  - Configurações (shell, cards e rodapé);
  - Perguntar, dwell e prévia do print;
  - controles de aba do painel.
- Texto, ícones e controles permanecem opacos; muda só a camada de fundo.
- O botão "Salvar Alterações" continua salvando os demais campos.

## Critérios de aceite

- [x] Mover o slider altera a opacidade de Chat, Sugestões, Histórico,
  Configurações, notch, Perguntar, dwell e prévia do print sem salvar.
- [x] A opacidade persiste automaticamente após o arraste do slider.
- [x] O botão global não é necessário para a opacidade.
- [x] Valores fora de 20–100 % são recusados no preview/save.
- [x] 100 % = superfícies sólidas; 20 % = superfícies discretas.
- [x] Configurações antigas continuam carregando (`overlay_opacity` legado).

## Verificação

- `pnpm test` **363 ✓** · `tsc --noEmit` ✓ · `pnpm build` ✓.
- `cargo test --lib` **545 ✓** (9 ignorados) · `cargo clippy --lib -D warnings` ✓.
- Layout validado no navegador em 20 %/75 %/100 % (fundo magenta para
  evidenciar a transparência) em Configurações e Chat.
- AppImage validado no Xvfb: janela de Configurações sem moldura
  (`_MOTIF_WM_HINTS` decorations = 0); slider alterou de 40 % → 85 % → 50 % →
  40 % e o `settings.json` acompanhou **sem clicar em Salvar**, com o overlay e
  o Histórico abertos.
- Pacotes finais em `src-tauri/target/release/bundle` (11:13).
