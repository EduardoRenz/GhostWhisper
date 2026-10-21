# 17 — Notch com largura dinâmica + menu de canais visível

- **Status:** CONCLUDED
- **Depende de:** 11, 15
- **Habilita:** —
- **Issues:** ISS-047 · **BR:** BR-032, BR-062

## Objetivo

1. O menu de canais do microfone precisa abrir **dentro da janela**, abaixo da
   barra, e ser clicável.
2. O notch deve ter **largura do conteúdo**: cresce quando entram itens (sessão,
   timer, canais) e encolhe quando saem, sem sobra fixa de 1040 px.

## Entregáveis

- **Core**:
  - `OverlayWindowController::set_actionbar_width(width)` e
    `set_popover_extra_height(extra)`;
  - `TauriOverlayWindow` guarda largura/altura extra, aplica `set_size` e
    reancora topo-centro/direita; modo livre preserva o canto superior;
  - IPC `overlay_set_actionbar_width` e `overlay_set_popover_extra_height`;
  - largura inicial de 700 px no `tauri.conf.json`.
- **Frontend**:
  - `Overlay` mede `.overlay__bar-wrap` com `ResizeObserver` e envia a largura;
  - `ChannelMenu` avisa abrir/fechar; `Overlay` reserva 180 px extras;
  - popover abre abaixo (`top: calc(100% + 10px)`);
  - CSS da barra usa `width: max-content`; o wrapper mede barra + padding.
- Testes do callback do menu e da barra; testes existentes preservados.

## Critérios de aceite

- [x] O menu de canais abre abaixo do notch, visível e clicável.
- [x] Fechar o menu/sessão devolve a altura da janela.
- [x] O notch encolhe/cresce conforme os itens, sem sobra fixa.
- [x] Topo-centro e canto direito continuam ancorados após a medição.
- [x] Modo livre não se move sozinho ao mudar de largura.
- [x] `cargo test`, `clippy -D warnings`, `rustfmt`, `pnpm test`, `tsc` e
      `pnpm build` limpos.

## Como testar

1. Inicie uma escuta e clique no ícone de microfone: o menu aparece **abaixo**
   da barra e as três opções respondem ao clique.
2. Pare a escuta: o notch encolhe; inicie: cresce com o switcher/timer.
3. Em "Topo Central" e "Canto Superior Direito", a barra continua ancorada ao
   crescer/encolher.
4. Em "Flutuante Livre", arraste e alterne o estado: a posição escolhida é
   preservada.

## Verificação (2026-10-20)

- `cargo test --lib` 544 ✓ (9 ignorados) · `clippy -D warnings` ✓ ·
  `rustfmt --check` nos arquivos tocados ✓.
- `pnpm test` 350 ✓ · `tsc --noEmit` ✓ · `pnpm build` ✓.
- Validado no AppImage (X11): idle **675×72**, ativo **758×72**,
  confirmação de encerrar **1058×72**; o menu abriu **758×252** (abaixo da
  barra), as opções responderam e, ao fechar, a janela voltou a **72** px de
  altura.
- AppImage/deb finais publicados às **23:04** (Vulkan).
