# Memory — PRD Contexto de tela (AT-SPI)

Histórico cronológico deste PRD. Formato em
[`../../.agents/rules.md`](../../.agents/rules.md) §6.2 (R15, BR-017). Entradas
mais recentes no fim.

## 2026-11-01 — PRD criada: contexto de tela por acessibilidade com pill

- **IDs:** BR-105, BR-106, ISS-090, ISS-091
- **Refs:** [`prd.md`](prd.md),
  [`architecture/screen-context.md`](architecture/screen-context.md),
  [`tasks/_tasks.md`](tasks/_tasks.md)

### Pedido
Usar o **sistema de acessibilidade (AT-SPI)** para identificar o contexto da
tela **sem print**: com o Chrome aberto, obter dados do navegador (app, título,
texto/URL da página) pela árvore de acessibilidade. Mostrar, numa **pill na
AskBar ("pesquisar")**, qual contexto o app está enxergando. Tudo atrás de uma
**feature flag nas Configurações**.

### Decisões
- **Opt-in estrito** (`screen_context_enabled`, padrão **desligado**):
  desligada = nenhuma conexão D-Bus de acessibilidade, nenhuma pill
  (BR-105, CA-04.1). Aviso de privacidade no card de settings.
- **AT-SPI é caminho adicional, não substituto do print** (BR-055 intacta):
  canvas/terminal/explicações visuais continuam com screenshot.
- **Pill de transparência** (BR-106): 5 estados (oculta/lendo/ativa/sem
  contexto/indisponível); tooltip com prévia ≤ 300 chars = inspeção antes de
  perguntar. Texto completo nunca sai do core.
- **Janela ativa:** X11 via `_NET_ACTIVE_WINDOW`+pid; **Wayland via evento de
  foco do próprio AT-SPI** (D-Bus não depende de compositor — resolve a
  ausência de "janela ativa" documentada em `portal_targets.rs`, BR-007).
  **Revisão (2026-11-01, implementação):** o foco AT-SPI virou **caminho
  único** dos dois ambientes — o `ApplicationProxy` do atspi 0.30 não expõe
  pid, então o matching X11 por pid caiu; o evento de foco vem do toolkit do
  app e funciona igual nos dois compositor. Sem foco desde o boot → `None`
  ("Sem contexto"); fallback X11 (`_NET_ACTIVE_WINDOW`+WM_CLASS) fica como
  evolução.
- **Prompt-only:** bloco `<screen_context>` (≤ 2 000 chars, idade ≤ 30 s)
  entre `<previous_questions>` e transcript; `JournalRequestSource` não muda
  (persistência fica em ISS-091).
- Arquitetura nas regras de sempre: porta `ScreenContextSource`, adaptação em
  `adapters/screen/atspi_screen_context_source.rs`, composição só em
  `lib.rs`, poda determinística no domain (BR-011–BR-014).

### Riscos registrados
Chrome só monta a árvore com cliente a11y conectado (atraso inicial → estado
"Lendo" + retry); árvores gigantes (teto de nós/chars + timeout); privacidade
reputacional (default off + pill sempre visível quando ligado); crate `atspi`
jovem (atrás da porta).

---

## 2026-11-01 — Tasks 01–06 implementadas (backend + pill + prompt)

- **IDs:** BR-105, BR-106, ISS-090, ISS-091
- **Refs:** [`tasks/_tasks.md`](tasks/_tasks.md) (01–06 `CONCLUDED`),
  [`architecture/screen-context.md`](architecture/screen-context.md)

### O que entrou
- **Domain puro** (`domain/screen_context.rs`): `ScreenContextSnapshot`,
  `AccessibleNode`/`NodeRole` e `extract_text` com dedupe, corte de budget em
  chars (multibyte seguro) e constantes (`CONTEXT_MAX_CHARS` 2 000,
  `PREVIEW_MAX_CHARS` 300, `CONTEXT_FRESH_MS` 30 s). 10 testes.
- **Porta + dobras:** `ScreenContextSource` (2 métodos) e
  `ScreenContextController` (snapshot/status) com
  `FakeScreenContextSource`/`FailingScreenContextSource`.
- **Serviço** (`application/screen_context_service.rs`): flag desligada →
  **nunca toca na porta**; timeout de 500 ms (thread própria); cache de 5 s;
  gate de frescor 30 s no prompt (a pill publica mesmo o snapshot velho);
  debounce de foco 500 ms; evento `screen_context` com preview ≤ 300 chars;
  status `disabled`/`available`/`unavailable` (falha real da última leitura).
- **Adaptador AT-SPI** (`atspi` 0.30): thread de foco com conexão própria,
  caminhada DFS com teto de 5 000 nós/64 níveis, campos via interface `Text`,
  `PasswordText` nunca expõe valor, logs só com métricas. Teste `#[ignore]`
  `atspi_live_snapshot_respects_contract` roda contra o D-Bus real.
- **Settings:** `screen_context_enabled` (Rust `serde(default)` + TS, default
  `false`), card "Contexto de tela" em Geral & HUD com aviso de privacidade e
  linha de status; testes de JSON antigo.
- **Pill:** `ContextPill` (5 estados, `aria-live`, tooltip com prévia) na
  AskBar entre os badges e o divisor; consulta ao abrir + assinatura do
  evento; só renderiza com a flag ligada (CA-02.1).
- **Prompt:** bloco `<screen_context app="…" window="…">` entre
  `<previous_questions>` e `<transcript>`, regra nova no `ASK_RULES`;
  `AskService::with_screen_context` (None = prompt byte a byte igual).

### Desvio registrado (task 04)
Foco AT-SPI como **caminho único** X11+Wayland (ApplicationProxy do atspi
0.30 não expõe pid → matching por pid descartado; o evento de foco vem do
toolkit do app). Ver §3.5 do prd.md e nota na task 04.

### Bateria
`cargo test --lib` 945 ✅ · `cargo clippy -- -D warnings` ✅ · `cargo fmt` ✅ ·
`pnpm test` 558 ✅ · `tsc --noEmit` ✅ · teste live AT-SPI (D-Bus real) ✅.
**Pendente:** task 07 (teste manual nos cenários CU-01…CU-07 com
Chrome/VS Code/terminal e Wayland).

---

## 2026-11-01 — ISS-092: pill presa em "Lendo contexto..." corrigida

- **IDs:** ISS-092, BR-105, BR-106
- **Refs:** [`issues.md`](issues.md) ISS-092,
  [`tasks/07-task.md`](tasks/07-task.md)

### Relatório
Na build 0.4.0, com Chrome aberto, a pill ficava infinitamente em "Lendo
contexto…". Log: `WARN ... contexto de tela indisponível: timeout no snapshot
source="atspi"` (repetido a cada foco).

### Causas (três somadas)
1. `TauriEventPublisher` sem `emit_screen_context` (default no-op do trait) →
   front nunca recebia o evento.
2. Command devolvia `null` na falha → `screenContextState(true, null)` =
   `loading` eterno; estado `unavailable` inalcançável.
3. Timeout 500 ms vs. caminhada D-Bus de até 5 000 nós (~4 roundtrips/nó).

### Correções
- Evento emitido (`SCREEN_CONTEXT_EVENT = "screen_context"`).
- Contrato: flag ligada → sempre DTO (`available` + `failed`); `null` só
  desligada; flag ligada sem reiniciar → `failed:true`.
- `failed` no evento/DTO: distingue "Sem contexto" × "Contexto
  indisponível"; preenchido de `last_fetch_failed`.
- Timeout → 2 000 ms; **parada antecipada** na caminhada (chars acumulados
  via `node_line` do domain ≥ `CONTEXT_MAX_CHARS` → corta).
- Front: `failed` vence `available`; invoke com erro vira `failed`.

### Bateria
`cargo test --lib` 947 ✅ · clippy `-D warnings` ✅ · fmt ✅ · `pnpm test` 559 ✅ ·
tsc ✅ · atspi live ✅. Pendente: rebuild da 0.4.0 + teste manual com Chrome.

---

## 2026-11-01 — ISS-093: pill mostrava GW/gnome-shell; Chrome sem contexto

- **IDs:** ISS-093 · **Refs:** [`issues.md`](issues.md) ISS-093

### Relatório
Depois do ISS-092, a pill mostrava "GNOME Shell" e o próprio "GhostWhisper";
o Chrome não aparecia.

### Correções
1. **Filtro de foco** no listener: pid do nosso processo (a AskBar rouba o
   foco ao abrir) e shells de desktop (`is_desktop_shell`: gnome-shell,
   plasmashell, ksplash) são **descartados** — o foco real é preservado.
2. **Defesa no snapshot:** pid próprio/shell → `Ok(None)`.
3. **Seed `find_active_window`:** sem evento de foco (Chrome focado antes do
   boot), resolve a janela com `State::Active` no registry (teto 50×20).
4. **Retry no front:** snapshot vazio reconsulta 1× após 1,5 s (Chrome pode
   estar montando a árvore a11y).

### Bateria
`cargo test --lib` 949 ✅ · clippy `-D warnings` ✅ · fmt ✅ · `pnpm test`
559 ✅ · tsc ✅. Pendente: rebuild + teste manual.

---

## 2026-11-01 — ISS-094: Chromium não expõe árvore (diagnóstico + instrução)

- **IDs:** ISS-093 (resolvido), ISS-094 · **Refs:** [`issues.md`](issues.md)

### Diagnóstico (sonda `atspi_tree_probe` + D-Bus cru)
1. Seed/foco OK: janela ativa do Chrome/VS Code achada em ~5 ms; ancestors
   ~40 ms; mas walk terminava `nodes=2 chars=0`.
2. Cru no bus: `GetChildren` do Frame do Chromium → `"/org/a11y/atspi/null"`
   — **o app devolve null**, não o adaptador. Apps GTK expõem completo.
3. ORCA conectado e registro de eventos no registry: **não** ativam o
   Chromium.
4. **Chrome com `--force-renderer-accessibility`: árvore completa
   confirmada** (Frame → Panel → …). Causa-raiz fechada.
5. Chrome de teste quase não subiu: meu shell tinha
   `GSETTINGS_SCHEMA_DIR` contaminado pelo snap do VS Code (schemas sem a
   key `antialiasing` → abort no GTK). Corrigido com `env -u`.

### Decisões
- Instrução distribuída em **dois pontos**: tooltip do estado "Sem contexto"
  (`ask_bar_context_empty_title`) e nota de compatibilidade no card de
  Configurações (`hud_screen_context_compatibility`).
- Logs de fase (`conectado ao bus`, `seed`, `ancestrais`, `app resolvido`)
  adicionados ao adaptador para diagnóstico futuro via RUST_LOG=debug.
- ISS-094 registrada como causa externa com workaround.

### Testes de suporte
- `atspi_live_snapshot_respects_contract` (ignorado, com subscriber + timing)
- `atspi_tree_probe` (ignorado, sonda de estrutura com dump de erros)

---

## 2026-11-01 — BR-106 v2: pill vira checkbox; pill também na aba Sessão

- **IDs:** BR-106 (revisão), ISS-090 (FIXED)
- **Refs:** [`../business-rules.md`](../business-rules.md) BR-106,
  [`issues.md`](issues.md) ISS-090, [`prd.md`](prd.md) §3.3

### Pedido
Com a flag habilitada: (1) a pill da barra de pesquisa vira **checkbox** para
incluir/excluir o contexto quando encontrado; (2) a **tela de Sessão** ganha a
mesma opção.

### Decisões
- **`Settings.screen_context_include`** (default **marcado**; settings
  antigos carregam marcados) + command `settings_set_screen_context_include`
  (grava só o campo, sincroniza janelas via `settings_changed`).
- **Porta ganhou `snapshot_for_display` × `snapshot_for_ask`:** a pill mostra
  o contexto **mesmo desmarcado** (só o prompt é filtrado — teste
  `unchecked_checkbox_keeps_pill_but_skips_prompt`); o `status` continua
  refletindo só a flag mestre.
- **Hook único `useScreenContext`** (consulta + retry ISS-093 + evento +
  checked/toggle) usado pela AskWindow **e** pelo JournalView — o toggle numa
  janela reflete na outra automaticamente.
- **`ContextPill` checkbox:** no estado ativo renderiza `<button
  role="checkbox" aria-checked>` com caixa ✓ (cyan = marcado; tracejado +
  esmaecido = desmarcado); tooltip de prévia mantido; sem `onToggle` (ou sem
  contexto) continua `role="status"` informativo.
- **Composer da aba Sessão:** a pill entra **entre o campo e o botão enviar**
  (local escolhido: é o mesmo papel do print — anexo de contexto, alinhado à
  câmera à esquerda).

### Bateria
`cargo test --lib` 951 ✅ · clippy `-D warnings` ✅ · fmt ✅ · `pnpm test`
562 ✅ (inclui fix de font-size em px → rem, BR-087) · tsc ✅.

---

## 2026-11-01 — ISS-095: checkbox visível em todo estado + refresh + sync de save

- **IDs:** ISS-095 · **Refs:** [`issues.md`](issues.md) ISS-095

### Relato
Build 20:57: a opção de ligar/desligar o contexto aparecia só na tela de
Sessão, não na AskBar; e o usuário pediu garantia de persistência do
checkbox.

### Correções
1. **Checkbox em todos os estados visíveis** da pill (não só `active`) — a
   preferência fica sempre à vista; aria combina estado + ação (chaves
   `ask_bar_context_toggle_hint_*`).
2. **`refresh()` no hook** — a AskBar reconsulta a cada foco/abertura
   (`focusSignal`), matando o estado velho do mount (leitura antes do
   Chrome receber foco).
3. **Stale-save:** `SettingsPage` sincroniza `screen_context_include` no
   rascunho ao receber `settings_changed` (padrão `assistant_mode_id`) —
   "Salvar Alterações" não regrava mais a preferência.
4. **Persistência comprovada:** roundtrip JSON com `false` persiste; JSON
   sem o campo → default `true`.

### Bateria
`cargo test --lib` 952 ✅ · clippy `-D warnings` ✅ · fmt ✅ · `pnpm test`
566 ✅ · tsc ✅.

---

## 2026-11-02 — ISS-096: checkbox desliga a leitura e só a caixa alterna

- **IDs:** ISS-096 · **Refs:** [`issues.md`](issues.md) ISS-096 ·
  BR-106 v3 · PRD §3.3

### Relato
1. "Clicar na pill tira o contexto, e quando quero deschecar o select ele
   primeiro tira o contexto, assim eu perco o check" — o corpo inteiro da
   pill era o checkbox (v2), então clicar para ver a prévia já excluía.
2. "Se checkbox desativado, não busque por contexto, a ideia é limpar e não
   usar processamento" — desmarcado continuava consultando o AT-SPI e
   exibindo o contexto.

### Decisões
- **Alvo do clique separado:** corpo da pill = informativo (tooltip com a
  prévia); **só a caixa** (13 px, alvo 18 px) é o `role="checkbox"`.
- **Desmarcado = desligado:** estado novo `paused` (`⏸ Contexto desativado`),
  sem consulta no mount/foco/reabertura, eventos de contexto ignorados.
- **Core idem:** `current()`/`on_focus_changed` exigem o checkbox;
  `snapshot_for_display` não toca na porta; `clear()` novo na
  `ScreenContextController` descarta o cache ao desmarcar (settings command).
- **`checked` otimista:** o clique responde na hora; religar consulta só
  depois de gravar a preferência (sem corrida com settings antigas) e
  cliques seguidos alternam de fato.

### Bateria
`cargo test --lib` 953 ✅ · clippy `-D warnings` ✅ · fmt ✅ · `pnpm test`
571 ✅ · tsc ✅.

---

## 2026-11-02 — ISS-096 v4 (pill inteira) + ISS-097 (main thread)

- **IDs:** ISS-096 (v4), ISS-097 · **Refs:** [`issues.md`](issues.md) ·
  BR-106 v4 · PRD §3.3

### Relato
1. "Ainda acontece" com o alvo separado; pedido explícito: **a pill inteira
   ser o checkbox, 1 clique ativa e 1 clique desativa sempre**.
2. "Sinto um leve congelamento da UI" — verificar o que segura a main thread
   nessa feature.

### Decisões
- **Pill inteira é o checkbox** (volta ao `<button role="checkbox">`): caixa,
  ícone e texto = um único alvo; cada clique alterna uma vez.
- **Toggle determinístico:** o hook serializa as gravações em fila
  (`writeChainRef`) e ignora `settings_changed` intermediário enquanto há
  gravação em voo (`pendingWritesRef`) — a caixa não "volta sozinha" com
  cliques rápidos.
- **Congelamento = comando síncrono:** `screen_context_snapshot` e
  `screen_context_status` eram `#[tauri::command]` (main thread). O serviço
  espera até 2 s pela árvore AT-SPI. Passaram a `#[tauri::command(async)]`
  (`sync_threadpool`) — leitura fora da main thread (ISS-097).

### Bateria
`cargo test --lib` 953 ✅ · clippy `-D warnings` ✅ · fmt ✅ · `pnpm test`
571 ✅ · tsc ✅. Teste manual do congelamento fica no cenário 8 da task 07.

---

## 2026-11-02 — ISS-098: conexão AT-SPI sem p2p + orçamento no adaptador

- **IDs:** ISS-098 · **Refs:** [`issues.md`](issues.md) ISS-098 ·
  architecture §3

### Relato
Depois de ativar/desativar o contexto algumas vezes, ele não voltava mais ao
religar.

### Diagnóstico (medido)
`AccessibilityConnection::new()` do `atspi` 0.30 leva **>120 s** no ambiente
porque a feature **`p2p`** (default do crate) faz `Peers::initialize_peers`
enumerar **todos** os apps do registry a cada conexão. Connect/RegistryProxy/
DBusProxy isolados levam ~1 ms. O adapter chamava `new()` **por snapshot**;
o serviço abortava em 2 s mas a thread continuava presa — os toggles
acumulavam threads/conexões até o registry degradar e nada mais ser lido.

### Decisões
- `atspi` com `default-features = false` (`proxies`, `wrappers`): sem
  `connection`/`p2p`; `atspi-connection` removido do grafo.
- `A11yBus`: conexão própria (sessão → `GetAddress` → a11y → registry +
  DBus) e `object_as_accessible` com destino = **unique name** do app.
- `ADAPTER_BUDGET = 1,8 s` (serviço: 2 s): thread nunca fica presa além do
  orçamento.
- Seed do registry promovido a **foco observado** (a barra não perde a janela
  real nas próximas leituras).

### Bateria
`cargo test --lib` 953 ✅ · clippy `-D warnings` ✅ · fmt ✅ · live test do
adaptador ~5–75 ms (antes >120 s) · `atspi_tree_probe` lê Chrome/VS Code ✅ ·
`pnpm test` 571 ✅ · tsc ✅.

---

## 2026-11-02 — ISS-099: foco nas janelas do GW não pode zerar o contexto

- **IDs:** ISS-099 · **Refs:** [`issues.md`](issues.md) ISS-099 ·
  architecture §3

### Relato
"Às vezes o contexto é perdido quando clico nas janelas do GhostWhisper."

### Causa
O filtro do listener caía no `false` quando a resolução de pid falhava
(D-Bus transitório / nome sem dono / proxy indisponível) e **sobrescrevia** o
foco guardado com a nossa janela. No snapshot, o próprio era descartado e o
código retornava `None` sem tentar a janela anterior (sem fallback).

### Decisões
- `FocusOrigin::{Own, External, Unknown}`; só substitui com app externo
  confirmado; `Unknown` valida pelo nome do app e, se não der, mantém o
  anterior.
- `is_own_app` (`ghostwhisper`/`GhostWhisper`) cobre webview com pid distinto.
- `last_good` (última janela externa validada) na ordem de candidatos
  `observed → last_good → seed`; candidato inválido não zera o contexto.
- Logs de decisão/candidato sem conteúdo.

### Bateria
`cargo check --lib --tests` ✅ · `cargo test --lib` 954 ✅ · testes do
adaptador 7 ✅ (2 ignorados) · fmt ✅. Build de release **não** gerado nesta
rodada (pedido do usuário).

---

## 2026-11-02 — ISS-100: troca de programa/aba invalida o contexto

- **IDs:** ISS-100 · **Refs:** [`issues.md`](issues.md) ISS-100 ·
  architecture §2.3/§3 · BR-106 v5

### Relato
Depois do ISS-099, o contexto **antigo** permanecia ao focar outra aba/app —
risco de enviar o texto errado. Regra: trocar de programa descarta o
contexto; só seleção nas janelas do GhostWhisper preserva.

### Causas
1. Cache de 5 s em `current()` sem saber da troca de foco.
2. Debounce de 500 ms em `on_focus_changed` **descartava** a última troca.
3. `last_good` sobrevivia à troca de programa.

### Decisões
- `on_focus_changed` invalida o cache **antes** e relê; debounce removido
  (só foco externo confirmado chega; evolução = trailing debounce).
- Listener limpa `last_good` ao aceitar foco externo; não confirmado
  (`ClearAndNotify`) limpa `focused` + `last_good` e notifica — nunca serve a
  janela antiga.
- `focus_action` puro (Keep/Accept/ClearAndNotify) testável.

### Bateria
`cargo test --lib` 956 ✅ · fmt ✅ · clippy ✅ (antes do último doc).
Build de release será gerado em seguida (pedido do usuário).

---

## 2026-11-02 — ISS-101: app sem a11y não emite foco; limpar pela perda

- **IDs:** ISS-101 · **Refs:** [`issues.md`](issues.md) ISS-101 ·
  architecture §3 · BR-106 v6

### Relato
Terminal → Chrome **sem** a árvore de acessibilidade: o terminal permanecia
no contexto.

### Causa
O listener só reagia a `Focused=true`. Chrome sem a11y não emite evento; o
foco guardado ficava no terminal e o serviço relia o terminal.

### Decisão
- Tratar `Focused=false` (perda de foco) do app anterior: agendar checagem
  de **300 ms** com geração de foco; se nenhum foco novo chegar, limpar
  `focused`/`last_good` e notificar (cache invalidado, pill sem contexto).
- Qualquer `Focused=true` incrementa a geração e cancela; clique direto nas
  janelas do GW fica dentro da graça (poucos ms).
- Testes com runtime tokio: limpa após a graça; cancela com foco novo.

### Bateria
`cargo test --lib` 958 ✅ · clippy `-D warnings` ✅ · fmt ✅. Build em seguida.
