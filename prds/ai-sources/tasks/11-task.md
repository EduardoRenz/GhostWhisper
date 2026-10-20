# 11 — Janela do notch: ancoragem e densidade

- **Status:** CONCLUDED
- **Depende de:** 10
- **Habilita:** 13
- **Issues:** — · **BR:** BR-062

## Objetivo

Fazer a janela `overlay` respeitar a ancoragem escolhida (topo central / canto
superior direito / livre) e a densidade (altura da barra), inclusive ao vivo
quando o usuário salva as configurações.

## Entregáveis

- `src-tauri/src/ports/overlay_window.rs`:
  - `fn set_hud_layout(&self, density: HudDensity, anchor: NotchAnchor)`.
- `src-tauri/src/adapters/overlay/tauri_overlay_window.rs`:
  - guarda `(density, anchor)`; `set_surface(ActionBar)` usa a altura da
    densidade em vez de `72.0`;
  - `apply_anchor()` posiciona pela **área útil do monitor primário**
    (`primary_monitor()`, fallback `current_monitor()`) usando
    `domain::hud_layout::anchor_position`;
  - âncora `Free` mantém a posição e a reancoragem pela base;
  - fakes da porta (`state.rs`, `cursor_click_through.rs`) ganham o método.
- `src-tauri/src/lib.rs`:
  - no boot, aplica `set_hud_layout` e, em `Free`, restaura
    `overlay_position` (comportamento atual);
  - `overlay_start_drag` (via `ipc::overlay`) grava `notch_anchor = "free"` e
    persiste antes de arrastar, emitindo `settings_changed`.
- `src-tauri/src/ipc/settings_commands.rs`:
  - `settings_set` reaplica `state.overlay.set_hud_layout(...)` depois de salvar.
- Testes: geometria coberta na task 10; aqui, teste do adaptador de fakes
  (não quebra a troca de superfície).

## Critérios de aceite

- [ ] Salvar "Topo Central" move o notch para o centro no topo, sem reiniciar.
- [ ] Salvar "Canto Sup. Dir." encosta o notch à direita da área útil.
- [ ] "Expandido" aumenta a janela/barra; "Compacto" diminui.
- [ ] Arrastar o notch em modo ancorado o torna "Flutuante Livre".
- [ ] Em "Flutuante Livre", a posição lembrada é restaurada no boot.
- [x] `cargo test` (519 ✓), `cargo clippy -D warnings` ✓ e os arquivos tocados formatados.

## Como testar

1. Configurações → Geral & HUD → escolher "Canto Sup. Dir." → salvar → o notch
   salta para o canto direito.
2. Arrastar o notch → reabrir Configurações → a opção "Flutuante Livre" está
   ativa.
3. Alternar as três densidades e conferir a altura da barra.

## Verificação (2026-10-20)

- Implementado: `set_hud_layout` na porta e no adaptador; `place_at` usa a
  **área útil** do monitor primário; `overlay_start_drag` grava
  `notch_anchor = "free"`; `settings_set` reaplica o layout.
- Automático: `cargo test --lib` **519 ✓** · `clippy -D warnings` ✓ ·
  `rustfmt --check` nos arquivos tocados ✓.
- Manual (usuário): mover/ancorar/densidade numa sessão real — os itens acima
  seguem desmarcados de propósito até esse teste.

> Nota: `cargo fmt --check` global ainda acusa arquivos **não tocados** por esta
> task (dívida pré-existente, anterior a esta entrega).
