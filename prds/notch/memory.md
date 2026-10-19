# Histórico — PRD Notch

Histórico deste PRD no formato fixo do
[`rules.md`](../../.agents/rules.md) §6.2 (busca por palavra-chave, ID e período).

| Objetivo | Comando |
|----------|---------|
| Palavra-chave | `rg -n -i 'notch\|abas\|idioma' prds/notch/memory.md` |
| Entrada do dia | `rg -n '^## 2026-10-18' prds/notch/memory.md` |
| Por ID | `rg -n 'BR-032\|BR-033\|BR-034\|ISS-033' prds/notch/memory.md` |
| Idioma em tempo real | `rg -n -i 'idioma em tempo real\|update_language' prds/notch/memory.md` |
| Painel em abas | `rg -n -i 'abas\|summary aposentado' prds/notch/memory.md` |

---

## 2026-10-18 — Kickoff: PRD Notch (limpeza do overlay, abas e idioma em tempo real)

- **IDs:** BR-032, BR-033, BR-034, ISS-033, ISS-034, T01–T06
- **Refs:** [`prd.md`](prd.md), [`architecture/notch-ui.md`](architecture/notch-ui.md),
  [`issues.md`](issues.md), [`tasks/_tasks.md`](tasks/_tasks.md),
  [`../chat/prd.md`](../chat/prd.md) §US-06/US-07,
  [`../assistant/prd.md`](../assistant/prd.md) BR-023/BR-026

### Contexto
Pedido do usuário: criar uma PRD nova de **UX e layout** inspirada no Perssua
(BR-015), com três frentes: (1) **notch limpo** quando não há sessão ativa — só
engrenagem e uma ação **verde** "Começar a ouvir" com o atalho em badges; (2) mais
**ícones** em vez de texto; (3) unificar a conversa (renomeada **Transcrição**) e o
**Resumo** em **abas** de uma janela redimensionável; e (4) um **seletor de idioma**
na **barra inferior** da Transcrição, que muda a transcrição **em tempo real**.

### Conflitos com o trabalho em andamento (resolvidos antes de escrever)
Ao revisar as *working changes* (PRD chat, **tasks 07/08/09 `CONCLUDED`**), vimos que
a task 07 **acabou de criar a janela `chat`** dedicada. Para não desfazer trabalho:

- **Decisão:** **manter o label `chat`** e **estendê-lo** com as abas
  (Transcrição \| Resumo); **aposentar apenas `summary`**. Assim não há migração de
  `chat_position`/`chat_size` nem reversão da T07.
- O botão "Resumo" do `ChatHeaderActions` passa a **trocar de aba**; "Sugestões"
  segue abrindo a janela própria.
- **Sem conflito** com o novo `ActionBar`/`Surface` (`actionbar|hidden`): o notch em
  dois estados é estado da barra, não uma nova superfície — os ícones constroem em
  cima do que a T07/T09 deixaram.

### Decisões de escopo (respostas do usuário)
- Abas: **Transcrição + Resumo** apenas (Sugestões e Histórico seguem janelas
  próprias); aba **"Sessão" reservada** para PRD futura.
- Atalho do botão verde: **reusar o atual** (`toggle_session`, hoje `Ctrl+Shift+S`),
  em badges — sem criar atalho novo.
- Seletor da barra inferior: **idioma de origem** da transcrição (com **Auto**); o
  destino da tradução permanece só nas Configurações.
- Cor da ação primária: **verde** (o Perssua usa azul cheio) — desvio consciente para
  não copiar a paleta.

### Viabilidade confirmada (código lido)
- `WhisperSpeechToText` já guarda o **prompt** num `Mutex` com `update_prompt()`
  (troca sem recarregar o modelo); o **idioma** é campo imutável e a fábrica cacheia
  por `(model, language, gpu)`. A correção (T04) espelha o prompt: `language` vira
  `Mutex<String>`, sai da **chave do cache** e é propagado por `update_language`
  + novo IPC `session_set_language` (ISS-033).

### Entregue neste dia
- PRD, arquitetura (`notch-ui.md`), issues (`ISS-033`/`ISS-034`) e **6 tasks**.
- **BR-032** (notch em dois estados), **BR-033** (painel em abas; `summary`
  aposentado; resumo auto só na aba ativa) e **BR-034** (idioma em tempo real), com
  revisões de BR-021/BR-023/BR-026.

### Pendências / próximos passos
- Revisão da PRD pelo usuário; **execução só depois** do aval.
- Extrair no bundle do Perssua os anchors de: pill de abas, botão "Start Listening"
  e seletor de idioma (tasks 01/02/04 → `notch-ui.md` §7).
- T01 base visual → T02 notch → T03 abas → T04 idioma → T05 acabamento → T06 docs.

---

## 2026-10-18 — Implementação: notch por ícones, painel em abas e idioma em tempo real

- **IDs:** BR-032, BR-033, BR-034, ISS-033, ISS-034, T01–T06
- **Refs:** [`tasks/_tasks.md`](tasks/_tasks.md),
  [`architecture/notch-ui.md`](architecture/notch-ui.md),
  [`issues.md`](issues.md)

### Entregue (tasks 01–06)
- **T01** — `components/icons.tsx` (Gear/Transcript/Summary/History/Eye/EyeOff/
  ChevronDown/Grip/Language; `SparkleIcon` reexportado), `IconButton` (com
  `aria-label` obrigatório) e `ShortcutBadges`; `parseAccelerator`/
  `acceleratorBadges` puros; token `--ghostwhisper-primary` (verde).
- **T02** — `ActionBar` em **dois estados** (`data-state=idle|active`): repouso
  (arraste, marca, engrenagem, **"Começar a ouvir"** verde + badges, ocultar) e
  ativo (status, transporte, canais, pílula, ícones Transcrição/Sugestões/
  Histórico, stealth, engrenagem, ocultar). `Overlay` passa o `startAccelerator`.
- **T03** — a janela `chat` virou o **painel da sessão** com as abas
  **Transcrição | Resumo**: `panel/{PanelTabs,PanelWindow,TranscriptionView,
  SummaryView}`. A janela **`summary` foi aposentada** (fora do `tauri.conf.json`,
  `capabilities`, `PANEL_LABELS`, `App.tsx`, `panels.ts`; `Settings` troca
  `summary_*` por `panel_tab`). O botão "Resumo" do cabeçalho **troca de aba**; o
  tray "Abrir resumo" abre `chat` e emite o evento `panel_tab`. O automático do
  resumo só roda com a **aba Resumo ativa** (revisa a BR-026).
- **T04** — idioma **em tempo real**: `language` em `Mutex` no
  `WhisperSpeechToText`, idioma **fora da chave do cache** da fábrica e
  `update_language` (motor em cache, sem recarregar), `SessionController::
  set_language`, IPC `session_set_language` e a `LanguageBar` na barra inferior da
  aba Transcrição.
- **T05** — nomes: título/aba **Transcrição**, tray "Abrir transcrição", README
  reescrito; acessibilidade (`aria-label`, foco visível, `role=tablist`).
- **T06** — BR-032/033/034 + revisões de BR-021/023/026, esta entrada, `issues.md`
  (`ISS-033`/`ISS-034` → `FIXED`) e referências cruzadas em
  `chat/architecture/chat-ui.md` §7.1 e `assistant/architecture/assistant-windows.md` §9.

### Verificação
- **Rust 334 testes** (8 `ignored`) · `cargo clippy --all-targets` e
  `cargo fmt --check` limpos.
- **Frontend 155 testes** · `tsc --noEmit` e `pnpm build` limpos.
- Pacotes: **AppImage com Vulkan** (`pnpm tauri:build`) — tamanhos no fecho da T06.

### Notas de conflito (resolvidas)
- As *working changes* do PRD chat (T07–T09) acabaram de criar a janela `chat`;
  para não desfazer trabalho, **mantivemos o label `chat`** e o estendemos com
  abas, aposentando só `summary` (sem migração de `chat_position`/`chat_size`).

### Pendências
- **Teste manual**: notch em repouso × ativo, alternância de abas, idioma em tempo
  real (exige áudio) e a aba Resumo.
- Extrair no bundle do Perssua os anchors de pill de abas, "Start Listening" e
  seletor de idioma → [`architecture/notch-ui.md`](architecture/notch-ui.md) §7.

---

## 2026-10-19 — Ajustes: Histórico sempre visível e chat só transcrição

- **IDs:** BR-021 (revisão), BR-032, BR-033
- **Refs:** [`prd.md`](prd.md) §4 (US-03/US-04),
  [`architecture/notch-ui.md`](architecture/notch-ui.md) §2,
  [`tasks/02-task.md`](tasks/02-task.md), [`tasks/03-task.md`](tasks/03-task.md)

### Pedido do usuário
1. Deixar o botão do **Histórico sempre visível** (não só em sessão), para acessar
   o histórico sem passar por uma sessão.
2. Remover o botão **Resumo duplicado** dentro da aba Transcrição (a aba Resumo já
   cumpre esse papel).
3. **Eliminar o "Sugerir" no chat** (que gerava uma nova bolha): a feature será
   revista na aba/janela **Sugestões** mais adiante.

### Entregue
- `ActionBar`: o ícone de **Histórico** saiu do bloco "ativo" e foi para o grupo fixo
  à direita (junto de engrenagem/ocultar) — visível em **repouso e em sessão**.
- `ChatHeaderActions`: sobrou **só** o atalho **Sugestões**; saíram "Resumo" e
  "Sugerir" (e a prop `onOpenSummary`).
- `TranscriptionView`: sem `useInsight`; a linha temporal usa
  `buildChatTimeline(segments, [])` — a aba é **só falas**. Sugestões já
  persistidas ("Fixar") continuam no histórico e no export.
- Docs: PRD (CA-03.4/CA-04.1 + nota de revisão), BR-021/BR-032/BR-033, README,
  arquitetura e tasks 02/03.

### Verificação
- `pnpm test` **154 ✓** · `tsc --noEmit` ✓ · `pnpm build` ✓.
- Rust intocado nesta rodada (334 ✓ na rodada anterior).
- AppImage (Vulkan) reconstruído com estas mudanças.

### Pendências
- Aprimorar a superfície de **Sugestões** (feature própria); o chat não gera mais
  sugestões nem bolhas da IA ao vivo.

---

## 2026-10-19 — Resumo: pill de progresso e título visível

- **IDs:** BR-023 · **Refs:** [`prd.md`](prd.md) §4 (US-05),
  [`tasks/03-task.md`](tasks/03-task.md), `src/features/panel/SummaryView.tsx`

### Pedido do usuário
O **título** do resumo aparece rápido e dá um panorama, mas durante a geração o
usuário confundia a primeira frase com o resumo inteiro. Pediu um **pill**
("Gerando resumo completo").

### Entregue
- `SummaryView`: pill com spinner e `role="status"`:
  - **`loading`** → "Gerando resumo completo…";
  - **`hasMore`** (há trechos pendentes, sem chamada em voo) → "Processando o
    restante do resumo…".
- O **`subject`** do resumo aparece como **título** acima do Markdown (quando
  existe), separando "panorama" de "conteúdo".
- O rodapé deixou de repetir "processando o restante…" (agora é o pill).
- CSS: `.summary-view__status`, spinner com `prefers-reduced-motion`.

### Verificação
- `pnpm test` **157 ✓** (+3 de `SummaryView.test.tsx`) · `tsc --noEmit` ✓ ·
  `pnpm build` ✓ · `cargo test` 343 ✓ (fix de resposta vazia do provider).
- AppImage (Vulkan) reconstruído com estas mudanças.

---

## 2026-10-19 — Design system "Obsidian HUD" (task 07 especificada)

- **IDs:** BR-035 (revisa BR-032) · **Refs:**
  [`architecture/design-tokens.md`](architecture/design-tokens.md),
  [`tasks/07-task.md`](tasks/07-task.md), [`prd.md`](prd.md) §4 (US-09)

### Pedido do usuário
Padronizar a UI com o design **Obsidian HUD** (tokens fornecidos: cores ink,
vidro, cyan/violeta/esmeralda/coral, Geist + JetBrains Mono, raios pill e espaço
de 4px).

### Entregue (documentação, sem código ainda)
- [`architecture/design-tokens.md`](architecture/design-tokens.md): **fonte única**
  dos tokens (cores, tipografia, raio, espaço, vidro, borda hairline, glow),
  aliases semânticos, mapa de componentes, acessibilidade/desempenho e as
  divergências prosa × YAML resolvidas.
- [`tasks/07-task.md`](tasks/07-task.md): escopo, entregáveis, critérios de aceite,
  roteiro e riscos (status `PENDING`).
- PRD ganhou **US-09**; **BR-035** criado; **BR-032** revisado (verde → esmeralda);
  `_tasks.md` com a 07.

### Decisões
- Onde a prosa e o YAML divergem, **vale o token** (superfícies ink).
- **Vidro só nas cascas** (notch/painel/popovers); bolhas/itens nunca com blur
  (ISS-020); leitura em superfície quase opaca.
- Ação primária **esmeralda** (`#10b981`); "pausado" sem âmbar (usa outline).
- Fontes **Geist** e **JetBrains Mono** empacotadas localmente (OFL), sem CDN.

### Pendências
- Executar a **task 07** (aplicar tokens e re-skin por camada, com revisão visual).

---

## 2026-10-19 — Notch ativo: plano (task 08)

- **IDs:** BR-032 (revisão), BR-035 · **Refs:**
  [`tasks/08-task.md`](tasks/08-task.md),
  [`architecture/notch-ui.md`](architecture/notch-ui.md) §2.1,
  [`architecture/design-tokens.md`](architecture/design-tokens.md) §6,
  [`prd.md`](prd.md) US-10

### Pedido do usuário
Aplicar o design ao notch; ao iniciar a transcrição, a aba fica **destacada** e o
**waveform** ao lado anima; **play/pause contextual**; o estado de **repouso**
continua parecido com o de hoje. O HTML do protótipo é referência, não cópia.

### Análise do protótipo
- **Adotar:** switcher **Transcrição / Histórico / Resumo**; waveform ao lado da aba
  ativa; botão circular de play/pause; timer mono `HH:MM:SS` com dot esmeralda.
- **Mapear:** `mic_off` → **Canais** (popover Ambos/Mic/Sistema + Nova sessão);
  `close_fullscreen` → **Ocultar**.
- **Preservar:** Sugestões, stealth, engrenagem e Histórico. **Correção:** o
  protótipo omitiu a engrenagem; ela entra como **último botão redondo da direita**
  (depois de ocultar).
- **Não adotar:** Tailwind/Material Icons/Google Fonts via CDN — mantemos CSS
  tokens, SVG próprios e fontes locais (BR-008/BR-035).
- **Fora:** OCR/contexto de tela e captura de área.

### Entregue (documentação, sem código)
- [`tasks/08-task.md`](tasks/08-task.md) — `PENDING`, depende da 07, com estrutura,
  comportamento, componentes, critérios e riscos.
- Arquitetura: [`notch-ui.md`](architecture/notch-ui.md) §2/§2.1 reescrito;
  [`design-tokens.md`](architecture/design-tokens.md) §6 com switcher/waveform.
- PRD: **US-10** e revisões de CA-01.3/CA-02.4; **BR-032** revisada;
  `_tasks.md` com a 08 (07 → 08).

### Decisões
- O notch é **navegação**: `panel_tab` continua a fonte da aba do painel;
  `panel_open` ganha `tab` opcional e emite `panel_tab`.
- Waveform dirigido pelo `audio_level` (máx. dos canais) por **refs/CSS vars**, sem
  re-render; fallback CSS-only se o custo aparecer.
- Timer em `HH:MM:SS`; **encerrar** continua só no timer (BR-022).
- Repouso mantém a estrutura atual (só re-skin da 07).

### Pendências
- Executar **07 → 08**; validar visual em repouso/gravando/pausado.

---

## 2026-10-19 — Design system e notch ativo implementados (07/08)

- **IDs:** BR-032, BR-033, BR-035 · **Refs:**
  [`tasks/07-task.md`](tasks/07-task.md), [`tasks/08-task.md`](tasks/08-task.md),
  [`architecture/design-tokens.md`](architecture/design-tokens.md),
  [`architecture/notch-ui.md`](architecture/notch-ui.md)

### Entregue
- **T07** — `src/styles/tokens.css` (tokens `--oh-*`, `@font-face` e utilitários),
  fontes **locais** Geist/JetBrains Mono + licenças OFL; `globals.css` re-skin
  (vidro no notch, painel de leitura, esmeralda/cyan/violeta/coral, pill, borda
  hairline, scrollbars, fallback opaco sem blur).
- **T08** — `NotchTabs`, `NotchWaveform`, `ChannelMenu` e `ActionBar` recomposta;
  timer `HH:MM:SS`; `formatClockLong`/`waveformLevel`; `panel_open` com `tab` +
  `panelOpenTab`; tray via `open_panel_tab`; `SessionControls` removido.
- A **engrenagem é o último** botão redondo; Histórico no repouso e no switcher
  ativo; Sugestões/stealth preservados.

### Verificação
- Frontend **155 ✓** · `tsc --noEmit` ✓ · `pnpm build` ✓ (fontes em `dist/assets`).
- Rust **343 ✓** · `cargo clippy --all-targets` e `cargo fmt --check` limpos.
- AppImage (Vulkan) reconstruído.

### Pendências
- Revisão visual do usuário (notch repouso/gravando/pausado e waveform com áudio
  real); ajustes finos de contraste/espaçamento se necessário.
