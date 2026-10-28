# Histórico — PRD Chat

Histórico deste PRD no formato fixo do
[`rules.md`](../../.agents/rules.md) §6.2 (busca por palavra-chave, ID e período).

| Objetivo | Comando |
|----------|---------|
| Palavra-chave | `rg -n -i 'bolha' prds/chat/memory.md` |
| Entrada do dia | `rg -n '^## 2026-10-18' prds/chat/memory.md` |
| Por ID | `rg -n 'ISS-018\|BR-021\|BR-022\|BR-026' prds/chat/memory.md` |
| Bolha / uma fala por bolha | `rg -n -i 'teto da bolha\|uma fala finalizada\|split de 180' prds/chat/memory.md` |
| Corte de sobreposição | `rg -n -i 'sobreposição\|ISS-075' prds/chat/memory.md` |
| Janelas de chat/histórico | `rg -n -i 'janela de chat\|janela de histórico\|toggle único' prds/chat/memory.md` |
| Transcrição ao Vivo | `rg -n -i 'transcrição ao vivo\|largura total\|microfone/outros' prds/chat/memory.md` |
| Sugestão manual | `rg -n -i 'sugestão manual\|manual_context\|contexto por parâmetro' prds/chat/memory.md` |
| Concorrência | `rg -n -i 'single-flight\|em andamento\|janela oculta\|prioridade' prds/chat/memory.md` |

---

## 2026-10-18 — PRD Chat: bolhas, transporte por ícones e o bug de linha temporal

- **IDs:** ISS-018, BR-021, BR-022, T01–T05
- **Refs:** [`prd.md`](prd.md), [`issues.md`](issues.md),
  [`architecture/chat-ui.md`](architecture/chat-ui.md),
  [`tasks/_tasks.md`](tasks/_tasks.md)

### Como a PRD nasceu
O usuário pediu a interface de chat e os controles de play/pause/stop **iguais aos
do Perssua**, extraídos do bundle (`app.asar`). Durante a investigação apareceu um
bug funcional de verdade: as falas **não entravam na linha temporal corretamente** —
uma frase antiga continuava sendo preenchida enquanto outra fonte de áudio falava.

### ISS-018 — causa raiz e correção
- O id do segmento era `{session_id}-{origin}-t{turn}`, **sem o canal**; mic e
  sistema compartilham sessão, origem e `turn` (cada pipeline começa em 0) ⇒
  **ids idênticos**. A UI substituía a bolha (texto do outro canal na posição
  antiga) e o `INSERT` do segundo final falhava na chave primária (log `warn`,
  dado perdido do histórico e do export).
- Correção: canal no id (`{session_id}-{channel}-{origin_ms}-t{turn}`) + falas em
  andamento fora da linha temporal + `mergeSegment` nunca deixa uma prévia
  sobrescrever um final.

### Decisões (respostas do usuário)
- Timeline **única**: falas e sugestões da IA na mesma conversa.
- **Sem** quebra de fala longa (paridade pura: uma bolha por segmento).
- Pasta `prds/chat`, issues continuando de `ISS-018`.
- Extras de paridade aceitos: bolinha de status, pílula "Iniciar escuta" e pílula
  de sessão com tempo + confirmação; tradução por bolha ficou **de fora**.

### Desvios conscientes do Perssua (R12)
- Play e pause são **dois botões** (o Perssua alterna num só) — pedido do usuário.
- O ⏹ também confirma o encerramento (no Perssua a confirmação vive só na pílula de
  sessão): evita um caminho que encerra sem confirmação.
- "Iniciar escuta" acumula os papéis de "Start Listening" e "Restart".
- Ícones SVG próprios em vez da fonte Material Icons (BR-008).
- `mm:ss` em cada bolha (o Perssua não tem tempo no ao vivo) — BR-019.

### Entregue
- **T01** bolhas intercaladas (`features/chat/{ChatView,ChatBubble}.tsx`,
  `core/domain/chat.ts`) + correção do id do segmento no pipeline.
- **T02** sugestões da IA na mesma conversa (o `InsightsPanel` deixou de existir;
  "Sugerir" foi para o cabeçalho) e detalhe do histórico em bolhas.
- **T03** transporte por ícones (`components/TransportIcons.tsx`,
  `SessionControls` com a pílula `[▶][⏸][⏹]` e "Iniciar escuta").
- **T04** pílula de sessão com `mm:ss` + confirmação "Encerrar sessão? Sim/Não",
  bolinha de status; `SessionStateEvent.started_at_ms` novo.
- **T05** BR-021/BR-022, README, ponteiro em `mvp/architecture/overlay.md`,
  verificação e pacotes.

### Verificação
- **217 testes Rust** (+2 desta rodada; 8 `ignored`) · clippy/fmt limpos.
- **74 testes frontend** (+10) · `tsc --noEmit` limpo · `pnpm build` ok.
- Binário release sobe; `.deb` e AppImage reconstruídos.

### Pendências
- Sessões antigas com fala simultânea podem ter um lado perdido (ISS-018,
  irreversível — o áudio não é gravado).
- Tradução dentro da bolha, busca no histórico e paginação continuam fora de escopo.
- Loop de frase do Whisper: corrigido no núcleo de ASR — ver **ISS-019** em
  [`../mvp/issues.md`](../mvp/issues.md) (task 21 do PRD do MVP).

## 2026-10-18 — Teto da bolha de fala: fala longa continua em bolha seguinte

- **IDs:** ISS-031, BR-031, T06
- **Refs:** [`issues.md`](issues.md), [`business-rules.md`](../business-rules.md),
  [`architecture/chat-ui.md`](architecture/chat-ui.md) §2–§6,
  [`tasks/06-task.md`](tasks/06-task.md)

### Problema (relato no teste)
O **Interlocutor** fala sem pausas e o turno só fecha no silêncio (1 s) ou no teto
do turno (**25 s**) — a bolha virava um bloco gigante. O usuário pediu "um limite
do tamanho do balão, e continua em um próximo".

### Decisão
- Teto de **180 caracteres** por bolha (`MAX_SPEECH_BUBBLE_CHARS`), com
  **continuação** em bolha seguinte (`SpeechBubble.continues`, id `#N`), sem
  repetir rótulo/tempo (`chat__row--continues`).
- Corte **puro e determinístico** (`splitSpeechText`): fim de frase primeiro
  (`.!?…`, `\n`), fim de **palavra** quando a frase sozinha passa do teto — nunca
  no meio de uma palavra.
- Aplicado na fala finalizada, na **prévia em andamento** e no **histórico**
  (`bubblesFromTimeline`); o `…` fica só na última parte.
- É **exibição**: o áudio e o texto do ASR não mudam; `max_turn_ms` continua sendo
  só o teto de custo do whisper (BR-029).

### Evidência do desvio (R12)
O Perssua **não** limita a bolha: `.unified-message-bubble` (CSS
`4598.dc3b562d.css`) só tem `max-width:75%` — nenhum `max-height`/`line-clamp`.
Lá o *commit* em nuvem (~2 s) mantém a bolha curta; no whisper local o commit é por
silêncio, então o teto visual faz o papel que o *commit* faz lá em cima.

### Verificação
- `pnpm test` **134 ✓** (24 arquivos, +11) · `tsc --noEmit` ✓ · `cargo test --lib`
  330 ✓ (backend intocado) · AppImage/deb reconstruídos.

## 2026-10-18 — Janelas de Chat/Histórico e transporte com um só stop

- **IDs:** BR-022, BR-026, T07–T09
- **Refs:** [`prd.md`](prd.md), [`tasks/_tasks.md`](tasks/_tasks.md),
  [`architecture/chat-ui.md`](architecture/chat-ui.md) §7,
  [`../business-rules.md`](../business-rules.md) BR-022/BR-026

### Pedido (respostas do usuário)
- A conversa ao vivo vira uma **janela própria**, movível e redimensionável "como o
  resumo"; um botão no overlay esconde/mostra.
- O **histórico de sessões** (aba do painel que saiu) também virou janela própria —
  decisão entre três alternativas, escolhida a que mantém cada superfície isolada.
- O stop estava **duplicado** (⏹ do transporte + pílula vermelha): remover o do
  transporte, deixar play/pause como **um switch** e o encerrar com vermelho +
  confirmação "como funciona hoje".

### Entregue
- **T07** janela `chat`: `ChatWindow` (`PanelFrame` + medidores + `ChatView`) e
  `ChatHeaderActions` (Sugerir/Resumo/Sugestões); rota `chat`; `Settings.chat_*`; o
  overlay virou **só a ActionBar** (`Surface` = `actionbar|hidden`, sem painel,
  abas ou `expanded-tall`).
- **T08** janela `history`: `HistoryWindow` embrulha o `HistoryPanel`;
  `Settings.history_*`; "Continuar sessão" volta a capturar e abre a janela de Chat.
- **T09** `SessionControls` com **um** controle (parado → "Iniciar escuta";
  gravando → ⏸; pausado → ▶); o encerrar ficou **só** na `SessionPill`. BR-022
  revisada.
- Layout dos quatro painéis unificado em `domain::settings::PANEL_LABELS` + acessores
  (`panel_position`/`panel_size`), usados por boot, tracker, fechar-da-janela e
  `panel_open`/`panel_hide`; `capabilities` e `tauri.conf.json` ganharam `chat` e
  `history`.

### Verificação
- **331 testes Rust** (8 `ignored`) · clippy/fmt limpos.
- **138 testes frontend** (+20: `ChatWindow`, `HistoryWindow`, rotas, matriz de
  `SessionControls` reescrita) · `tsc --noEmit` e `pnpm build` ok.

## 2026-10-19 — Planejamento: "Transcrição ao Vivo" e sugestão manual por mensagem

- **IDs:** BR-040, BR-041, BR-042, T10–T15
- **Refs:** [`prd.md`](prd.md) §9,
  [`architecture/transcription-panel.md`](architecture/transcription-panel.md),
  [`tasks/10-task.md`](tasks/10-task.md)…[`tasks/15-task.md`](tasks/15-task.md),
  [`../suggestions/architecture/suggestions-window.md`](../suggestions/architecture/suggestions-window.md) §5.4

### Pedido (print anexado)
- Layout o mais fiel possível ao print: **"Transcrição ao Vivo"**, abas
  **Transcrição | Resumo** em largura total, mensagens em cartões que preenchem a
  janela.
- **Microfone** em cor primária com o nome e a **hora da mensagem**; **Outros**
  na outra cor indicada (violeta).
- Balões **full-width**, independente do tamanho do texto.
- Cada mensagem com um botão `[✦]` no canto superior direito; ao clicar, a
  mensagem vai **via parâmetro** para o Sugestões, onde o usuário gera uma
  sugestão **Manual**.

### Decisões de planejamento
- **Sem falsa paridade:** o rodapé do print mostra "Latência: 48ms · WebRTC"; não
  medimos latência e o produto não usa WebRTC. O espaço equivalente mostra
  `Whisper {modelo} · {Gravando|Pausado|Sem sessão}` (BR-040).
- **Hora de parede:** o print mostra `HH:MM:SS`; a UI deriva de
  `started_at_ms + at_ms`. O banco continua com `at_ms` relativo à sessão
  (BR-019 revisada), sem migração.
- **Ação de IA só em fala finalizada e só na primeira parte** do split de 180
  caracteres (BR-031); a prévia ainda muda e a continuação é a mesma mensagem. O
  `source_text` do segmento garante que a sugestão receba o texto inteiro.
- **Clique no ✦ não gera:** abre/foca o Sugestões com o contexto e o usuário
  aperta **"Gerar sugestão manual"**. Evita custo/LLM surpresa (o pedido diz
  "pelo usuário").
- **Rótulo "Manual"** vem de `Suggestion.trigger = manual`, não de heurística de
  texto; rápida/completa continuam valendo (BR-038).

### Análise da arquitetura (contexto via parâmetro)
- A intuição do usuário está correta: o caso de uso já recebe contexto por
  parâmetro (`SuggestionRequest`), e `useSuggestions` já é parametrizado. O que
  faltava era um **parâmetro tipado de origem manual** e um **handoff robusto**
  entre duas janelas Tauri.
- Evento efêmero sozinho seria frágil: se a janela de Sugestões estivesse
  fechada, o listener ainda não existiria. A solução planejada é um
  **`ManualSuggestionContextStore`** no núcleo (um contexto por vez, keyed por
  sessão): o chat grava e chama `panel_open("suggestions")`; a janela busca ao
  montar e quando recebe `panel_opened`. O **caso de uso** recebe o contexto como
  `SuggestionRequest.manual_context`; o store é só correio, não estado global de
  UI.
- O automático do core não usa `manual_context`; "Sugerir agora" continua a
  sugestão normal. Sem schema novo, sem backend novo.

### Concorrência e janela (aviso do usuário + código de 2026-10-19)
O usuário alertou para não se perder quando **já existe uma sugestão em
andamento** e a janela está aberta/fechada. O código evoluiu antes da
implementação (commit `f3ea741`): o automático **só gera com a janela de
Sugestões aberta**, **abrir a janela não gera** (a contagem reinicia na
abertura) e o `SuggestionService` já tem um **`gate`** que serializa as gerações
com o cursor mais avançado. Decisões (BR-042, task 15) alinhadas a isso:
- **Single-flight no serviço** (`gate` existente): automático e manual nunca
  emitem `quick`/`full` intercalados; a manual **espera** a atual. Sem
  cancelamento de LLM em voo.
- **Prioridade do manual:** com contexto manual ativo, a automática `full` vai
  para o histórico e a `quick` é ignorada; o cartão manual não é substituído.
  Ao limpar o contexto, a automática volta a poder ocupar o cartão.
- **Abrir não gera:** a janela hidrata com `suggestionLast` (cache, sem LLM); o
  automático gera com a janela aberta e após o intervalo, e o botão “Sugerir
  agora” é a ação explícita.
- **Novo `[✦]` durante a manual:** o contexto novo substitui o antigo; a manual
  antiga termina com `Suggestion.source` (falante/`at_ms`/preview) e o botão
  libera para o contexto novo.
- **Troca de sessão:** limpa a leitura e ignora eventos/resultados antigos.
- **`generation_id` (UUID por geração):** a UI usa o id como chave de
  `current`/`quick`/histórico; `at_ms` pode empatar (relógio fixo/teste) e não
  serve como identidade. O gate evita intercalação; o id identifica a geração;
  `trigger`/`source` rotulam.

### Entregável desta rodada
> Esta entrada registra o **planejamento**; a implementação está na entrada
> seguinte, de 2026-10-19.
- **Nada implementado nesta etapa.** PRD §9, arquitetura `transcription-panel.md`, tasks
  10–15 (`PENDING`), BR-040/BR-041/BR-042 e revisões de BR-019/BR-021
  registradas. A task 14 é o fechamento e vem depois da 15 (`13 → 15 → 14`).
- Próximo passo: task 10 (casca) e task 11 (cartões), depois 12 (handoff), 13
  (Sugestões manual), 15 (concorrência) e 14 (fechamento).

## 2026-10-19 — Implementação da v0.2: transcrição ao vivo, sugestão manual e concorrência

- **IDs:** BR-040, BR-041, BR-042, T10–T15
- **Refs:** [`prd.md`](prd.md) §9,
  [`architecture/transcription-panel.md`](architecture/transcription-panel.md),
  [`tasks/10-task.md`](tasks/10-task.md)…[`tasks/15-task.md`](tasks/15-task.md)

### Código que chegou antes da implementação
A base já tinha evoluído (commits `f3ea741`/`feeba77`): o automático **só gera
com a janela de Sugestões aberta**, abrir a janela **reinicia** a contagem e não
gera, e o `SuggestionService` já tinha um **`gate`** com cursor mais avançado.
A implementação foi alinhada a isso (não reintroduziu geração ao abrir).

### Entregue
- **T10** cabeçalho “Transcrição ao Vivo” com `WaveformIcon`, abas full-width com
  ativa em contorno cyan, rodapé “Idioma de Entrada:” + `Whisper {modelo} ·
  {estado}`; medidores e atalho “Sugestões” saíram da aba; grip oculto no chat e
  arraste pelo heading.
- **T11** cartões full-width, Microfone/Outros com cores, hora de parede
  `HH:MM:SS` (`formatMessageClock`), “Falando agora…” + Streaming, `source_text`
  do segmento e histórico com o mesmo contrato.
- **T12** ação `[✦]` por fala finalizada (texto inteiro), `ManualSuggestionContext`
  + store em memória, IPC `suggestion_prepare_manual`/`suggestion_manual_context`/
  `suggestion_clear_manual_context`; o clique não chama LLM.
- **T13** `SuggestionRequest.manual_context`, `trigger`/`generation_id`/`source`,
  prompt `<selected_message>`, `useManualSuggestionContext`,
  `ManualContextCard` e rótulos “Sugestão Manual”/“Manual”.
- **T15** `suggestion_last` + hidratação sem LLM; prioridade do manual
  (automática full → histórico, quick ignorada); abertura nunca gera;
  `generation_id` como chave de estado/histórico; gate existente cobre a manual.
- **T14** documentação/regras/memória atualizadas; tasks 10–15 `CONCLUDED`.

### Verificação
- `cargo test --lib` **378 ✓** (8 `ignored`) · `cargo clippy --all-targets` ✓ ·
  `cargo fmt --check` ✓.
- `pnpm test` **261 ✓** (47 arquivos) · `tsc --noEmit` ✓ · `pnpm build` ✓.
- Pacotes `.deb`/AppImage **não** foram reconstruídos nesta rodada.

### Desvios/decisões mantidas
- Rodapé sem latência/“WebRTC” inventados (usa estado real do Whisper).
- Hora de parede na UI; banco continua relativo à sessão (BR-019).
- Ação de IA só em fala finalizada/primeira parte; prévia/continuação não.
- Manual espera o automático (sem cancelamento de LLM); automática não rouba o
  cartão enquanto o contexto manual está ativo.
- Contexto manual é efêmero (memória), sem schema.

## 2026-10-19 — Intercalação por sobreposição e grip padrão (task 16)

- **IDs:** BR-043, T16
- **Refs:** [`prd.md`](prd.md) §9 (US-14),
  [`architecture/transcription-panel.md`](architecture/transcription-panel.md) §3.4,
  [`tasks/16-task.md`](tasks/16-task.md)

### Pedido (teste do usuário)
- “Se os outros estão falando sem parar e eu falo em seguida, ele vai jogando
  minha mensagem atrás.” Queria cortar a fala longa no ponto em que o outro
  começou, colocar a nova no meio e continuar o resto em outra bolha.
- O grip de arraste precisava ser padrão no canto superior direito de todas as
  janelas.

### Entregue
- `splitSpeechSpans` em `core/domain/chat.ts`: encontra o início do outro
  falante dentro do intervalo, divide o texto por proporção de tempo (fim de
  frase → palavra), mantém a primeira parte no tempo original e coloca as
  seguintes em `boundary + 1` ms. Cada parte vira uma bolha normal, com
  rótulo/hora; se o corte não for seguro, mantém a fala inteira.
- Aplicado no `buildChatTimeline` (ao vivo) e `bubblesFromTimeline` (histórico);
  `TimelineKind::Speech` ganhou `end_ms` (sem schema) para o histórico.
- `PanelFrame`: `DragHandle` movido para o canto superior direito, visível em
  todas as janelas; o heading continua arrastando.
- Testes: intercalação simétrica, sem sobreposição e histórico com `end_ms`.

### Verificação
- `pnpm test` **265 ✓** · `cargo test --lib` **378 ✓** · `tsc`/`build` ✓.
- AppImage/deb reconstruídos (19:47) e app aberto com o novo build.

## 2026-10-21 — Header padrão e janelas menos arredondadas (task 17)

- **IDs:** BR-064, T17
- **Refs:** [`prd.md`](prd.md) (CA-14.5),
  [`architecture/transcription-panel.md`](architecture/transcription-panel.md) §2,
  [`../notch/architecture/design-tokens.md`](../notch/architecture/design-tokens.md) §4/§6,
  [`tasks/17-task.md`](tasks/17-task.md)

### Pedido (mockup do usuário)
- Grip no canto esquerdo em todas as janelas; hoje algumas não tinham.
- Título com margem; alguns headers colavam o texto na borda.
- Cantos menos arredondados, usando os balões de conversa como referência.
- Remover a moldura nativa do SO na janela de Configurações.

### Entregue
- `WindowHeader` comum: grip à esquerda, selo 32 px, título/subtítulo, ações e
  `×` à direita; padding 16/8 px e altura mínima 56 px.
- `PanelFrame` passou a usar o header comum (`icon`/`titleExtra`); Chat troca o
  ícone por aba, Sugestões usa a faísca + LIVE e Histórico usa o ícone próprio.
- `--oh-window-radius` (= `--oh-radius-sm`, 8 px) aplicado ao `.panel` e ao
  `.settings` (e ao `.app`), no lugar de 24/12 px.
- Configurações sem moldura: `decorations: false`, `transparent: true`,
  `shadow: false`, shell de 8 px, `ResizeHandles` e o mesmo header; o fechar
  continua em `settings_close`.
- Em painéis estreitos as ações quebram e o rótulo “Auto-sugerir” some (o anel
  e a contagem permanecem), para o título não ser truncado.

### Verificação
- `pnpm test` **359 ✓** · `tsc --noEmit` ✓ · `pnpm build` ✓.
- `cargo test --lib` **545 ✓** · `clippy`/`rustfmt` ✓.
- Validação visual no navegador (Chat, Sugestões, Histórico e Configurações)
  em 1020/560/540/520/360 px; AppImage/deb gerados (09:53).
- Nesta sessão o WebKitGTK parou de pintar o conteúdo do AppImage (afeta também
  o build anterior e o MiniBrowser); DOM/bundle foram validados no WebKit e no
  Chrome, e a janela sem moldura está configurada no `tauri.conf.json`.

## 2026-10-24 — PRD `translation` removido; tradução fica como está

- **IDs:** BR-030, ISS-029, ISS-030
- **Refs:** [`../business-rules.md`](../business-rules.md) BR-030,
  [`architecture/chat-ui.md`](architecture/chat-ui.md),
  `src/features/chat/ChatBubble.tsx`, `src/core/application/useTranslation.ts`

### Decisão
- O usuário encerrou a frente de tradução: a **T1** (tradução sob demanda na
  bolha, já implementada) fica **como está** e a **T2** (gate por idioma
  detectado, ISS-030) **não será feita**.
- O PRD `prds/translation/` foi **removido** (recuperável no histórico do Git);
  BR-030 passa a ser a fonte única do comportamento, agora sem a promessa de
  gate por idioma.
- Referências quebradas corrigidas em `business-rules.md`, `notch/prd.md`,
  `precision-and-performance/tasks/02-task.md` e `memory.md`.

## 2026-10-24 — Arraste pelo header inteiro das janelas (revisão da BR-064)

- **IDs:** BR-064, T17
- **Refs:** [`../business-rules.md`](../business-rules.md) BR-064,
  [`tasks/17-task.md`](tasks/17-task.md),
  [`architecture/transcription-panel.md`](architecture/transcription-panel.md) §2,
  `src/components/WindowHeader.tsx`, `src/components/windowHeaderDrag.ts`

### Pedido
- O usuário pediu que o arraste da janela funcione em **todo o header**, não só
  no ícone de grip à esquerda; o ícone continua visível como indicação.
- Vale para qualquer janela do app que use o header padrão.

### Entregue
- `WindowHeader` ganhou `onPointerDown`: áreas inertes (grip, selo, título,
  subtítulo, espaços e a dica `Esc`) chamam `onStartDrag`; controles e a própria
  alça são ignorados (a alça já tem handler, evitando `startDragging` duplo).
- Regra pura `shouldStartHeaderDrag` cobre botões, abas, links, campos,
  `contenteditable`, `role=...`, `.drag-handle` e a escotilha
  `data-no-window-drag`; só o botão primário arrasta.
- CSS: `.window-header` com `user-select: none`; o grip mantém `cursor: move`.
- Painéis, Configurações, Sessão Finalizada e Seletor de Gravação herdam o
  comportamento por usarem `WindowHeader`; notch/AskBar continuam só no grip.

### Verificação
- `pnpm test` **483 ✓** (75 arquivos) · `tsc --noEmit` ✓.
- Testes novos: `WindowHeader.test.tsx`, `windowHeaderDrag.test.ts`; casos de
  header em `PanelFrame.test.tsx` e `SettingsPage.test.tsx`.
- AppImage/deb Vulkan gerados por `pnpm tauri:build --bundles appimage`
  (`GhostWhisper_0.1.0_amd64.AppImage`, 15:58); o asset `index-CF0mEfMm.js`
  embutido confere com o `dist/` novo.

## 2026-10-24 — Arraste e resize só no segundo clique (ISS-063)

- **IDs:** ISS-063, BR-064, T17
- **Refs:** [`issues.md`](issues.md) ISS-063,
  [`../business-rules.md`](../business-rules.md) BR-064,
  [`architecture/transcription-panel.md`](architecture/transcription-panel.md) §2,
  `src-tauri/src/ipc/window_commands.rs`, `src/adapters/ipc/windowDrag.ts`

### Sintoma
- O grip à esquerda (e depois o header inteiro) só arrastava no **segundo**
  clique; o primeiro apenas ativava a janela. As alças de resize tinham o mesmo
  comportamento.
- Causa: o `start_dragging` do Tauri usa `gtk_window_begin_move_drag` com
  **timestamp zero**. Em Mutter/X11 o primeiro gesto de uma janela inativa era
  ignorado e o estado ficava alternando entre gestos aceitos e recusados. O
  tooltip nativo (`title`) do grip/alças também capturava o primeiro clique.
  Focar/esperar `isFocused` e iniciar no `pointermove` não resolveram.

### Entregue
- O gesto parte do **`mousedown`** em `DragHandle`, `WindowHeader` e
  `ResizeHandles`.
- Novos comandos `window_start_drag`/`window_start_resize`
  (`window_commands.rs`) chamam a API do GTK
  (`begin_move_drag`/`begin_resize_drag`) com o timestamp atual do servidor X11
  (`x11_get_server_time`). Em Wayland/fallback usam o caminho do Tauri.
- `title` nativo removido do grip e das alças (o `aria-label` do grip fica).
- Comando `panel_start_drag` removido (arraste direto na janela atual).

### Verificação
- `pnpm test` **485 ✓** (76 arquivos) · `tsc --noEmit` ✓ ·
  `cargo test --lib` **682 ✓** (10 ignorados) · `rustfmt` ✓.
- `windowDrag.test.ts`: comandos e direções; `window_commands` testa o
  mapeamento das 8 direções; header/grip/alças em `WindowHeader`,
  `PanelFrame`, `SettingsPage` e `AskWindow`.
- Validação manual com XTest no binário Vulkan: 4/4 arrastes pelo grip e 3/3
  pelo header saindo de janela inativa; resize 3/3 no canto sudeste.
- AppImage/deb Vulkan republicados (21:04); SHA-256 do AppImage
  `06026cf6…a218794`.

## 2026-10-25 — Atalho de pesquisa não ativava a barra (ISS-064)

- **IDs:** ISS-064, BR-052, T18 · **Refs:** [`issues.md`](issues.md),
  [`tasks/18-task.md`](tasks/18-task.md)

### Sintoma
O atalho global de pesquisa (`focus_ask`, Ctrl+Shift+C no perfil do usuário)
abria a barra, mas **não entregava o foco** com o GhostWhisper em segundo
plano; clicar na lupa focava. O problema aparecia tanto ao abrir quanto com a
barra já aberta.

### Causa
O `set_focus` do Tauri termina em
`gtk_window_present_with_time(GDK_CURRENT_TIME)` — timestamp **0**. Em
Mutter/X11 a ativação de um app inativo com timestamp zero é recusada pela
proteção contra roubo de foco. O clique na lupa funciona porque o próprio
clique ativa o app antes. É o mesmo mecanismo do arraste (ISS-063), que já
usava `x11_get_server_time`; o foco nunca tinha recebido o mesmo tratamento.

### Entregue
- `ipc/window_focus.rs`: `focus_window` apresenta na thread principal do GTK
  com o timestamp atual do servidor X11; fallback `set_focus` e `present` no
  Wayland. `should_refocus(visible, focused)` centraliza a decisão de repetir.
- `show_ask_window` ativa **sempre** (não só ao sair de oculta), emite
  `ask_focus` e repete ~150 ms depois se a janela estiver visível e sem foco.
- Sem mudança no frontend: `onFocusChanged` + `ask_focus` continuam devolvendo
  o cursor ao campo.

### Verificação
- Xvfb: oculta → abre com `XGetInputFocus` na janela `ask`; aberta sem foco →
  atalho refoca.
- Mutter: com o Chrome ativo e a barra aberta, o atalho físico levou o cursor
  ao campo (confirmado pelo usuário). XTest sintético não conta como interação
  do usuário para o Mutter; a validação válida é com o teclado real.
- `should_refocus` (4 estados), `AskWindow.test.tsx`; `cargo test --lib`
  **693 ✓**, clippy `-D warnings` ✓.

## 2026-10-27 — Uma fala finalizada é uma bolha (o split de 180 saiu)

- **IDs:** ISS-073, BR-102, T19 · **Refs:** [`issues.md`](issues.md),
  [`../business-rules.md`](../business-rules.md) BR-102,
  [`tasks/19-task.md`](tasks/19-task.md)

### Sintoma
Depois da ISS-031, falas longas ainda apareciam como várias bolhas "quebradas"
enquanto o turno estava aberto. O usuário: *"às vezes múltiplas bolhas quebradas
ficam sendo processadas. O que eu quero é que existisse bolhas menores, mas
processamento concluído."*

### Causa
A BR-031 aplicava o teto de 180 caracteres (`splitSpeechText`) na fala finalizada
**e na prévia**. Era um corte de **exibição**, não um commit: não existia "ouvir
um pedaço → processar → commitar → continuar". A prévia era re-cortada a cada
atualização do Whisper e as partes pareciam mensagens fechadas. O usuário pediu
que, sem esse commit, a fala ficasse inteira.

### Decisão
- **Removido** o split de 180: `MAX_SPEECH_BUBBLE_CHARS`, `splitSpeechText`,
  `chopAtWords`, `continues` e `source_text` saíram do domínio.
- Fala **finalizada** = uma bolha com o texto inteiro; fala **em andamento** =
  uma bolha por falante, com Streaming. O único corte que permanece é o de
  **sobreposição** dos canais (BR-043).
- IA, tradução e copiar usam o texto da própria bolha; a prévia não tem IA.
- Commit em blocos ("ouvir → processar → commitar → continuar") **não é
  possível** hoje sem degradar o ASR: o pipeline só fecha no silêncio (1 s), no
  teto de 25 s ou no experimento de pontuação forte rejeitado (ISS-058). O
  caminho por prefixo autoritativo ficou registrado como experimento futuro na
  ISS-074 (`precision-and-performance`).
- Trade-off aceito: a bolha pode ficar alta (até ~25 s); sem recolhimento
  automático.

### Entregue
- `src/core/domain/chat.ts`: `SpeechBubble` sem `continues`/`source_text`;
  timeline e histórico com uma bolha por fala (só a sobreposição divide).
- `ChatView`: prévia única; IA/tradução/cópia com o texto da bolha.
- `ChatBubble`: prop `continues` removida; header e ações sempre na bolha.
- CSS: `.chat__row--continues` removido.

### Verificação
- Anti-regressão (falhariam antes): `chat.test.ts` ("a fala finalizada continua
  em uma bolha só", "a prévia longa fica em uma única linha em andamento") e
  `ChatView.test.tsx` ("mantém a fala longa em uma bolha só", "mantém a prévia
  longa em uma bolha só").
- `pnpm test` **517 ✓** (82 arquivos) · `tsc --noEmit` ✓ · `pnpm build` ✓.

## 2026-10-27 — Corte de sobreposição conservador (fim das bolhas de 1–5 caracteres)

- **IDs:** ISS-075, BR-043, T20 · **Refs:** [`issues.md`](issues.md),
  [`../business-rules.md`](../business-rules.md) BR-043,
  [`tasks/20-task.md`](tasks/20-task.md)

### Sintoma
*"Algumas vezes a transcrição picou em muitos balões, com frases curtas demais e
imprecisas."* Na sessão `911fcf10…` (17:15), 24 falas finais viravam **30
bolhas** na tela.

### Causa
`splitSpeechSpans` cortava a fala longa em **qualquer** início do outro canal,
sem exigir fala longa, interrupção real ou partes legíveis. Saíam bolhas de
`"e"` (1c), `"vez."` (4c), `"79."` (3c), `"Ei,"` (3c) e
`"Beleza!"`/`"Seguim!"`. O texto no banco estava inteiro — o corte era só de
exibição.

### Decisão
- Só dividir fala com **≥ 80 caracteres e ≥ 6 s**.
- Só considerar interrupção do outro canal com **≥ 1,5 s**.
- Exigir **≥ 20 caracteres e ≥ 3 palavras** em todas as partes; senão, manter a
  fala inteira.
- BR-043 revisada; nada no ASR/banco.

### Entregue
- `src/core/domain/chat.ts`: limiares `MIN_OVERLAP_*` em `splitSpeechSpans`.
- Testes: fala curta + outro canal; interjeição curta; sobra sem contexto; caso
  positivo de intercalação (ao vivo e histórico).

### Verificação
- Simulação com os dados reais: **30 → 25** bolhas na `911fcf10…` e **11 → 9** na
  `a60933ea…`; os fragmentos de 1–5 caracteres somem.
- `pnpm test` **520 ✓** (82 arquivos) · `tsc --noEmit` ✓ · `pnpm build` ✓.

## 2026-10-27 — Fim da fala, não fim do turno: ordem cronológica no chat

- **IDs:** ISS-079, BR-043, T21 · **Refs:** [`issues.md`](issues.md),
  [`../business-rules.md`](../business-rules.md) BR-043,
  [`tasks/21-task.md`](tasks/21-task.md)

### Sintoma
Na sessão `5984de1d…`, a fala do usuário (“Está bom, deixa eu pensar…”)
aparecia antes do restante da pergunta do THEY (“…sugerir melhorias.”), embora a
pergunta tivesse terminado antes.

### Causa
`publish_turn` gravava `end_ms = start_ms + cut_ms`; `cut_ms` é o fim do
turno/buffer, não da fala. O `DecodedTurn.speech_end_ms` era calculado e
ignorado. O THEY t7 ia até 127,937 s no banco, mas a fala terminou ~122,5 s; o ME
começou em 122,765 s. O corte de sobreposição acreditava na sobreposição falsa.

### Decisão
- `publish_turn` recebe `speech_end_ms` e grava
  `end_ms = start + speech_end.clamp(0, cut)`; prévia, final, promoção do VAD e
  fallback usam o mesmo fim.
- O overlap/linha temporal/classificador passam a usar o fim real.
- Sessões antigas não são corrigidas (sem áudio).

### Verificação
- Pipeline: `EarlyEndAsr` (fala 500 ms, cut 2 s) → `end_ms=500`.
- Chat: caso real (THEY termina antes do ME) → uma bolha de THEY, ME depois,
  sem `#ov`.
- `cargo test --lib` **844 ✓** · `pnpm test` **521 ✓** · clippy/fmt/tsc ✓.

## 2026-10-28 — Streaming intercalado no bloco finalizado

- **IDs:** ISS-082, BR-043, T22 · **Refs:** [`issues.md`](issues.md),
  [`../business-rules.md`](../business-rules.md) BR-043,
  [`tasks/22-task.md`](tasks/22-task.md)

### Sintoma
Sessão `9e6d8300…`: o bloco do interlocutor já estava formado e a fala do ME em
andamento (**Streaming**) aparecia depois do bloco inteiro, mesmo tendo começado
no meio dele.

### Causa
`splitSpeechSpans` só usava falas **finalizadas** como fronteira. A prévia entrava
na linha temporal pelo `start_ms`, mas não dividia o bloco finalizado; como o
bloco é uma bolha única ancorada no início, a prévia caía depois dele.

### Decisão
- `splitSpeechSpans(spans, interruptions)`: as prévias entram como fronteira, mas
  **não** são divididas (continuam uma bolha por falante, BR-102).
- `buildChatTimeline` passa `pending` como `interruptions`.
- O corte conservador da ISS-075 continua (80 chars/6 s, interrupção ≥ 1,5 s,
  partes ≥ 20 chars/3 palavras).
- A posição da prévia é a mesma antes e depois de virar final.

### Verificação
- `chat.test.ts` **25 ✓** (caso genérico + caso real `9e6d8300…`, prévia curta,
  prévia posterior, paridade prévia/final).
- `pnpm test` **525 ✓** · `tsc` ✓.

## 2026-10-28 — Indicador de fala da aba não fica preso

- **IDs:** ISS-084, BR-033, T23 · **Refs:** [`issues.md`](issues.md),
  [`tasks/23-task.md`](tasks/23-task.md)

### Sintoma
O selo Streaming/gradiente da aba continuava aceso sem ninguém falando.

### Decisão
- `publish_activity` exige fala recente (`ACTIVITY_IDLE_TIMEOUT = 2 s`),
  decodificação em voo ou final pendente; `tick` reavalia a cada 50 ms.
- `useTranscriptActivity(sessionId, enabled)` zera ao pausar/parar.
- Testes do helper, do store e do hook.

### Verificação
- `cargo test --lib` **855 ✓** · `pnpm test` **526 ✓** · clippy/tsc ✓.
