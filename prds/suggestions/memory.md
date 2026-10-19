# Memory — PRD Sugestões

Histórico cronológico deste PRD (entradas `## YYYY-MM-DD — Título`, com linha
`IDs:`; convenções em [`../../.agents/rules.md`](../../.agents/rules.md)).

---

## 2026-10-19 — PRD criada (planejamento do novo layout)

- **IDs:** BR-037, BR-038, BR-039 · **Refs:** [`prd.md`](prd.md),
  [`architecture/suggestions-window.md`](architecture/suggestions-window.md),
  [`architecture/agent-identity.md`](architecture/agent-identity.md),
  [`tasks/_tasks.md`](tasks/_tasks.md)

### Contexto
Pedido do usuário: redesenhar a **janela de Sugestões** conforme o print de
referência (protótipo Stich AI) e do protótipo real, com ajustes de usabilidade e
mudanças em outros lugares — em especial o **domínio de agente**, que ganha
**ícone** e **cor** distintas.

### Decisões (do pedido)
- **Header:** ícone da faísca (o mesmo do botão do notch), título **“Sugestões”**,
  badge **LIVE**; primeiro controle da direita = **auto-sugerir** com **progress
  circular regressivo** (rótulo = contagem regressiva rodando / “Pausado”
  pausado); ao zerar, **auto sugere**; **“Sugerir agora”** **sem glow**.
- **Seletor de agente:** botão de **largura total** (ícone + nome + ATIVO +
  chevron) com popover; **fluido**, sem largura fixa do print.
- **Resposta rápida:** bloco próprio (curto, gerado antes da completa) com
  título/cor/estilo próprios — a completa **não** a substitui.
- **Resposta completa:** Markdown livre, com chip de metadados e blocos de código
  com cabeçalho/rodapé.
- **Histórico:** seção colapsável “Histórico de Sugestões da Sessão” (contador) +
  accordions (ícone do agente + título + pill do agente); expandir **rola até** o
  bloco e **nunca** substitui o topo. No protótipo, o **editor ficava por cima**
  do histórico → não repetir (CA-04.6).
- **Domínio de agente:** ícone + cor (catálogo/paleta do design system).
- **Sem som**, sem library nova, ícones do projeto (BR-008).

### Decisões fechadas com o usuário (2026-10-19)
- **Composer do print:** **fora do escopo** (print da ferramenta de UX).
- **Título da sugestão:** é **dinâmico** e o mecanismo **já existe** no código
  (`subject` do resumo + `summaryTopic`) → **reusar** (derivativo), sem campo
  novo no modelo e sem mudar o contrato de prompt.
- **Cadência:** o **timer de auto-sugerir já existe** → **reusar**; a mudança é
  só o **feedback na tela** (o core expõe o timer restante).
- **Cor dos agentes personalizados:** escolhida na **paleta** (default slate).

### Pontos que ficam para a execução
- **X de fechar** no header: a janela hoje não tem (o notch controla); fica
  fora até alguém pedir.

### Verificação
- PRD, arquitetura (2 docs), tasks 01–07, issues e BRs escritas; **sem código**
  (execução começa após a revisão).

---

## 2026-10-19 — Implementação (tasks 01–07)

- **IDs:** BR-037, BR-038, BR-039 · **Refs:** [`prd.md`](prd.md),
  [`tasks/_tasks.md`](tasks/_tasks.md),
  [`../business-rules.md`](../business-rules.md)

### O que foi feito
- **Task 01 — identidade do agente (BR-037):** `AssistantMode` ganhou `icon` e
  `color` (Rust + TS), com `serde(default)`, validação em `validate_mode` e
  normalização (`with_valid_identity`) — settings antigas seguem válidas.
  Catálogo de 8 ícones (`components/AgentIcon.tsx`, faísca reaproveitada do
  notch — BR-016) e paleta de 5 tokens (`--oh-agent-*`). Embutidos fixos
  (users/cyan, braces/violet, code/emerald, layers/coral); personalizados
  escolhem nas Configurações.
- **Task 02 — duas fases (BR-038):** `SuggestionPhase` (`quick`/`full`) no
  contrato; o *fast-first* publica `quick` e o passe completo `full` — **sem
  mudar o contrato de prompt**. A snapshot do agente (`mode_icon`/`mode_color`)
  viaja com a sugestão. No front, `useSuggestions` separa `quick` de
  `current`/`history`; a rápida só convive com a completa da mesma geração.
- **Task 03 — cabeçalho e cadência (BR-039):** evento `suggestion_cadence`
  publicado pelo **agendador** (a decisão é pura:
  `application::suggestion_scheduler::cadence`) — expõe o **timer que já
  existe**, não muda a cadência. Front: `useSuggestionCadence` +
  `AutoSuggestControl` (anel SVG, “12s” / “Pausado” / “aguardando fala”).
  `PanelFrame` ganhou `heading` (cabeçalho próprio): faísca + “Sugestões” +
  badge LIVE/PAUSADO + “Sugerir agora” **sem glow**.
- **Task 04 — seletor de agente:** `AgentSelector` (botão full-width com
  ícone + nome + ATIVO + “Alternar agente” + chevron; popover com todos os
  agentes, ícone e cor; teclado). O `<select>` saiu.
- **Task 05 — blocos:** `SuggestionCard` (tag “Sugestão Atual · Há 12s”, título
  **derivativo** — reuso do padrão do resumo (`subject` → heading → 1ª linha,
  com corte), `AgentBadge`, bloco de resposta rápida e bloco de resposta
  completa com chip “Markdown · Código/Explicativo”). `MarkdownView` ganhou o
  **caption** do código (`linguagem — arquivo` + complexidade à direita); os
  prompts técnicos passaram a emitir `**File:**`.
- **Task 06 — histórico:** `SuggestionHistory` (seção colapsável “Histórico de
  Sugestões da Sessão” + contador + accordions com ícone do agente, título e
  pill). Expandir abre **no lugar** e rola até o bloco; o topo **não** é
  substituído (o `select` do hook saiu).
- **Task 07 — acabamento:** contexto extra recolhido atrás de um ícone no
  cabeçalho, estados vazio/erro no layout novo, janela de Sugestões com
  mínimo de **520×420**, validação visual por screenshot e pacote gerado.

### Decisões que mudaram no caminho
- O **título** saiu do modelo: é derivativo (o modelo não emite título), e a
  UI corta em 72 caracteres para ficar curto.
- O **caption** do código lê os metadados do texto **antes** de limpá-lo do
  corpo; quando não há código, as linhas `**Big O:**` continuam visíveis.
- O histórico é a **última faixa** do corpo (nada fica por cima dele — o
  protótipo cobria o bloco com o editor).

### Verificação
- `cargo test` 364 ✓ · `cargo clippy` ✓ · `cargo fmt --check` ✓
- `pnpm test` 233 ✓ · `npx tsc --noEmit` ✓ · `pnpm build` ✓

---

## 2026-10-19 — Ajustes pedidos pelo usuário (controle do automático + timer)

- **IDs:** BR-039 (revisto) · **Refs:** [`../business-rules.md`](../business-rules.md),
  [`architecture/suggestions-window.md`](architecture/suggestions-window.md)

### Problema 1 — o botão do auto-sugerir sem texto, contagem e progresso
- O controle virou **anel + rótulo “Auto-sugerir” + pill de contagem** (“12s”),
  como no protótipo; pausado, a pill vira “Pausado”; sem fala nova, “0s” com o
  anel vazio e o estado `waiting`.
- O cabeçalho passou a caber em **uma linha** na janela de 540 px (medição no
  navegador: título sem truncamento).

### Problema 2 — não fazer sugestão automática sem mensagens novas
- Descoberto: **`settings.suggestion_interval_secs` nunca era usado** — o
  agendador usava 3 s fixos (a configuração era morta) e, sem o `cursor` do
  agendador, a janela **forçava** uma sugestão ao abrir (duplicando a do
  automático).
- Agora: o agendador usa o **intervalo das settings** (10–120 s) + *debounce* de
  700 ms; o serviço guarda a **última sugestão da sessão** e a devolve quando
  não há fala nova (**nenhuma chamada ao LLM**); “Sugerir agora”, o “Sugerir” do
  chat e o atalho global **forçam** a geração.
- Efeito: a contagem regressiva é real (o timer que o usuário achava que
  existia) e a janela que abre no meio da conversa mostra a sugestão existente.

### Verificação
- `cargo test` 367 ✓ · `cargo clippy` ✓ · `cargo fmt --check` ✓
- `pnpm test` 236 ✓ · `npx tsc --noEmit` ✓ · `pnpm build` ✓

---

## 2026-10-19 — Correção: contagem/anel do auto-sugerir parados

- **IDs:** BR-039 (corrigida) · **Refs:** [`../business-rules.md`](../business-rules.md),
  [`architecture/suggestions-window.md`](architecture/suggestions-window.md) §3.4

### Sintoma
O controle mostrava **“0s” para sempre** e o anel ficava vazio, mesmo com a
janela aberta e sugestões chegando; não reagia a fala nova.

### Causa raiz (três)
1. A cadência devolvia `remaining_ms = 0` **assim que não havia fala nova**,
   ignorando o intervalo que ainda faltava — por isso o anel/contagem zeravam
   logo depois de cada geração.
2. O core só publicava a cadência com `window.is_visible()`; se o SO reportasse
   a janela como oculta, a UI ficava no estado inicial (0 s, anel vazio).
3. O frontend só assinava o evento quando `usePanelOpen` dizia “aberto” — um
   segundo ponto de falha para o mesmo sintoma.

### Correção
- `cadence()` agora conta o **intervalo** primeiro (`remaining = intervalo −
  desde_a_última_execução`, mesmo sem fala nova); só depois olha o *debounce* e
  o “aguardando fala”.
- O agendador publica a cadência **sempre** (o frontend só assina quando
  montado); o evento ocioso carrega o `enabled` real das settings.
- `useSuggestionCadence()` assina **incondicionalmente** (sem depender da
  visibilidade da janela).
- `last_run` zera na troca de sessão (a primeira sugestão da sessão nova não
  espera o intervalo da anterior).

### Prova
- Log de debug do agendador (`RUST_LOG=ghostwhisper_lib=debug`): a cadência é
  publicada a cada 1 s.
- Testes novos: `cadence_ticks_from_the_interval_down_to_zero`,
  `cadence_keeps_counting_the_interval_without_new_speech` e
  `useSuggestionCadence` (assinatura + atualização).

### Verificação
- `cargo test` 369 ✓ · clippy/fmt ✓ · `pnpm test` 239 ✓ · `tsc` ✓.

---

## 2026-10-19 — Ajustes: timer por silêncio, feedback e títulos

- **IDs:** BR-038, BR-039 · **Refs:** [`../business-rules.md`](../business-rules.md)

### Timer (bug relatado: contador não reiniciava)
- O contador agora é o **silêncio desde a última fala**: `remaining = intervalo
  − silêncio`; **cada fala nova reinicia**. Quando o silêncio completa o
  intervalo, a sugestão é gerada. Uma rajada de perguntas vira **uma**
  sugestão (antes era uma por balão — “3 balões, 4 sugestões”).
- O evento passou a ser publicado **sempre** e a assinatura no front é
  incondicional (dois pontos de falha eliminados). Prova por log: 1 evento/s.

### Feedback na nova sugestão
- Enquanto só a **resposta rápida** existe, o cartão mostra **“Gerando resposta
  completa…”** (ponto pulsante); some quando a completa chega.

### Títulos repetidos (“Easy”, “Easy”, …)
- `suggestionTitle` agora **pula** blocos de código, tabelas, metadados
  (`**Big O:**`, `**File:**`), linhas inteiras em negrito e **seções genéricas**
  (`Easy`, `Practical`, `Deep`, `Glossary`, `Initial implementation`, …). O
  primeiro conteúdo real vira o título — cada pergunta ganha um nome próprio.
- O cartão usa o título da **completa** quando ela existe; senão o da rápida.

### Título gerado pela IA (retorno estruturado leve)
- Os quatro contratos de sugestão passaram a pedir `Subject: <título curto>` na
  primeira linha (o mesmo padrão do resumo). O core extrai com
  `prompts::parse_suggestion_response` → `Suggestion.title`, **remove a linha**
  do Markdown (inclusive no streaming) e persiste só o corpo.
- O cartão e o histórico preferem o título do agente; sem `Subject:` (sugestões
  antigas), caem para `suggestionTitle`, que agora pula seções genéricas.
- Resultado no cenário de perguntas técnicas: “Como inverter uma árvore binária
  em Python”, “O que é um hash map”, “Diferença entre array e matriz”.

### Verificação
- `cargo test` 369 ✓ · clippy/fmt ✓ · `pnpm test` 247 ✓ · `tsc` ✓.

### Ajuste posterior
- **Trocar de agente não gera** sugestão: saiu o `generate(force)` do caminho
  de troca; a próxima sugestão segue a regra normal (fala nova + contagem) ou o
  botão “Sugerir agora”. A escolha é salva nas settings e vale para a próxima
  geração. Teste atualizado (`não gera ao trocar de agente`).

---

## 2026-10-19 — Nova geração limpa o topo (a anterior vai para o histórico)

- **IDs:** BR-038 (complemento) · **Refs:** [`../business-rules.md`](../business-rules.md),
  [`architecture/suggestions-window.md`](architecture/suggestions-window.md) §4

### Pedido
“Toda vez que uma nova sugestão está surgindo, jogue a que estava antes para o
histórico, no accordion. Assim veremos a nova sugestão sendo montada, e já
exibindo logo que possível a versão resumida — evita confusão de saber se é a
nova ou ainda a antiga.”

### Implementação
- `useSuggestions.applyIncoming`: a **rápida** de uma geração nova
  (`at_ms` diferente da atual) limpa `current` — a anterior **já está** no
  histórico (cada completa entra nele ao chegar) e o cartão passa a mostrar a
  nova: só o bloco de resposta rápida, depois a completa em streaming.
- `SuggestionCard`: o relógio do “Há Xs” reinicia por geração
  (`current?.at_ms` **ou** `quick?.at_ms`), então a tag acompanha a nova.
- Efeito: o histórico ganha a anterior no mesmo instante em que a nova começa
  (contador “N anteriores”), sem esperar a completa.

### Testes
- `useSuggestions`: quick de geração nova → `current` nulo, anterior no
  histórico, completa nova assume o topo.
- `SuggestionsWindow`: com só a rápida da nova, a anterior aparece no accordion.
- `pnpm test` 241 ✓ · `tsc` ✓.
- Screenshot do layout (header, seletor, cartão com os dois blocos + código,
  histórico com accordions, popover do agente) conferido no navegador headless.
- AppImage e `.deb` gerados (Vulkan, `libvulkan.so.1` embutido).
