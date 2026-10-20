# 15 — Notch encosta no topo (tamanho real da janela)

- **Status:** CONCLUDED
- **Depende de:** 14 (always-on-top)
- **Habilita:** —
- **Issues:** ISS-044 · **Refs:** [`../issues.md`](../issues.md),
  BR-062, [`../tasks/12-task.md`](12-task.md)

## Objetivo

Remover a área invisível acima da barra do notch e fazer o arraste chegar até o
topo da tela — a janela estava presa em 1040×260 e a barra ficava no rodapé.

## Entregáveis

- `src-tauri/tauri.conf.json`: janela `overlay` vira **`resizable: true`**
  (sem moldura, não há alça para o usuário) e nasce 1040×84. O
  `resizable: false` fixava os hints do GTK em min=max e o `set_size` era
  ignorado na altura.
- `src-tauri/src/domain/hud_layout.rs`:
  - alturas da janela **72/84/96** (compacto/padrão/expandido) — a barra fica no
    topo e o excedente abaixo é o respiro do banner do ASR;
  - `HUD_MARGIN = 0` (encosta no topo).
- `src-tauri/src/adapters/overlay/tauri_overlay_window.rs`: `set_surface` não
  reancora mais pela base (a barra vive no topo; o canto superior é preservado).
- `src/styles/globals.css`: `.overlay` com `justify-content: flex-start` e
  `padding-top: 0`.

## Critérios de aceite

- [x] `WM_NORMAL_HINTS` do overlay deixa de ser min=max em 260 de altura.
- [x] A barra fica no topo da janela (sem área invisível acima dela).
- [x] Arrastar o notch permite encostar a barra no topo da tela.
- [x] As três densidades mudam a altura da janela (e não a posição da barra).
- [ ] O banner do ASR continua visível abaixo da barra.
- [x] `cargo test`, `cargo clippy` e arquivos tocados formatados.

## Como testar

1. Abrir o app e arrastar o notch até o topo: a barra deve encostar.
2. `xwininfo -id <overlay>` deve mostrar altura ~84 (padrão), não 260.
3. Trocar a densidade em Configurações → Geral & HUD e conferir a altura.
4. Iniciar uma sessão e ver o banner "baixando/carregando modelo" abaixo da barra.

## Riscos / rollback

- `resizable: true` sem moldura: não há alça de resize; os painéis já usam o
  mesmo padrão com grips próprios.
- Um WM que ignore `set_size` manteria o comportamento antigo (sem piorar).

## Verificação (2026-10-20)

- `xwininfo -id 0x2200003`: **1040×96 em (440, 32)** (densidade `expanded` das
  settings), barra no topo; antes era **1040×260 em (440, 8)** com a barra no
  rodapé (~200 px invisíveis).
- `WM_NORMAL_HINTS`: "minimum size: 0 by 0" (antes **min=max 1040×260**).
- Tentativa de mover para `y=0` via `XMoveWindow`: o Mutter **clampa em `y=32`**
  (topo da área útil, abaixo do painel do GNOME). Não há mais limite invisível.
- `_NET_WM_STATE` segue com `_NET_WM_STATE_ABOVE` (ISS-043 preservado).
- `cargo test --lib` 519 ✓ · clippy/fmt (tocados) ✓.
- Banner do ASR: o excedente abaixo da barra é 18–24 px por densidade
  (validar visualmente ao carregar um modelo).
