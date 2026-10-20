# 14 — Notch sempre à frente (reafirmar always-on-top)

- **Status:** CONCLUDED
- **Depende de:** 02 (notch em dois estados)
- **Habilita:** —
- **Issues:** ISS-043 · **Refs:** [`../issues.md`](../issues.md),
  [`../../mvp/architecture/overlay.md`](../../mvp/architecture/overlay.md) §1

## Objetivo

Fazer o notch continuar **acima das outras janelas** depois de ser mapeado,
redimensionado ou reexibido — o `alwaysOnTop: true` do `tauri.conf.json` só vale
na criação e o GTK/Mutter perde o `_NET_WM_STATE_ABOVE` no caminho.

## Entregáveis

- `src-tauri/src/adapters/overlay/tauri_overlay_window.rs`:
  - `apply_stacking()` reafirma `set_always_on_top(true)`;
  - `apply_stealth()` (chamado no `show()` e no fim de `set_surface()`) passa a
    chamar `apply_stacking()`.
- `src-tauri/src/lib.rs`:
  - `spawn_overlay_stacking_reassertion()` — reafirma uma vez ~900 ms após o
    boot (o overlay nasce visível e não passa pelo `show()`).

## Critérios de aceite

- [x] Com o app recém-aberto, `xprop -id <overlay> _NET_WM_STATE` contém
      `_NET_WM_STATE_ABOVE`.
- [ ] Depois de ocultar/reexibir (atalho ou bandeja), o notch continua por cima.
- [ ] Depois de mudar a densidade/âncora (resize), continua por cima.
- [x] `cargo test` (519 ✓), `cargo clippy -D warnings` ✓ e arquivos tocados
      formatados.

## Como testar

1. Abrir o app e uma janela de outro programa maximizada por cima.
2. Conferir que o notch permanece visível.
3. `xprop -id <id do overlay> _NET_WM_STATE` deve listar `_NET_WM_STATE_ABOVE`.
4. Ocultar e reexibir o overlay; repetir a conferência.

## Riscos / rollback

- Reafirmar a cada `set_surface` é barato (uma chamada ao WM) e idempotente.
- Se algum WM ignorar `set_always_on_top`, o comportamento continua o de antes
  (não piora).

## Verificação (2026-10-20)

- **Antes:** o overlay (`0x2200003`, sem título) tinha
  `_NET_WM_STATE_SKIP_PAGER, _NET_WM_STATE_SKIP_TASKBAR` — sem `ABOVE`; as
  janelas `chat`/`suggestions` tinham `ABOVE`.
- **Depois** (AppImage de 17:41, instância nova):
  `_NET_WM_STATE_SKIP_PAGER, _NET_WM_STATE_SKIP_TASKBAR, _NET_WM_STATE_ABOVE`.
- `cargo test --lib` **519 ✓** · `clippy -D warnings` ✓.
- Os caminhos de ocultar/reexibir e resize usam o mesmo `apply_stealth` →
  `apply_stacking`; validação visual do usuário ao usar o notch.

> Efeito colateral do teste: a instância anterior foi encerrada com `kill`
> (sem o `RunEvent::Exit`), então a marca de crash desligou a GPU nesta execução
> (BR-029). Fechar o app normalmente reabilita a GPU no próximo boot.
