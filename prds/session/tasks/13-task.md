# 13 — Todas as janelas sempre à frente após a captura

- **Status:** CONCLUDED
- **Depende de:** 12; BR-055, BR-072
- **Issues:** ISS-056 · **BR:** BR-073
- **Refs:** [`../issues.md`](../issues.md),
  [`../architecture/session-journal.md`](../architecture/session-journal.md) §8.1

## Objetivo

Garantir que **todas** as janelas do GhostWhisper permaneçam à frente das
janelas de outros apps, especialmente o notch, depois do ciclo
`hide`/`captura`/`show` do atalho de print.

## Diagnóstico

- A captura esconde todas as janelas e as restaura com `show()` genérico.
- A correção da ISS-055 passou a reaplicar posição/tamanho depois do `show`;
  no GTK/Mutter esse remapeamento derruba `_NET_WM_STATE_ABOVE` (ISS-043).
- O overlay tinha reafirmação no seu próprio `show()`/`apply_stealth`, mas o
  caminho da captura chamava `tauri::WebviewWindow::show()` direto e não passava
  por ele.

## Entregáveis

- `src-tauri/src/ipc/window_visibility.rs`:
  - `assert_above(window)` — reafirma `always_on_top(true)`;
  - `show_above(window)` — reafirma antes/depois de `show()`.
- `screen_commands`:
  - restauração da captura reaplica geometria só quando difere e termina com
    `assert_above`;
  - prévia do print usa `show_above`.
- `ask_commands`, `panel_commands`, `settings_commands`,
  `session_end_commands`, `recording_commands` e `lib.rs` usam a reafirmação.
- `tauri_dwell_popover` reafirma o topo após o `show`.
- BR-073 documenta a regra geral.

## Critérios de aceite

- [x] Print pelo atalho: o notch volta visível e acima das outras janelas.
- [x] A captura preserva a geometria de todas as janelas (ISS-055) e reafirma
  o `alwaysOnTop` no final.
- [x] Perguntar, painéis, Configurações, prévia, modais e dwell também
  reafirmam o topo ao aparecer.
- [x] `cargo test`, clippy e rustfmt limpos; sem regressão de UI.

## Verificação

- `cargo test --lib` **596 ✓** (10 ignorados) · `cargo clippy --lib -D warnings`
  ✓ · rustfmt dos arquivos tocados ✓.
- `pnpm test` **387 ✓** · `tsc --noEmit` ✓ · `pnpm build` ✓.
- AppImage no Xvfb: antes da captura o overlay não tinha `_NET_WM_STATE`;
  após o ciclo completo (atalho → esconde → captura → restaura), `xprop`
  mostrou `_NET_WM_STATE_ABOVE, _NET_WM_STATE_SKIP_TASKBAR,
  _NET_WM_STATE_SKIP_PAGER` no overlay, no Perguntar e na prévia.
- Geometria preservada na captura: overlay `554,855`, Perguntar `567,32`,
  prévia `1664,16`.
- Pacotes finais em `src-tauri/target/release/bundle` (15:36).
