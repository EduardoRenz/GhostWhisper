# 20 — Menus em janela própria: o notch não se move nem fica sem clique (ISS-062)

- **Status:** CONCLUDED
- **Depende de:** 17 (menu de canais), 11/15 (notch dinâmico), 19 (drill-hover)
- **BR:** BR-072 (revisão), BR-080 · **Rules:** R16/R17
- **Refs:** [`../issues.md`](../issues.md) ISS-062,
  [`../architecture/notch-ui.md`](../architecture/notch-ui.md) §2,
  [`../../assistants/architecture/assistants-panel.md`](../../assistants/architecture/assistants-panel.md) §11

## Objetivo

Abrir/fechar o seletor de agentes ou o menu de canais **não pode mover,
redimensionar nem desabilitar os cliques do notch**, em nenhuma posição da tela.

## Decisão

Duas tentativas anteriores mantinham o menu **dentro** da janela do notch:

1. crescer a janela na abertura (compensação por padding) — o compositor
   reposicionava a janela e o padding chegava alguns frames depois;
2. reservar altura fixa e trocar o lado quando o notch era arrastado — a troca
   movia a janela e o conteúdo em relógios diferentes (pulo) e deixava as
   regiões de clique do X11 desatualizadas (notch morto).

A solução definitiva separa as superfícies: o menu é uma **janela Tauri
própria** (`menu`), posicionada acima/abaixo do gatilho. O notch nunca participa
da abertura do menu; abrir/fechar só mostra/esconde o popup.

## Entregue

- **Domínio (puro):** `menu_placement(trigger, notch, menu_size, work_area,
  gap, align_right)` — X pelo gatilho (centro/direita), Y pelo **corpo do
  notch** (abaixo; acima quando não cabe), clamp na área útil.
- **Porta/adaptador:** `MenuWindowController` + `TauriMenuWindow` (janela
  `menu`; largura 300 para agentes e 200 para canais; **altura dinâmica** medida
  pelo conteúdo, 64–360 px e cache por tipo; ancorada no **corpo do notch**
  (abaixo da barra ou acima do pill, 6 px), `show/focus/always-on-top` e evento
  `menu_show`).
- **IPC:** `menu_open(kind, anchor)`, `menu_close`, `menu_state`.
- **UI:** rota `menu` (`MenuWindow`) com `AgentMenu` (catálogo, teclado,
  `settings_set_assistant_mode`) e `ChannelMenu` (troca ao vivo + nova sessão).
  `AgentSwitcher`/`ChannelMenu` do notch viram apenas **gatilhos** que enviam o
  `DOMRect` do botão.
- **Fechamento:** escolher item, `Escape`, clique em qualquer outro ponto do
  notch, clique no próprio gatilho (alterna) e perda de foco do popup. O core
  também fecha ao esconder o overlay ou iniciar um arraste.
- **Sem reserva/modo/padding:** removidos `POPOVER_RESERVE`, `reserve_up`,
  `overlay_reserve_state`, `overlay_reserve_changed`, `useOverlayReserve`,
  `data-reserve` e `--popover-reserve`; a janela do notch volta à altura natural
  (`AGENT_SELECTOR_BAND + densidade`) e `overlay_position` volta a guardar o
  topo da janela.
- `tauri.conf.json`/capabilities ganham a janela `menu`.

## Critérios de aceite

- [x] Abrir/fechar agentes e canais não altera a geometria do overlay.
- [x] Notch embaixo: o menu aparece acima; no resto, abaixo.
- [x] Selecionar um agente/canal aplica a ação e fecha o popup.
- [x] O notch continua clicável depois de qualquer menu (regiões não mudam).
- [x] `Escape`, clique fora e clique no gatilho fecham o popup.
- [x] O notch só se move por arraste (ou âncora/densidade).

## Verificação

- `pnpm test` **462 ✓** (73 arquivos) · `tsc --noEmit` ✓.
- `cargo test --lib` **665 ✓** (10 ignorados) · clippy `-D warnings` ✓.
- **Xvfb/AppImage:**
  - overlay no meio `692x106+700+300`; abrir agentes → overlay **idêntico** e
    popup `300x183` imediatamente abaixo da barra (6 px, sem cobrir o notch);
    escolher "Reunião" → pill atualiza e popup fecha; reabrir/fechar funciona;
  - mover o overlay para `700,900` (sem pulo): `overlay_position` `[700,900]`;
    abrir agentes → overlay **idêntico** e popup `300x183` acima do pill
    (6 px, `y=717`);
  - sessão ativa `1001x106+700+900`: menu de canais com largura 200 e altura
    dinâmica acima; "Sistema" aplica e fecha; o notch segue clicável.
- **Pacote final (17:38)** revalidado no AppImage: boot `692x106+700+300`;
  abrir agentes → overlay idêntico, popup `300x183` 6 px abaixo da barra; fechar pelo notch;
  reabrir; `overlay_position` em `[700,300]`.
- Fechamento por perda de foco depende do WM; validado o caminho de clique no
  notch no Xvfb e o listener de foco nos testes.

## Correção no WM real (frame/CSD de 37 px)

No Mutter, a janela do overlay tem `_NET_FRAME_EXTENTS` com **37 px no topo**:
o conteúdo (cliente) começa 37 px abaixo do *outer*. O Tauri no Linux
`set_position` posiciona o **cliente**, mas o tracker antigo salvava o
`outer_position` — a janela subia um frame a cada boot. O menu, por sua vez,
usava o outer do overlay como âncora: para baixo invadia a barra (atrás do
notch); para cima ficava 37–43 px longe.

- Tracker salva `inner_position` (cliente); boot/restore e `set_position`
  ficam na mesma origem (sem deriva).
- O menu ancora no `inner_position` do overlay e é posicionado como cliente.
- O popup reafirma `_NET_WM_STATE_ABOVE` ao abrir e ~150 ms depois (o overlay
  também reafirma o topo).
- `overlay_position` passa a significar o topo do **cliente** (visível).
