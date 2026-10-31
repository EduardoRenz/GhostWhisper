# 26 — Wayland: notch em XWayland, atalhos e captura por portais

- **Status:** CONCLUDED
- **Depende de:** 23 (Wayland/X11), 24/25 (release e áudio)
- **Habilita:** Ubuntu 26.04/GNOME 50 (Wayland-only)
- **Issues:** ISS-088 · **BR:** BR-007, BR-055, BR-073

## Objetivo

Restaurar o comportamento do app no Ubuntu 26.04, que passou a usar **Wayland
puro** (sem sessão Xorg): notch sempre no topo, atalhos globais (inclusive com
o Chrome ativo) e captura de tela.

## Causa

- `keep_above`/`set_position` são ignorados em janelas Wayland nativas;
- `XGrabKey` não recebe teclas de apps Wayland nativos;
- `GetImage` na root do XWayland falha com `BadMatch` e não vê janelas nativas.

## Entregáveis

- **Detecção de sessão** (`domain/session_backend.rs`, pura/testada) e
  `GDK_BACKEND=x11` automático no Wayland para o overlay continuar em XWayland
  (topmost, posição e input-shape X11).
- **Atalhos globais:** `PortalGlobalShortcutRegistry` (XDG GlobalShortcuts) no
  Wayland; `TauriGlobalShortcutRegistry` continua no X11. Mesmos
  `ShortcutBinding`; mapeamento `CommandOrControl+Shift+C` → `CTRL+SHIFT+c`.
- **App id do portal:** `com.ghostwhisper.app`, registrado no host
  `Registry.Register` na mesma conexão; `.desktop` oculto em
  `$XDG_DATA_HOME/applications` (necessário no xdg-desktop-portal ≥1.20).
- **Captura:** `PortalScreenCapture` no Wayland, em modo interativo (UI do
  GNOME; o compositor exige confirmação). O arquivo é recomprimido em JPEG
  ≤1280 px e apagado. X11 mantém o `X11ScreenCapture` não-interativo.
- **Build único:** runtime atual do AppImage (o antigo não monta no Ubuntu
  26.04).

## Critérios de aceite

- [x] Notch fica acima do Chrome no XWayland.
- [x] `Ctrl+Shift+C` foca a barra com o Chrome ativo (portal aprovado uma vez).
- [x] `Ctrl+Shift+K` captura via portal do GNOME e mostra a prévia.
- [x] Em X11, o caminho antigo continua (plugin/atalhos e `GetImage`).
- [x] `cargo test`, clippy, fmt, `pnpm test`, `tsc`.
- [x] AppImage final com runtime novo e os recursos Vulkan+Laya+modelo.

## Verificação (2026-10-30)

- GNOME 50.1 / Ubuntu 26.04.1, sessão Wayland; `_NET_WM_STATE_ABOVE` presente
  com o Chrome ativo; atalho aprovado e funcionando; captura pela UI do GNOME.
- Rust **894 ✓** (12 ignorados) · clippy `-D warnings` ✓ · fmt ✓.
- Frontend **543 ✓** · `tsc --noEmit` ✓.
- AppImage final **830,42 MiB**, sha256
  `da3ef539e8d08cd8bd5dd679b8f2272708a292c10f74b507202a4e728ec91020`,
  em `src-tauri/target/release/bundle/appimage/GhostWhisper_0.3.1_amd64.AppImage`.
  Runtime atual do AppImage embutido; o arquivo montou e iniciou no Ubuntu
  26.04 (a versão antiga falhava no FUSE).
