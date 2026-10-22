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
- O rodapé passou a indicar **"Resumo completo"** (ou **"Resumo parcial"** enquanto
  há trechos pendentes) + hora da atualização, em vez de repetir "processando o
  restante…".
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

---

## 2026-10-19 — Sugestões movida para o switcher (ao lado de Resumo)

- **IDs:** BR-032, CA-10.6/CA-10.7 · **Refs:**
  `src/features/overlay/NotchTabs.tsx`, `src/features/overlay/ActionBar.tsx`,
  `src/features/overlay/ActionBar.test.tsx`

### Contexto
Feedback visual: o atalho **Sugestões** não deve ser um ícone redondo solto na
barra; ele pertence ao **switcher**, ao lado de **Resumo**, como no protótipo.

### Entregue
- `NotchTabs` ganhou o item **Sugestões** (faísca + rótulo), com `aria-pressed` e
  destaque `--active` enquanto a janela está aberta; o switcher agora tem quatro
  itens + transporte.
- `ActionBar` só passa `suggestionsOpen`/`onToggleSuggestions`; o `IconButton`
  solto foi removido. A janela continua **só no contexto de sessão iniciada**,
  porque o switcher só existe no estado ativo.
- `SparkIcon` aceita `size` (default 12) para alinhar aos ícones de 15px do
  switcher.
- Layout preservado: a barra de 900px coube sem wrap (~26px de folga, medido em
  Chrome headless); a engrenagem continua o último botão.

### Verificação
- `pnpm test` **162 ✓** (2 novos em `ActionBar.test.tsx`: só com sessão + destaque)
  · `tsc` ✓ · `pnpm build` ✓ · screenshot headless 900×72 conferido.
- AppImage (Vulkan) reconstruído.

---

## 2026-10-19 — Drill-hover (dwell) no notch (tasks 09–12)

- **IDs:** BR-036, CA-11.1…11.8, ISS-035 · **Refs:**
  [`architecture/drill-hover.md`](architecture/drill-hover.md),
  [`tasks/09-task.md`](tasks/09-task.md)…[`12-task.md`](tasks/12-task.md)

### Contexto
Pedido do usuário: acionar **Resumo** e **Sugestões** sem clique (ajuda no
stealth), com fill da esquerda para a direita, confirmação com segundo dwell e
duração configurável (0,5 s padrão). O protótipo Stich AI foi referência de
comportamento, **não** de implementação (Tailwind/Material/som ficaram de fora).

### Entregue
- **Settings:** `dwell_enabled` (padrão ligado) e `dwell_ms` (padrão 500, faixa
  300–2000), sanitize/validate, UI em **Ajustes**; `settings_changed` faz o overlay
  recarregar as settings ao salvar.
- **Núcleo:** `domain/dwell.ts` (clamp/rótulo), `useDwellTimer`, `DwellButton`,
  `DwellPopover`, `DwellProvider` e a janela **`dwell`** (260×60, transparente, sem
  foco/taskbar). Fill **CSS-only** (`data-arming` + `--dwell-ms`).
- **Rust:** porta `DwellPopoverController`, adaptador
  `TauriDwellPopover`, posicionamento puro `dwell_popover_position` (acima se
  couber, senão abaixo; clamp na work area), IPC
  `dwell_show/hide/context/result` e `dwell` no `AppState`.
- **Notch:** só **Resumo/Sugestões** usam `DwellButton`; **clique executa na hora**;
  hover abre a confirmação; "Sim" tem dwell e clique; **sem som/toast**.
- **Grace:** sair do gatilho **e** da confirmação inicia fechamento após `dwell_ms`;
  voltar a qualquer uma das áreas cancela; sobre o "Sim" o dwell continua. O overlay
  oculto (atalho/tray/superfície) fecha a confirmação via `dwell_closed`.
- **Janela própria** (não expansão): um `position: absolute` não sai da janela
  nativa de 72 px; a janela separada flutua sem mexer no notch (ISS-035).

### Verificação
- Frontend **181 ✓** (dwell/domínio/timer/botão/popover/action bar/settings) ·
  `tsc --noEmit` ✓ · `pnpm build` ✓.
- Rust **351 ✓** · `cargo clippy --all-targets` e `cargo fmt --check` limpos.
- Screenshot headless da janela `dwell` (260×60) conferido.
- AppImage (Vulkan) reconstruído ao final.

### Pendências
- Revisão manual no app: acima/abaixo perto das bordas, multi-monitor/DPI,
  click-through e stealth (X11/Wayland), foco do popover e o grace entre janelas.

---

## 2026-10-19 — Correção: grace do drill-hover pausa na confirmação

- **IDs:** BR-036, CA-11.5 · **Refs:** `src/features/dwell/DwellPopoverWindow.tsx`,
  `src/components/dwell/DwellProvider.tsx`, `src-tauri/src/ipc/dwell.rs`

### Sintoma
Ao mover o cursor de **Resumo** para o **Sim**, a confirmação desaparecia antes
de completar o segundo dwell.

### Causa
O timer de dispensa ficava no overlay e dependia de um evento (`dwell_pointer`)
saindo da janela `dwell`. Na travessia entre as duas janelas, o `pointerleave` do
gatilho iniciava o grace e o `pointerenter` da confirmação podia chegar depois —
ou o evento se perdia — fechando o popover antes do "Sim".

### Correção
- O grace agora vive **na janela de confirmação**: `pointerenter` **e
  `pointermove`** na janela **pausam/limpam** o timer; ele só corre quando o
  cursor está fora do gatilho **e** da confirmação.
- O overlay informa o hover do gatilho pelo comando `dwell_trigger_hover`; o core
  reemite `dwell_trigger` para a janela `dwell` (relay confiável, sem evento
  cross-window saindo da janela de confirmação).
- Removido o evento `dwell_pointer` e o grace do `DwellProvider`.

### Verificação
- Frontend **189 ✓** (novos testes: não fecha ao entrar na confirmação; movimento
  dentro também pausa; fecha depois do grace fora; voltar ao gatilho cancela; poll
  do cursor na segunda abertura; matemática física→CSS) · `tsc` ✓ · `cargo test`
  351 ✓ · clippy/fmt limpos.
- AppImage (Vulkan) reconstruído com a correção.

### Correção 2 (raiz do “só na primeira vez”)
- **Causa raiz:** ao esconder a janela de confirmação com o cursor dentro dela, o
  webview pode manter o estado de hover; na reabertura ele não reemite
  `pointerenter` e o grace fecha o popover antes do "Sim".
- **Correção:** a janela passou a **amostrar a posição global do cursor** a cada
  ~90 ms. O core envia a **geometria real** da janela no `dwell_context`
  (posição/tamanho/scale após `show`), então a amostra é usada como fonte de
  verdade para pausar o grace e iniciar o segundo dwell. Eventos de ponteiro
  continuam como atalho.
- **Permissão:** apenas `core:window:allow-cursor-position` para a janela `dwell`;
  a geometria vem do core (sem consultar `outerPosition` etc. no webview).
- **Teste de regressão:** abre/fecha/abre e confirma duas vezes sem eventos de
  hover (poll do cursor) · frontend **189 ✓**.
- AppImage (Vulkan) reconstruído com a correção.

---

## 2026-10-19 — Histórico no trailing e ícone de recolher (task 13)

- **IDs:** BR-032 (revisão), CA-10.1/10.6/10.7 · **Refs:**
  [`tasks/13-task.md`](tasks/13-task.md), `src/features/overlay/ActionBar.tsx`,
  `src/features/overlay/NotchTabs.tsx`, `src/components/icons.tsx`

### Entregue
- O item **Histórico** com rótulo saiu do switcher; virou `IconButton` no grupo da
  direita, na ordem **ocultar · Histórico · Configurações** (engrenagem continua a
  última), nos estados ativo e de repouso.
- O botão **ocultar** passou a usar `CollapseIcon` (duas setas diagonais apontando
  para o centro), no lugar do chevron.
- `NotchTabs` não recebe mais `historyOpen`/`onToggleHistory`.

### Verificação
- Frontend **189 ✓** (ordem do trailing nos dois estados, Histórico fora do
  switcher, ícone no catálogo) · `tsc` ✓ · Rust 351 ✓ · clippy/fmt limpos.
- Screenshot headless do notch em repouso conferido.
- AppImage (Vulkan) reconstruído.

---

## 2026-10-20 — Notch não ficava sempre à frente (ISS-043, task 14)

- **IDs:** ISS-043, T14
- **Refs:** [`issues.md`](issues.md),
  [`tasks/14-task.md`](tasks/14-task.md),
  [`../mvp/architecture/overlay.md`](../mvp/architecture/overlay.md) §1

### Relato
*“notch não tá com o recurso de ficar sempre à frente”* — descoberto durante o
teste da aba "Geral & HUD" (BR-062), mas fora do escopo dela.

### Diagnóstico
- `xprop` com o app rodando: o overlay (`0x2200003`, sem título) tinha apenas
  `_NET_WM_STATE_SKIP_PAGER, _NET_WM_STATE_SKIP_TASKBAR`; `chat` e `suggestions`
  tinham `_NET_WM_STATE_ABOVE`.
- O `alwaysOnTop: true` do `tauri.conf.json` é aplicado **na criação**. O
  GTK/Mutter perde o hint quando a janela é mapeada/remodelada (`set_surface`,
  resize, reexibição). Não havia **nenhum** `set_always_on_top` em runtime.
- Os painéis escapavam por serem `show()`n depois; o overlay nasce visível.

### Correção
- `TauriOverlayWindow::apply_stacking()` reafirma `set_always_on_top(true)` e é
  chamado dentro de `apply_stealth()` — que já roda no `show()` e no fim de
  `set_surface()` (junto com skip-taskbar/título).
- `spawn_overlay_stacking_reassertion()` reafirma uma vez ~900 ms após o boot,
  cobrindo o mapeamento inicial.

### Verificação
- Depois do rebuild (AppImage 17:41):
  `_NET_WM_STATE_SKIP_PAGER, _NET_WM_STATE_SKIP_TASKBAR, _NET_WM_STATE_ABOVE`.
- `cargo test --lib` 519 ✓ · clippy/fmt (arquivos tocados) ✓.

### Observação
- O teste matou a instância anterior sem o `RunEvent::Exit`; a marca de crash
  deixou esta execução em CPU (BR-029). Um fechamento normal reabilita a GPU.

---

## 2026-10-20 — Notch parava ~200 px antes do topo (ISS-044, task 15)

- **IDs:** ISS-044, T15
- **Refs:** [`issues.md`](issues.md), [`tasks/15-task.md`](tasks/15-task.md),
  BR-062

### Relato
*“o Notch vai só até um limite no topo, como se tivesse uma div invisível acima
dele”* — depois do ajuste de always-on-top (ISS-043).

### Diagnóstico
- `xwininfo` do overlay: **1040×260** em (440, 8), com a barra (52 px) no
  **rodapé** — ~200 px de janela transparente acima dela.
- `WM_NORMAL_HINTS`: **min=max = 1040×260**. O `overlay` estava com
  `resizable: false`, então o GTK/Mutter travava os hints e o `set_size` do HUD
  (densidade/âncora) era **ignorado na altura**.
- O `.overlay` usava `justify-content: flex-end`; a barra nunca subia junto com o
  topo da janela.

### Correção
- `overlay` vira **resizable** (sem moldura, sem alça) e nasce 1040×84.
- Barra no **topo**: `justify-content: flex-start`, `padding-top: 0`.
- Alturas da janela 72/84/96 (compacto/padrão/expandido); o excedente abaixo é o
  banner do ASR.
- `HUD_MARGIN = 0` e remoção da reancoragem pela base ao redimensionar.

### Verificação
- Janela nova: **1040×96** em **(440, 32)** (densidade `expanded` do usuário),
  `WM_NORMAL_HINTS` sem min/max.
- `XMoveWindow` para `y=0` foi clampado pelo Mutter em **y=32** — o topo da área
  útil (abaixo do painel do GNOME). Não há mais área invisível.
- `_NET_WM_STATE` mantém `_NET_WM_STATE_ABOVE`.
- Rust 519 ✓ · clippy/fmt (tocados) ✓ · AppImage 18:26.

### Observação
- O desktop reporta `_NET_WORKAREA` como tela cheia, mas o Tao entrega
  `work_area.y = 32` (abaixo do painel) — foi o que usamos na âncora.

---

## 2026-10-20 — Notch voltava a uma posição antiga ao salvar settings (ISS-045, task 16)

- **IDs:** ISS-045, T16 · **Refs:** [`issues.md`](issues.md),
  [`tasks/16-task.md`](tasks/16-task.md), BR-062

### Relato
*“toda vez que troco de aba na Sessão, clico em botão, mudo selects, o Notch se
move sozinho, fica pulando e resetando na tela”*.

### Diagnóstico
- `settings_set` recebe um snapshot **completo** de `Settings` de cada janela e
  reaplicava o HUD em todo save. Em `free`, chamava
  `set_position(overlay_position)` com a posição **antiga** do snapshot; em
  âncora fixa, `set_hud_layout` recentralizava.
- O tracker do core gravava a posição real no disco, mas o save seguinte de
  qualquer outra janela regravava o snapshot velho — daí o efeito de "pular"
  entre posições conforme a janela que salvava.

### Correção
- `Settings::preserve_window_geometry` faz posição/tamanho persistidos (dos
  trackers) vencerem o payload em todo save.
- `Settings::hud_layout_changed` só reaplica `set_hud_layout` quando âncora ou
  densidade mudam de fato; `set_stealth` idem.
- Nenhuma janela é movida por save de aba/agente/select.

### Verificação
- `cargo test --lib` **543 ✓** (9 ignorados) · `clippy -D warnings` ✓ ·
  `rustfmt --check` nos arquivos tocados ✓.
- Teste manual (abas, selects e arraste) pendente com o binário novo.

---

## 2026-10-20 — Largura dinâmica do notch + menu de canais (ISS-047, task 17)

- **IDs:** ISS-047, T17 · **Refs:** [`issues.md`](issues.md),
  [`tasks/17-task.md`](tasks/17-task.md), BR-032, BR-062

### Relato
*“O menu do microfone fica por trás do notch e não dá para selecionar; o notch
tem largura fixa e sobra espaço.”*

### Diagnóstico
- O popover abria com `bottom: calc(100% + 10px)` (acima da barra), mas a janela
  vive no topo e só tem a altura da densidade — ficava fora dos limites.
- A largura era `ACTIONBAR_WIDTH = 1040`, independente do conteúdo.

### Correção
- Popover abre **abaixo** (`top: calc(100% + 10px)`); o overlay reserva 180 px
  extras via `overlay_set_popover_extra_height` enquanto ele está aberto.
- `Overlay` mede `.overlay__bar-wrap` com `ResizeObserver` e envia
  `overlay_set_actionbar_width`; o adaptador aplica `set_size` e reancora
  topo-centro/direita. Modo livre preserva o canto superior.
- CSS: barra com `width: max-content`; wrapper mede barra + padding; largura
  inicial de 700 px no `tauri.conf.json`.

### Verificação
- `cargo test --lib` 544 ✓ · `clippy` ✓ · `rustfmt` ✓ ·
  `pnpm test` 350 ✓ · `tsc` ✓ · `pnpm build` ✓.
- No AppImage (X11): idle **675×72**, ativo **758×72**, confirmação de encerrar
  **1058×72**; menu abriu **758×252** abaixo da barra, opções responderam e a
  janela voltou a 72 px ao fechar.

## 2026-10-21 — Dwell do "Sim" perdia o foco ao mover sobre texto/contador (ISS-048, task 18)

- **IDs:** ISS-048, T18, BR-036, R16/R17
- **Refs:** [`issues.md`](issues.md), [`tasks/18-task.md`](tasks/18-task.md),
  [`architecture/drill-hover.md`](architecture/drill-hover.md) §2.1/§8

### Relato
*“A ação sem clique do drill-hover às vezes falha ao passar em cima do botão
de confirmação ‘Sim’; parece se perder ao arrastar para cima de um texto ou
no contador.”*

### Diagnóstico
- O `useDwellTimer.start()` sempre reiniciava o timeout, mesmo já ativo.
- `DwellPopover` armava só no `pointerenter`; reentrâncias e um `leave`
  transitório durante a travessia de ícone/texto/badge podiam zerar o estágio 2.
- Os filhos do botão podiam virar alvo do ponteiro, tornando o hover dependente
  do elemento interno sob o cursor.

### Correção
- `start()` idempotente: chamadas repetidas enquanto o timer está ativo não
  reiniciam a contagem.
- `pointermove` no "Sim" retoma o dwell se um `leave` transitório o parar.
- `.dwell-target > * { pointer-events: none; }`: o botão é o alvo único.
- O teste no app mostrou que o `pointerleave` da **janela** também podia ser
  emitido ao cruzar filhos; botão e janela ganharam debounce de 120 ms com
  checagem de `:hover` + `elementFromPoint` no último ponto — movimento dentro
  cancela a parada; sair de fato encerra.
- **Correção 2 (após teste):** o fallback de `:hover` no poll foi removido. O
  webview mantinha hover obsoleto no "Sim" depois de esconder/mostrar a janela,
  então só manter o cursor no Resumo iniciava o segundo timer. O início do "Sim"
  agora usa exclusivamente a posição global do cursor; `elementFromPoint` fica
  só no debounce de saída.
- Testes cobrem travessia texto→contador, leave transitório, saída real,
  retomada por `pointermove`, timer idempotente e travessia no gatilho.

### Verificação
- `pnpm test` **394 ✓** · `tsc --noEmit` ✓.
- AppImage (21:20) reconstruído; manter o cursor no Resumo abre a confirmação
  mas não inicia o "Sim"; mover para o texto/badge e permanecer confirma no
  tempo configurado.

## 2026-10-21 — Drill-hover simplificado para um estágio (BR-036, task 19)

- **IDs:** BR-036 (revisão), T19 · **Rules:** R16/R17
- **Refs:** [`tasks/19-task.md`](tasks/19-task.md),
  [`architecture/drill-hover.md`](architecture/drill-hover.md),
  [`../business-rules.md`](../business-rules.md) BR-036

### Relato/decisão
O fluxo de dois estágios (preencher → popup "Sem clique" + "Sim" → segundo
timer) continuou instável mesmo após as correções de hover. O usuário pediu
para simplificar: **se o timer do botão principal completar, executa a ação
diretamente**.

### Entregue
- `DwellButton`: timer único; ao completar chama `onActivate`, clique continua
  imediato; saída antes do fim cancela.
- `DwellProvider` enxuto (`enabled` + `durationMs`), sem ação pendente/estado
  entre janelas.
- Removidos popup, janela Tauri `dwell`, segundo timer, porta/adaptador/IPC,
  rota `dwell`, permissão de cursor, posicionamento e testes associados.
- `Settings.dwell_enabled`/`dwell_ms` e o fill CSS do botão permanecem.
- BR-036, US-11, arquitetura, notch-ui e design tokens atualizados.

### Verificação
- Frontend **377 ✓** · `tsc --noEmit` ✓.
- Rust **614 ✓** (10 ignorados) · clippy `-D warnings` ✓.
- AppImage (21:41) validado: hover em Resumo executa ao completar, sem janela
  `dwell`/popup; sair antes não executa; clique imediato.
