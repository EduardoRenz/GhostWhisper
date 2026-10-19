# 10 — Fundação reutilizável do drill-hover + janela de confirmação

- **Status:** CONCLUDED
- **Depende de:** 09
- **Habilita:** 11
- **Issues:** ISS-035 (hover entre janelas) · **BR:** BR-036

## Objetivo

Entregar o mecanismo **genérico** de drill-hover, sem integrar no notch: lógica pura,
timer, `DwellButton`, `DwellProvider`, janela `dwell` (Tauri) e o conteúdo da
confirmação. Qualquer botão futuro reaproveita.

## Escopo

### Frontend

- `core/domain/dwell.ts`: constantes, `clampDwellMs`, `dwellLabel`,
  `chooseDwellPlacement` (pura).
- `core/application/useDwellTimer.ts`: timer de permanência (start/stop/reset,
  cleanup, fake timers).
- `components/dwell/DwellButton.tsx`: botão opt-in com fill/linha, clique imediato,
  `aria-haspopup="dialog"`, `data-arming`; fill **CSS-only**.
- `components/dwell/DwellPopover.tsx`: pill **"Sem clique"** + **"Sim"** + badge da
  duração (segundo estágio com dwell).
- `features/dwell/DwellPopoverWindow.tsx` + `components/dwell/DwellPopover.tsx`:
  rota `dwell`; "Sim" por tempo/clique; `Escape`; `dwell_pointer`.
- `components/dwell/DwellProvider.tsx`: contexto do overlay; `request(action,
  anchor)`, `pendingAction`, `overTrigger/overPopover`, grace, `dwell_closed`.
- `core/application/useInteractiveRegions.ts`: ignorar mutações em `.dwell-fill`
  (evitar IPC por quadro).
- Tokens/CSS: `--oh-dwell-fill`, `--oh-dwell-line`, classes do fill/popover.

### Core (Rust)

- Porta `ports/dwell_popover.rs`:
  `show(anchor, dwell_ms)`, `hide()`, `context()`.
- Adaptador `adapters/dwell/tauri_dwell_popover.rs`:
  janela `dwell`; posiciona pela **work area** (acima se couber, senão abaixo),
  centraliza/clampa; guarda `dwell_ms`; mostra/esconde.
- Função pura de posicionamento em `domain/geometry.rs` + testes.
- IPC `ipc/dwell.rs`: `dwell_show`, `dwell_hide`, `dwell_result`, `dwell_context`.
- `tauri.conf.json`: janela `dwell` (260×60, transparente, alwaysOnTop, skipTaskbar,
  sem foco/sombra, `visible:false`).
- `capabilities/default.json`: incluir `"dwell"`.
- `lib.rs`/`state.rs`: composition root + porta no `AppState`; registrar comandos.
- `App.tsx`: rota `dwell`.

## Critérios de aceite

- [ ] `DwellButton` arma após `dwell_ms`, cancela na saída e executa no clique.
- [ ] A janela `dwell` abre acima/abaixo conforme a work area, sem mover o notch.
- [ ] "Sim" confirma por dwell **ou** clique; `Escape` cancela.
- [ ] `dwell_closed` devolve `confirmed` ao overlay; o provider executa a ação
      pendente **uma vez**.
- [ ] Grace: sair do gatilho + confirmação fecha após `dwell_ms`; voltar cancela.
- [ ] Fill não dispara `overlay_set_interactive_regions` a cada quadro.
- [ ] Testes de domínio/hook/componentes/posicionamento; `cargo test/clippy/fmt`,
      `pnpm test`, `tsc`, `pnpm build` verdes.

## Como testar

1. `?window=dwell` no navegador (dev) renderiza a confirmação.
2. No app, abrir a janela via comando de teste/`dwell_show`; conferir acima/abaixo
   perto das bordas e o clique no "Sim".
3. Medir que o notch não muda de tamanho/posição ao abrir/fechar.
