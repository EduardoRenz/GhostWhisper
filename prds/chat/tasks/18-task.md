# 18 — Foco da barra de pesquisa pelo atalho global

- **Status:** CONCLUDED
- **Depende de:** 03/04 (barra "Perguntar"), 17 (casco das janelas)
- **Habilita:** atalho confiável com o app em segundo plano
- **Issues:** ISS-064 · **BR:** BR-052

## Objetivo

Garantir que o atalho global de pesquisa (`focus_ask`) **ative** a janela da
barra e coloque o cursor no campo **sempre**: com a barra oculta, já aberta ou
com outro aplicativo em foco. Antes, o clique na lupa focava, mas o atalho só
abria a janela — em Mutter a ativação era recusada.

## Causa

O `set_focus` do Tauri usa `gtk_window_present_with_time(GDK_CURRENT_TIME)`,
isto é, timestamp **0**. Quando o GhostWhisper não é o app ativo (caso do
atalho global), o Mutter recusa a ativação pela proteção contra roubo de foco.
O clique na lupa funciona porque o próprio clique ativa o app antes do foco.
É o mesmo mecanismo do arraste (ISS-063), corrigido com o timestamp do
servidor X11.

## Entregáveis

- **`ipc/window_focus.rs`**: `focus_window` apresenta a janela na thread
  principal do GTK com `x11_get_server_time()`; fallback `set_focus` e
  `present` no Wayland. `should_refocus(visible, focused)` decide a repetição.
- **`show_ask_window`**: ativa **sempre** (não só ao sair de oculta), emite
  `ask_focus` e repete a ativação ~150 ms depois se a janela estiver visível e
  sem foco (cobre o grab do atalho e o mapeamento do WM).
- **Frontend**: sem mudança; `onFocusChanged` + `ask_focus` continuam levando o
  cursor ao campo.

## Critérios de aceite

- [x] Atalho com a barra oculta abre **e** foca o campo.
- [x] Atalho com a barra aberta e outro app ativo **refoca** a barra.
- [x] Clicar na lupa continua focando (sem regressão).
- [x] Sem mudança de posição/tamanho da barra; sem regressão nos testes.
- [x] `cargo test`, `cargo clippy`, `pnpm test`, `tsc` e pacote Vulkan.

## Como testar

1. Abra outro aplicativo (ex.: o terminal) e pressione o atalho de pesquisa.
2. Confirme que a barra aparece na frente e o cursor está no campo.
3. Sem fechar a barra, clique no outro aplicativo e pressione o atalho de novo:
   a barra deve voltar a receber o foco.
4. Repita pelo clique na lupa para comparar o comportamento.

## Verificação (2026-10-25)

- Xvfb (sem WM): oculta → abre com `XGetInputFocus` na janela `ask`; aberta sem
  foco → o atalho devolve o foco.
- Mutter (sessão real): com o Chrome ativo e a barra aberta, o atalho físico
  levou o cursor ao campo (confirmado pelo usuário). XTest sintético não é
  contabilizado como interação do usuário pelo Mutter; a validação válida é
  com o teclado real.
- `should_refocus` (4 estados) + `AskWindow.test.tsx` (cursor no campo).
- `cargo test --lib` **693 ✓** (10 ignorados) · clippy `-D warnings` ✓.
