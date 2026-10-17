# 01 — Janela overlay (X11)

- **Status:** CONCLUDED
- **Depende de:** 00
- **Habilita:** 07, 09, 10

## Objetivo
Criar a janela **overlay**: transparente, frameless, always-on-top, skip-taskbar e
com **click-through dinâmico**, estável em X11/XWayland.

## Entregáveis (implementados)
- `src-tauri/src/domain/geometry.rs` — `Point`/`Rect` + `should_pass_through`
  (lógica pura, com testes).
- `src-tauri/src/ports/overlay_window.rs` — porta `OverlayWindowController`.
- `src-tauri/src/ports/pointer_source.rs` — porta `PointerSource`.
- `src-tauri/src/ports/click_through.rs` — porta `ClickThroughController`.
- `src-tauri/src/adapters/overlay/tauri_overlay_window.rs` — implementação Tauri
  (show/hide/toggle/stealth/click-through/posição).
- `src-tauri/src/adapters/overlay/cursor_click_through.rs` — thread de amostragem
  do cursor + alternância de `set_ignore_cursor_events`.
- `src-tauri/src/adapters/system/x11_pointer_source.rs` — `X11PointerSource`
  (XQueryPointer) e `NullPointerSource` (fallback Wayland).
- `src-tauri/src/ipc/overlay.rs` — comandos `overlay_toggle`,
  `overlay_set_stealth`, `overlay_set_click_through`,
  `overlay_set_interactive_regions`.
- `src-tauri/src/{lib.rs,state.rs}` — composition root (DI) e `AppState` com as
  portas.
- `src-tauri/Cargo.toml` — dependência `x11rb`.
- Frontend: `src/features/overlay/Overlay.tsx`,
  `src/core/application/useInteractiveRegions.ts`, `src/adapters/ipc/client.ts`,
  `src/App.tsx` (roteia overlay), `src/styles/globals.css`.

## Critérios de aceite
- [x] Overlay definido com transparência/frameless/always-on-top/skip-taskbar
  (`tauri.conf.json` + adapter).
- [x] Click-through por posição do cursor implementado (regiões interativas da UI).
- [x] Lógica de decisão testada (5 casos em `domain::geometry`).
- [x] Sem roubo de foco: janela criada com `focus: false`; `show`/`hide` não
  ativam foco.
- [~] Verificação visual em GNOME/KDE (hover/clique) — **pendente de interação
  manual** (não automatizável neste ambiente).

## Evidências
- `cargo test` → **9 testes** (geometria, state, ping, error) passam.
- `cargo clippy --all-targets` → sem warnings; `cargo fmt --check` limpo.
- `cargo build` + smoke run (`timeout 8 ./target/debug/ghostwhisper`) → `exit=124`
  (inicia e permanece ativo, sem erro; X11 conectado).
- `pnpm test` (vitest) e `pnpm build` (tsc + vite) ok.

## Limitações conhecidas
- **Wayland puro:** sem acesso global ao cursor → `NullPointerSource`; o
  click-through automático fica desativado (documentado em `overlay.md`, BR-007).
- A depuração fina de hover/clique (regiões exatas) precisa de teste manual em
  sessão gráfica.

## Testes
- Unitários de geometria (limites half-open, escala, fora da janela, sem regiões).
- Manual (futuro): hover sobre ActionBar vs. área vazia em GNOME/KDE X11.
