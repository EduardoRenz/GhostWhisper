# Issues — PRD Chat

Registry de bugs e problemas deste PRD. Convenções em
[`../../.agents/rules.md`](../../.agents/rules.md) (`ISS-NNN` é canônico por
arquivo de PRD; a numeração **continua** a de
[`../mvp/issues.md`](../mvp/issues.md) para o ID ser único no repositório).
Próximo ID livre: **ISS-065**.

| ID | Resumo | Severidade | Status |
|---|---|---|---|
| ISS-018 | Falas de canais diferentes colidiam na linha temporal (id sem canal) | Alta | FIXED |
| ISS-031 | Bolha de fala crescia sem limite em fala contínua (Interlocutor) | Média | FIXED |
| ISS-063 | Grip só começava o arraste no segundo clique (janela inativa) | Média | FIXED |
| ISS-064 | Atalho não ativa a barra de pesquisa com o app inativo | Média | FIXED |

---

## ISS-018 — Falas de canais diferentes colidiam na linha temporal

- **Severidade:** Alta · **Status:** FIXED (task 01)
- **Sintoma (relato do usuário):** *"não está deixando as falas na timeline
  corretamente, ele continua preenchendo uma frase antiga enquanto outra de outra
  fonte de áudio fala"*.
- **Causa raiz:** o id do segmento era
  `format!("{session_id}-{time_origin_ms}-t{turn}")`
  (`application/transcription_pipeline.rs::publish_turn`) — **sem o canal**.
  Microfone e sistema compartilham `session_id`, origem de tempo e a numeração de
  turno (cada pipeline começa em `turn = 0`), então o turno 0 dos dois canais
  gerava o **mesmo id**. Consequências:
  1. **UI:** `mergeSegment` substitui por id, então a bolha já existente
     (posição e tempo antigos) passava a exibir o texto do **outro** canal —
     exatamente o "preenchendo uma frase antiga".
  2. **Banco:** o `INSERT INTO transcriptions` do segundo final falhava na chave
     primária e o erro só era registrado em `tracing::warn!` — **um lado da
     conversa era perdido** do histórico e do export, silenciosamente.
- **Correção (task 01):** o canal entra no id
  (`{session_id}-{channel}-{origin_ms}-t{turn}`) e as falas em andamento deixam de
  disputar posição na linha temporal (bolhas "em andamento" no fim, como o
  Perssua). Teste de regressão: dois pipelines (mic/sistema) na mesma sessão, com
  o mesmo turno, persistem **dois** segmentos distintos.
- **Arquivos:** `src-tauri/src/application/transcription_pipeline.rs`,
  `src/core/domain/chat.ts`, `src/core/application/useLiveTranscription.ts`,
  `src/features/chat/*`.
- **Task:** [`01-task.md`](tasks/01-task.md)
- **Dano em dados existentes:** sessões antigas com fala simultânea podem ter
  apenas um dos lados no banco; não é recuperável (o áudio não é gravado).

---

## ISS-031 — Bolha de fala crescia sem limite em fala contínua

- **Severidade:** Média · **Status:** FIXED (task 06)
- **Sintoma (relato do usuário):** *"ainda estou incomodado com o Interlocutor,
  como eles falam direto sem interrupções, o balão da mensagem cresce demais;
  queria que tivesse um limite do tamanho do balão, e continua em um próximo"*.
- **Causa raiz:** o turno só fecha com **1 s de silêncio** (`silence_ms`) ou no
  **teto do turno** (`max_turn_ms = 25 s`). Quem fala sem pausas (o outro lado da
  chamada) acumulava até 25 s de texto numa única bolha — e, durante a fala, a
  prévia já ia crescendo do mesmo jeito.
- **Correção (task 06):** teto de **180 caracteres** por bolha de fala
  (`MAX_SPEECH_BUBBLE_CHARS`) com **continuação em bolha seguinte**
  (`splitSpeechText` + `SpeechBubble.continues`), aplicado na fala finalizada, na
  prévia e no histórico. O corte é **de exibição**: o áudio e o texto do ASR não
  mudam. Prefere o fim de frase e nunca parte uma palavra.
- **Arquivos:** `src/core/domain/chat.ts`, `src/features/chat/ChatView.tsx`,
  `src/features/chat/ChatBubble.tsx`, `src/styles/globals.css`.
- **Task:** [`06-task.md`](tasks/06-task.md)
- **Dano em dados existentes:** nenhum (o texto persistido segue inteiro; a
  divisão acontece só na renderização).

---

## ISS-063 — Arraste e redimensionamento só funcionavam no segundo clique

- **Severidade:** Média · **Status:** FIXED (revisão da T17)
- **Sintoma (relato do usuário):** *"o botão drag ao lado esquerdo só funciona
  no segundo click; se eu tento arrastar diretamente não funciona"*. O mesmo
  acontecia no header inteiro e nas alças de redimensionamento: o primeiro gesto
  em uma janela inativa só a ativava.
- **Causa raiz:** em Mutter/X11, o primeiro `ButtonPress` de uma janela inativa é
  consumido pela ativação. O `startDragging()`/`startResizeDragging()` disparado
  no mesmo `pointerdown` chegava antes de a janela estar ativa e o WM o ignorava;
  o segundo clique, com a janela já ativa, funcionava. Havia ainda um segundo
  agravante: o **tooltip nativo** (`title`) do grip e das alças de resize
  aparecia com o mouse parado sobre eles e engolia o primeiro clique.
- **Correção:** o gesto parte do **`mousedown`** e o core usa a API do GTK
  (`gtk_window_begin_move_drag`/`begin_resize_drag`) com o **timestamp atual do
  servidor X11** (`src-tauri/src/ipc/window_commands.rs`). O `start_dragging`
  padrão do Tauri passa timestamp zero; era isso que fazia o primeiro gesto de
  uma janela inativa ser ignorado e deixava o estado alternando entre gestos
  aceitos e recusados. O `title` nativo foi removido do grip e das alças para o
  tooltip não capturar o clique. O comando IPC `panel_start_drag` foi removido
  (arraste/resize agora são comandos `window_start_drag`/`window_start_resize`).
- **Testes anti-regressão (R16/R17):** `windowDrag.test.ts` garante que o
  frontend chama os comandos certos (arraste e cada direção de resize);
  `window_commands.rs` cobre o mapeamento das 8 direções; `WindowHeader.test.tsx`,
  `PanelFrame.test.tsx`, `SettingsPage.test.tsx` e `AskWindow.test.tsx` cobrem
  `mousedown` no header, grip e alças. Validação manual com XTest no binário
  Vulkan: 4/4 arrastes pelo grip e 3/3 pelo header partindo de janela inativa;
  resize 3/3 no canto sudeste.
- **Arquivos:** `src-tauri/src/ipc/window_commands.rs`,
  `src-tauri/Cargo.toml`, `src/adapters/ipc/windowDrag.ts`,
  `src/components/DragHandle.tsx`, `src/components/WindowHeader.tsx`,
  `src/components/windowHeaderDrag.ts`,
  `src/features/panels/ResizeHandles.tsx`, `src/features/panels/PanelFrame.tsx`,
  `src/features/settings/SettingsPage.tsx`,
  `src/features/session-end/SessionEndWindow.tsx`,
  `src/features/recording/RecordingPickerWindow.tsx`.
- **Task:** [`17-task.md`](tasks/17-task.md) (revisão)
- **Dano em dados existentes:** nenhum.


---

## ISS-064 — Atalho não ativa a barra de pesquisa com o app inativo

- **Severidade:** Média · **Status:** FIXED (task 18)
- **Sintoma (relato do usuário):** *"O atalho para pesquisa deveria sempre
  mudar o foco para a barra de pesquisa, mesmo que ela já esteja aberta. mas
  parou de funcionar. clicar no botao muda foco, mas usar o atalho nao"*.
- **Causa raiz:** o `set_focus` do Tauri termina em
  `gtk_window_present_with_time(GDK_CURRENT_TIME)` — **timestamp 0**. Em
  Mutter/X11 a ativação de um app **inativo** com timestamp 0 é recusada pela
  proteção contra roubo de foco; o atalho global roda justamente fora do app
  ativo. Clicar na lupa funciona porque o clique ativa o GhostWhisper antes do
  `show`/foco. É o mesmo mecanismo do ISS-063 (arraste com timestamp zero).
  O código de foco não mudou; a pilha de janelas always-on-top e a barra que
  permanece aberta (ISS-056/ISS-062) tornaram o caminho visível.
- **Correção:** novo `ipc/window_focus.rs` apresenta a janela com o
  **timestamp atual do servidor X11** (`x11_get_server_time`) na thread
  principal do GTK (`run_on_main_thread`), com fallback `set_focus` e
  `present` no Wayland. `show_ask_window` ativa **sempre** (oculta ou já
  aberta), emite `ask_focus` e repete a ativação ~150 ms depois se a janela
  estiver visível e sem foco (`should_refocus`). O frontend mantém o
  `onFocusChanged` + `ask_focus` para devolver o cursor ao campo.
- **Testes anti-regressão (R16/R17):** `should_refocus` cobre os quatro estados
  (visível/oculta × focada/sem foco); `AskWindow.test.tsx` garante que o
  `ask_focus` leva o `document.activeElement` ao campo. Validação manual no
  Xvfb (barra oculta → abre focada; barra aberta sem foco → atalho refoca) e
  na sessão real (Mutter): com o Chrome ativo e a barra aberta, o atalho
  físico levou o cursor ao campo (confirmado pelo usuário). Eventos sintéticos
  do XTest **não** contam como interação do usuário para o Mutter — por isso a
  ativação precisa ser testada com o teclado real.
- **Arquivos:** `src-tauri/src/ipc/window_focus.rs`,
  `src-tauri/src/ipc/mod.rs`, `src-tauri/src/ipc/ask_commands.rs`.
- **Task:** [`18-task.md`](tasks/18-task.md)
- **Dano em dados existentes:** nenhum.
