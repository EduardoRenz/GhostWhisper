# Business Rules — GhostWhisper

Registro de decisões e regras do produto. Cada regra tem um ID, a decisão, o
motivo e a data. Regras novas são adicionadas no fim; regras alteradas recebem
uma nota de revisão.

---

## BR-001 — Plataforma-alvo inicial: Linux
- **Decisão:** o MVP suporta Linux. macOS/Windows ficam fora do escopo do MVP.
- **Motivo:** o produto original é deficiente no Linux; é a dor que justifica o projeto.
- **Data:** 2026-10-17
- **Status:** Ativa

## BR-002 — Sem backend próprio (local-only)
- **Decisão:** o GhostWhisper não possui servidor. Não há login, conta, licença ou
  sincronização.
- **Motivo:** simplicidade, privacidade e alinhamento com open-source.
- **Consequência:** recursos de LLM dependem de chave do usuário (BYO key) ou de
  modelo local.
- **Data:** 2026-10-17
- **Status:** Ativa

## BR-003 — BYO key e segredos no keyring
- **Decisão:** chaves de API são fornecidas pelo usuário e armazenadas no keyring
  do sistema (Secret Service), nunca em arquivo de configuração em texto puro.
- **Motivo:** segurança; evitar vazamento de credenciais.
- **Data:** 2026-10-17
- **Status:** Ativa

## BR-004 — Sem telemetria por padrão
- **Decisão:** nenhuma telemetria, analytics ou crash report é enviado. Qualquer
  coleta futura deve ser opt-in explícito e documentada.
- **Data:** 2026-10-17
- **Status:** Ativa

## BR-005 — Áudio nunca sai do dispositivo sem ação do usuário
- **Decisão:** o áudio bruto é processado localmente. Só é enviado a um provider
  de nuvem o que o usuário configurar (ex.: transcrição cloud ou prompts de LLM).
- **Data:** 2026-10-17
- **Status:** Ativa

## BR-006 — Invisibilidade ao compartilhamento é "best-effort" no Linux
- **Decisão:** não prometemos paridade com `WDA_EXCLUDEFROMCAPTURE`/macOS. A
  mitigação primária é o overlay rodar em um **output virtual dedicado**; a
  documentação deve ser explícita sobre o limite.
- **Motivo:** não existe API de exclusão de janela na captura no Linux.
- **Status (2026-10-18, task 19):** revisada e **mantida**, agora explícita também
  na UI (*tooltip* do stealth e aba Geral das configurações). O "modo stealth" do
  GhostWhisper é **igual ao do Perssua no Linux** (`skip-taskbar` + título vazio);
  o `setContentProtection` do Perssua é no-op aqui e, por paridade, passamos a
  chamá-lo também (efeito real só em macOS/Windows). Único caminho real no Linux:
  **KDE Plasma 6.6+** ("Hide from Screencast" / `PlasmaWindowInterface`, por
  compositor) — opt-in futuro. Alternativas práticas hoje: compartilhar **uma
  janela** na reunião ou deixar o overlay em outro monitor.
- **Detalhes:** [`mvp/tasks/19-task.md`](mvp/tasks/19-task.md),
  [`mvp/architecture/overlay.md`](mvp/architecture/overlay.md) §4,
  [`mvp/architecture/wayland-spike.md`](mvp/architecture/wayland-spike.md).
- **Data:** 2026-10-17 · **Status:** Ativa (revisada na task 19)

## BR-007 — X11/XWayland primeiro, Wayland nativo depois
- **Decisão:** o overlay é entregue primeiro em X11/XWayland. Suporte Wayland nativo
  (via `gtk-layer-shell`) entra como spike/tarefa posterior.
- **Motivo:** janelas always-on-top/posicionamento preciso não existem no
  `xdg-shell`; reduzir risco do MVP.
- **Data:** 2026-10-17
- **Status:** Ativa

## BR-008 — Reconstrução de comportamento, não de código proprietário
- **Decisão:** o GhostWhisper se inspira em comportamento observado do Perssua
  (formato de dados, fluxos), mas não copia código-fonte, marca ou assets
  proprietários. O driver BlackHole (GPL-3) não é empacotado.
- **Motivo:** conformidade legal/licenciamento.
- **Data:** 2026-10-17
- **Status:** Ativa

## BR-009 — Modelos locais por padrão quando viável
- **Decisão:** ASR local (whisper.cpp) é o caminho padrão do MVP; cloud é opção.
- **Motivo:** privacidade e independência de provider.
- **Data:** 2026-10-17
- **Status:** Ativa

## BR-010 — Idioma e i18n
- **Decisão:** UI e documentação primárias em Português; tradução de conteúdo é
  função do produto (não confundir com idioma da interface).
- **Data:** 2026-10-17
- **Status:** Ativa

## BR-011 — Arquitetura hexagonal com inversão de dependência
- **Decisão:** o core segue `domain → application → ports → adapters`, com portas
  (traits) entre aplicação e adaptadores. `domain`/`application` nunca importam
  `adapters`. Composition root exclusivo em `src-tauri/src/lib.rs`.
- **Motivo:** testabilidade, independência de tecnologia e substituição de
  adaptadores (providers, áudio, DB) sem tocar no núcleo.
- **Detalhes:** [`.agents/rules.md`](../.agents/rules.md),
  [`engineering-guidelines.md`](mvp/architecture/engineering-guidelines.md).
- **Data:** 2026-10-17
- **Status:** Ativa

## BR-012 — Princípios SOLID
- **Decisão:** SOLID obrigatório; portas pequenas e específicas (I), extensão por
  novos adaptadores (O), contratos honrados por implementação (L).
- **Data:** 2026-10-17
- **Status:** Ativa

## BR-013 — Testabilidade como requisito
- **Decisão:** toda porta tem dobra de teste; `domain`/`application` sem I/O;
  cobertura alta no núcleo (> 80%). Ei de DI para permitir fakes.
- **Data:** 2026-10-17
- **Status:** Ativa

## BR-014 — Nomenclatura sem prefixo `I` (abstração antes da implementação)
- **Decisão:** abstrações sem prefixo (`TranscriptionsRepository`,
  `ChatBotProvider`); implementações prefixadas por contexto
  (`LocalTranscriptionsRepository`, `OpenAiChatBotProvider`,
  `WhisperSpeechToText`); dobras de teste `InMemory*`/`Fake*`.
- **Data:** 2026-10-17
- **Status:** Ativa

## BR-015 — Perssua como fonte de decisão de produto
- **Decisão:** ao decidir *como* resolver um problema, priorizar a solução já
  adotada pelo **Perssua** (produto maduro e validado). A transposição é apenas de
  tecnologia (Electron/JS/whisper.cpp → Tauri/Rust), preservando comportamento,
  nomes e valores observáveis (limiares, ordem das etapas). Desvios só com motivo
  técnico explícito e documentado (ex.: captura de áudio no Linux).
- **Motivo:** reduzir risco de reinventar heurísticas piores que as de um produto
  em produção; a referência é auditável via REA no `Perssua-*.AppImage`.
- **Detalhes:** [`.agents/rules.md`](../.agents/rules.md) §6.1 (R11–R13).
- **Data:** 2026-10-17
- **Status:** Ativa

## BR-016 — Documentação como fonte única (referência, não duplicação)
- **Decisão:** nenhum conteúdo já documentado é duplicado; sempre que possível,
  cite o arquivo (link relativo ao doc em que se está) ou o ID canônico
  (`BR-NNN`, `ISS-NNN`, `NN-task`, doc de arquitetura/tech spec). Informação
  nova é escrita **uma única vez** no doc dono e referenciada a partir dos
  demais — inclusive entre PRDs.
- **Motivo:** evitar divergência entre cópias; a documentação vira um grafo
  navegável (`prd.md` ↔ `business-rules.md` ↔ `issues.md` ↔ `tasks/`).
- **Detalhes:** [`.agents/rules.md`](../.agents/rules.md) §6 (R14).
- **Data:** 2026-10-17
- **Status:** Ativa

## BR-017 — Histórico (`memory.md`) por PRD e buscável
- **Decisão:** o `memory.md` vive **dentro do PRD** (`prds/<prd>/memory.md`;
  hoje [`mvp/memory.md`](mvp/memory.md)); não há memory global. Formato:
  entradas `## YYYY-MM-DD — Título` em ordem cronológica (append), linha de
  `IDs:`/`Refs:` após o título, separadas por `---` — busca por palavra-chave,
  ID e período via `rg`/`awk` (§6.2 de [`.agents/rules.md`](../.agents/rules.md)).
- **Motivo:** delimitar e recuperar decisões por data/ID conforme o histórico
  cresce; manter o contexto perto do PRD a que pertence.
- **Data:** 2026-10-17
- **Status:** Ativa

## BR-018 — Áudio do usuário: transpor os defaults do Perssua
- **Decisão:** o front-end de captura do microfone segue o Perssua: **ganho de mic
  8×** (faixa 1×–12×, configurável na aba *Transcrição & Áudio*) e **noise gate**
  com *hangover* de 600 ms que **silencia** (não descarta) o que está abaixo do
  limiar. O limiar do gate usa o limiar de fala do app, não os 0,025 do Perssua
  (aquele valor silenciaria fala hoje transcrita). Sem AEC; AGC descartado.
- **Motivo:** são o comportamento de produto observado no Perssua para o mesmo
  `ggml-silero` de VAD e a mesma cadeia de streaming; sem o ganho, microfones
  quietos nunca cruzam o gate de fala (limiar absoluto) e o app parece surdo.
- **Consequência:** o áudio do sistema (loopback) **não** é amplificado nem
  filtrado. Implementado na task 14; o monitor atrasado do microfone fica na 15.
- **Detalhes:** [`mvp/architecture/transcription-streaming.md`](mvp/architecture/transcription-streaming.md) §6;
  [`mvp/tasks/14-task.md`](mvp/tasks/14-task.md) e [`mvp/tasks/15-task.md`](mvp/tasks/15-task.md).
- **Data:** 2026-10-18 · **Status:** Ativa (implementada na task 14)

## BR-019 — Sessão é cumulativa (pausar/retomar não reinicia a linha do tempo)
- **Decisão:** a sessão é a unidade de continuidade. **Pausar** apenas suspende a
  captura e mantém a sessão aberta (mesmo id); **retomar** volta a capturar nela;
  **continuar** reativa uma sessão encerrada. Só "Nova sessão" (ou "Parar", que a
  encerra) começa outro id. As mensagens de uma sessão **nunca são descartadas**
  pela UI — elas são gravadas e recarregadas do banco. Todo segmento tem o tempo
  (`mm:ss`) desde o **início da sessão**, não desde o início da captura.
- **Motivo:** o comportamento do Perssua (`transcriptionPreview.pause/resume` +
  `DatabaseManager.resumeSession`) e a reclamação do usuário: parar/iniciar
  apagava as mensagens da tela. A pausa também preserva o custo do modelo (não
  recarrega o contexto do ASR a cada interrupção).
- **Consequência:** `sessions` permanece `active` durante a pausa (o `CHECK` do
  schema não muda e não há migração); sessões órfãs (app fechado gravando) são
  encerradas no boot; o id do segmento carrega a origem de tempo para não
  colidir ao continuar. O histórico e o export ficam nas tasks 17/18.
  **Pausar ≠ Parar**: parar **encerra** a sessão (para voltar, só pelo Histórico →
  Continuar); pausar mantém a sessão aberta e retoma em um clique. **Trocar os
  canais captados é sempre permitido** — inclusive gravando ou pausado — sem
  encerrar a sessão (task 20).
- **Detalhes:** [`mvp/tasks/16-task.md`](mvp/tasks/16-task.md);
  [`mvp/architecture/session-history.md`](mvp/architecture/session-history.md).
- **Revisão (2026-10-19, BR-040):** o **dado** persistido continua em
  milissegundos relativos ao início da sessão; a UI do painel ao vivo mostra a
  **hora de parede** (`HH:MM:SS`) derivada de `started_at_ms + at_ms`, com
  fallback para `mm:ss` quando o início não é conhecido. Não há migração.
- **Data:** 2026-10-18 · **Status:** Ativa (task 16)

## BR-020 — Sugestões da IA entram na linha temporal; export é Markdown
- **Decisão:** toda sugestão pedida (painel ou atalho) é **persistida** como
  mensagem da sessão (tabela `messages`, papel `assistant`) com o **tempo desde o
  início da sessão** — a mesma escala das falas — e aparece na linha temporal ao
  vivo, no histórico e no export. O export de uma sessão é um **Markdown** da linha
  temporal: cabeçalho (título, início/fim, duração) + `- **[mm:ss] Você:** fala` +
  a sugestão como bloco destacado `> 🤖 **Sugestão [mm:ss]** — texto`. O histórico
  vive numa **janela própria** (`history`) e a conversa ao vivo na janela `chat`
  (tasks 07/08 do PRD chat).
- **Motivo:** pedido do usuário — sem tempo não há linha temporal, e sugestão não
  persistida desaparece ao fechar o painel. O Perssua guarda as mensagens na mesma
  tabela, mas **não exporta arquivo** (só "copiar resumo/chat/transcrição"); o
  Markdown é um diferencial nosso, com a sugestão marcada e datada.
- **Consequência:** a UI nunca monta o Markdown (o core renderiza a partir do banco,
  uma única implementação em `application/session_timeline.rs` + `markdown_export.rs`);
  `messages.timestamp` passa a ser relativo à sessão (o Perssua usa relógio de
  parede) para as duas séries compartilharem um eixo.
- **Detalhes:** [`mvp/tasks/17-task.md`](mvp/tasks/17-task.md),
  [`mvp/tasks/18-task.md`](mvp/tasks/18-task.md),
  [`mvp/architecture/session-history.md`](mvp/architecture/session-history.md).
- **Data:** 2026-10-18 · **Status:** Ativa (tasks 17/18)

## BR-021 — A conversa é uma linha temporal de bolhas intercaladas
- **Decisão:** falas e sugestões da IA (BR-020) aparecem numa **única** conversa em
  bolhas, ordenada pelo tempo desde o início da sessão. "Você" fica à direita (tom
  azul), "Interlocutor" à esquerda (tom neutro) e "Sugestão da IA" à esquerda com
  tonalidade própria; cada bolha mostra `mm:ss` e a sugestão tem "Copiar". A fala
  **ainda em revisão** (prévia do ASR) é uma bolha "em andamento" exibida **depois**
  da linha temporal, com indicador de digitação — nunca no meio dela. O detalhe do
  histórico usa as mesmas bolhas. Valores visuais exatos (paridade com o Perssua) em
  [`chat/architecture/chat-ui.md`](chat/architecture/chat-ui.md) §3.
- **Motivo:** pedido do usuário — a referência de produto usa bolhas; e a linha de
  texto plana permitia que uma fala em andamento "recebesse" o texto de outro canal
  na posição antiga (ISS-018).
- **Consequência:** o id do segmento carrega o **canal**
  (`{session_id}-{channel}-{origin_ms}-t{turn}`); a separação entre linha temporal e
  falas em andamento é uma função pura (`core/domain/chat.ts`). Sem mudança de
  schema.
- **Detalhes:** [`chat/issues.md`](chat/issues.md) (ISS-018),
  [`chat/tasks/01-task.md`](chat/tasks/01-task.md),
  [`chat/tasks/02-task.md`](chat/tasks/02-task.md).
- **Revisão (2026-10-18, PRD notch/T03):** a conversa deixou de ser a janela `chat`
  isolada e passou a ser a **aba Transcrição** do painel em abas (BR-033); o
  comportamento das bolhas (esta regra) não muda.
- **Revisão (2026-10-19):** no painel **ao vivo**, a aba Transcrição é **só falas** —
  o "Sugerir" e as bolhas de sugestão ao vivo saíram (a feature será revista na aba
  **Sugestões**). Sugestões **persistidas** ("Fixar") continuam no histórico e no
  export, que usam esta mesma linha temporal.
- **Revisão (2026-10-19, BR-040):** o **formato** das bolhas do ao vivo passa a
  ser o do print de referência: **largura total** (sem `max-width:75%` e sem
  alinhamento à direita/esquerda), rótulos **Microfone**/**Outros** com cores
  distintas, hora ao lado do nome e ação de IA por fala (BR-041). A **linha
  temporal** (ordenação, prévia fora da linha, final vence a prévia) e o teto de
  180 caracteres (BR-031) **não mudam**; o histórico usa o mesmo cartão.
- **Data:** 2026-10-18 · **Status:** Ativa (tasks 01/02)

## BR-022 — Transporte da sessão: toggle único e encerramento confirmado
- **Decisão:** o transporte é **um único controle** que alterna start/pausa/retomar,
  como o `[play_arrow ↔ pause]` do Perssua: parado mostra "Iniciar escuta" (play +
  rótulo); gravando mostra ⏸; pausado mostra ▶. O **encerrar** existe **só** na
  pílula de sessão vermelha, com o tempo desde o início e confirmação inline
  ("Encerrar sessão? Sim / Não"). Uma bolinha de status acompanha o estado (vermelha
  pulsando gravando, âmbar pausado, cinza parado). Os ícones são SVG desenhados no
  projeto (não a fonte Material Icons do Perssua, BR-008).
- **Motivo:** pedido do usuário — um só stop (havia dois) e um controle de
  start/pause "como um switch", como o Perssua (evidência:
  `unified-session-controls` = `[play_arrow ↔ pause]` + `[stop]`, e o "End session?"
  na `.actionbar-session-pill`). **Desvio consciente** restante (R12): "Iniciar
  escuta" acumula "Start Listening" e "Restart" (não temos a janela pós-sessão do
  Perssua).
- **Histórico:** a versão original (tasks 03/04) tinha play e pause como **dois**
  botões e o ⏹ do transporte **também** confirmando; a task 09 alinhou ao Perssua
  mantendo a garantia de que o encerramento sempre pede confirmação.
- **Consequência:** `SessionStateEvent` carrega `started_at_ms` para a UI calcular o
  tempo decorrido; pausar/retomar **não** zeram o relógio (BR-019).
- **Detalhes:** [`chat/tasks/03-task.md`](chat/tasks/03-task.md),
  [`chat/tasks/04-task.md`](chat/tasks/04-task.md),
  [`chat/tasks/09-task.md`](chat/tasks/09-task.md).
- **Data:** 2026-10-18 · **Status:** Ativa (tasks 03/04/09)

## BR-023 — Resumo vivo, incremental e em janela separada
- **Decisão:** o resumo da sessão vive numa **janela própria e móvel**
  (`summary`) e é atualizado de tempos em tempos (30 s por padrão, configurável)
  enquanto a janela está aberta. A atualização é **incremental**: parte do resumo
  anterior e incorpora apenas as falas novas, com um cursor
  (`summarized_until_ms`); sessões longas são processadas em chunks, sem perder o
  começo. "Atualizar agora" regenera do zero. O resumo é um **Markdown** com o
  contrato adaptado do Perssua (`notetakerSummaryPrompt.js`) e é persistido em
  `meeting_summaries`, espelhado em `sessions.overview` (entra no export e no
  detalhe do histórico).
- **Motivo:** pedido do usuário — acompanhar uma reunião longa com um resumo que
  se atualiza sozinho, numa janela posicionável ao lado da reunião.
- **Consequência:** tick sem fala nova **não** chama o LLM; `subject` só preenche
  `sessions.subject` quando ele está vazio (o título do usuário vence).
- **Detalhes:** [`assistant/tasks/02-task.md`](assistant/tasks/02-task.md),
  [`assistant/tasks/03-task.md`](assistant/tasks/03-task.md).
- **Revisão (2026-10-18, PRD notch/T03):** o resumo **deixa de ter janela própria**
  (a janela `summary` é aposentada) e passa a ser a **aba Resumo** do painel (BR-033);
  o **automático** só chama o LLM com a **aba Resumo ativa** (revisa a parte desta
  regra que exigia só a janela aberta). Conteúdo e persistência (`meeting_summaries`
  / `sessions.overview`) **não mudam**.
- **Data:** 2026-10-18 · **Status:** Ativa

## BR-024 — Sugestões por agentes, com modos personalizados
- **Decisão:** a sugestão ao vivo tem **agentes** selecionáveis — embutidos
  (Reunião, Entrevista Técnica, LeetCode, System Design) e **personalizados**
  (nome, prompt de sistema, prompt de sugestão e contexto fixo). O contexto são os
  **últimos momentos** da conversa (≈3 min, até 40 falas, teto de 6 k caracteres),
  com a última sugestão para evitar repetição. O agente **sempre tenta** produzir
  algo útil; `...` só sem transcrição/ininteligível. Sugestões automáticas são
  **efêmeras**; "Fixar" grava em `messages` (linha temporal/export).
- **Motivo:** pedido do usuário ("modos sugestores personalizados, para dar mais
  contexto") e a referência do Perssua (Cody/Note Taker).
- **Consequência:** sem captura de tela, o modo LeetCode trabalha com o enunciado
  falado e com a caixa "Contexto extra"; personalizar prompts é diferente de
  copiar o Perssua (prompts **adaptados**, BR-008).
- **Detalhes:** [`assistant/tasks/02-task.md`](assistant/tasks/02-task.md),
  [`assistant/tasks/04-task.md`](assistant/tasks/04-task.md),
  [`assistant/tasks/05-task.md`](assistant/tasks/05-task.md).
- **Data:** 2026-10-18 · **Status:** Ativa

## BR-025 — Markdown renderizado com código colorido
- **Decisão:** resumo e sugestões são Markdown e são renderizados pelo mesmo
  componente (`MarkdownView`): títulos, listas, tabelas GFM, citações, código
  inline e blocos cercados com realce (highlight.js) e botão **Copiar**. **HTML
  bruto não é renderizado** e links abrem no navegador externo. As bolhas de
  sugestão da IA no chat também renderizam Markdown.
- **Motivo:** pedido do usuário — "código tem que ser formatado e colorizado".
- **Consequência:** os prompts não usam HTML inline (Big O em **negrito** em vez
  de `<span style="color: green">`), o que também é mais seguro.
- **Detalhes:** [`assistant/tasks/03-task.md`](assistant/tasks/03-task.md),
  [`assistant/architecture/assistant-windows.md`](assistant/architecture/assistant-windows.md).
- **Data:** 2026-10-18 · **Status:** Ativa

## BR-026 — Janelas de assistente separadas e móveis
- **Decisão:** resumo e sugestões vivem em **janelas próprias**, sem moldura,
  sempre no topo, arrastáveis pelo cabeçalho e com a posição lembrada. O automático
  só roda com a janela **aberta** (eventos `panel_opened`/`panel_hidden`) e a
  sessão gravando; pausar/ocultar interrompe as chamadas.
- **Motivo:** pedido do usuário; o Perssua usa um modal/Notch interno (R12), então
  este é um desvio consciente.
- **Consequência:** como qualquer janela no Linux, elas **aparecem** no
  compartilhamento de tela (BR-006); o limite está documentado.
- **Detalhes:** [`assistant/architecture/assistant-windows.md`](assistant/architecture/assistant-windows.md) §5-§6.
- **Revisão (2026-10-18, PRD notch/T03):** as janelas de assistente continuam
  existindo para **Sugestões** e **Histórico**; a janela **`summary`** foi
  **aposentada** (vira aba do painel — BR-033) e a janela **`chat`** passa a ser o
  **painel em abas** (Transcrição `|` Resumo). O `Surface` do overlay segue
  `actionbar | hidden`.
- **Data:** 2026-10-18 · **Status:** Ativa

## BR-027 — Precisão do ASR: prompt de decodificação, anti-alias e modelo por GPU
- **Decisão:** toda decodificação local envia um `initial_prompt`
  (`whisper_prompt`, default `"Only transcribe what is clearly spoken. If
  uncertain, transcribe nothing."`) acrescido da **regra de idioma**
  (`pt-br` → `pt`, com "Transcribe it in <idioma>. Do not translate it to
  English."); prompt **vazio desliga** o recurso. A decimação do microfone aplica
  **passa-baixa antes de reduzir a taxa** (anti-aliasing). O catálogo ganha
  `large-v3` e `large-v3-turbo` (recomendados **com GPU**; o default do modelo
  continua `tiny`). O app **detecta GPU** (Vulkan/DRM no Linux-first) e **sugere**
  o modelo adequado na tela de configurações, **sem trocar sozinho**. Os limiares
  de gate/VAD **permanecem os do BR-018**.
- **Motivo:** paridade com o Perssua (BR-015) e o pedido do usuário ("mais
  acertivo na transcrição"); o prompt e o anti-alias são ganhos baratos e o
  modelo grande só faz sentido junto com GPU.
- **Consequência:** as features de GPU (`cuda`/`hipblas`/`metal`) são declaradas
  agora e a **aceleração real** é o entregável E3
  ([`precision-and-performance/tasks/03-task.md`](precision-and-performance/tasks/03-task.md));
  a sugestão de GPU é informativa e não altera `Settings`.
- **Detalhes:** [`precision-and-performance/architecture/precision-and-performance.md`](precision-and-performance/architecture/precision-and-performance.md)
  §2/§5; [`precision-and-performance/tasks/01-task.md`](precision-and-performance/tasks/01-task.md).
- **Data:** 2026-10-18 · **Status:** Ativa (implementada na task 01, aguardando teste manual)

## BR-028 — Sugestão ao vivo: fast-first, streaming e automático por fala nova
- **Decisão:** a sugestão ao vivo tem **dois passes**: um **fast-first** curto
  (≈3 linhas, teto de 420 tokens, temperatura 0, contrato compacto) publicado
  assim que fica pronto, e o **passe completo** em seguida. O texto chega à UI em
  **streaming** pelo evento `suggestion` (texto acumulado; a janela substitui a
  bolha a cada delta). O contexto inclui **falas em andamento** (prévias), não só
  as finalizadas. Os **insights automáticos** rodam no core a cada **fala nova**
  (o intervalo das settings é o **silêncio** depois da última fala; uma fala nova
  reinicia a contagem) e só com a **janela de Sugestões aberta** — fechada, não
  há geração (ninguém está vendo e não se gasta chamada); o toggle é
  `Settings.suggestion_auto_refresh` e a janela exibe/força/fixa. No chat, a
  bolha da IA começa **colapsada em ~3 linhas** com "mostrar mais".
- **Motivo:** paridade com o Perssua (`fastFirstInsight`, `streamAutomaticInsights`,
  `questionsRequireFinalizedTranscript: false`, notch compacto) e o pedido do
  usuário ("bolhas menores, mais rápido e mais preciso").
- **Revisto em 2026-10-19:** o automático voltou a exigir a **janela de
  Sugestões aberta** (pedido do usuário: “não gerar nenhuma sugestão enquanto a
  janela estiver fechada”). O *debounce* fixo de 700 ms + teto de 3 s foi
  substituído pelo **silêncio das settings** (BR-039). **Abrir a janela não
  gera**: a contagem reinicia na abertura (o auto começa do intervalo); com o
  automático pausado, só o botão “Sugerir agora” gera.
- **Consequência:** `suggestion_interval_secs` deixa de controlar o ritmo do
  automático (o critério passa a ser fala nova, com teto de frequência); providers
  sem SSE caem no passe único sem erro visível; falha do rascunho ou do streaming
  não derruba a sugestão final; sugestões automáticas continuam **efêmeras** (só
  **Fixar** entra no export, BR-023).
- **Detalhes:** [`precision-and-performance/tasks/02-task.md`](precision-and-performance/tasks/02-task.md),
  [`precision-and-performance/architecture/precision-and-performance.md`](precision-and-performance/architecture/precision-and-performance.md) §3.
- **Revisão (2026-10-19):** a janela de Sugestões mostra um **pill dentro do bloco
  do texto** ("Gerando uma nova sugestão…") enquanto a IA gera/transmite — a
  sugestão atual continua visível até a nova chegar e a anterior vai para o
  histórico. O pill fica junto do conteúdo (não no cabeçalho) para o usuário saber
  que é *aquela* sugestão sendo substituída, e não uma geração genérica.
- **Data:** 2026-10-18 · **Status:** Ativa (implementada na task 02, aguardando teste manual)

## BR-030 — Tradução sob demanda dentro da bolha
- **Decisão:** a tradução de uma fala é **sob demanda** e vive **dentro da bolha**:
  um ícone no canto superior direito pede a tradução, que aparece **abaixo** do
  original, separada por **linha tracejada**, em **itálico e menor**; um novo
  clique **volta ao original**. O painel de tradução abaixo da conversa é
  **removido** e não há botão de copiar por tradução. Quando o idioma detectado
  pelo ASR (origem `auto`) é igual ao destino, a tradução **não** é oferecida. A
  tradução **automática em massa** fica fora (pode voltar, amadurecida, depois).
- **Motivo:** feedback do usuário — o painel duplicado repetia a conversa e era
  ruim de ler/testar; e com origem `auto` + destino pt o app traduzia pt→pt.
  **Revoga** o não-objetivo do PRD `chat` ("tradução dentro da bolha continua
  separada").
- **Consequência:** a fala só vai ao provider quando o usuário **clica** (BR-005);
  a tradução é **efêmera** (não vai ao banco nem ao export); a fila é serial e o
  resultado é cacheado por fala.
- **Detalhes:** [`translation/prd.md`](translation/prd.md),
  [`translation/tasks/01-task.md`](translation/tasks/01-task.md),
  [`translation/tasks/02-task.md`](translation/tasks/02-task.md).
- **Data:** 2026-10-18 · **Status:** Ativa (T1 implementado; T2 pendente)

## BR-031 — Teto da bolha de fala: fala longa continua em bolha seguinte
- **Decisão:** a bolha de fala tem **teto de 180 caracteres**
  (`MAX_SPEECH_BUBBLE_CHARS`) e o excedente **continua numa bolha seguinte** da
  mesma fala (`continues`, id `#N`), sem repetir rótulo nem tempo. A divisão é
  **pura e determinística** (`splitSpeechText`), aplicada na fala finalizada, na
  **prévia em andamento** e no **histórico**. O corte prefere o fim da frase
  (`.!?…`, quebra de linha) e, quando a frase sozinha passa do teto, cai no limite
  de **palavra** — nunca no meio de uma palavra. É um teto de **exibição**: o texto
  do ASR e o áudio continuam inteiros, e o `max_turn_ms` (25 s) segue como teto de
  custo do whisper (BR-029).
- **Motivo:** o Interlocutor fala sem pausas e o turno só fecha no silêncio (1 s)
  ou no teto do turno (25 s): a bolha crescia sem limite (ISS-031, reclamação do
  usuário no teste). O Perssua não sofre disso porque o *commit* em nuvem é de ~2 s;
  no whisper local, cortar o **áudio** por tempo picaria frase no meio e pioraria a
  transcrição — cortar a **exibição** não custa nada.
- **Consequência:** fala curta continua em uma bolha só (caso comum inalterado); a
  tradução passa a ser por parte da fala; o tamanho da bolha se ajusta editando uma
  constante no domínio.
- **Detalhes:** [`chat/tasks/06-task.md`](chat/tasks/06-task.md),
  [`chat/architecture/chat-ui.md`](chat/architecture/chat-ui.md) §2–§6,
  [`chat/issues.md`](chat/issues.md) ISS-031.
- **Data:** 2026-10-18 · **Status:** Ativa

## BR-029 — Engine: GPU/flash attention, modelo por hardware e reuso do passe final
- **Decisão:** o build declara features de GPU (`vulkan`/`cuda`/`hipblas`/`metal`)
  e, quando compilado com elas, o Whisper roda em **GPU com flash attention** —
  desligado no macOS, como o Perssua (`--no-flash-attn`); sem elas, CPU (nada
  muda). No **Linux sem ROCm/CUDA** (o caso desta máquina: AMD Polaris), o caminho
  é **Vulkan** (`whisper-rs/vulkan` → `whisper-rs-sys/vulkan`, backend Vulkan do
  ggml via RADV/Mesa); o SDK de build é montado por `scripts/vulkan-env.sh`
  (usa `libvulkan-dev`/`glslc` do sistema ou baixa os `.deb` **sem sudo**), e os
  scripts `pnpm tauri:dev:vulkan` / `tauri:build:vulkan` compilam com a feature.
  **Um artefato só**: o pacote de release (`pnpm tauri:build`) já vem com Vulkan e
  o **loader `libvulkan.so.1` embutido** (AppImage; `libvulkan1` como dependência
  no `.deb`), então o app **escolhe a GPU em runtime e cai para CPU sozinho** sem
  dispositivo. O usuário escolhe o modo em **Aceleração** (`auto`/`cpu`/`gpu`); se
  o app morrer com a GPU ativa, uma **marca de crash** desliga a GPU no próximo
  boot (padrão Perssua). O modelo recomendado com GPU é o **`small`** — sugerido
  com um clique na tela de transcrição, **sem** troca automática (override
  manual). O `large-v3-turbo` continua disponível, mas na RX 480 mediu **piso de
  ~2,4 s por chamada** (RTF < 1 em fala curta), então não é o default de GPU; o
  `small` decodifica 8 s em ~0,7 s contra ~1,5–2 s do `tiny` na CPU. Quando
  o VAD nativo do Whisper fecha o turno dentro da própria prévia, o texto já
  decodificado é **promovido a final**, sem um segundo passe beam search
  (Perssua: `endedInNoise`).
- **Motivo:** paridade com o Perssua e o pedido do usuário ("mais rápido"); o
  passe duplicado era trabalho jogado fora no fim de cada turno.
- **Consequência:** em CPU o comportamento é idêntico ao anterior; falha de driver
  cai em CPU (`use_gpu` do `whisper-rs`); a promoção só acontece quando a prévia
  **cobre o turno inteiro** (o VAD marcou o fim), preservando o passe autoritativo
  quando não cobre. **AEC** e ajuste de **commits** ficam pendentes na task 03.
  No áudio, o microfone ganha **AEC referenciado** no áudio do sistema (NLMS com
  guarda de *double-talk*, ligado por padrão com toggle), aplicado **antes** do
  ganho. Os **commits** de turno ficam em **1 s de silêncio** — paridade com o
  `local_whisper_vad_silence_duration_ms` do Perssua; os 650 ms são do provider
  realtime em nuvem, e cortar mais picaria frase no meio.
- **Detalhes:** [`precision-and-performance/tasks/03-task.md`](precision-and-performance/tasks/03-task.md),
  [`precision-and-performance/architecture/precision-and-performance.md`](precision-and-performance/architecture/precision-and-performance.md) §4.
- **Data:** 2026-10-18 · **Status:** Ativa (E3 completo: reuso do passe final + GPU/Vulkan + AEC + commits avaliados)

## BR-032 — Notch em dois estados, por ícones, com ação primária verde
- **Decisão:** o overlay (`ActionBar`) tem **dois estados**: **repouso** (sem sessão)
  e **ativo** (com sessão). Em repouso mostra só alça de arraste, marca, **engrenagem**
  (Configurações), a ação primária **"Começar a ouvir"** em **verde** com o **atalho
  em badges** e ocultar; em atividade mostra os controles da sessão (transporte único,
  canais, pílula de tempo) e os **ícones** de Transcrição/Sugestões, mais stealth,
  engrenagem e ocultar. O **Histórico** fica **sempre visível** (nos dois estados),
  para abrir sessões anteriores sem iniciar uma sessão. Todos os botões de ação são
  **ícones SVG próprios**
  (sem rótulo de texto visível), com `title` **e** `aria-label`. O verde é a cor da
  ação primária; o azul fica reservado a estados "ligado/ativo" de toggles. A
  superfície segue `actionbar | hidden` (sem painel expansível).
- **Motivo:** pedido do usuário ("notch limpo quando não há sessão ativa", "mais
  ícones em vez de texto", "engrenagem para configurações", "botão destacado em verde
  para dar início, junto da indicação do atalho"); paridade de **forma** com o
  Perssua (BR-015) sem copiar a paleta.
- **Consequência:** ícones não têm rótulo visível → `title`/`aria-label` são
  obrigatórios; os badges derivam do binding persistido de `toggle_session` (mudam
  quando o atalho muda); o seletor de canais continua um `select` (é dado, não ação).
- **Detalhes:** [`notch/prd.md`](notch/prd.md) §4 (US-01/US-02/US-07),
  [`notch/architecture/notch-ui.md`](notch/architecture/notch-ui.md) §2 e §6.
- **Revisão (2026-10-19, BR-035):** o verde da ação primária passa a ser o
  **esmeralda `#10b981`** do design system Obsidian HUD; o azul de "ligado" cede
  lugar a **cyan** (foco/live) e **violeta** (IA), conforme o papel.
- **Revisão (2026-10-19, task 08):** o estado **ativo** ganha o **switcher**
  Transcrição/Histórico/Resumo + **waveform** (reage ao áudio sem re-render) +
  **play/pause contextual** + **timer `HH:MM:SS`**; canais viram popover (com "Nova
  sessão"). O **repouso** não muda de estrutura.
- **Revisão (2026-10-19, task 13):** o **Histórico** sai do switcher e vira
  **ícone no grupo da direita, antes de Configurações**; o botão **ocultar** passa a
  usar o ícone de **recolher** (duas setas diagonais apontando para o centro), como
  no protótipo.
- **Data:** 2026-10-18 · **Status:** Ativa (task 01/02)

## BR-033 — Painel da sessão em abas (Transcrição | Resumo); `summary` aposentado
- **Decisão:** a janela `chat` passa a ser o **painel da sessão**, com uma *pill* de
  abas **Transcrição | Resumo** (aba ativa lembrada em `Settings.panel_tab`). A
  conversa é a aba **Transcrição** (termo "Chat" sai da UI, mantendo o label técnico
  da janela) e o resumo é a aba **Resumo**. A janela **`summary` é aposentada** (fora
  do `tauri.conf.json`, `capabilities`, `PANEL_LABELS` e `App.tsx`; `Settings` perde
  `summary_position`/`summary_size`). Não há botão "Resumo" duplicado no cabeçalho da
  Transcrição: o resumo é a aba ao lado (revisão de 2026-10-19). O **automático** do
  resumo só chama o LLM com a **aba Resumo ativa**; "Atualizar agora" vale em qualquer
  aba. A aba **"Sessão"** fica reservada para PRD futura.
- **Motivo:** pedido do usuário — unificar a conversa e o resumo em **abas**
  redimensionáveis "como o Perssua", sem manter duas janelas irmãs para a mesma
  sessão; e conter o custo do resumo com a janela sempre aberta.
- **Consequência:** **sem** migração de `chat_position`/`chat_size` (o label da
  janela continua `chat`); config antiga com `summary_*` é ignorada no load (serde);
  a janela `chat` e a `history`, da task 07/08 do PRD chat, continuam existindo. A
  task 03 do PRD notch **evolui** a task 07 do chat (não a reverte).
- **Detalhes:** [`notch/prd.md`](notch/prd.md) §4 (US-03/US-04/US-05),
  [`notch/architecture/notch-ui.md`](notch/architecture/notch-ui.md) §1 e §5,
  [`chat/tasks/07-task.md`](chat/tasks/07-task.md).
- **Data:** 2026-10-18 · **Status:** Ativa (task 03)

## BR-034 — Troca do idioma da transcrição em tempo real
- **Decisão:** a **barra inferior** da aba Transcrição tem um seletor do **idioma de
  origem** (com **Auto**). Trocar o idioma aplica ao **próximo passe de decodificação**
  da sessão **em andamento** — **sem** reiniciar a sessão e **sem** recarregar o
  modelo — e persiste em `settings.source_language`. O seletor muda **apenas** a
  origem; o idioma de **destino** da tradução permanece nas Configurações.
- **Motivo:** pedido do usuário ("mudar o idioma reflete em tempo real na transcrição
  de áudio"); hoje o idioma só valia na próxima sessão, porque faz parte da chave de
  cache do motor ([`notch/issues.md`](notch/issues.md) → ISS-033).
- **Consequência:** no núcleo, `language` vira `Mutex<String>` com `update_language`
  (mesmo padrão já usado para o **prompt**) e **sai da chave do cache** da fábrica;
  a regra de idioma do `initial_prompt` (`decode_prompt`) é recomputada. Novo IPC
  `session_set_language`. Um passe **em voo** termina no idioma anterior; o próximo
  já usa o novo.
- **Detalhes:** [`notch/prd.md`](notch/prd.md) §4 (US-06),
  [`notch/architecture/notch-ui.md`](notch/architecture/notch-ui.md) §4,
  [`notch/tasks/04-task.md`](notch/tasks/04-task.md),
  [`precision-and-performance/tasks/01-task.md`](precision-and-performance/tasks/01-task.md) (prompt/idioma).
- **Data:** 2026-10-18 · **Status:** Ativa (task 04)

## BR-035 — Sistema visual "Obsidian HUD"
- **Decisão:** a interface adota o design system **Obsidian HUD**, com os tokens
  canônicos em [`notch/architecture/design-tokens.md`](notch/architecture/design-tokens.md):
  - **Superfícies** ink (`--oh-surface` `#0f131c` e containers `#181c24`…`#31353e`);
    **vidro** (`backdrop-filter`) apenas nas **cascas** (notch, painel, popovers),
    nunca por bolha/item (ISS-020); sem suporte, superfície opaca.
  - **Sinais** com papéis fixos: **cyan `#00f2fe`** (foco/transcrição ao vivo),
    **violeta `#8b5cf6`** (IA), **esmeralda `#10b981`** (gravação/sucesso e ação
    primária) e **coral `#f43f5e`** (erro/parar). "Pausado" usa o *outline* (sem
    âmbar).
  - **Borda hairline** em gradiente de 1px; **glow** de foco cyan; sombra difusa
    nos elementos inativos; raio em pill (`--oh-radius-full`) e espaçamento base
    de 4px.
  - **Tipografia:** **Geist** (UI) e **JetBrains Mono** (tempos, atalhos e código),
    empacotadas **localmente** com fallback; números tabulares em timers.
  - **Leitura:** painel/bolhas/configurações usam superfície quase opaca
    (`--oh-read-surface`); o vidro é do HUD, não do conteúdo.
- **Motivo:** pedido do usuário (design novo a padronizar); manter leitura e
  desempenho (ISS-020) e dar uma identidade única às superfícies.
- **Consequência:** o verde de BR-032 passa a ser o **esmeralda** do sistema; o
  azul de "ligado" cede lugar a cyan/violeta conforme o papel; cores/tamanhos
  soltos migram para `--oh-*` (escrita única — R14).
- **Detalhes:** [`notch/prd.md`](notch/prd.md) §4 (US-09/US-10),
  [`notch/architecture/design-tokens.md`](notch/architecture/design-tokens.md),
  [`notch/tasks/07-task.md`](notch/tasks/07-task.md),
  [`notch/tasks/08-task.md`](notch/tasks/08-task.md).
- **Data:** 2026-10-19 · **Status:** Ativa (tasks 07/08 concluídas)

## BR-036 — Drill-hover (ativação por permanência) no notch
- **Decisão:** botões **opt-in** do notch podem ser acionados por **permanência do
  cursor** (*drill-hover*/dwell), sem clique, pensado para o modo stealth. São **dois
  estágios**: (1) o botão se preenche da esquerda para a direita durante `dwell_ms`;
  ao completar, abre uma **confirmação flutuante**; (2) o botão **"Sim"** da
  confirmação também exige `dwell_ms`. **Clique** no botão original executa a ação
  **imediatamente**, sem confirmação; clique no "Sim" confirma (fallback acessível).
- **Configuração:** `Settings.dwell_enabled` (padrão **ligado**) e
  `Settings.dwell_ms` (padrão **500 ms**, faixa **300–2000 ms**), em Configurações →
  Ajustes. Desligado, o hover não faz nada e o clique continua normal. A duração vale
  para os dois estágios.
- **Escopo inicial:** somente **Resumo** e **Sugestões** no notch ativo. O mecanismo
  é genérico e opt-in: qualquer botão futuro usa o mesmo componente/porta.
- **Confirmação:** **janela Tauri própria**, pequena e transparente, posicionada
  **acima** do gatilho quando há espaço na **área útil do monitor**; senão **abaixo**.
  Ela **não** redimensiona nem move a janela do notch. Conteúdo mínimo: dica
  **"Sem clique"** + botão **"Sim"** + duração. **Sem som/toast**.
- **Cancelamento:** a confirmação aberta fecha após `dwell_ms` quando o cursor **não**
  está no gatilho nem na confirmação (grace); voltar a qualquer uma das áreas cancela
  o fechamento. Sobre o "Sim", o dwell de confirmação corre normalmente. O
  preenchimento do gatilho (estágio 1) cancela **na saída**.
- **Motivo:** pedido do usuário — acionar sem clique ajuda no stealth; o hover não
  deve acionar direto (evita acidente) e a confirmação precisa tolerar saídas
  involuntárias do cursor.
- **Consequência:** `Settings` ganha `dwell_enabled`/`dwell_ms`; nova janela `dwell`
  + porta/adaptador/IPC; `DwellProvider` no overlay; sem som; sem mudança de schema.
- **Detalhes:** [`notch/prd.md`](notch/prd.md) §4 (US-11),
  [`notch/architecture/drill-hover.md`](notch/architecture/drill-hover.md),
  [`notch/tasks/09-task.md`](notch/tasks/09-task.md)…[`12-task.md`](notch/tasks/12-task.md).
- **Data:** 2026-10-19 · **Status:** Ativa (tasks 09–12)

## BR-037 — Identidade visual do agente (ícone + cor)
- **Decisão:** todo agente de sugestão (embutido **e** personalizado) tem **ícone**
  (catálogo: `sparkle`, `users`, `braces`, `code`, `layers`, `book`, `briefcase`,
  `terminal`) e **cor** (tokens do design system: `cyan`, `violet`, `emerald`,
  `coral`, `slate`). Os embutidos são fixos (Reunião `users/cyan`, Entrevista
  Técnica `braces/violet`, LeetCode `code/emerald`, System Design `layers/coral`);
  os personalizados escolhem nas Configurações (default `sparkle/slate`). A identidade
  é **snapshotada** na sugestão, para o histórico mostrar quem respondeu. Ícones são
  SVG do projeto (BR-008); a cor é o **nome do token** (não hex), então o tema
  continua sendo a fonte da verdade.
- **Motivo:** pedido do usuário — o agente precisa ser reconhecível de relance
  (ícone + cor) no seletor, no título da sugestão e no histórico.
- **Consequência:** `AssistantMode` ganha `icon`/`color` (Rust + TS), com
  `serde(default)`/`sanitized()` (settings antigos seguem válidos); validação de
  catálogo/paleta em `validate_mode`; novos `AgentIcon`/`AgentBadge`; **sem
  mudança de schema** do banco.
- **Detalhes:** [`suggestions/prd.md`](suggestions/prd.md) §4/§5 (US-02),
  [`suggestions/architecture/agent-identity.md`](suggestions/architecture/agent-identity.md),
  [`suggestions/tasks/01-task.md`](suggestions/tasks/01-task.md),
  [`suggestions/tasks/04-task.md`](suggestions/tasks/04-task.md).
- **Data:** 2026-10-19 · **Status:** Ativa (task 01 concluída)

## BR-038 — Sugestão em duas fases: título, resposta rápida e resposta completa
- **Decisão:** a sugestão passa a ter **resposta rápida** e **resposta completa**,
  com **fase explícita** no modelo (`quick` | `full`). A rápida (curta, gerada
  primeiro, com o **contrato de prompt que já existe**) fica em **bloco próprio** e
  **permanece** quando a completa chega; a completa é Markdown livre, com chip de
  metadados e blocos de código com cabeçalho/rodapé. O **título** exibido é
  **gerado pelo agente** via retorno estruturado leve: a resposta começa com
  `Subject: <título curto>` (mesmo padrão do resumo), que o core extrai para
  `Suggestion.title` e remove do Markdown. Sem `Subject:` (ou em sugestões
  antigas), cai para o título derivado (`subject` → primeira linha
  significativa → rótulo neutro), que **pula seções genéricas** (`## Easy`,
  `## Practical`, `**Initial implementation**`, `**Big O:**`…) — sem isso,
  perguntas diferentes viravam “Easy”, “Easy”, “Easy”. O cartão e o histórico
  usam o mesmo título. O **histórico** é uma
  seção colapsável (“Histórico de Sugestões da Sessão” + contador) com accordions
  (ícone do agente + título + pill do agente): expandir **rola até** o bloco e
  **nunca** substitui a sugestão do topo. A resposta rápida é **efêmera** (só a
  completa é fixada/copiada/exportada).
- **Motivo:** pedido do usuário — a resposta curta “do que falar agora” não pode
  sumir quando o material completo chega, e o histórico não pode roubar a
  leitura. O editor do protótipo (que cobria o histórico) não é adotado.
- **Consequência:** `Suggestion` ganha `phase`, `mode_icon` e `mode_color`; o
  evento `suggestion` transporta a fase; **sem** mudança de contrato de prompt e
  **sem migração** do banco (`messages.content` continua com o Markdown
  completo). Ao chegar a **rápida de uma geração nova**, a sugestão anterior sai
  do topo (já está no histórico) e o cartão mostra a nova sendo montada — nunca
  se confunde a nova com a antiga. Enquanto só existe a rápida, o cartão mostra
  **“Gerando resposta completa…”**.
- **Detalhes:** [`suggestions/prd.md`](suggestions/prd.md) §4/§5 (US-03/US-04),
  [`suggestions/architecture/suggestions-window.md`](suggestions/architecture/suggestions-window.md),
  [`suggestions/tasks/02-task.md`](suggestions/tasks/02-task.md),
  [`suggestions/tasks/05-task.md`](suggestions/tasks/05-task.md),
  [`suggestions/tasks/06-task.md`](suggestions/tasks/06-task.md).
- **Data:** 2026-10-19 · **Status:** Ativa (tasks 02/05/06 concluídas)

## BR-039 — Cadência visível do auto-sugerir (progress circular)
- **Decisão:** o cabeçalho da janela de Sugestões mostra **LIVE** e o controle de
  **auto-sugerir** com **texto, contagem regressiva e progress circular** (anel
  que esvazia). O controle é: anel + rótulo **“Auto-sugerir”** + pill com a
  contagem (“12s”); pausado, a pill vira **“Pausado”**. A cadência vem do
  **agendador** (evento `suggestion_cadence`: `enabled`, `running`,
  `remaining_ms`, `awaiting_speech`, `window_ms`) — a UI **não** recalcula.
  O intervalo das **settings** (`suggestion_interval_secs`, 10–120 s) é o
  **tempo de silêncio** depois da última fala: **cada fala nova reinicia a
  contagem**; quando o silêncio completa o intervalo, a sugestão é gerada.
  Uma sequência de perguntas próximas vira **uma** sugestão (não uma por
  balão). **Trocar de agente não gera** sugestão: o agente escolhido vale para
  a próxima geração (fala nova + contagem, ou “Sugerir agora”). “Sugerir agora”
  fica sem glow.
- **Sem fala nova, não há nova sugestão:** nenhuma chamada ao LLM. O serviço
  guarda a última sugestão da sessão e a devolve; o botão “Sugerir agora” (e o
  atalho/chat “Sugerir”) **forçam** a geração.
- **Sem duplicata:** o cursor efetivo é o **mais avançado** entre o pedido e a
  última sugestão (o agendador pode estar com um cursor velho se a janela gerou
  depois dele), e as gerações são **serializadas** — duas chamadas concorrentes
  (janela × agendador) não geram duas sugestões para a mesma fala.
- **Motivo:** pedido do usuário — o usuário precisa ver **quando** a próxima
  sugestão vem e se o automático está rodando; e o automático não deve gastar
  chamada (nem repetir sugestão) sem conteúdo novo.
- **Consequência:** novo evento no `EventPublisher`/agendador; hook
  `useSuggestionCadence`; círculo em SVG (`stroke-dashoffset`, sem re-render por
  quadro); a pausa/retomada continua em `settings.suggestion_auto_refresh`;
  `SuggestionRequest.force` + cache da última sugestão no serviço; o agendador
  passa a usar `settings.suggestion_interval_secs` (que antes era configuração
  morta).
- **Detalhes:** [`suggestions/prd.md`](suggestions/prd.md) §4/§5 (US-01),
  [`suggestions/architecture/suggestions-window.md`](suggestions/architecture/suggestions-window.md) §3.3,
  [`suggestions/tasks/03-task.md`](suggestions/tasks/03-task.md).
- **Data:** 2026-10-19 · **Status:** Ativa (task 03 concluída)

## BR-040 — "Transcrição ao Vivo": cartões em largura total e identidade por canal

- **Decisão:** o painel ao vivo (`chat`, aba **Transcrição**) passa a seguir o
  print de referência do usuário (2026-10-19):
  - cabeçalho com **ícone + "Transcrição ao Vivo"** e *pill* de abas
    **Transcrição | Resumo** em **largura total**, com a ativa em contorno
    **cyan**;
  - **toda mensagem** é um cartão de **largura total** (sem `max-width:75%` nem
    alinhamento à direita/esquerda), independente do tamanho do texto;
  - **Microfone** (canal do usuário) em cor **primária/cyan** e **Outros** (canal
    do sistema) em **violeta**, com a **hora da mensagem** ao lado do nome;
  - a hora exibida é **de parede** (`HH:MM:SS`, derivada de
    `started_at_ms + at_ms`), com fallback `mm:ss`; o dado persistido continua
    **relativo à sessão** (BR-019);
  - a fala em andamento tem borda/fundo cyan, **"Falando agora…"** e selo
    **Streaming**; continuação não repete nome/hora;
  - cada fala **finalizada** ganha a ação de IA no canto superior direito
    (BR-041); prévia/continuação não;
  - o rodapé mostra **"Idioma de Entrada:"** + seletor e o **estado real** do
    Whisper à direita. **Não** exibir latência/“WebRTC” inventados;
  - a linha de medidores e o botão de texto "Sugestões" **saem** da aba (o
    waveform e o atalho ficam no notch); o header arrasta pelo título/ícone e o
    **grip de arraste** fica no canto superior direito de todas as janelas;
  - o histórico usa o mesmo cartão e a mesma hora, sem a ação de IA.
- **Motivo:** pedido do usuário — "deixe o layout o mais fiel possível" ao print
  anexado, com microfone/outros em cores distintas, hora por mensagem e cartões
  que preenchem a janela.
- **Consequência:** a parte visual da BR-021 (alinhamento direita/esquerda e
  `max-width:75%`) é **revisada** para o ao vivo; a linha temporal,
  prévia/final e o teto de 180 caracteres (BR-031) não mudam. O layout do print
  é a fonte desta revisão; a evidência Perssua 0.27.5 em
  [`chat/architecture/chat-ui.md`](chat/architecture/chat-ui.md) §3 fica como
  histórica. Sem mudança de schema.
- **Detalhes:** [`chat/prd.md`](chat/prd.md) §9,
  [`chat/architecture/transcription-panel.md`](chat/architecture/transcription-panel.md)
  §2–§3, [`chat/tasks/10-task.md`](chat/tasks/10-task.md),
  [`chat/tasks/11-task.md`](chat/tasks/11-task.md).
- **Data:** 2026-10-19 · **Status:** Ativa (tasks 10/11 implementadas)

## BR-041 — Sugestão manual a partir de uma mensagem (contexto por parâmetro)

- **Decisão:** cada mensagem finalizada da Transcrição tem uma ação de **IA** que
  leva a mensagem (sessão, falante, texto inteiro e instante) para a janela de
  **Sugestões**, onde o usuário vê um bloco de **contexto manual** e só gera a
  sugestão ao clicar em **"Gerar sugestão manual"**. O clique no `[✦]` **não**
  chama o LLM.
  - O contexto é um **parâmetro explícito** do caso de uso
    (`SuggestionRequest.manual_context`), não um estado global de UI; o store
    entre as janelas é apenas o correio (preparar → abrir/buscar).
  - A sugestão gerada é marcada com `trigger = manual` e exibida como
    **"Sugestão Manual"** no cartão e no histórico; as fases rápida/completa
    (BR-038) valem normalmente.
  - O **automático** do core **não** usa o contexto manual; "Sugerir agora"
    continua sendo a sugestão normal.
  - A sugestão manual é **efêmera**; "Fixar" continua sendo o caminho para a
    linha temporal/export (BR-020). Sem migração de banco e sem backend novo.
- **Motivo:** pedido do usuário — "ao clicar, a mensagem é levada via parâmetro
  para o Sugestões, e dali será gerada uma sugestão Manual pelo usuário"; e a
  arquitetura já recebe contexto por parâmetro, então o handoff deve preservar
  isso.
- **Consequência:** novo valor `ManualSuggestionContext`, porta
  `ManualSuggestionContextStore` + adaptador em memória, comandos IPC
  `suggestion_prepare_manual` / `suggestion_manual_context` /
  `suggestion_clear_manual_context`; `SuggestionRequest.manual_context` e
  `Suggestion.trigger`; prompt com bloco `<selected_message>` (fonte não
  confiável). O evento `panel_opened` acorda a janela e ela busca o store — um
  evento efêmero sozinho poderia se perder com a janela fechada. A geração
  manual também **avança o cursor** da sessão; o agendador consulta o cursor da
  última sugestão do serviço para não gerar uma duplicata automática da mesma
  fala. O caso de já existir sugestão em andamento e a janela aberta/fechada é
  regido pela **BR-042** (single-flight e prioridade do manual).
- **Detalhes:** [`chat/prd.md`](chat/prd.md) §9,
  [`chat/architecture/transcription-panel.md`](chat/architecture/transcription-panel.md)
  §4–§6, [`chat/tasks/12-task.md`](chat/tasks/12-task.md),
  [`chat/tasks/13-task.md`](chat/tasks/13-task.md),
  [`suggestions/architecture/suggestions-window.md`](suggestions/architecture/suggestions-window.md)
  §5.4.
- **Data:** 2026-10-19 · **Status:** Ativa (tasks 12/13 implementadas)

## BR-042 — Sugestões: single-flight e prioridade do contexto manual

- **Decisão:** a geração de sugestões é **single-flight no serviço**: no máximo
  um fast-first + streaming por vez (janela × agendador × manual). Automático e
  manual não emitem `quick`/`full` intercalados; a segunda chamada **espera** a
  primeira. Não há cancelamento de LLM em voo (seria uma task própria). Enquanto
  houver **contexto manual ativo**:
  - a `quick` automática é ignorada (efêmera);
  - a `full` automática vai para o **histórico**, sem substituir o cartão manual;
  - a manual tem prioridade de leitura; ao limpar o contexto, a automática volta
    a poder ocupar o cartão.
  A janela de Sugestões **continua montada quando oculta**: os eventos acumulam
  enquanto ela estava aberta e reabrir não dispara geração. **Abrir nunca
  gera** (BR-028 revisada): a abertura só consulta `suggestionLast` (cache, sem
  LLM) para hidratar o cartão; quem gera é o automático (com a janela aberta e
  depois do intervalo) ou o botão. Um novo `[✦]` durante uma manual substitui o
  contexto;
  a manual antiga termina rotulada com `Suggestion.source` (snapshot leve de
  falante, `at_ms` e `preview`; o texto completo fica no `manual_context`/
  prompt). Cada geração carrega um `generation_id` (UUID) compartilhado por
  `quick`/`full`/deltas; a UI usa o id como chave, não o `at_ms`. Trocar de
  sessão limpa a leitura e ignora eventos/resultados antigos.
- **Motivo:** aviso do usuário — é preciso cuidar de já existir uma sugestão em
  andamento e do estado da janela (aberta/fechada), sem streams misturados,
  geração duplicada ao reabrir ou perda do contexto.
- **Consequência:** o `gate` do `SuggestionService` (já existente) serializa
  tudo; `Suggestion` ganha `generation_id` (UUID por geração; cache preserva o
  id), `trigger` e `source: Option<SuggestionSource>` (falante/`at_ms`/preview);
  `useSuggestions` usa o id como chave de `current`/`quick`/histórico, filtra a
  automática para o histórico quando o manual está ativo e **não gera ao
  abrir**. Sem schema; sem backend novo.
- **Detalhes:** [`chat/prd.md`](chat/prd.md) §9 (US-13),
  [`chat/architecture/transcription-panel.md`](chat/architecture/transcription-panel.md)
  §5.4, [`chat/tasks/15-task.md`](chat/tasks/15-task.md).
- **Data:** 2026-10-19 · **Status:** Ativa (task 15 implementada)

## BR-043 — Linha temporal intercalada por sobreposição dos canais

- **Decisão:** quando a fala de um canal começa antes e termina depois do início
  da fala do **outro** canal, a fala longa é **cortada no início do outro** para
  a linha temporal ficar coerente:
  - o texto é dividido proporcionalmente ao tempo, preferindo fim de frase e,
    depois, fim de palavra; se não houver corte seguro, a fala fica inteira;
  - a primeira parte mantém o tempo original; a nova fala entra no tempo dela; o
    restante entra **1 ms depois**, como uma nova bolha com rótulo/hora (não como
    continuação silenciosa);
  - vale nos dois sentidos (Microfone/Outros) e no histórico, cuja fala passa a
    carregar `end_ms`;
  - é um corte de **exibição**: ASR, banco e export continuam com a fala
    inteira.
- **Motivo:** relato do usuário — “se os outros estão falando sem parar e eu falo
  em seguida, ele vai jogando minha mensagem atrás”; a fala longa deve ser
  intercalada pela nova, não engolir a ordem.
- **Consequência:** `splitSpeechSpans` em `core/domain/chat.ts`, usado pelo
  `buildChatTimeline` e `bubblesFromTimeline`; `TimelineKind::Speech` ganha
  `end_ms` (sem schema); o **grip** de arraste passa a ser padrão no canto
  superior direito de todas as janelas de painel.
- **Detalhes:** [`chat/prd.md`](chat/prd.md) §9 (US-14),
  [`chat/architecture/transcription-panel.md`](chat/architecture/transcription-panel.md)
  §3.4, [`chat/tasks/16-task.md`](chat/tasks/16-task.md).
- **Data:** 2026-10-19 · **Status:** Ativa (task 16 implementada)
