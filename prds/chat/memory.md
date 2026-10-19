# Histórico — PRD Chat

Histórico deste PRD no formato fixo do
[`rules.md`](../../.agents/rules.md) §6.2 (busca por palavra-chave, ID e período).

| Objetivo | Comando |
|----------|---------|
| Palavra-chave | `rg -n -i 'bolha' prds/chat/memory.md` |
| Entrada do dia | `rg -n '^## 2026-10-18' prds/chat/memory.md` |
| Por ID | `rg -n 'ISS-018\|BR-021' prds/chat/memory.md` |
| Teto da bolha | `rg -n -i 'teto da bolha\|continuação' prds/chat/memory.md` |

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
