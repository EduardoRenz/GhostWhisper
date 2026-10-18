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
- **Data:** 2026-10-18 · **Status:** Ativa (task 16)

## BR-020 — Sugestões da IA entram na linha temporal; export é Markdown
- **Decisão:** toda sugestão pedida (painel ou atalho) é **persistida** como
  mensagem da sessão (tabela `messages`, papel `assistant`) com o **tempo desde o
  início da sessão** — a mesma escala das falas — e aparece na linha temporal ao
  vivo, no histórico e no export. O export de uma sessão é um **Markdown** da linha
  temporal: cabeçalho (título, início/fim, duração) + `- **[mm:ss] Você:** fala` +
  a sugestão como bloco destacado `> 🤖 **Sugestão [mm:ss]** — texto`. O histórico
  vive no **painel expandido do overlay** (aba `Ao vivo | Histórico`).
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
