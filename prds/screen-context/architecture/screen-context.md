# Arquitetura — Contexto de tela (AT-SPI)

Especificação técnica do PRD [`../prd.md`](../prd.md). Complementa as regras
BR-105/BR-106 em [`../../business-rules.md`](../../business-rules.md).

## 1. Visão geral

```
┌─ frontend ──────────────────────────────┐
│ AskWindow ─▶ AskBar ─▶ ContextPill       │  evento "screen_context"
│ HudSettings (card "Contexto de tela")    │  ◀── IPC snapshot/estado
└───────────────────┬──────────────────────┘
                    │ tauri commands / events
┌─ core ────────────▼──────────────────────┐
│ application/screen_context_service.rs     │  cache, idade, timeout, evento
│ domain/screen_context.rs                  │  tipos + poda() pura
│ ports/screen_context_source.rs            │  trait ScreenContextSource
└───────────────────┬──────────────────────┘
                    │ DI (lib.rs, só com flag ligada)
┌─ adapters ────────▼──────────────────────┐
│ screen/atspi_screen_context_source.rs     │  atspi + zbus; _NET_ACTIVE_WINDOW
│ screen/fake_screen_context_source.rs      │  testes
└───────────────────────────────────────────┘
```

Fluxo: a AskBar, ao abrir, chama `screen_context_snapshot` (command). O
serviço consulta a porta (se a flag ligada), poda, cacheia e publica o evento
`screen_context` com o **preview** (≤ 300 chars). O texto completo nunca sai do
core — é montado no `build_prompt` na hora da pergunta.

## 2. Contratos

### 2.1 Domain (`domain/screen_context.rs`) — puro

```rust
/// Snapshot da janela ativa, já podado.
pub struct ScreenContextSnapshot {
    /// Nome curto do app ("google-chrome", "code").
    pub app: String,
    /// Título da janela (truncado no domain).
    pub window_title: String,
    /// Texto extraído, com orçamento de chars aplicado.
    pub text: String,
    /// Idade do snapshot em ms (derivada no serviço, não pelo adaptador).
    pub captured_at_ms: i64,
}

/// Orçamentos (fonte única — R14).
pub const CONTEXT_MAX_CHARS: usize = 2_000;      // texto entregue ao prompt
pub const TITLE_MAX_CHARS: usize = 120;          // título na pill/prompt
pub const PREVIEW_MAX_CHARS: usize = 300;        // tooltip/preview ao front

/// Poda determinística da árvore crua em texto.
pub fn extract_text(root: &AccessibleNode, budget: usize) -> String;
```

`AccessibleNode` é um DTO mínimo do domain (`role`, `name`, `value`,
`children`, `states`) definido pelo adaptador ao caminhar a árvore AT-SPI; o
domain **não** importa `atspi`. Regras da poda (todas testáveis sem I/O):

- ignorar roles estruturais puros (`application`, `frame`, `panel`,
  `filler`, `scroll pane`) mantendo a hierarquia de texto;
- **visibilidade**: pular nós com estado `invisible`/`hidden`;
- texto = `name` quando for role de texto (`heading`, `label`, `static text`,
  `link`, `paragraph`) ou `value`/`text` de campos (`entry`, `document`,
  `paragraph`);
- dedupe de linhas idênticas consecutivas (menus repetidos);
- ordenação em ordem de leitura (children na ordem do AT-SPI);
- corte por budget com `…` no fim; nunca corta no meio de uma linha sem aviso.

### 2.2 Porta (`ports/screen_context_source.rs`)

```rust
/// Lê o contexto da janela ativa via sistema de acessibilidade (BR-105).
pub trait ScreenContextSource: Send + Sync {
    /// Snapshot da janela ativa/focada. `None` quando não há contexto
    /// (janela sem árvore, sem foco observado, DeniedAccess).
    /// `Err` só para falha do mecanismo (D-Bus inacessível).
    fn snapshot(&self) -> Result<Option<ScreenContextSnapshot>>;
    /// Nome estável para UI/logs ("atspi").
    fn name(&self) -> &'static str;
}
```

Dobras: `FakeScreenContextSource` (devolve snapshot fixo ou `None`,
registra chamadas) e `FailingScreenContextSource` (para o estado
`Indisponível`).

### 2.3 Serviço (`application/screen_context_service.rs`)

- `snapshot_for_ask()` — usado pelo command IPC:
  1. lê `Settings.screen_context_enabled`; desligada → `None` **sem tocar na
     porta**;
  2. consulta a porta com **timeout de 2 s** (thread dedicada) e o adaptador
     aplica 1,8 s internamente;
  3. cacheia por `captured_at_ms`; reutiliza cache com idade ≤ 5 s **apenas
     enquanto o foco não mudou** — qualquer foco novo invalida o cache
     (ISS-100);
  4. publica evento `screen_context` com preview.
- **Idade máxima do snapshot usado no prompt: 30 s** (`CONTEXT_FRESH_MS`);
  mais velho que isso, a pergunta vai sem o bloco (silenciosamente — a pill
  continua mostrando o último estado com o selo de idade? **Não**: pill mostra
  o snapshot atual da porta, que sempre relê; a idade só gateia o prompt).
- `on_focus_changed()` — chamado pelo adaptador (observer) quando o foco
  externo muda; **invalida o cache e relê** (sem debounce): uma troca de
  programa/aba não pode servir o snapshot anterior (ISS-100). O debounce foi
  removido porque os eventos são só de foco confirmado (não é polling) e a
  leitura tem orçamento; se o custo incomodar, a evolução é um debounce
  *trailing* (nunca descartar a última troca).

### 2.4 IPC

- Command `screen_context_snapshot() -> ScreenContextDto` — invocado quando a
  AskWindow monta.
- Evento `screen_context` `{ available: bool, app, window_title, preview,
  captured_at_ms }` — publicado pelo serviço (abertura + mudança de foco).

### 2.5 Prompt (`ask_service.rs::build_prompt`)

Inserção, nesta ordem: `<meeting_summary>` → `<previous_questions>` →
**`<screen_context>`** → transcript. Bloco:

```
<screen_context app="google-chrome" window="API Reference — docs…">
…texto podado (≤ 2 000 chars)…
</screen_context>
```

Regra nova no `ASK_RULES`: "The <screen_context> block is what ME is looking
at right now; the transcript is the conversation. When they conflict, the
transcript wins as the conversation and the screen is the material being
discussed."

## 3. Adaptador AT-SPI (`atspi_screen_context_source.rs`)

- Crates: `atspi` 0.30 (mantida pelo pessoal do AccessKit, base zbus 5) —
  mesma major do `zbus` do projeto, com `default-features = false`
  (`connection`, `proxies`, `wrappers`) e a feature **`p2p` desligada**.
- **Conexão (ISS-098):** camada própria `A11yBus` (sessão → `org.a11y.Bus.
  GetAddress` → bus de acessibilidade → `RegistryProxy` + `DBusProxy`). O
  `AccessibilityConnection::new()` do crate fica de fora: com `p2p` (default
  dele) ele enumera **todos** os apps do registry a cada conexão e podia
  travar por tempo indeterminado — como o snapshot abre uma conexão por
  leitura, após alguns toggles sobravam threads presas e o contexto parava.
  Os proxies de objeto são montados com o **unique name** do app como destino.
  A conexão é aberta **só** quando `snapshot()` é chamado com a flag ligada;
  a thread de foco tem a própria conexão (nascem juntas no `new()`).
- **Janela ativa (caminho único): foco AT-SPI.** O evento
  `object:state-changed:focused` é gerado pelo **toolkit do app** (GTK/Qt/
  Chromium), não pelo compositor — funciona igual em **X11 e Wayland** e não
  depende de `_NET_ACTIVE_WINDOW`. Motivo da simplificação (registrado na
  task 04): o `ApplicationProxy` do atspi 0.30 **não expõe pid**, então o
  matching por pid (X11) previsto aqui não era viável. Sem foco observado
  (app focado antes do boot) → **seed** pelo registry (`State::Active`) e o
  resultado é **promovido a foco observado** (ISS-098), sobrevivendo ao foco
  que a barra rouba. Evolução: fallback X11 via `_NET_ACTIVE_WINDOW` + WM_CLASS.
- **Filtro de foco conservador (ISS-099):** o listener só substitui o foco
  quando o app é confirmadamente externo (`FocusOrigin::External` e nome
  diferente do nosso). Pid não resolvido (`Unknown`) valida pelo nome; se
  ainda assim não confirmar, **mantém** o foco anterior. `is_own_app` cobre o
  webview com pid diferente. Nunca sobrescrever com a nossa janela é mais
  importante que atualizar rápido.
- **Troca de programa sem evento de foco (ISS-101):** um app sem a11y não
  emite `Focused=true`. O listener trata `Focused=false` do app anterior:
  agenda uma checagem de **300 ms** (geração de foco cancela); sem foco novo
  (externo/nosso), limpa `focused`/`last_good` e notifica. Cliques diretos nas
  janelas do GW chegam em poucos ms e cancelam — exceção preservada.
- **Fallback de candidatos (ISS-099):** `snapshot_connected` tenta, nesta
  ordem, **foco observado → `last_good` → seed**. `last_good` é a última
  janela **externa validada** (atualizada antes de caminhar, então vale também
  para terminal sem texto). Candidato próprio/shell/expirado → próximo; app
  externo válido → lê e para (um terminal não ressuscita o Chrome anterior).
- **Árvore:** a partir do app do elemento focado (`get_application`), caminhada
  DFS com teto de **5 000 nós** e **64 níveis**; campos (`Entry`/`Text`)
  leem a interface `Text` (`get_text(0, -1)`); `PasswordText` é tratado como
  estrutura (**nunca** extrai valor de senha); `Defunct` é ignorado.
- **Ativação do Chrome/Chromium:** o browser só monta a árvore completa quando
  um cliente a11y conecta; pode levar 1–2 s na primeira leitura. Mitigação:
  estado "Lendo" na pill + o snapshot é reconsultado na pergunta (o cache de
  5 s evita leitura dupla imediata).
- **DeniedAccess/árvore vazia:** nó que não resolve → ignorado; app sem texto
  e sem título → `Ok(None)` ("Sem contexto"), sem `Err`.
- **Timeout:** o adaptador aplica **1,8 s** internamente
  (`ADAPTER_BUDGET`) e o serviço **2 s** por snapshot (thread própria); a
  caminhada corta no teto de nós/profundidade. O orçamento interno garante que
  a thread não fica presa em D-Bus (sem acúmulo entre leituras).
- **Logs:** só nós, chars extraídos e latência. **Nunca** app/título/conteúdo
  (RNF-03).

## 4. Frontend

### 4.1 `ContextPill.tsx` (novo, em `src/features/overlay/`)

Props: `state: "hidden" | "paused" | "loading" | "active" | "empty" |
"unavailable"`, `event`, `checked`, `onToggle?`. Renderiza a pill com
`aria-live="polite"`, `title` = preview (ativa) ou a explicação do estado.
Com `onToggle`, a **pill inteira** é um `role="checkbox"` (`aria-checked`):
um clique alterna ligar/desligar, em todos os estados visíveis (ISS-096 v4);
`paused` (checkbox desmarcado) não lê nada. Sobe dentro do `AskBar` entre
`askbar__keys` e o divisor antes do botão de captura, e no composer da aba
Sessão. CSS novo em `src/styles` (tokens `--oh-*`, BR-035): pill hairline,
cyan para ativa (papel "foco"), outline para os estados neutros (mesma
linguagem de "pausado" do notch).

### 4.2 `HudSettings.tsx`

Novo card "Contexto de tela" logo após o card de Privacidade: toggle
(`screen_context_enabled`), descrição com o aviso de privacidade (BR-105) e
linha de status chamando um command novo `screen_context_status -> "available"
| "unavailable" | "disabled"` (o status só consulta a porta se a flag estiver
ligada; desligada → "disabled", sem instanciar nada).

## 5. Migração/compatibilidade

- `Settings.screen_context_enabled` com `serde(default)` (Rust) e default
  `false` no objeto TS; nenhum campo novo no banco (settings vivem em JSON
  store, como os demais). `screen_context_include` (checkbox) com default
  `true`; desmarcado não lê nem envia (BR-106 v3) e o cache do core é
  descartado no setter.
- Commands IPC do contexto (`screen_context_snapshot`/`_status`) são
  `#[tauri::command(async)]` — a leitura do AT-SPI nunca segura a main thread
  (ISS-097).
- `JournalRequestSource` **sem** novo variant (contexto é campo de prompt).
- AskBar com flag desligada: sem `ContextPill` no DOM (não é `display:none` —
  não renderiza), garantindo o CA-02.1.
