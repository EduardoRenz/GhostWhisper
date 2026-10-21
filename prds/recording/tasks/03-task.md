# 03 — Alvo da gravação: janela ativa, seletor e eventos de ciclo de vida

- **Status:** IN PROGRESS
- **Depende de:** 01, 02
- **Habilita:** 04, 08
- **Issues:** ISS-052, ISS-053 · **BR:** BR-070
- **Refs:** [`../prd.md`](../prd.md) US-06;
  [`../architecture/recording.md`](../architecture/recording.md) §5.3

## Objetivo

Capturar **uma janela** (padrão) em vez da root, mantendo o overlay do
GhostWhisper visível na tela e **fora** do vídeo — o Linux não tem API de
exclusão de janela (BR-006), então o alvo é o caminho.

## Entregáveis

- `src-tauri/src/adapters/recording/x11_windows.rs` (XCB/`x11rb`):
  - `active_window() -> Option<WindowInfo>` via `_NET_ACTIVE_WINDOW`;
  - `list_windows() -> Vec<WindowInfo>` (top-level, com `WM_CLASS`, título e
    geometria; ignora `override-redirect` e janelas 1×1);
  - `WindowInfo { id, title, app, width, height }`.
- `src-tauri/src/adapters/recording/window_watch.rs`:
  - assina `ConfigureNotify` / `DestroyNotify` / `UnmapNotify` da janela alvo;
  - emite `WindowResized(w,h)`, `WindowClosed`, `WindowMinimized`.
- `ffmpeg_screen_recorder.rs`:
  - `RecordingTarget::Window(id)` → `-window_id <id>`;
  - `RecordingTarget::Screen` → `:0.0` + `-video_size`;
  - ao receber `WindowResized`: fecha o segmento e abre outro na nova geometria;
  - `WindowClosed|Minimized`: `stop` com `status = interrupted` (nada é
    apagado).
- `src-tauri/src/ipc/recording_commands.rs`:
  - `recording_windows() -> Vec<WindowDto>`;
  - `recording_active_window() -> Option<WindowDto>`.
- `src/features/settings/WindowPicker.tsx` (ou modal no `session-end`? não —
  no início): lista de janelas com app + título; opção **"Tela inteira"**.
- `src/adapters/ipc/recording.ts`: `listRecordingWindows`, `getActiveWindow`.

## Estado (2026-10-21)

- **Feito:** janela ativa/topo (`_NET_ACTIVE_WINDOW` + `_NET_CLIENT_LIST_STACKING`),
  seletor de janelas (janela `recording-picker`, clique secundário no botão),
  alvo explícito no `recording_start`, **troca de alvo no meio** e opção de
  seguir a janela ativa; **monitores**
  (RandR 1.5: todos os monitores ou um específico, com offset) e print de tela
  no monitor sob o cursor.
- **Pendente:** observar `ConfigureNotify`/`DestroyNotify`/`UnmapNotify`
  (`window_watch.rs`) para redimensionar/fechar a janela durante a captura
  (CA-06.5/06.6, ISS-053).

## Critérios de aceite

- [ ] Com `recording_target = "window"`, o início usa a janela ativa; se ela for
      o próprio GhostWhisper (ou não houver), cai para **tela inteira** com
      aviso no *tooltip*.
- [ ] O seletor lista janelas reais com app e título, e "Tela inteira"; a
      escolha vale para a sessão (congelada — CA-02.7).
- [ ] O overlay **não** aparece no vídeo da janela (verificação manual com a
      janela do GhostWhisper aberta sobre a alvo).
- [ ] Redimensionar a janela entre segmentos não perde a gravação: o segmento é
      fechado e outro abre com a nova geometria (ISS-053).
- [ ] Fechar/minimizar a janela encerra a gravação como `interrupted`, preserva
      os segmentos e mostra "Abrir local" no modal.
- [ ] `WindowInfo` nunca inclui janelas do próprio processo.
- [ ] Em Wayland puro, `list_windows` devolve vazio e a capability fica
      indisponível (ISS-051).

## Verificação

- `cargo test --lib` (parsers de `_NET_ACTIVE_WINDOW`/`WM_CLASS` com buffers
  fixos, sem X server) · teste manual `#[ignore]` com duas janelas
- `pnpm test` para o seletor

## Notas

- `x11grab -window_id` foi verificado no ambiente em 2026-10-21 (ffmpeg 7.0.2):
  1888×1006 a 8 fps, ~20% de um núcleo.
- A janela é lida como *drawable*: com compositor ativo, o conteúdo vem do
  pixmap da janela (não do que está por cima).
