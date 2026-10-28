# Issues — PRD Chat

Registry de bugs e problemas deste PRD. Convenções em
[`../../.agents/rules.md`](../../.agents/rules.md) (`ISS-NNN` é canônico por
arquivo de PRD; a numeração **continua** a de
[`../mvp/issues.md`](../mvp/issues.md) para o ID ser único no repositório).
Próximo ID livre: **ISS-085**.

| ID | Resumo | Severidade | Status |
|---|---|---|---|
| ISS-018 | Falas de canais diferentes colidiam na linha temporal (id sem canal) | Alta | FIXED |
| ISS-031 | Bolha de fala crescia sem limite em fala contínua (Interlocutor) | Média | FIXED |
| ISS-063 | Grip só começava o arraste no segundo clique (janela inativa) | Média | FIXED |
| ISS-064 | Atalho não ativa a barra de pesquisa com o app inativo | Média | FIXED |
| ISS-073 | Split de 180 caracteres fragmentava falas e parecia processamento múltiplo | Média | FIXED |
| ISS-075 | Corte de sobreposição fatiando falas em bolhas de 1–5 caracteres | Média | FIXED |
| ISS-079 | Fim do segmento usava o cut do turno e invertia a ordem no chat | Média | FIXED |
| ISS-082 | Fala em andamento ficava atrás do bloco finalizado que ela interrompeu | Média | FIXED |
| ISS-084 | Indicador de fala da aba continuava aceso sem ninguém falando | Média | FIXED |

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

- **Severidade:** Média · **Status:** FIXED (task 06) — **revogada em 2026-10-27** pela ISS-073/BR-102 (o split de 180 saiu; a fala é uma bolha só).
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

---

## ISS-073 — Split de 180 caracteres fragmentava falas e parecia processamento múltiplo

- **Severidade:** Média · **Status:** FIXED (task 19)
- **Sintoma (relato do usuário):** *"Na época pedi que as bolhas fossem menores,
  mas no sentido de concluir a transcrição em bolhas menores. (...) às vezes
  múltiplas bolhas quebradas ficam sendo processadas. O que eu quero é que
  existisse bolhas menores, mas processamento concluído."*
- **Causa raiz:** a BR-031 aplicava o teto de 180 caracteres (`splitSpeechText`)
  na fala finalizada **e na prévia em andamento**. O corte era só de exibição:
  não havia "ouvir um pedaço → processar → commitar → continuar". Uma fala longa
  virava várias bolhas com cara de mensagem fechada enquanto o turno ainda estava
  aberto, e a prévia era re-cortada a cada atualização do Whisper.
- **Correção (task 19):** o split de 180 foi **removido**. Uma fala finalizada é
  **uma** bolha com o texto inteiro e a fala em andamento é **uma** bolha por
  falante (com Streaming). O único corte que permanece é o de sobreposição dos
  canais (BR-043), que existe para intercalar quem falou no meio. IA, tradução e
  copiar usam o texto da própria bolha. Nova **BR-102**; BR-031 revogada.
- **Testes anti-regressão (R16/R17):** `chat.test.ts` ("a fala finalizada continua
  em uma bolha só", "a prévia longa fica em uma única linha em andamento") e
  `ChatView.test.tsx` ("mantém a fala longa em uma bolha só", "mantém a prévia
  longa em uma bolha só"); falhariam no código anterior.
- **Arquivos:** `src/core/domain/chat.ts`, `src/features/chat/ChatView.tsx`,
  `src/features/chat/ChatBubble.tsx`, `src/styles/globals.css`,
  `src/core/domain/chat.test.ts`, `src/features/chat/ChatView.test.tsx`.
- **Task:** [`19-task.md`](tasks/19-task.md)
- **Dano em dados existentes:** nenhum (o texto no banco sempre foi inteiro; a
  divisão era só de renderização).

---

## ISS-075 — Corte de sobreposição fatiando falas em bolhas de 1–5 caracteres

- **Severidade:** Média · **Status:** FIXED (task 20)
- **Sintoma (relato do usuário):** *"Algumas vezes a transcrição picou em muitos
  balões, com frases curtas demais e imprecisas"*.
- **Causa raiz:** `splitSpeechSpans` cortava a fala longa em **qualquer** início
  do outro canal, sem exigir que a fala fosse longa, que a interrupção tivesse
  duração real ou que as partes ficassem legíveis. Na sessão `911fcf10…` (17:15),
  24 falas finais viravam **30 bolhas** na tela, incluindo `"e"` (1 caractere),
  `"vez."` (4), `"79."` (3), `"Ei,"` (3) e `"Beleza!"`/`"Seguim!"`. Os textos no
  banco estavam inteiros — o corte era só de exibição.
- **Correção (task 20):** corte conservador. Só divide fala com **≥ 80
  caracteres e ≥ 6 s**; só considera interrupção com **≥ 1,5 s**; e só divide se
  **todas as partes** tiverem **≥ 20 caracteres e ≥ 3 palavras**. Caso contrário,
  mantém a fala inteira. Na sessão real, cai de 30 para 25 bolhas (e de 11 para 9
  na última), sem os fragmentos de 1–5 caracteres. BR-043 revisada.
- **Testes anti-regressão (R16/R17):** "não parte uma fala curta só porque o outro
  canal começou", "ignora interjeição curta ao intercalar (e não deixa sobras de
  1–5 caracteres)", "mantém a fala inteira quando o corte deixaria um pedaço sem
  contexto" e o caso positivo de intercalação (fala longa + interrupção real).
  Falhariam no código anterior.
- **Arquivos:** `src/core/domain/chat.ts`, `src/core/domain/chat.test.ts`.
- **Task:** [`20-task.md`](tasks/20-task.md)
- **Dano em dados existentes:** nenhum (o texto persistido sempre foi inteiro; a
  divisão é só de renderização).

---

## ISS-079 — Fim do segmento usava o cut do turno e invertia a ordem no chat

- **Severidade:** Média · **Status:** FIXED (task 21)
- **Sintoma (sessão `5984de1d…`):** a fala do usuário (“Está bom, deixa eu
  pensar…”) aparecia **antes** do restante da pergunta do Interlocutor
  (“…sugerir melhorias.”), embora a pergunta tivesse terminado antes.
- **Causa raiz:** `TranscriptionPipeline::publish_turn` gravava
  `end_ms = start_ms + cut_ms`, onde `cut_ms` é o **fim do turno/buffer**, não o
  fim da fala. O `DecodedTurn.speech_end_ms` (VAD/whisper) era calculado e
  ignorado. Na sessão, t7 ia até 127,937 s no banco, mas a fala terminou ~122,5 s;
  o ME começou em 122,765 s. O corte de sobreposição acreditava na sobreposição
  falsa e inseria a bolha do ME antes do resto.
- **Correção (task 21):** `publish_turn` recebe `speech_end_ms` e grava
  `end_ms = start + speech_end.clamp(0, cut)`; prévia, final e promoção do VAD
  usam o fim real. O overlap, a linha temporal e o classificador passam a usar
  esse fim.
- **Testes anti-regressão (R16/R17):** `EarlyEndAsr` (fala 500 ms, cut 2 s) →
  `end_ms=500` na prévia/final; domínio do chat com o caso real (THEY termina
  antes do ME) → uma bolha de THEY, ME depois, sem `#ov`.
- **Arquivos:** `src-tauri/src/application/transcription_pipeline.rs`,
  `src/core/domain/chat.test.ts`.
- **Task:** [`tasks/21-task.md`](tasks/21-task.md)
- **Dano em dados existentes:** sessões antigas mantêm o `end_ms` do turno (o
  áudio não é gravado; não há como reconstruir com precisão). Vale para novas
  sessões.

---

## ISS-082 — Fala em andamento ficava atrás do bloco finalizado que ela interrompeu

- **Severidade:** Média · **Status:** FIXED (task 22)
- **Sintoma (relato de 2026-10-28, sessão `9e6d8300…`):** o bloco do
  interlocutor já estava formado e a fala do ME em andamento (pill **Streaming**)
  aparecia **depois** do bloco inteiro, mesmo tendo começado no meio dele.
- **Causa raiz:** o corte de sobreposição (`splitSpeechSpans`) só considerava
  falas **finalizadas** como fronteira. A prévia entrava na linha temporal pelo
  `start_ms` (ISS-018), mas não dividia o bloco finalizado; como o bloco é uma
  bolha única ancorada no início, a prévia caía depois dele.
- **Correção (task 22):** as prévias passam a ser passadas como `interruptions`
  para `splitSpeechSpans`; elas **não** são divididas (continuam uma bolha por
  falante, BR-102), mas servem de fronteira para o bloco finalizado. O corte
  conservador (≥ 80 caracteres/6 s, interrupção ≥ 1,5 s, partes ≥ 20
  caracteres/3 palavras) continua valendo.
- **Testes anti-regressão (R16/R17/R24):**
  - `chat.test.ts`: prévia intercalada no bloco (genérico) e o caso real da
    sessão `9e6d8300…` (THEY 101,053–121,013 s, prévia ME em 119,462 s);
  - prévia curta (< 1,5 s) e prévia depois do bloco não dividem;
  - a posição é a mesma antes e depois de a prévia virar final.
- **Arquivos:** `src/core/domain/chat.ts`, `src/core/domain/chat.test.ts`,
  `src/features/chat/ChatView.tsx`.
- **Task:** [`tasks/22-task.md`](tasks/22-task.md)
- **Dano em dados existentes:** nenhum (corte de exibição).

---

## ISS-084 — Indicador de fala da aba continuava aceso sem ninguém falando

- **Severidade:** Média · **Status:** FIXED (task 23)
- **Sintoma:** o selo **Streaming**/gradiente da aba Transcrição continuava
  animado depois que a fala terminou, mesmo com ninguém falando.
- **Causa raiz (dupla):**
  1. `publish_activity` usava `had_speech` sozinho; se o buffer guardava áudio
     antigo/ruído e nenhum passe final estava pendente, o sinal ficava preso;
  2. a UI nunca zerava o estado ao pausar/encerrar a sessão.
- **Correção (task 23):**
  - a atividade agora exige **fala recente** (`ACTIVITY_IDLE_TIMEOUT = 2 s`),
    decodificação em voo ou final pendente; o `tick` reavalia a cada 50 ms;
  - `useTranscriptActivity(sessionId, enabled)` zera quando a sessão está
    pausada/parada (`state.listening && !state.paused`).
- **Testes anti-regressão (R16/R17):**
  - `activity_signal_expires_without_recent_speech`;
  - `transcript_activity_store_tracks_the_channel_until_the_final`;
  - `useTranscriptActivity` (“zera o sinal quando a sessão pausa ou para”).
- **Arquivos:** `src-tauri/src/application/transcription_pipeline.rs`,
  `src/core/application/useTranscriptActivity.ts`,
  `src/core/application/useTranscriptActivity.test.tsx`,
  `src/features/panel/PanelWindow.tsx`.
- **Task:** [`tasks/23-task.md`](tasks/23-task.md)
- **Dano em dados existentes:** nenhum.
