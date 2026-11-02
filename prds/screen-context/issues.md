# Issues — PRD Contexto de tela (AT-SPI)

Registry de bugs e limitações deste PRD. Convenções em
[`../../.agents/rules.md`](../../.agents/rules.md) (`ISS-NNN` é canônico por
arquivo de PRD; a numeração **continua** a dos PRDs anteriores para o ID ser
único no repositório). Próximo ID livre: **ISS-099**.

| ID | Resumo | Severidade | Status |
|---|---|---|---|
| ISS-090 | Pill não era clicável: prévia só no tooltip (sem popover) | Baixa | FIXED (clique virou checkbox de envio — BR-106 v2; popover de prévia segue como evolução) |
| ISS-091 | Contexto de tela não é persistido no diário (prompt-only) | Baixa | OPEN (evolução, PRD §11) |
| ISS-092 | Pill presa em "Lendo contexto...": evento não emitido + null eterno + timeout de 500 ms | Alta | FIXED |
| ISS-093 | Pill mostrava o próprio GhostWhisper/gnome-shell; Chrome sem contexto | Alta | FIXED |
| ISS-094 | Chromium (Chrome/VS Code) não expõe a árvore sem acessibilidade forçada — GetChildren devolve /org/a11y/atspi/null | Alta | OPEN (causa externa; instrução documentada) |
| ISS-095 | "Salvar Alterações" podia regrar o checkbox da pill com rascunho velho; opção não aparecia na AskBar em todo estado | Média | FIXED |
| ISS-096 | Clique na pill desligava o contexto sem querer; desmarcado continuava lendo | Média | FIXED (v4: pill inteira é o checkbox, 1 clique = 1 alternância) |
| ISS-097 | UI congela ao abrir a barra: snapshot do AT-SPI síncrono na main thread | Alta | FIXED |
| ISS-098 | Contexto para de aparecer após alguns toggles: `AccessibilityConnection::new()` enumera todos os apps (p2p) e trava; threads presas se acumulam | Alta | FIXED |

---

## ISS-090 — Pill não era clicável

- **Severidade:** Baixa · **Status:** FIXED (2026-11-01, revisão BR-106 v2)
- **Sintoma original:** a prévia do contexto só aparece em tooltip hover; em
  tela de toque/dispositivo apontador sem hover não há como inspecionar.
- **Resolução (2026-11-01):** o clique na pill **passou a ser útil** — no
  estado ativo a pill virou **checkbox** de incluir/excluir o contexto da
  pergunta (`screen_context_include`), presente na AskBar **e** no composer
  da aba Sessão (pedido do usuário). A prévia continua no tooltip; um
  clique-que-abre-popover passou a competir com o toggle, então ficou
  **evolução**: se voltar, o popover deve conviver com o checkbox (ex.:
  ícone separado ou long-press), com botão "copiar contexto".

## ISS-091 — Contexto não persistido no diário

- **Severidade:** Baixa · **Status:** OPEN (evolução)
- **Sintoma:** o histórico de perguntas não registra se houve contexto de
  tela nem qual app/título; impossível auditar depois "o que o copiloto
  enxergava".
- **Evolução sugerida:** campo `context_attached` (+ app/título) no
  `journal_entries`, exibido no cartão do histórico (sem o texto — só metadado).

## ISS-092 — Pill presa em "Lendo contexto..."

- **Severidade:** Alta · **Status:** FIXED (2026-11-01)
- **Sintoma:** com a AskBar aberta e o Chrome em foco, a pill de contexto
  ficava infinitamente em "Lendo contexto…" (relatório do usuário na build
  0.4.0).
- **Causa raiz (três bugs somados):**
  1. `TauriEventPublisher` **não implementava** `emit_screen_context` — o
     método caía no default no-op do trait, então o front nunca recebia o
     evento `screen_context` (única fonte de estado depois da consulta
     inicial).
  2. Contrato do command: `screen_context_snapshot` devolvia `null` quando o
     snapshot falhava, e `screenContextState(true, null)` = `loading` para
     sempre. O estado `unavailable` da derivação era inalcançável.
  3. Timeout de 500 ms no serviço vs. caminhada D-Bus do AT-SPI (~4
     roundtrips × até 5 000 nós) — log do usuário: `timeout no snapshot
     source="atspi"` repetido a cada mudança de foco.
- **Correção:**
  - `emit_screen_context` implementado (evento `screen_context`);
  - contrato do command: flag ligada → **sempre** DTO (`available` +
    `failed`); `null` só com flag desligada. Flag ligada sem reiniciar (sem
    adaptador) → `failed:true` em vez de null eterno;
  - `ScreenContextEvent`/DTO ganham `failed` — distingue "Sem contexto" de
    "Contexto indisponível" (serviço preenche de `last_fetch_failed`);
  - timeout 500 ms → **2 000 ms**;
  - **parada antecipada** na caminhada: acumula chars via `node_line` (o
    critério público do domain) e corta ao chegar em `CONTEXT_MAX_CHARS` —
    não visita 5 000 nós para coletar 2 000 chars;
  - front: `failed` vence `available` na derivação; invoke com erro vira
    `failed` (não `null`).
- **Testes:** `empty_context_is_not_marked_as_failed`,
  `port_error_degrades_to_none` (com `failed`), `node_line_is_the_shared_walk_criterion`,
  TS `failed → unavailable` + "nunca volta a loading depois da primeira
  consulta".

## ISS-093 — Pill mostrava o próprio GhostWhisper/gnome-shell

- **Severidade:** Alta · **Status:** FIXED (2026-11-01)
- **Sintoma:** na build com o ISS-092 corrigido, a pill mostrava "GNOME
  Shell" e o próprio "GhostWhisper" como contexto; o Chrome (janela que o
  usuário estava usando) não aparecia (relatório do usuário).
- **Causa raiz:**
  1. A **AskBar rouba o foco** ao abrir (o input recebe foco) → o listener
     AT-SPI gravava o foco do próprio GhostWhisper e o snapshot virava a árvore
     do próprio app.
  2. O listener gravava **qualquer** foco — inclusive o gnome-shell (overview,
     notificações, painel).
  3. O Chrome já focado **antes** do boot do listener não gera evento de foco
     (`state-changed` só dispara em **mudanças**) → sem seed, o snapshot nunca
     encontrava a janela ativa.
- **Correção:**
  - **Filtro de foco** no listener (funções puras `is_own_process` /
    `is_desktop_shell` testadas): pid igual ao nosso → descarta; shell de
    desktop (gnome-shell, plasmashell, ksplash) → descarta. O foco **real é
    preservado** — a pill continua mostrando a última janela do usuário.
  - **Defesa em profundidade** no snapshot: app for nosso pid ou shell →
    `Ok(None)` mesmo se o foco passou sem filtro (daemon indisponível).
  - **Seed da janela ativa** (`find_active_window`): sem foco observado,
    caminha as aplicações do registry procurando `State::Active` (teto de
    50 apps × 20 janelas) — cobre "Chrome focado antes do app subir".
  - **Retry no front:** primeiro snapshot "sem contexto" (árvore a11y ainda
    sendo montada pelo Chrome) reconsulta uma vez após 1,5 s.
- **Testes:** `desktop_shells_are_recognized`,
  `own_process_focus_is_ignored` (funções puras do filtro).

## ISS-094 — Chromium não expõe a árvore sem acessibilidade forçada

- **Severidade:** Alta · **Status:** OPEN (causa externa; instrução
  documentada na UI — workaround do usuário)
- **Sintoma:** a pill fica em "Sem contexto de tela" com o Chrome/VS Code
  abertos, mesmo com foco correto e seed funcionando (diagnóstico da ISS-093).
- **Diagnóstico (2026-11-01, com sondas `atspi_tree_probe` e chamadas cruas
  no D-Bus):**
  - `GetChildren` do **Frame** do Chrome/VS Code devolve
    `a(so) ":1.10" "/org/a11y/atspi/null"` — **o próprio Chromium** responde
    null; o adaptador recebe `ObjectRefOwned(Null)` e a caminhada para em 2
    nós (`nodes=2 chars=0`).
  - Apps **GTK** (gnome-shell, gjs) expõem a árvore completa normalmente.
  - Conectar o **ORCA** (6 s) e registrar eventos no registry **não** ativou
    o Chromium.
  - **Chrome com `--force-renderer-accessibility`: árvore completa confirmada**
    (`Frame → Panel → Panel → …`, `Active/Showing/Visible`) — mesmo
    diagnóstico com processo de teste controlado.
  - Causa raiz: o Chromium só monta a árvore do **renderer** com
    acessibilidade forçada/detectada; sem isso expõe só a casca de janelas.
- **Workaround (documentado na UI):**
  - Chrome: iniciar com `--force-renderer-accessibility`;
  - VS Code/Electron: `ELECTRON_ENABLE_ACCESSIBILITY=1`;
  - apps GTK: funcionam sem nada.
- **Distribuição da instrução:** tooltip do estado "Sem contexto" na pill
  (`ask_bar_context_empty_title`) + nota de compatibilidade no card de
  Configurações (`hud_screen_context_compatibility`).
- **Não é bug do adaptador:** a evidência cru prova que o dado não sai do
  app. Evolução futura: detectar o padrão (árvore null em app com janela
  ativa) e sugerir o workaround na pill automaticamente.

## ISS-095 — Checkbox: opção ausente na AskBar e risco de stale-save

- **Severidade:** Média · **Status:** FIXED (2026-11-01)
- **Sintoma (relato do usuário na build 20:57):** a opção de ligar/desligar o
  contexto aparecia **só na tela de Sessão**, não na AskBar; e dúvida sobre
  se a última preferência do checkbox é persistida.
- **Causas:**
  1. O checkbox só era renderizado no estado `active` — se a leitura da
     AskBar (feita no **mount** da janela, antes do usuário focar o Chrome)
     vinha vazia/antiga, a pill ficava informativa sem a opção, e sem novo
     evento de foco o estado não atualizava ao reabrir a barra.
  2. Persistência: o `SettingsPage` no `settings_changed` só sincronizava
     `assistant_mode_id`/`ui_language` no rascunho — um "Salvar Alterações"
     reescrevia o campo inteiro e **regraria** `screen_context_include` com o
     valor velho.
- **Correções:**
  - `ContextPill` renderiza o **checkbox em todos os estados visíveis** (a
    preferência é uma opção permanente; com estado vazio o aria explica que
    vale quando houver contexto);
  - hook `useScreenContext` ganhou **`refresh()`** — a AskBar reconsulta em
    cada foco/abertura (`focusSignal`), nunca com estado velho do mount;
  - `SettingsPage` sincroniza `screen_context_include` no rascunho ao
    receber `settings_changed` (mesmo padrão do `assistant_mode_id`);
  - persistência comprovada por roundtrip (`include=false` → JSON → load →
    `false`; ausente do JSON → default `true`).
- **Testes:** AskBar (pass-through do checkbox em ativo/sem contexto/hidden),
  ContextPill (checkbox em todo estado), SettingsPage (sync + payload do
  Salvar), Rust (`screen_context_include_survives_roundtrip_when_unchecked`).

## ISS-096 — Clique na pill desligava o contexto; desmarcado continuava lendo

- **Severidade:** Média · **Status:** FIXED (2026-11-02)
- **Sintoma (relato do usuário):** "clicar na pill tira o contexto, e quando
  quero deschecar o select ele primeiro tira o contexto, assim eu perco o
  check"; e "se checkbox desativado, não busque por contexto, a ideia é limpar
  e não usar processamento".
- **Causas:**
  1. No v2 a pill inteira era o `<button role="checkbox">` — qualquer clique
     (inclusive no texto, para olhar o tooltip) alternava
     `screen_context_include` e tirava o contexto da pergunta sem intenção.
  2. O checkbox controlava só o **envio**: desmarcado, a pill continuava
     consultando o AT-SPI e exibindo o contexto — custo de leitura oposto ao
     que o usuário queria ("limpar").
- **Correções:**
  - `ContextPill`: o corpo (app/título/prévia/tooltip) é informativo; **só a
    caixa** de 13 px (alvo 18 px) é o checkbox. Clicar no corpo não alterna.
  - Estado novo **`paused`**: desmarcado → `⏸ Contexto desativado` + caixa
    vazia, em todas as superfícies (AskBar e Sessão).
  - `useScreenContext`: `query`/`refresh` e o handler do evento
    `screen_context` são no-op quando desmarcado; o evento é limpo na hora.
  - `ScreenContextService`: `current()` e `on_focus_changed` exigem o
    checkbox; `snapshot_for_display` não toca na porta desmarcado.
  - `settings_set_screen_context_include(false)` chama
    `ScreenContextController::clear()` (novo na porta): descarta o cache e
    publica "sem contexto" — o snapshot anterior não reaparece.
  - `checked` otimista no hook (caixa responde no clique; religar lê **depois**
    de gravar, sem corrida com settings antigas; cliques seguidos não repetem
    o mesmo valor).
- **Testes:** Rust (`unchecked_checkbox_never_touches_port`,
  `clear_drops_cache_and_next_read_is_fresh`), hook
  (`useScreenContext.test.tsx`: não consulta desmarcado, limpa ao desmarcar,
  relê ao remarcar, cliques seguidos), ContextPill (clique no corpo não
  alterna; `paused`), AskBar (pass-through).

## ISS-097 — UI congela: snapshot do AT-SPI síncrono na main thread

- **Severidade:** Alta · **Status:** FIXED (2026-11-02)
- **Sintoma (relato do usuário):** "sinto um leve congelamento da UI" ao
  abrir/interagir com a barra com o contexto de tela ligado.
- **Causa:** `screen_context_snapshot` e `screen_context_status` eram
  `#[tauri::command]` **síncronos**. No Tauri 2, comando síncrono roda na
  **main thread** (`ExecutionContext::Blocking`); o serviço espera a leitura
  da árvore AT-SPI por até 2 s (`SNAPSHOT_TIMEOUT`) — a janela ficava sem
  responder nesse intervalo. Cada foco/reabertura da AskBar disparava uma
  consulta, então o congelamento aparecia ao abrir a barra.
- **Correção:** `#[tauri::command(async)]` nos dois comandos — o macro usa
  `sync_threadpool` e a leitura sai da main thread. O timeout e o cache
  continuam iguais.
- **Testes:** bateria completa (não há teste de thread no harness); a
  verificação é manual: abrir a barra com o contexto ativo não deve travar o
  cursor/digitação nem atrasar o `Enter`.

## ISS-096 (continuação) — v4: a pill inteira volta a ser o checkbox

- **Pedido do usuário (2026-11-02):** "a pill inteira ser o checkbox, 1 click
  ativa, e 1 click desativa sempre" — o alvo separado (só a caixa) não
  resolveu a percepção. A pill inteira voltou a ser o `<button
  role="checkbox">`; o corpo, o ícone e a caixa contam como **um** clique.
- **Robustez do toggle:** o hook serializa as gravações em fila
  (`writeChainRef`) — cliques rápidos gravam em ordem e o último vence — e
  ignora reloads intermediários de `settings_changed` enquanto há gravação em
  voo (`pendingWritesRef`), então a caixa nunca "volta sozinha".
- **Congelamento:** ver ISS-097 — era a causa provável de cliques
  percebidos como perdidos/atrasados.

## ISS-098 — Contexto para de aparecer após alternar o checkbox algumas vezes

- **Severidade:** Alta · **Status:** FIXED (2026-11-02)
- **Sintoma (relato do usuário):** depois de ativar/desativar o contexto
  algumas vezes, ele não conseguia mais ler o contexto ao religar.
- **Causa raiz:** o crate `atspi` habilita a feature **`p2p`** por padrão
  (`default = [connection, proxies, p2p, wrappers]`). Com ela,
  `AccessibilityConnection::new()` chama `Peers::initialize_peers`, que
  **enumera todos os apps do registry** (`get_children`, `get_application_bus_
  address`, `Peer::try_new` por app) **a cada conexão**. O adapter abria uma
  conexão por snapshot; se qualquer app demorasse/não respondesse, o `new()`
  travava (medido: >120 s). O serviço aborta em 2 s, mas a **thread continua
  presa** — cada toggle/leitura deixava uma thread e uma conexão D-Bus
  penduradas, até o registry ficar lento e todas as leituras estourarem o
  timeout (pill sem contexto/"indisponível").
- **Correção:**
  - `atspi` com `default-features = false` (`proxies`, `wrappers`) — sem
    `connection`/`p2p`;
  - camada própria `A11yBus` (sessão → `org.a11y.Bus.GetAddress` → bus de
    acessibilidade → `RegistryProxy` + `DBusProxy`), com
    `object_as_accessible` montando o proxy pelo **unique name** do app
    (o fallback de bus que o p2p fazia, sem a enumeração de peers);
  - orçamento interno de **1,8 s** no adapter (`ADAPTER_BUDGET`) para a
    thread nunca ficar presa além do serviço (2 s);
  - o resultado do **seed** (`State::Active`) é promovido a foco observado:
    a barra roubar o foco não faz a próxima leitura perder a janela real.
- **Testes:** live test do adaptador (ignorado) agora conclui em ~5–75 ms
  (antes: >120 s); `atspi_tree_probe` caminha Chrome/VS Code normalmente;
  bateria completa verde. A regressão estrutural é a ausência de
  `atspi-connection`/`p2p` no grafo (`cargo tree -i atspi-connection` não
  resolve).
