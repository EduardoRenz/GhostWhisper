# PRD — Notch limpo e painel em abas (Transcrição + Resumo)

- **Versão:** 0.1
- **Data:** 2026-10-18
- **Status:** Implementado (tasks 01–08 `CONCLUDED`)
- **Relacionados:** [`../business-rules.md`](../business-rules.md),
  [`memory.md`](memory.md), [`issues.md`](issues.md),
  [`architecture/notch-ui.md`](architecture/notch-ui.md),
  [`architecture/design-tokens.md`](architecture/design-tokens.md),
  [`tasks/_tasks.md`](tasks/_tasks.md), PRDs anteriores:
  [`../mvp/prd.md`](../mvp/prd.md), [`../chat/prd.md`](../chat/prd.md),
  [`../assistant/prd.md`](../assistant/prd.md),
  [`../translation/prd.md`](../translation/prd.md)

---

## 1. Objetivo

Melhorar **UX e layout** por paridade com o **Perssua** (BR-015), sem mudar
comportamento de produto, em três frentes de **forma**:

1. **Notch em dois estados.** Sem sessão ativa, a barra fica **limpa** — marca,
   **engrenagem** (Configurações) e a ação primária **"Começar a ouvir"** em
   **verde** com o atalho em *badges*; com sessão, aparecem os controles dela.
   Tudo **por ícone**, com `title`/`aria-label` (BR-032).
2. **Painel da sessão em abas.** A conversa — renomeada **Transcrição** — e o
   **Resumo** deixam de ser duas janelas e passam a ser **abas** de uma janela só
   (a janela `chat`), com uma *pill* de abas e a aba ativa lembrada (BR-033).
3. **Idioma em tempo real.** A **barra inferior** da aba Transcrição ganha um
   **seletor de idioma** (origem) que muda a transcrição **na hora**, sem
   reiniciar a sessão e sem recarregar o modelo (BR-034).

## 2. Problema

- **Notch poluído em repouso.** A `ActionBar` mistura rótulos de texto (`Chat`,
  `Histórico`, `Stealth: on/off`, `Ajustes`, `Ocultar`) e os controles de sessão
  (seletor de canais, transporte, "Nova sessão"), ocupando espaço mesmo quando
  não há sessão. O Perssua mantém o notch **compacto** em repouso e por ícones.
- **Duas janelas para a mesma sessão.** A conversa vive na janela `chat` (task 07
  do PRD [chat](../chat/prd.md)) e o resumo na janela `summary` (BR-023); são
  janelas **irmãs** para a mesma sessão, com cabeçalho e persistência duplicados e
  exigindo abrir/fechar as duas. O Perssua reúne as superfícies numa **única
  superfície com abas**.
- **Idioma preso à sessão.** Hoje o idioma da transcrição só é lido **na criação
  do motor** (`whisper_speech_engine_factory.rs` cacheia por
  `(modelo, idioma, gpu)`): trocar o idioma nas Configurações **só vale na próxima
  sessão**, o que obriga a encerrar a reunião para corrigir um idioma errado
  ([`issues.md`](issues.md) → ISS-033).

## 3. Público-alvo e casos de uso

Mesmas personas do MVP (`../mvp/prd.md` §3). Casos de uso desta PRD:

- **CU-01 — Abrir e começar rápido:** em repouso, um clique na ação verde inicia a
  escuta (com o atalho visível ao lado).
- **CU-02 — Configurar por engrenagem:** chegar às Configurações por um ícone, sem
  botão de texto.
- **CU-03 — Ler transcrição e resumo juntos:** alternar entre as abas
  **Transcrição** e **Resumo** na mesma janela, redimensionável e posicionável.
- **CU-04 — Corrigir o idioma no meio da reunião:** trocar o idioma na barra
  inferior e ver a próxima fala já no idioma novo.
- **CU-05 — Encontrar cada janela por ícone:** Sugestões, Histórico, stealth e
  ocultar como ícones no notch.

## 4. Histórias de usuário e critérios de aceite

### US-01 — Notch limpo em repouso

Como usuário, quero que o overlay fique discreto quando não estou em uma sessão.

- **CA-01.1** Sem sessão ativa, a barra mostra **apenas**: alça de arraste, marca,
  a ação primária **"Começar a ouvir"** (verde, US-07), **Histórico**, **ocultar** e
  a **engrenagem** (Configurações) como **último** botão redondo.
- **CA-01.2** Sem sessão, **não** aparecem: seletor de canais, transporte, pílula
  de tempo, "Nova sessão" e os atalhos de janela ligados à sessão.
- **CA-01.3** Com sessão (gravando ou pausada), a barra mostra o **switcher**
  (Transcrição + waveform · play/pause · Histórico · Resumo · Sugestões), o
  **timer**, os canais, stealth, ocultar e a **engrenagem (último)** (ver US-10).
- **CA-01.4** A superfície continua `actionbar | hidden` (sem painel expansível,
  como já vigente desde a task 07 do PRD chat).

### US-02 — Barra de ações por ícones

- **CA-02.1** Todos os botões da barra são **ícones SVG próprios** (BR-008), sem
  rótulo de texto visível, cada um com `title` **e** `aria-label`.
- **CA-02.2** Ícones previstos: engrenagem (Configurações), balão/legenda
  (Transcrição), documento (Resumo), faísca (Sugestões), relógio (Histórico),
  olho/olho-cortado (stealth on/off), chevron (ocultar) e a alça de arraste.
- **CA-02.3** O ícone de uma janela **aberta** fica em estado destacado (cor de
  "ligado"), como os toggles atuais.
- **CA-02.4** O seletor de **canais** passa a um **botão com popover**
  (Ambos / Microfone / Sistema, com **Nova sessão**), com rótulo de acessibilidade;
  a troca ao vivo continua permitida (BR-019).

### US-03 — Painel da sessão em abas

- **CA-03.1** A janela `chat` passa a ser o **painel da sessão**, com uma *pill*
  de abas segmentada: **Transcrição | Resumo** (ícones + rótulo).
- **CA-03.2** A janela segue **sem moldura**, arrastável pelo cabeçalho,
  redimensionável (mín. 360×320), sempre no topo, `skipTaskbar`, com posição e
  tamanho lembrados (`Settings.chat_position`/`chat_size`, BR-026).
- **CA-03.3** A **aba ativa é lembrada** entre aberturas/reinícios
  (`Settings.panel_tab`).
- **CA-03.4** A janela `summary` **deixa de existir**; o Resumo é a **aba ao lado**
  (não há botão "Resumo" duplicado no cabeçalho da Transcrição — revisão de
  2026-10-19).
- **CA-03.5** A aba reservada **"Sessão"** não é criada nesta PRD — o desenho
  prevê o encaixe, sem implementá-lo.
- **CA-03.6** Trocar de aba **não** reinicia transcrição, resumo nem rolagem já
  carregada.

### US-04 — Aba Transcrição (conversa)

- **CA-04.1** A aba contém o que era a janela `chat`: medidores de nível, um
  cabeçalho de ações com **apenas o atalho Sugestões** e a conversa em bolhas
  (`ChatView`), com prévia "em andamento" no fim (BR-021).
- **CA-04.2** O título da aba e o rótulo do produto passam a ser **Transcrição**
  (o termo "Chat" sai da UI, mantendo o label técnico da janela).
- **CA-04.3** A **barra inferior** da aba contém o **seletor de idioma** (US-06) e
  **não** contém play/pause/stop (o transporte vive no notch) — desvio consciente
  do Perssua (BR-015/R12).
- **CA-04.4** A tradução sob demanda na bolha (BR-030) continua funcionando.

> **Revisão (2026-10-19):** o **Sugerir** saiu do chat — não se gera mais uma bolha
> de sugestão ao vivo ali. A aba **Sugestões** (janela `suggestions`) é o lugar da
> feature, a ser aprimorada depois. O que já estiver **persistido** (ex.: "Fixar")
> continua no histórico e no export (BR-020).

### US-05 — Aba Resumo

- **CA-05.1** A aba contém o que era a janela `summary` (BR-023): Markdown
  renderizado (BR-025), **Auto / Atualizar / Copiar** e o histórico de versões.
- **CA-05.2** O **automático** do resumo roda enquanto a **aba Resumo estiver
  ativa** (janela aberta e aba selecionada); trocar para a Transcrição **pausa** as
  chamadas. **Atualizar agora** funciona em qualquer aba. _(Revisa a BR-026, que
  exigia só a janela aberta.)_
- **CA-05.3** A aba mantém o rodapé com a hora da última atualização e o acesso ao
  histórico de versões (congelar/voltar ao atual).
- **CA-05.4** Sem chave configurada, mostra o erro amigável com atalho para as
  Configurações (como hoje).
- **CA-05.5** Enquanto o resumo é gerado, um **pill de progresso** fica visível:
  "Gerando resumo completo…" durante a chamada e "Processando o restante do
  resumo…" quando ainda há trechos pendentes — para não confundir o título/primeiro
  trecho com o resumo inteiro. Quando o resumo tem `subject`, ele aparece como
  **título** acima do Markdown; o rodapé indica **"Resumo completo"** (ou **"Resumo
  parcial"** enquanto processa).

### US-06 — Seletor de idioma na barra inferior (troca em tempo real)

- **CA-06.1** A barra inferior da aba **Transcrição** tem um seletor de **idioma
  da transcrição (origem)** com ícone + rótulo; as opções são as de
  `source_language` (incluindo **Auto**).
- **CA-06.2** Trocar o idioma aplica ao **próximo passe de decodificação** da
  sessão **em andamento**: sem reiniciar a sessão, sem recarregar o modelo e sem
  perder a linha temporal (BR-034).
- **CA-06.3** O idioma escolhido é **persistido** em `settings.source_language`; a
  aba *Transcrição & Áudio* das Configurações reflete a mudança e continua sendo o
  controle completo.
- **CA-06.4** Com a sessão **pausada** ou sem sessão, a escolha é salva e vale ao
  (re)captar.
- **CA-06.5** O seletor muda **apenas** o idioma de origem; o idioma de **destino**
  da tradução permanece só nas Configurações.

### US-07 — Ação primária verde com o atalho

- **CA-07.1** A ação de iniciar é um botão **verde** (não azul) com o rótulo
  **"Começar a ouvir"** (substitui "Iniciar escuta").
- **CA-07.2** Ao lado do rótulo, o **atalho atual** de iniciar/encerrar sessão
  (`toggle_session`, hoje `Ctrl+Shift+S`) aparece em **badges** (ex.: `Ctrl`
  `Shift` `S`).
- **CA-07.3** Os badges **derivam do binding persistido** — mudar o atalho nas
  Configurações muda o que aparece.
- **CA-07.4** O verde é a cor da **ação primária**; azul continua reservado a
  estados "ligado/ativo" de toggles (não copiar a paleta do Perssua — R12).

### US-08 — Privacidade, estados e acessibilidade

- **CA-08.1** Nenhum backend, login ou telemetria (BR-002, BR-004); o áudio segue
  local (BR-005).
- **CA-08.2** O click-through é preservado: só elementos `data-interactive`
  capturam o mouse; o resto atravessa
  ([`../mvp/architecture/overlay.md`](../mvp/architecture/overlay.md) §3).
- **CA-08.3** Todo botão de ícone tem `aria-label` e estado de foco visível;
  `prefers-reduced-motion` continua respeitado.
- **CA-08.4** Pausar, retomar e ocultar **não** mudam a aba ativa nem o idioma
  escolhido.

### US-09 — Identidade visual "Obsidian HUD"

Como usuário, quero uma interface coerente e legível, com cara de HUD discreto.

- **CA-09.1** A UI adota os tokens de
  [`architecture/design-tokens.md`](architecture/design-tokens.md) (cor, tipografia,
  raio, espaço, vidro, sombra/glow); não há hex/rgba solto fora de `tokens.css`.
- **CA-09.2** **Geist** (UI) e **JetBrains Mono** (tempos, atalhos e código) são
  empacotadas **localmente**, com fallback e sem requisições de rede.
- **CA-09.3** O vidro (`backdrop-filter`) existe **só nas cascas** (notch, painel,
  popovers), nunca por bolha/item (ISS-020); sem suporte, a superfície cai para
  opaca.
- **CA-09.4** As cores de sinal têm papéis fixos: **cyan** = foco/live,
  **violeta** = IA, **esmeralda** = gravação/ação primária, **coral** = erro/parar;
  "pausado" usa o *outline* (sem âmbar).
- **CA-09.5** Contraste, foco visível, `prefers-reduced-motion` e `forced-colors` são
  respeitados.
- **Detalhes:** [`tasks/07-task.md`](tasks/07-task.md).

### US-10 — Notch ativo (switcher, waveform e play/pause)

Como usuário, quero que o notch, durante a sessão, mostre o modo ativo e o áudio
sendo captado de forma imediata.

- **CA-10.1** O notch ativo segue a ordem do protótipo: alça · marca (dot cyan +
  nome) · **switcher** (Transcrição + waveform · play/pause · Histórico · Resumo ·
  Sugestões) · **timer** · canais · stealth · ocultar · **configurações (engrenagem,
  último botão)**.
- **CA-10.2** **Transcrição** fica **destacada** quando gravando/pausada (ou com o
  painel na aba Transcrição); as inativas usam o tom mutado.
- **CA-10.3** Um **waveform** de 4 barras ao lado de "Transcrição" reage ao
  `audio_level` **sem re-renderizar a barra** (refs/CSS vars); pausado → estático e
  esmaecido; sem áudio → baixo.
- **CA-10.4** O botão **play/pause** muda conforme o contexto: gravando → `pause`;
  pausado → `play`; clique pausa/retoma. Encerrar continua só no timer (BR-022).
- **CA-10.5** O timer usa **`HH:MM:SS`** em mono/tabular; clique abre a confirmação
  "Encerrar sessão? Sim / Não".
- **CA-10.6** Clicar **Transcrição**/**Resumo** abre/foca o painel na aba
  correspondente; **Histórico** e **Sugestões** mostram/escondem as respectivas
  janelas.
- **CA-10.7** Canais (popover com Ambos/Mic/Sistema + Nova sessão), Sugestões (no
  switcher), stealth e ocultar são preservados; a **engrenagem** é o **último botão
  redondo da direita** e abre as Configurações.
- **CA-10.8** O estado **repouso** mantém a estrutura atual (CA-01.1) — **sem**
  switcher/waveform/timer — com a engrenagem como último botão; só o re-skin.
- **Detalhes:** [`tasks/08-task.md`](tasks/08-task.md).

## 5. Requisitos

### Funcionais (RF)

- **RF-01** Notch em dois estados: **repouso** (sem sessão) e **ativo** (com
  sessão).
- **RF-02** Botões de ação como ícones com `title`/`aria-label`.
- **RF-03** Janela `chat` em abas (**Transcrição | Resumo**) com aba ativa
  persistida.
- **RF-04** Aposentar a janela `summary` (apontar o botão "Resumo" para a aba).
- **RF-05** Seletor de **idioma de origem** na barra inferior da aba Transcrição.
- **RF-06** Troca de idioma **em tempo real** no ASR (próximo passe).
- **RF-07** Engrenagem abre as Configurações.
- **RF-08** Ação primária verde com **badge do atalho**.

### Não funcionais (RNF)

- **RNF-01 — Testabilidade (BR-013):** funções puras para *badges* de atalho e
  estado de abas; componentes com `@testing-library/react`.
- **RNF-02 — Desempenho:** sem regressão — a conversa segue memoizada
  (`ChatView`), os ícones são leves e não há re-render por evento de nível na
  barra (o notch não assina `audio_level`).
- **RNF-03 — Paridade (BR-015):** geometria/hierarquia do Perssua; desvios
  conscientes documentados em [`architecture/notch-ui.md`](architecture/notch-ui.md) §6.
- **RNF-04 — Acessibilidade:** `aria-label`, foco visível e contraste adequado em
  todos os ícones.
- **RNF-05 — Sem dependência nova:** ícones SVG desenhados no projeto (BR-008);
  nenhuma lib de UI/ícones adicionada.
- **RNF-06 — Sem mudança de schema:** banco intacto; apenas `Settings` ganha/retira
  campos de layout (sem segredos — BR-003).
- **RNF-07 — Documentação única (BR-016):** IDs `BR-032`…`BR-034`, `ISS-033`,
  `NN-task`; sem duplicar conteúdo já documentado.

## 6. Não-objetivos

- A aba **"Sessão"** (fica para uma PRD futura; o desenho apenas a reserva).
- **play/pause/stop dentro do painel** (o transporte permanece no notch).
- Seletor do idioma de **destino** na barra inferior.
- Captura de tela/OCR, tradução automática em massa, tema claro/temas.
- Reordenar/reposicionar abas por arraste.
- i18n além de PT-BR (BR-010).
- Qualquer mudança no **pipeline de captura de áudio** ou no schema do banco.

## 7. Riscos

| Risco | Mitigação |
|---|---|
| Trocar o idioma no meio de um passe de decodificação | aplica no **próximo** passe; o passe em voo termina no idioma anterior (CA-06.2) |
| Recarregar o modelo ao trocar idioma (custo alto) | idioma sai da **chave do cache** do motor (mesmo padrão já usado para o prompt) |
| Aposentar `summary` quebrar posições salvas | serde ignora campos órfãos; `Settings` remove `summary_*` e é **sanitizado** no load |
| Custo do resumo automático com a janela sempre aberta | automático só com a **aba Resumo ativa** (CA-05.2) |
| Abas + resize em Wayland | mesmo caminho X11/XWayland do overlay (BR-007) |
| Ícones sem rótulo virarem "adivinhação" | `title` + `aria-label` obrigatórios e testados (CA-02.1, RNF-04) |

## 8. Métricas de sucesso

- O notch em repouso exibe **só** os controles da CA-01.1.
- Trocar o idioma reflete na **próxima fala** (atraso ≤ ~1 s de decodificação).
- **Uma** janela para transcrição + resumo; o botão "Resumo" **troca de aba** (não
  abre janela).
- `cargo test`, `cargo clippy`, `cargo fmt --check`, `pnpm test`, `tsc --noEmit`
  e `pnpm build` verdes.

## 9. Evidência no Perssua (BR-015)

Paridade medida no bundle do `Perssua-0.27.5-x86_64.AppImage` (asar em
`/tmp/opencode/perssua/squashfs-root/resources/app.asar`). Os valores já
catalogados em [`../chat/architecture/chat-ui.md`](../chat/architecture/chat-ui.md)
§3/§6 não são repetidos aqui (BR-016).

| Assunto | Âncora | Onde está registrado |
|---|---|---|
| Pill de controle `[play ↔ pause] + [stop]` | `.unified-session-controls`, `.unified-control-button` | `chat-ui.md` §3/§6 |
| Pílula de sessão (tempo) e "End session?" | `.actionbar-session-pill`, `.actionbar-end-*` | `chat-ui.md` §3/§6 |
| Pill de **abas** do notch (conjunto fechado com ícone + rótulo) | *a registrar* | extração na task 01 → `notch-ui.md` §7 |
| Seletor de **idioma** na barra inferior da transcrição | *a registrar* | extração na task 04 → `notch-ui.md` §7 |
| Botão "Start Listening" (cor cheia + rótulo) | *a registrar* | extração na task 02 → `notch-ui.md` §7 |

> Os anchors marcados *a registrar* são extraídos com o REA durante a
> implementação e escritos **uma vez** em
> [`architecture/notch-ui.md`](architecture/notch-ui.md) §7 (R14/BR-016); aqui fica
> só o índice.
