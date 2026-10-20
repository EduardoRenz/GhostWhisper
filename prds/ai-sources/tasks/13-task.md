# 13 — Overlay consome opacidade/densidade, docs e verificação

- **Status:** CONCLUDED
- **Depende de:** 11, 12
- **Habilita:** —
- **Issues:** — · **BR:** BR-062

## Objetivo

Aplicar opacidade e densidade na barra do notch (frontend), registrar BR-062 na
documentação do PRD e fechar a verificação completa.

## Entregáveis

- `src/features/overlay/Overlay.tsx`:
  - `data-density={settings.hud_density}` no contêiner `.overlay`;
  - variável `--hud-alpha` derivada de `settings.overlay_opacity`.
- `src/styles/tokens.css`: `--oh-surface-container-low-rgb` (canal RGB do token
  de superfície, para compor a opacidade sem `color-mix`).
- `src/styles/globals.css`:
  - `.overlay` define `--hud-bar-height` / `--hud-pad` / `--hud-gap` por
    densidade;
  - `.overlay__bar` usa essas variáveis e
    `rgba(var(--oh-surface-container-low-rgb), var(--hud-alpha, 0.75))`;
  - fallback sem `backdrop-filter` mantém a mesma opacidade.
- Docs:
  - `prds/ai-sources/prd.md` §10, `architecture/settings-redesign.md` §7,
    `tasks/10..13` e `_tasks.md` (v0.2);
  - `prds/business-rules.md` **BR-062**;
  - `memory.md` do PRD com a entrada do dia (`IDs:`/`Refs:`).
- Verificação final: `cargo test`, `cargo clippy`, `cargo fmt --check`,
  `pnpm test`, `tsc --noEmit`, `pnpm build`.

## Critérios de aceite

- [ ] Opacidade 20 % deixa a barra visivelmente translúcida; 100 % sólida.
- [ ] As três densidades mudam altura, padding e gap da barra.
- [ ] O notch continua click-through fora das regiões interativas.
- [x] BR-062 registrado e `memory.md` atualizado.
- [x] Toda a verificação automática verde.

## Como testar

1. Em "Geral & HUD", mover a opacidade e salvar: a barra muda na hora.
2. Alternar Compacto/Padrão/Expandido e conferir a altura da barra e da janela.
3. Reiniciar o app e confirmar que as três configurações persistiram.

## Verificação (2026-10-20)

- `Overlay.tsx` aplica `data-density` e `--hud-alpha`; tokens
  `--oh-surface-container-low-rgb`/`--oh-surface-container-rgb` criados; a barra
  compõe a opacidade sem `color-mix`.
- Automático: `cargo test --lib` **519 ✓** · `clippy -D warnings` ✓ ·
  `pnpm test` **339 ✓** · `tsc --noEmit` ✓ · `pnpm build` ✓.
- Manual (usuário): conferir a transparência e as três densidades no notch real.
