# Arquitetura — Janela de Sugestões (novo layout)

Especificação técnica do redesenho da janela `suggestions`: blocos do layout de
referência, modelo de dados em duas fases, cadência visível do auto-sugerir,
histórico em accordion e identidade dos agentes. Contrato em **BR-037**…**BR-039**,
história em [`../prd.md`](../prd.md).

---

## 1. Estrutura da janela

```
┌──────────────────────────────────────────────────────────────┐
│ (✦) Sugestões  ●LIVE          ◔ 12s  [Pausar]  ⚡ Sugerir agora │  header
├──────────────────────────────────────────────────────────────┤
│ (⌨) LeetCode / Algoritmos   ATIVO                    ⌄         │  agente
├──────────────────────────────────────────────────────────────┤
│ ▢ Sugestão Atual · Há 12s                                     │
│ Propor abordagem com Sliding Window…            [pill agente] │  título
│ ┌──────────────────────────────────────────────────────────┐ │
│ │ ✎ RESPOSTA RÁPIDA                                       │ │  bloco rápido
│ │ "Para evitar a complexidade quadrática…"                 │ │
│ └──────────────────────────────────────────────────────────┘ │
│ ┌──────────────────────────────────────────────────────────┐ │
│ │ ▤ RESPOSTA COMPLETA            Markdown · Explicativo     │ │  bloco completo
│ │ Estratégia Sliding Window: …                             │ │
│ │ ┌ rust — solution.rs ────────────  O(n) · O(1) ┐         │ │  código
│ │ └───────────────────────────────────────────────────────┘ │ │
│ └──────────────────────────────────────────────────────────┘ │
├──────────────────────────────────────────────────────────────┤
│ ⌄ Histórico de Sugestões da Sessão      [2 anteriores]      │  seção
│   ▸ (🟪) Planejar um System Design…   [System Design]       │  accordions
│   ▸ (🟪) Otimizar o cache…             [System Design]       │
└──────────────────────────────────────────────────────────────┘
```

- O cabeçalho e o seletor de agente são **fixos**; só o corpo rola.
- O cartão da sugestão atual é o **topo** do corpo e **nunca** é substituído pelo
  histórico.
- O histórico é uma **seção colapsável** (“Histórico de Sugestões da Sessão” +
  contador) e cada sugestão dentro dela é um **accordion** com o **ícone do
  agente** (na cor dele), o **título** e a **pill com o nome do agente**.
- Nada é desenhado **por cima** do histórico (no protótipo, o editor cobria o
  bloco — ver §8).

## 2. Componentes

```
src/features/suggestions/SuggestionsWindow.tsx   # casca: header + seletor + corpo
src/features/suggestions/AutoSuggestControl.tsx  # anel (progress) + contagem + pausar
src/features/suggestions/AgentSelector.tsx        # botão full-width + popover da lista
src/features/suggestions/SuggestionCard.tsx       # tag, título, blocos, ações
src/features/suggestions/SuggestionHistory.tsx    # seção + accordions + scroll até o item

src/core/domain/assistant.ts     # + AgentIcon/AgentColor, identidade
src/core/domain/suggestion.ts     # NOVO: título derivativo e metadados do Markdown
src/core/application/useSuggestions.ts   # + fase (rápida/completa) e histórico
src/core/application/useSuggestionCadence.ts # NOVO: contagem regressiva (evento do core)

src/components/AgentIcon.tsx      # NOVO: ícones dos agentes (SVG do projeto)
src/components/AgentBadge.tsx     # NOVO: pill com ícone + cor do agente
src/components/MarkdownView.tsx   # + caption de código (linguagem/arquivo/complexidade)
```

> O cabeçalho da janela vive no `SuggestionsWindow` (faísca + título + badge) e
> usa o `heading` do `PanelFrame`; os blocos de resposta rápida/completa são
> seções do `SuggestionCard` (não componentes separados).

## 3. Modelo de dados

### 3.1 Sugestão em duas fases (`ports::suggestion_generator`)

```rust
pub enum SuggestionPhase { Quick, Full }   // "quick" | "full"

pub struct Suggestion {
    pub session_id: String,
    pub mode_id: String,
    pub mode_name: String,
    /// Snapshot da identidade do agente no momento da resposta.
    pub mode_icon: String,     // id de ícone
    pub mode_color: String,    // token de cor
    /// Fase: resposta curta (`Quick`) ou texto completo (`Full`).
    pub phase: SuggestionPhase,
    /// Texto da fase (rápida: 1–3 frases; completa: Markdown).
    pub text: String,
    pub at_ms: i64,
    pub context_until_ms: i64,
    pub message_id: Option<String>,
    pub skipped: bool,
}
```

- O evento `suggestion` continua sendo o canal; a **fase** diz ao frontend se o
  texto substitui o bloco rápido ou o bloco completo.
- **Sem migração de banco:** a persistência (`messages`) continua guardando o
  Markdown completo em `content`; a resposta rápida é **efêmera** (CA-03.6).
- O **título** não é campo: é **derivativo** na UI, com o mesmo padrão já usado
  pelo resumo (`subject` → primeiro heading → rótulo neutro). Ver §3.4.

### 3.2 Contrato do modelo (passe rápido)

O passe **fast-first** **não muda de contrato**: ele continua devolvendo um
**texto curto** (o rascunho que hoje já existe). O que muda é que ele passa a
ser publicado com `phase = Quick`, para a UI guardá-lo em bloco próprio em vez de
substituir a resposta completa.

- O passe completo continua devolvendo **Markdown livre** (BR-028), com a
  instrução de incluir, quando houver, o cabeçalho/rodapé do bloco de código.
- Sem JSON, sem campo novo de título, sem parsing tolerante novo: menos
  superfície de erro (a degradação atual continua valendo).

### 3.3 Título da sugestão (gerado pelo agente)

- O título vem do **retorno estruturado leve** do agente: a resposta começa com
  `Subject: <título curto, mesmo idioma, até 60 caracteres>` — o mesmo padrão
  que o resumo já usa. O core (`prompts::parse_suggestion_response`) extrai o
  título para `Suggestion.title` e **remove a linha** do Markdown (inclusive
  durante o streaming, para o corpo não piscar).
- Os quatro contratos de sugestão (Reunião, Entrevista Técnica, LeetCode,
  System Design) pedem a linha; exemplos no próprio prompt
  (“Como inverter uma árvore binária em Python”).
- **Fallback** para sugestões antigas/sem `Subject:`: `suggestionTitle(text)` em
  `core/domain/suggestion.ts` (irmão do `summaryTopic`), que **pula** blocos de
  código, tabelas, metadados (`**Big O:**`, `**File:**`), linhas em negrito e
  **seções genéricas** (`Easy`, `Practical`, `Deep`, `Glossary`,
  `Initial implementation`, …). Sem isso, perguntas diferentes viravam “Easy”.
- Cartão e histórico preferem `suggestion.title`; sem ele, o derivado.

### 3.4 Cadência do auto-sugerir (novo evento, timer das settings)

```rust
pub struct SuggestionCadenceEvent {
    pub enabled: bool,          // automático ligado
    pub running: bool,          // rodando (não pausado)
    pub remaining_ms: i64,      // até a próxima janela automática
    pub awaiting_speech: bool,  // janela vencida, mas sem fala nova
    pub window_ms: i64,         // duração do ciclo (anel de progresso)
    pub generating: bool,       // geração automática em andamento
}
```

- Publicado pelo **core** (o agendador é a fonte da verdade) ~1/s **sempre**
  (o frontend só assina quando montado); a UI **não** recalcula a cadência.
- O intervalo é o **tempo mínimo entre sugestões** (*cooldown*), contado desde
  a última geração ou desde a abertura da janela. **Fala nova não reinicia** a
  contagem: ela é o conteúdo que libera a geração. Ao zerar com fala nova,
  gera; sem fala nova, `remaining = 0` + `awaiting_speech` (a pill mostra
  “Aguardando”) e a geração sai assim que a próxima fala finalizar.
- A geração automática só roda com a **janela de Sugestões aberta**
  (`suggestions_window_open`): fechada, o agendador só mantém a cadência — sem
  chamada ao LLM (revisa o BR-028 a pedido do usuário).
- **Abrir a janela não gera**: o contador **reinicia na abertura** e o auto
  começa do intervalo; com o automático pausado, nada é gerado — só o botão
  “Sugerir agora”.
- O **timer** é o intervalo das settings (`suggestion_interval_secs`, 10–120 s).
  O agendador usa esse intervalo como frequência máxima — antes ele usava uma
  constante de 3 s e a configuração era morta.
- **Feedback imediato:** antes de chamar o LLM, o agendador publica
  `generating = true`; a UI mostra “Gerando”/pill e o anel vira um arco
  indeterminado. A chamada é síncrona na thread do agendador, então esse evento
  é o único sinal até a primeira resposta chegar.
- A UI converte `remaining_ms` em **progress circular** (SVG `stroke-dashoffset`)
  e na pill de contagem (“12s” / “Pausado” / “Aguardando” / “Gerando” / “Sem
  escuta”). O rótulo do controle é sempre **“Auto-sugerir”**.

### 3.4.1 Sem fala nova, não há nova sugestão
- O serviço guarda a **última sugestão da sessão** (cursor + texto).
- `SuggestionRequest.force` decide: sem `force`, quando não há fala nova desde
  a última sugestão, o serviço **devolve a última** — nenhuma chamada ao LLM.
- `force = true` nas ações explícitas do usuário: **“Sugerir agora”**, o botão
  “Sugerir” do chat e o atalho global. **Trocar de agente não gera** — só
  atualiza a escolha; a próxima sugestão segue a regra normal (fala nova +
  contagem) ou o botão.
- **Sem duplicata:** o cursor efetivo é `max(cursor do pedido, cursor da última
  sugestão)` — o agendador pode mandar um cursor velho depois de a janela gerar
  — e um **mutex** serializa as gerações (janela × agendador). Sem isso, a
  mesma fala virava duas sugestões no histórico.
- Efeito colateral positivo: a janela que abre no meio da conversa mostra a
  sugestão existente (sem gerar outra) e o agendador deixa de repetir
  sugestão para a mesma fala.

## 4. Fluxo de dados (frontend)

```
useSuggestionCadence()  ── evento "suggestion_cadence" ──▶  SuggestionHeader
useSuggestions()        ── evento "suggestion" ─────────▶  SuggestionCard
                                                            ├─ QuickAnswerBlock
                                                            └─ FullAnswerBlock
useSuggestions()        ── histórico em memória ─────────▶  SuggestionHistory
                                                            └─ scrollIntoView do item
```

- `useSuggestions` separa por fase: `quick` (texto curto + título) e `full`
  (Markdown). O texto completo **não** apaga o rápido; quando chega, o card
  mostra os dois blocos.
- A **rápida de uma geração nova** (`at_ms` diferente) é o começo da nova
  sugestão: o topo é limpo (a anterior já está no histórico) e o card monta a
  nova — primeiro só a rápida, depois a completa em streaming. Assim fica claro
  que o que está na tela é a sugestão nova, não a antiga.
- Enquanto só a rápida existe, o card mostra **“Gerando resposta completa…”**
  (um ponto pulsante), que some quando a completa chega.
- O histórico guarda `{title, mode_id, mode_icon, mode_color, text, at_ms}`; o
  accordion abre **no lugar** e o scroll vai até o item (sem trocar o topo).

## 5. Interações

### 5.1 Seletor de agente
- Botão **full-width**: `AgentIcon` + nome + pill **ATIVO** + chevron.
- Popover: lista vertical com ícone + nome + cor; item ativo marcado; fecha ao
  clicar fora / `Escape`; navega por teclado (setas, `Enter`).
- Largura: **fluida** (acompanha a janela; `min-width` só como piso).

### 5.2 Histórico
- **Seção** colapsável: ícone + “Histórico de Sugestões da Sessão” + contador
  (“2 anteriores”) + chevron.
- Cada item é um accordion (botão + região) com **ícone do agente** (cor do
  agente), **título** e **pill com o nome do agente**; abrir o item **rola até**
  ele (`scrollIntoView({ block: "nearest" })`).
- Expandir **não** altera `current` (CA-04.2).
- Ações por item: copiar e fixar (fixar persiste a mensagem da sessão).
- Nenhum componente fica **por cima** do bloco (CA-04.6).

### 5.3 Resposta completa / código
- `MarkdownView` ganha **caption** de bloco de código: cabeçalho
  (`linguagem — arquivo`) e complexidade à direita (`O(n) time · O(1) space`).
- A **convenção de metadados** reusa as linhas que os agentes já pedem:
  `**Big O:** <tempo> time | <espaço> space` (já no contrato técnico) e a nova
  `**File:** <arquivo>` (task 05). Sem essas linhas, o caption mostra só a
  linguagem.
- Quando há bloco de código, as linhas de metadados **saem do corpo** (o
  caption é o lugar delas); sem código, elas continuam visíveis.

### 5.4 Contexto manual vindo da Transcrição (BR-041)

- A janela assina o `panel_opened` (label `suggestions`) e busca
  `suggestion_manual_context(session_id)` no núcleo; o contexto também é buscado
  ao montar, então não se perde se a janela estava fechada.
- Com contexto ativo, mostra o **`ManualContextCard`** (falante, hora, trecho,
  botão **“Gerar sugestão manual”** e ✕) e **não** gera automaticamente ao abrir.
- `useSuggestions` recebe o contexto **por parâmetro** e o envia em
  `SuggestionRequest.manual_context`; a resposta é marcada com
  `trigger = "manual"`, `source` snapshot da mensagem e exibida como
  **“Manual”** no cartão/histórico.
- O automático do core **não** usa o contexto manual; “Sugerir agora” continua a
  sugestão normal.
- **Concorrência (task 15):** uma geração por vez no serviço (`gate`); a manual
  espera a atual (botão “Aguardando…”); com contexto manual ativo, a automática
  vai para o histórico e não substitui o cartão manual; `suggestionLast` hidrata
  o cartão sem LLM. **Abrir a janela nunca gera** (BR-028 revisada): o automático
  só roda com a janela aberta e após o intervalo.
- Contrato completo (store, IPC, prompt, concorrência e fluxo):
  [`../../chat/architecture/transcription-panel.md`](../../chat/architecture/transcription-panel.md)
  §4–§5.

## 6. Tokens e estilos

Reusa os tokens do design system (BR-035):

| Elemento | Token |
|---|---|
| Bloco de resposta rápida | fundo `surface-container-high`, borda esquerda **violeta** (`--oh-ai`), texto **15px** (legível, sem itálico) |
| Bloco de resposta completa | fundo `surface-container`, borda hairline, título em `label-sm` mono |
| Chip de metadados | `surface-container-highest` + `status-badge` mono |
| Circle de cadência | trilho `rgba(255,255,255,.10)`, arco **cyan** (`--oh-accent`) |
| Pill do agente | cor do agente em 14% + texto na cor + borda 40% |

## 7. Evidência no Perssua (BR-015)

Extração: `examples/Perssua-0.27.5-x86_64.AppImage` →
`resources/app.asar` (sha256 `79ee1c37f8b9833415bfbc1847e7cd0b1ef1b96460f8771fb832fa1526d7aa94`),
analisada com o REA (`analyze_javascript_application` sobre `electron/flavors.js`
e `electron/main.js`; Evidence pai `ev_e16eab08…`, visão `ev_fdddd16…`).

| Assunto | Arquivo no bundle | Âncora | O que transpusemos |
|---|---|---|---|
| Agentes/flavors | `electron/flavors.js:15-21` | `FLAVOR_IDS` (`full`, `cody`, `notetaker`, `reuniao`) | os 4 agentes embutidos e suas categorias (BR-037) |
| Contrato do agente técnico | `electron/flavors.js:42` | `CODY_FLAVOR_ASSISTANT` | contrato do LeetCode/System Design (já em `prompts/assistant.rs`) |
| Contrato do agente de reunião | `electron/flavors.js:235` | `NOTETAKER_FLAVOR_ASSISTANT` | “live meeting copilot”, 1ª pessoa (BR-028) |
| **Resposta curta primeiro** (a fase `quick`) | `electron/flavors.js:148` | `CODY_FAST_FIRST_INSIGHT_PROMPT` | “Output only the first useful Notch insight. Be correct over complete.” → o passe *fast-first* (BR-038) |
| Resposta curta por agente | `electron/flavors.js:180`, `:201`, `:213` | `CODY_FAST_FIRST_LEETCODE_PROMPT`, `…_SYSTEM_DESIGN_PROMPT`, `…_REALTIME_PROMPT` | a resposta curta respeita o agente (LeetCode já é código) |
| **Formato do caption** do código | `electron/flavors.js:165-175` | “Exact shape: `# Big O: <time> time | <space> space`”, `# Language Detected: <Language>`, fence com a linguagem real | a linha `**Big O:**` e a `**File:**` que alimentam o caption (BR-038) |
| Atraso do *fast-first* | `electron/flavors.js:541`, `:673` | `fastFirstInsightDelayMs: 1` | o rascunho sai antes do texto completo |

**O que o bundle 0.27.5 _não_ tem:** as strings “Sugestões Inteligentes”,
“LIVE”, “Histórico de Sugestões da Sessão”, “RESPOSTA RÁPIDA” e “Alternar
agente” **não existem** nos 392 arquivos do app extraídos — o bundle sustenta o
**comportamento** (fases, contratos, metadados), não o layout. O **layout** vem
do **protótipo da ferramenta de UX** (o print de referência), e é por isso que
§8 lista os desvios conscientes (glow, composer, ícones/cores próprios).

> Reproduzir: `python3 /tmp/opencode/asar_cat.py <app.asar> electron/flavors.js`
> ou a ferramenta REA (`analyze_javascript_application`) para a análise do
> grafo de módulos.

## 8. Desvios conscientes

- O **composer** do print (input de linguagem natural) fica de fora (§7 do PRD) —
  e, no protótipo, ele ainda **cobria** o histórico; aqui não há sobreposição.
- O **glow** do “Sugerir agora” do print não é adotado (o pedido é explícito).
- Ícones e cores são **do projeto** (BR-008), não do Material do protótipo.
- A resposta rápida **não** é persistida (só a completa entra no export).
- O **layout** (header, accordion do histórico, blocos) vem do **protótipo da
  ferramenta de UX**; o bundle do Perssua 0.27.5 sustenta o **comportamento**
  (fases, contratos, metadados) — ver §7.
