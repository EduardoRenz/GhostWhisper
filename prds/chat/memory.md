# Histórico — PRD Chat

Histórico deste PRD no formato fixo do
[`rules.md`](../../.agents/rules.md) §6.2 (busca por palavra-chave, ID e período).

| Objetivo | Comando |
|----------|---------|
| Palavra-chave | `rg -n -i 'bolha' prds/chat/memory.md` |
| Entrada do dia | `rg -n '^## 2026-10-18' prds/chat/memory.md` |
| Por ID | `rg -n 'ISS-018\|BR-021\|BR-022\|BR-026' prds/chat/memory.md` |
| Teto da bolha | `rg -n -i 'teto da bolha\|continuação' prds/chat/memory.md` |
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

### Concorrência e janela (aviso do usuário)
O usuário alertou para não se perder quando **já existe uma sugestão em
andamento** e a janela está aberta/fechada. Decisões (BR-042, task 15):
- **Single-flight por sessão** no `SuggestionService`: automático e manual nunca
  emitem `quick`/`full` intercalados; a manual **espera** a atual. Sem
  cancelamento de LLM em voo.
- **Prioridade do manual:** com contexto manual ativo, a automática `full` vai
  para o histórico e a `quick` é ignorada; o cartão manual não é substituído.
  Ao limpar o contexto, a automática volta a poder ocupar o cartão.
- **Janela oculta continua montada:** os eventos acumulam; `suggestionLast`
  hidrata o cartão sem LLM. Abrir com `current`/`quick`/`generating`/contexto
  manual não chama `generate`; abrir vazia com auto **ligado** também não (o
  agendador é a fonte); com auto **desligado**, chama no máximo uma vez.
- **Novo `[✦]` durante a manual:** o contexto novo substitui o antigo; a manual
  antiga termina com `Suggestion.source` (falante/texto/`at_ms` original) e o
  botão libera para o contexto novo.
- **Troca de sessão:** limpa a leitura e ignora eventos/resultados antigos; o
  lock é por sessão.
- **`generation_id` (UUID por geração):** a UI usa o id como chave de
  `current`/`quick`/histórico; `at_ms` pode empatar (relógio fixo/teste) e não
  serve como identidade. O single-flight evita intercalação; o id identifica a
  geração; `trigger`/`source` rotulam.

### Entregável desta rodada
- **Nada implementado.** PRD §9, arquitetura `transcription-panel.md`, tasks
  10–15 (`PENDING`), BR-040/BR-041/BR-042 e revisões de BR-019/BR-021
  registradas. A task 14 é o fechamento e vem depois da 15 (`13 → 15 → 14`).
- Próximo passo: task 10 (casca) e task 11 (cartões), depois 12 (handoff), 13
  (Sugestões manual), 15 (concorrência) e 14 (fechamento).
