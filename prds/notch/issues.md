# Issues — PRD Notch

Registry de bugs e problemas deste PRD. Convenções em
[`../../.agents/rules.md`](../../.agents/rules.md) (`ISS-NNN` é canônico por
arquivo de PRD; a numeração **continua** a dos PRDs anteriores para o ID ser único
no repositório). Próximo ID livre: **ISS-045**.

> `ISS-032` segue reservada pelo PRD [`chat`](../chat/issues.md); esta PRD começa em
> `ISS-033` para não colidir.

| ID | Resumo | Severidade | Status |
|---|---|---|---|
| ISS-033 | Idioma da transcrição preso à sessão (troca só valia na próxima) | Média | FIXED |
| ISS-034 | Notch poluído em repouso e controles por texto (fora do padrão Perssua) | Baixa | FIXED |
| ISS-035 | Hover do drill-hover atravessa duas janelas e pode fechar sem querer | Média | FIXED |
| ISS-043 | Notch não fica sempre à frente (`_NET_WM_STATE_ABOVE` se perde no mapeamento) | Alta | FIXED |
| ISS-044 | Notch não encosta no topo: janela 1040×260 com a barra no rodapé (~200 px invisíveis) | Alta | FIXED |

---

## ISS-033 — Idioma da transcrição preso à sessão

- **Severidade:** Média · **Status:** FIXED ([`tasks/04-task.md`](tasks/04-task.md))
- **Sintoma:** trocar o "Idioma de origem" nas Configurações **não** muda a
  transcrição em andamento; só vale depois de encerrar e iniciar outra sessão.
- **Causa raiz:** o idioma faz parte da **chave de cache** do motor em
  `whisper_speech_engine_factory.rs` (`CachedEngine { model, language, use_gpu }`) e
  é um campo **imutável** em `WhisperSpeechToText` (`language: String`). Mudar o
  idioma nas settings não chega ao motor vivo: ou nada muda, ou seria preciso
  recriar o contexto (recarregar o modelo).
- **Correção (task 04):** `language` vira `Mutex<String>` com `update_language()`
  (espelho do `update_prompt` já existente); o idioma **sai da chave do cache** e é
  propagado ao motor vivo; novo IPC `session_set_language`. Vale para o **próximo
  passe** de decodificação (BR-034).
- **Arquivos:** `src-tauri/src/adapters/speech/whisper_speech_to_text.rs`,
  `src-tauri/src/adapters/speech/whisper_speech_engine_factory.rs`,
  `src-tauri/src/ports/speech_engine_factory.rs`,
  `src-tauri/src/ports/session_controller.rs`,
  `src-tauri/src/application/session_service.rs`,
  `src-tauri/src/ipc/session_commands.rs`.
- **Task:** [`tasks/04-task.md`](tasks/04-task.md)
- **Dano em dados existentes:** nenhum (o idioma é configuração; falas antigas já
  transcritas no idioma anterior permanecem).

---

## ISS-034 — Notch poluído em repouso e controles por texto

- **Severidade:** Baixa · **Status:** FIXED ([`tasks/01-task.md`](tasks/01-task.md),
  [`tasks/02-task.md`](tasks/02-task.md))
- **Sintoma:** sem sessão, o overlay ainda exibe rótulos de texto (`Chat`,
  `Histórico`, `Stealth: on/off`, `Ajustes`, `Ocultar`) e os controles de sessão,
  em vez de um notch compacto — diferente da referência de produto (BR-015).
- **Causa raiz:** a `ActionBar` foi construída com botões de texto e sem distinção
  entre o estado **repouso** e o **ativo**; não há pacote de ícones nem botão
  só-ícone.
- **Correção (tasks 01/02):** pacote de ícones SVG próprios, `IconButton` com
  `title`/`aria-label`, e o notch em dois estados com a ação primária verde e o
  atalho em badges (BR-032).
- **Arquivos:** `src/features/overlay/ActionBar.tsx`,
  `src/features/overlay/Overlay.tsx`, `src/components/icons.tsx`,
  `src/components/IconButton.tsx`, `src/components/ShortcutBadges.tsx`,
  `src/styles/globals.css`.
- **Task:** [`tasks/01-task.md`](tasks/01-task.md),
  [`tasks/02-task.md`](tasks/02-task.md)
- **Dano em dados existentes:** nenhum (apresentação).

---

## ISS-035 — Hover do drill-hover atravessa duas janelas

- **Severidade:** Média · **Status:** FIXED ([`tasks/10-task.md`](tasks/10-task.md))
- **Sintoma:** a confirmação é uma janela Tauri separada; ao mover o cursor do
  gatilho para ela, o `pointerleave` do notch pode fechar o popover antes de o
  `pointerenter` da janela `dwell` chegar.
- **Causa raiz:** são dois webviews nativos; a troca de janela gera leave/enter
  separados e não há um `hover` compartilhado.
- **Mitigação (task 10/11):** grace de `dwell_ms` após sair do gatilho **e** da
  confirmação; entrar em qualquer uma das áreas cancela o fechamento; o popover é
  posicionado com gap pequeno. Sem som.
- **Arquivos:** `src/features/dwell/DwellProvider.tsx`,
  `src/features/dwell/DwellPopoverWindow.tsx`,
  `src-tauri/src/adapters/dwell/tauri_dwell_popover.rs`.
- **Task:** [`tasks/10-task.md`](tasks/10-task.md), [`tasks/11-task.md`](tasks/11-task.md)
- **Dano em dados existentes:** nenhum.

---

## ISS-043 — Notch não fica sempre à frente

- **Severidade:** Alta · **Status:** FIXED ([`tasks/14-task.md`](tasks/14-task.md))
- **Sintoma:** o notch (janela `overlay`) some atrás das janelas de outros apps,
  apesar de `alwaysOnTop: true` no `tauri.conf.json`. As janelas de painel
  (Transcrição/Sugestões) continuam por cima; só o notch não.
- **Causa raiz:** o `alwaysOnTop` é aplicado **uma vez na criação** da janela. No
  GTK/Mutter o hint `_NET_WM_STATE_ABOVE` se perde quando a janela é
  mapeada/remodelada (resize do `set_surface`, `hide/show`) e não havia nenhum
  `set_always_on_top` em runtime. Os painéis escapam porque são `show()`n depois.
  Evidência (`xprop` com o app rodando): o overlay (`0x2200003`, sem título)
  tinha só `_NET_WM_STATE_SKIP_PAGER, _NET_WM_STATE_SKIP_TASKBAR`, enquanto
  `chat`/`suggestions` tinham `_NET_WM_STATE_ABOVE`.
- **Correção (task 14):** reafirmar `set_always_on_top(true)` no `show()` e depois
  de cada `set_surface` (dentro de `apply_stealth`, que já reafirma
  skip-taskbar/título) e uma reafirmação única ~900 ms após o boot (o overlay
  nasce visível e não passa pelo `show()`).
- **Arquivos:** `src-tauri/src/adapters/overlay/tauri_overlay_window.rs`,
  `src-tauri/src/lib.rs`.
- **Task:** [`tasks/14-task.md`](tasks/14-task.md)
- **Dano em dados existentes:** nenhum (empilhamento de janela).

---

## ISS-044 — Notch não encosta no topo (janela com ~200 px invisíveis)

- **Severidade:** Alta · **Status:** FIXED ([`tasks/15-task.md`](tasks/15-task.md))
- **Sintoma:** ao arrastar o notch para cima, ele para ~200 px antes do topo — como
  se houvesse uma `div` invisível acima. Barra e arraste parecem limitados.
- **Causa raiz:** duas somas.
  1. O `overlay` estava com `resizable: false` e `height: 260` no
     `tauri.conf.json`; o GTK/Mutter fixa os hints em **min=max=1040×260**
     (`WM_NORMAL_HINTS`) e o `set_size(1040, 72)` do HUD era **ignorado** na
     altura — então a densidade nunca mudava a janela.
  2. O `.overlay` usava `justify-content: flex-end`: a barra (52 px) ficava no
     **rodapé** da janela de 260 px. Ao arrastar, o topo da janela para em y=0,
     mas a barra fica ~200 px abaixo.
- **Correção (task 15):** `overlay` vira **resizable** (sem moldura, sem alça de
  resize), tamanho inicial 1040×84; a barra passa a ficar no **topo**
  (`justify-content: flex-start`, `padding-top: 0`); alturas da janela
  72/84/96 (compacto/padrão/expandido) reservam o banner do ASR abaixo da barra;
  `HUD_MARGIN = 0` (encosta no topo); a reancoragem pela base ao redimensionar
  foi removida (a barra vive no topo).
- **Arquivos:** `src-tauri/tauri.conf.json`,
  `src-tauri/src/domain/hud_layout.rs`,
  `src-tauri/src/adapters/overlay/tauri_overlay_window.rs`,
  `src/styles/globals.css`.
- **Task:** [`tasks/15-task.md`](tasks/15-task.md)
- **Dano em dados existentes:** nenhum (geometria; a posição salva é preservada
  em "Flutuante Livre").

---

<!--
Modelo:

## ISS-0NN — Título curto

- **Severidade:** Alta/Média/Baixa · **Status:** OPEN/FIXED
- **Sintoma:** …
- **Causa raiz:** …
- **Correção:** …
- **Arquivos:** …
- **Task:** [`tasks/NN-task.md`](tasks/NN-task.md)
-->
