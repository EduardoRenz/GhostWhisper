# PRD — Contexto de tela por acessibilidade (AT-SPI) com pill de contexto

- **Versão:** 0.1 (especificado)
- **Data:** 2026-11-01
- **Status:** Especificado — tasks `PENDING`
- **Relacionados:** [`../business-rules.md`](../business-rules.md) (BR-105,
  BR-106), [`memory.md`](memory.md), [`issues.md`](issues.md),
  [`architecture/screen-context.md`](architecture/screen-context.md),
  [`tasks/_tasks.md`](tasks/_tasks.md) · PRDs relacionados:
  [`../ai-sources/prd.md`](../ai-sources/prd.md) (Configurações),
  [`../mvp/prd.md`](../mvp/prd.md) (AskBar, BR-052/BR-055)

---

## 1. Objetivo

Dar ao GhostWhisper uma **segunda fonte de contexto de tela** além do print:
ler a **árvore de acessibilidade** da janela ativa via **AT-SPI2** (D-Bus) e
usar esse texto estruturado como contexto da pergunta — **sem capturar pixels**.

Três entregas juntas:

1. **Porta `ScreenContextSource`** com adaptador **AT-SPI** (`atspi` crate),
   atrás de uma **feature flag** (`Settings.screen_context_enabled`, padrão
   **desligado** — opt-in explícito, BR-105).
2. **Pill de contexto na AskBar** ("pesquisar"): um chip que mostra, em tempo
   real, **qual contexto a barra está enxergando** (app + título da janela, com
   prévia do texto no tooltip) — ou o estado do recurso quando não há contexto
   (BR-106).
3. **Injeção no prompt**: quando a flag está ligada e há contexto fresco, o
   texto extraído entra na pergunta como bloco `<screen_context>`, complementando
   — nunca substituindo — a transcrição e o print.

O print (BR-055) continua existindo como está. AT-SPI é **caminho adicional**,
não substituto: canvas/terminal/explicações visuais continuam sendo território
do screenshot.

## 2. Problema

Hoje a única forma de dar "o que está na tela" ao LLM é o **print anexado**
(`AskBar` → portal `Screenshot`/X11 → `JournalRequestSource::Screenshot`). Isso
tem limites concretos:

- **Imagem é cara e opaca**: exige modelo com visão, gasta tokens por pixel e o
  usuário não tem como ver **o que** o app extraiu da tela antes de perguntar.
- **No Wayland não há "janela ativa"**: `portal_targets.rs` documenta que o
  compositor não expõe nem a janela ativa nem a lista de janelas ao app; o
  print é do "monitor sob o cursor" e o ScreenCast tem fonte fixa. O AT-SPI é
  **D-Bus**, independente de compositor — funciona igual em X11 e Wayland.
- **Sem transparência**: o usuário não sabe se o contexto que o copiloto vai
  usar é a reunião, a aba errada ou nada.

A informação já existe na máquina: o Chrome/Firefox/Electron expõem a árvore
de acessibilidade da página (papéis, texto, URL, títulos) no Accessibility
D-Bus quando um cliente a11y conecta. Um terceiro lê tudo isso **sem nenhuma
permissão** — o que torna o recurso poderoso e, por isso, obrigatoriamente
opt-in e visível (§3.2, §3.3).

## 3. Decisões de produto

### 3.1 Feature flag: `Settings.screen_context_enabled` (padrão desligado)

- Toggle em **Configurações → Geral & HUD**, num card próprio **"Contexto de
  tela"** (junto do card de Privacidade, que já vive lá).
- **Padrão: desligado.** Ninguém acorda com o app lendo a árvore de
  acessibilidade de outros apps. Ao ligar, a UI mostra o aviso de privacidade
  (§3.2).
- O card mostra também uma **linha de status** derivada do adaptador:
  `AT-SPI disponível` / `AT-SPI indisponível (sem D-Bus de acessibilidade)` /
  `Nenhum contexto ainda`. Status é leitura, não configuração.
- Settings antigos (sem o campo) carregam com `serde(default)` → `false`.
  Sem migração de banco.

### 3.2 Privacidade: opt-in, texto, limitado, nunca logado

- Só lê com a flag ligada; desligada, **nenhuma conexão D-Bus de acessibilidade
  é aberta** (o adaptador nem é instanciado — composition root em `lib.rs`).
- A saída é **texto estruturado** (nunca imagem), com orçamento fixo de
  caracteres (default 2 000, ver `architecture/screen-context.md` §4). O texto
  segue as mesmas regras do prompt da sessão: só sai da máquina no corpo da
  pergunta, com ação explícita do usuário (BR-005 estendido ao contexto de
  tela).
- **Nunca em log**: logs do adaptador só podem conter app id, tamanho do texto
  e latência — jamais o conteúdo (regra dos prompts/RNF-11 do suggestion-mode).
- O aviso no card de settings é explícito: "Ligar este recurso permite ao
  GhostWhisper ler o **texto** das janelas ativas de outros programas via o
  sistema de acessibilidade (AT-SPI). Nada é salvo nem enviado sem você
  perguntar."
- Apps que declaram `org.a11y.DeniedAccess` não são lidos (o adaptador respeita
  o resultado do call).

### 3.3 Pill de contexto na AskBar (BR-106)

> **Revisão 2026-11-02 (v6):** app sem a11y (Chrome sem
> `--force-renderer-accessibility`) **não emite** evento de foco. A perda de
> foco do app anterior agenda uma checagem de 300 ms; sem foco novo, o
> contexto anterior é **limpo**. Cliques diretos nas janelas do GW chegam em
> poucos ms e cancelam a checagem (ISS-101).
>
> **Revisão 2026-11-02 (v5):** trocar de **programa/aba** descarta o
> contexto anterior — qualquer foco externo confirmado invalida o cache do
> serviço e o fallback `last_good`, e a próxima leitura é do foco novo. Só o
> foco nas **janelas do GhostWhisper** (clique na AskBar/Sessão/pill)
> preserva o contexto. Foco que não dá para confirmar **limpa** (nunca serve
> a janela antiga) — ISS-100. O debounce do foco foi removido para não
> descartar a última troca.
>
> **Revisão 2026-11-02 (v4):** a **pill inteira** é o checkbox — um clique
> alterna ligar/desligar, sempre (a caixa, o ícone e o texto são o mesmo
> alvo). O hook serializa as gravações (cliques rápidos não invertem o
> resultado) e ignora reloads intermediários. O snapshot do AT-SPI saiu da
> main thread (`#[tauri::command(async)]`) — era a causa do congelamento
> percebido (ISS-097).
>
> **Revisão 2026-11-02 (v3):** desmarcar o checkbox **desliga a leitura**
> (zero processamento: sem consulta no mount/foco, cache limpo no core) —
> antes a pill continuava lendo e mostrando o contexto, o que o usuário não
> queria. O estado novo **Pausado** substitui a exibição. O `checked` do
> hook é otimista (responde na hora; religar lê só depois de gravar a
> preferência).
>
> **Revisão 2026-11-01 (v2):** a pill virou **checkbox** — clicar
> marca/desmarca `screen_context_include` (padrão marcado). A mesma pill
> entrou no **composer da aba Sessão** (entre campo e enviar), com estado
> compartilhado via `settings_changed` (hook único `useScreenContext`).
> Decisão pendente 3 do §14 resolvida — não como popover, mas como toggle;
> popover de prévia segue como evolução (ISS-090).

A AskBar ganha uma **pill** entre o campo e o botão **Capturar Tela**:

```
[ 🔍 ] [ pergunta...                ] [Ctrl][K] │ [pill] │ [📷 Capturar Tela]
```

Estados da pill (todos localizados, R18–R22):

| Estado | Quando | Conteúdo |
|---|---|---|
| **Oculta** | flag desligada | nada (sem ruído para quem não optou) |
| **Pausado** | flag ligada + checkbox desmarcado | `⏸ Contexto desativado` (nada é lido/enviado) |
| **Lendo** | AskBar abriu e o snapshot ainda não voltou | `Lendo contexto…` (sutil, sem spinner) |
| **Ativa** | flag ligada + checkbox marcado + snapshot fresco | `🌐 {app} · {título da janela truncado}` |
| **Sem contexto** | flag ligada + janela ativa sem árvore/sem texto (terminal, canvas, app sem a11y) | `Sem contexto de tela` |
| **Indisponível** | flag ligada + AT-SPI inacessível (sem D-Bus) | `Contexto indisponível` |

- **Tooltip** no estado Ativa mostra a **prévia do texto** (primeiros ~300
  caracteres, claramente marcada como prévia) — é o mecanismo de
  transparência: o usuário vê **o que** o copiloto vai receber antes de
  perguntar.
- **Checkbox:** a **pill inteira** (caixa + ícone + texto) é o alvo
  `role="checkbox"`; **um** clique alterna `screen_context_include` e cada
  clique conta exatamente uma vez (gravações em fila no hook). A opção
  aparece em **todos** os estados visíveis (inclusive Pausado/Sem
  contexto/Indisponível).
- **Desmarcado = sem leitura:** nenhuma consulta ao AT-SPI (mount, foco,
  reabertura); o core descarta o cache e não publica leitura. É o que o
  usuário pediu: "a ideia é limpar e não usar processamento". Ao remarcar, a
  leitura recomeça na hora (depois de gravar a preferência, sem corrida).
- Quando um **print** está anexado, a pill continua mostrando o contexto
  AT-SPI (as duas fontes coexistem); a prévia do print já tem seu próprio
  affordance atual.
- A pill é **consultada no momento em que a AskBar abre** e a cada reabertura
  (foco; nunca com estado velho do mount); não há polling contínuo enquanto a
  barra está parada aberta (custo + priv). Atualização por evento
  `screen_context` quando o snapshot muda (eventos ignorados enquanto
  desmarcado). **Troca de programa/aba invalida o snapshot** e relê; só foco
  nas janelas do GW preserva (ISS-100). Os comandos IPC do contexto são `#[tauri::command(async)]`
  (thread pool) — a leitura do AT-SPI **nunca** segura a main thread
  (ISS-097).

### 3.4 Injeção no prompt

- Com flag ligada e snapshot com idade ≤ 30 s, o texto extraído entra no prompt
  da pergunta como bloco `<screen_context>`, depois de `<meeting_summary>` e
  antes do transcript (`ask_service.rs::build_prompt`), com regra curta no
  `ASK_RULES`: o contexto de tela é **o que o usuário está vendo agora**; em
  conflito com a transcrição, a transcrição é a conversa e a tela é o
  material.
- **Origem no diário não muda**: `search`/`preset`/`screenshot` continuam como
  estão; contexto de tela é um **campo adicional do prompt**, não uma nova
  origem (evolução: persistir `context_attached` no diário fica para depois —
  `issues.md`).
- Se o usuário anexou print **e** a flag está ligada, os dois vão
  (`<screen_context>` + imagem). O LLM decide; o system prompt já trata
  contexto como opcional.

### 3.5 Janela ativa: foco AT-SPI como caminho único (X11 e Wayland)

- **Decisão implementada (revisão em 2026-11-01):** a resolução da janela
  ativa usa o **evento de foco do próprio AT-SPI**
  (`object:state-changed:focused`), que é gerado pelo **toolkit do app** e
  funciona igual em X11 e Wayland — o composeitor não entra na equação. A
  proposta original (X11 via `_NET_ACTIVE_WINDOW`+pid) foi descartada: o
  `ApplicationProxy` do atspi 0.30 **não expõe pid**, tornando o matching por
  pid não viável.
- Sem foco observado (app focado antes do boot) → **seed** pelo registry
  (`State::Active`) e o resultado vira **foco observado** (ISS-098), para a
  próxima leitura não perder a janela quando a barra rouba o foco. Sem seed
  utilizável → snapshot `None` (pill "Sem contexto"). Evolução registrada:
  fallback X11 com `_NET_ACTIVE_WINDOW` + WM_CLASS.
- Clicar nas janelas do **próprio GhostWhisper** (AskBar, Sessão,
  Configurações) **não** zera o contexto: o filtro de foco é conservador e o
  snapshot cai para a última janela externa validada (`last_good`) — ISS-099.
- A conexão é **sem `p2p`** e com orçamento interno de 1,8 s (ISS-098/097):
  o `AccessibilityConnection` do crate enumerava todos os apps do registry a
  cada conexão e travava; o adapter usa uma camada própria (`A11yBus`).
- Detalhes em [`architecture/screen-context.md`](architecture/screen-context.md)
  §3 e na nota da [`tasks/04-task.md`](tasks/04-task.md).

## 4. Público-alvo e casos de uso

- **CU-01 — Pergunta sobre o que estou lendo:** o usuário está com uma doc no
  Chrome e pergunta "esse endpoint aceita POST?"; a pill mostra `Chrome · API
  Reference`; o prompt leva o texto da página e a resposta acerta.
- **CU-02 — Código no editor:** VS Code aberto; a pill mostra `Code ·
  ask_service.rs`; a pergunta sobre o arquivo responde com o código real.
- **CU-03 — Transparência antes de perguntar:** o usuário abre a AskBar só
  para conferir, pela pill e seu tooltip, o que o copiloto enxergaria.
- **CU-04 — App sem a11y (terminal/canvas):** a pill mostra `Sem contexto de
  tela`; o usuário sabe que precisa do print (botão ao lado) e usa-o.
- **CU-05 — Não quero isso ligado:** o usuário nunca liga a flag; nada muda no
  app — a pill não aparece e nenhuma conexão D-Bus é aberta.

## 5. Fluxos

### 5.1 Liga a flag

```
Settings → Geral & HUD → card "Contexto de tela" → toggle ON
  → grava screen_context_enabled=true (settings_store)
  → composition root instancia AtspiScreenContextSource
  → status do card passa a consultar o adaptador
```

### 5.2 AskBar abre com flag ligada

```
atalho Ctrl+K / ask_show
  → AskWindow monta e publica evento ask_shown
  → core: ScreenContextService.snapshot() (com timeout)
      → X11: janela ativa (_NET_ACTIVE_WINDOW) → pid → nó AT-SPI
      → Wayland: última janela focada (evento AT-SPI)
      → poda a árvore → texto ≤ 2 000 chars → publica "screen_context"
  → pill mostra 🌐 app · título (ou Sem contexto / Indisponível)
```

### 5.3 Pergunta com contexto

```
Enter na AskBar
  → ask_service: flag ligada + snapshot ≤ 30 s?
      → sim: prompt += <screen_context>…</screen_context>
      → não: prompt igual ao de hoje
  → LLM responde; diário registra search/preset (inalterado)
```

## 6. Requisitos funcionais

- **RF-01** `Settings.screen_context_enabled: bool` (default `false`,
  `serde(default)`), refletido em Rust e TS, sem migração de banco.
- **RF-02** Porta `ScreenContextSource` (nomenclatura BR-014) com adaptador
  `AtspiScreenContextSource` (crates `atspi`/`atspi-common`, zbus) e dobra
  `FakeScreenContextSource` para testes.
- **RF-03** Snapshot puro `ScreenContextSnapshot { app, window_title, text,
  age_ms }` com **poda determinística** da árvore (roles a ignorar, texto
  visível, deduplicação, orçamento de chars) em função pura testável — a árvore
  crua nunca sai do adaptador.
- **RF-04** Resolução da janela ativa: X11 via `_NET_ACTIVE_WINDOW` + pid; sem
  X11 (Wayland) via evento de foco AT-SPI; sem janela/foco → snapshot vazio com
  motivo.
- **RF-05** A flag desligada impede **qualquer** conexão/instanciação do
  adaptador (teste de composição: sem `Atspi*` no `AppState` com flag
  desligada).
- **RF-06** Evento IPC `screen_context` `{ available, app, window_title,
  preview, captured_at_ms }` publicado quando a AskBar abre e a cada mudança
  de foco **externa confirmada**; trocar de programa/aba invalida o snapshot
  anterior (ISS-100); nunca publica `text` completo para o front — o preview é
  cortado no core.
- **RF-07** Pill na AskBar com os 5 estados da §3.3, `aria-label` e `title`
  localizados; tooltip com prévia ≤ 300 chars no estado Ativa.
- **RF-08** Bloco `<screen_context>` no prompt quando flag ligada + snapshot ≤
  30 s; precedência documentada no `ASK_RULES`.
- **RF-09** Card "Contexto de tela" em **Geral & HUD** com toggle + linha de
  status (disponível/indisponível/sem contexto) + aviso de privacidade.
- **RF-10** Compatibilidade: settings antigos carregam; AskBar sem flag é
  pixel-idêntica à atual; print (BR-055) inalterado; `JournalRequestSource`
  inalterado.
- **RF-11** Nenhum texto de contexto em log, erro ou telemetria; erros do
  adaptador viram estado `Indisponível` na UI + log só com o motivo técnico.

## 7. Requisitos não funcionais

- **RNF-01 — Testabilidade (BR-013):** poda/montagem são funções puras no
  domain; porta com dobra; serviço sem I/O; adaptador AT-SPI coberto por teste
  de integração marcado `#[ignore]` (requer sessão D-Bus real).
- **RNF-02 — Desempenho:** snapshot ≤ 200 ms em árvore grande (poda
  incremental); o cache curto é **invalidado na troca de foco**; a AskBar
  **não bloqueia** no snapshot (estado "Lendo").
- **RNF-03 — Privacidade:** ver §3.2; conteúdo nunca em log; flag desligada =
  zero conexão.
- **RNF-04 — l10n:** toda copy nova em pt/en, chaves `snake_case` (BR-086).
- **RNF-05 — Wayland:** o recurso funciona sem X11 (foco via AT-SPI); a
  ausência de janela ativa nativa não é bloqueio (BR-007 revisada por
  inclusão, não por mudança).
- **RNF-06 — Acessibilidade da própria UI:** pill com `aria-live="polite"` para
  mudança de estado, `aria-label` descritivo, contraste dos 5 estados no
  design system Obsidian HUD (BR-035).
- **RNF-07 — Arquitetura (BR-011/BR-012):** `domain`/`application` sem `atspi`,
  sem `zbus`, sem `std::fs`; instanciação só em `lib.rs`; porta pequena
  (`snapshot()` + `name()`).

## 8. Histórias e critérios de aceite

### US-01 — Ligar/desligar o recurso nas Configurações

Como usuário, quero optar por ler o contexto da tela pela acessibilidade.

- **CA-01.1** O card "Contexto de tela" existe em Geral & HUD com toggle
  desligado por padrão.
- **CA-01.2** O aviso de privacidade é lido antes/ao ligar (texto do card).
- **CA-01.3** A linha de status reflete a disponibilidade real do AT-SPI.
- **CA-01.4** Settings antigos abrem com a flag desligada; `Salvar`/`Restaurar
  Padrões` não quebram.

### US-02 — Ver o contexto enxergado na AskBar

Como usuário, quero saber, pela pill, qual contexto a barra está enxergando.

- **CA-02.1** Flag desligada: nenhuma pill, nenhuma mudança visual na AskBar.
- **CA-02.2** Flag ligada + janela com texto: pill `🌐 app · título` com tooltip
  de prévia.
- **CA-02.3** Flag ligada + sem texto: pill `Sem contexto de tela`.
- **CA-02.4** AT-SPI inacessível: pill `Contexto indisponível` (nunca erro
  críptico).
- **CA-02.5** Trocar a janela focada/atualiza o snapshot atualiza a pill sem
  reabrir a barra.
- **CA-02.6** Todos os estados têm `aria-label` e texto l10n (pt/en).

### US-03 — Perguntar com o contexto da tela

Como usuário, quero que a resposta considere o que estou vendo.

- **CA-03.1** Com flag ligada e snapshot fresco, o prompt contém
  `<screen_context>` com app, título e texto podado.
- **CA-03.2** Snapshot com mais de 30 s **não** entra (sem contexto velho).
- **CA-03.3** Print anexado + flag ligada: os dois contextos vão juntos.
- **CA-03.4** Flag desligada: prompt idêntico ao atual (teste de regressão).
- **CA-03.5** Resposta do CU-01 (doc no Chrome) usa o conteúdo real da página
  em teste manual.

### US-04 — Respeitar privacidade

Como usuário, quero que nada disso rode sem eu permitir.

- **CA-04.1** Com a flag desligada não há conexão D-Bus de acessibilidade
  (teste: `FakeScreenContextSource` nunca chamado + adaptador não instanciado).
- **CA-04.2** Nenhum log contém texto de contexto (grep de teste nos logs do
  adaptador).
- **CA-04.3** O preview enviado ao front é ≤ 300 chars e o texto completo só
  existe no core.
- **CA-04.4** App com `DeniedAccess` resulta em snapshot vazio, sem erro.

## 9. Arquitetura (resumo)

Contrato completo em
[`architecture/screen-context.md`](architecture/screen-context.md). Resumo:

```
domain/screen_context.rs          # ScreenContextSnapshot, poda() pura, orçamentos
ports/screen_context_source.rs    # trait ScreenContextSource { fn snapshot() -> ...; fn name() }
adapters/screen/atspi_screen_context_source.rs   # atspi/zbus; X11 _NET_ACTIVE_WINDOW; foco Wayland
application/screen_context_service.rs            # cache, idade, evento "screen_context", timeout
ipc/screen_context_commands.rs    # invoke "screen_context_snapshot" (AskBar abre)
src/features/overlay/ContextPill.tsx             # pill + tooltip (AskBar)
src/features/settings/HudSettings.tsx            # card "Contexto de tela"
```

## 10. Riscos e mitigação

| Risco | Impacto | Mitigação |
|---|---|---|
| Chrome só expõe a árvore com cliente a11y conectado (atraso de ~1–2 s na 1ª leitura) | Médio | estado "Lendo"; retry único no snapshot; documentado no architecture §3 |
| Árvore gigante (páginas longas) estoura o orçamento/tokens | Alto | poda determinística + teto de 2 000 chars + dedupe; texto vira prévia de 300 no front |
| Ler app errado (janela não focada) responde errado | Alto | pill + tooltip = inspeção antes de perguntar; idade ≤ 30 s; matching por pid, não por título |
| AT-SPI ausente (sessão sem D-Bus a11y, AppImage sem plug) | Médio | estado `Indisponível` explicado; print continua disponível; feature nunca quebra o ask |
| Privacidade/reputação ("app lê telas") | Alto | default off, aviso explícito, texto nunca logado, pill sempre visível quando ligado (BR-105) |
| Dependência `atspi` imatura/quebra de API | Médio | atrás da porta; adaptador isolado; CI com build mínimo da porta |
| Custo de manter 2 fontes de contexto | Baixo | print (BR-055) intocado; sem duplicação de fluxo (só mais um bloco no prompt) |

## 11. Não-objetivos

- Substituir o print de tela (BR-055 permanece; screenshot continua o caminho
  para imagem/canvas).
- Capturar imagens, screenshots ou "visão" via acessibilidade.
- Sentar no modo contínuo (polling/escuta permanente) enquanto o app roda — a
  leitura é sob demanda (AskBar abre) + foco externo confirmado.
- Extrair dados de apps específicos por integração dedicada (Chrome via
  DevTools/CDP, VS Code via extensão) — AT-SPI é o caminho único deste PRD.
- UI de "painel de contexto" clicável / histórico do que foi lido.
- Persistir o contexto no diário/banco (prompt-only; evolução em `issues.md`).
- Suporte a macOS (`AXUIElement`) / Windows (UI Automation) — Linux-first
  (BR-001).
- Permitir leitura de campos de senha/sensíveis que o app marque como não
  expostos (o adaptador respeita a árvore como o app decide expor).

## 12. Regras de negócio

Registradas em [`../business-rules.md`](../business-rules.md) (escrita única,
R14):

- **BR-105 — Contexto de tela por acessibilidade (AT-SPI), opt-in:** fonte de
  contexto de texto lida da janela ativa via AT-SPI2 (D-Bus), atrás de
  `Settings.screen_context_enabled` (padrão **desligado**); texto limitado e
  nunca logado; funciona em X11 e Wayland; não substitui o print.
- **BR-106 — Pill de contexto na AskBar:** com a flag ligada, a AskBar mostra
  uma pill com o contexto enxergando (app · título) e prévia no tooltip;
  estados explícitos para "sem contexto" e "indisponível"; oculta quando a
  flag está desligada.

## 13. Entregáveis

| # | Entregável | Tasks |
|---|---|---|
| E1 | Domínio puro: snapshot + poda + orçamentos | [`01`](tasks/01-task.md) |
| E2 | Porta `ScreenContextSource` + fake + evento `screen_context` | [`02`](tasks/02-task.md) |
| E3 | Settings + card em Geral & HUD + l10n | [`03`](tasks/03-task.md) |
| E4 | Adaptador AT-SPI (janela ativa X11/Wayland, poda, cache) | [`04`](tasks/04-task.md) |
| E5 | Pill na AskBar (estados, tooltip, a11y) | [`05`](tasks/05-task.md) |
| E6 | Injeção no prompt (`<screen_context>`) + integração | [`06`](tasks/06-task.md) |
| E7 | Hardening, teste manual (Chrome/VS Code/terminal), docs | [`07`](tasks/07-task.md) |

## 14. Decisões pendentes

1. **Orçamento de chars:** 2 000 (recomendado) ou por-app configurável?
2. **Debounce do foco:** 500 ms (recomendado, espelha o follow de janela da
   gravação) ou imediato?
3. **Clique na pill:** no MVP só tooltip (recomendado) ou já abrir prévia em
   popover?
4. **Persistência:** sem diário por enquanto (recomendado) ou gravar
   `context_attached` já na task 06?
