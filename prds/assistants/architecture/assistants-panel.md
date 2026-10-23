# Arquitetura — Painel de Assistentes

Detalhe de implementação da PRD [`../prd.md`](../prd.md). Cobre a aba
"Assistentes", o modal de agente, a semeadura dos agentes de fábrica no core e o
restilo da barra de abas em pílulas.

---

## 1. Visão geral

A aba "Assistentes" substitui o formulário atual (`AssistantSettings.tsx`) por
um painel de cards. Nada muda no modelo de persistência: os agentes continuam
em `settings.json`, dentro de `custom_assistant_modes` (BR-047 — um "Salvar
Alterações" global).

```
SettingsPage
└── nav.settings__tabs            ← pílulas (BR-078)
└── .settings__body
    ├── tab "sources"        → AiSourcesSettings
    ├── tab "transcription"  → (inline)
    ├── tab "recording"      → RecordingSettings
    ├── tab "assistant"      → AssistantsSettings        ← novo painel
    │                          ├── AssistantsSettings
    │                          │     ├── .agent-card--default  (agente Padrão, protegido)
    │                          │     ├── .settings__agents-title "AGENTES CONFIGURADOS (n)"
    │                          │     ├── AgentCard × n
    │                          │     └── .agent-cta "Criar Novo Assistente de IA"
    │                          └── AgentEditorDialog (Dialog in-page)
    ├── tab "general"        → HudSettings
    │                          └── .settings__section "Cadências & Perguntas Prontas"  ← movido
    └── tab "shortcuts"      → (inline)
```

## 2. Componentes e arquivos (frontend)

| Arquivo | Ação | Responsabilidade |
|---|---|---|
| `src/features/settings/AssistantsSettings.tsx` | **novo** | O painel: card do Padrão, contagem, lista, CTA e a posse do estado do modal |
| `src/features/settings/AgentCard.tsx` | **novo** | Um agente: tile, nome, badges, ações e a leitura dos dois prompts |
| `src/features/settings/AgentEditorDialog.tsx` | **novo** | O formulário de criar/editar, dentro de um `Dialog` |
| `src/features/settings/CadenceSettings.tsx` | **novo** | A seção nova da "Geral & HUD" (cadências + perguntas prontas) |
| `src/features/settings/AssistantSettings.tsx` | **remover** | Substituído por `AssistantsSettings` + `CadenceSettings` |
| `src/components/Dialog.tsx` | **novo** | Primitivo de modal in-page (o app não tinha nenhum) |
| `src/components/icons.tsx` | **editar** | `PencilIcon`, `StarIcon`, `PlusIcon` (BR-008: SVG do projeto) |
| `src/components/icons.test.tsx` | **editar** | Registrar os ícones novos no contrato |
| `src/core/domain/assistant.ts` | **editar** | `summary_prompt`, `PROTECTED_MODE_ID`, `agentSummaryPrompt()` |
| `src/core/domain/settings.ts` | **editar** | `assistant_modes_seeded` no tipo + `defaultSettings()` |
| `src/adapters/ipc/settings.ts` | **editar** | `settingsDefaults()` (defaults do core) |
| `src/features/settings/SettingsPage.tsx` | **editar** | Rótulo da aba, "Restaurar Padrões" usando `settings_defaults` |
| `src/styles/globals.css` | **editar** | Classes do painel, do modal e a barra em pílulas |

### 2.1 Contrato do `Dialog`

```tsx
export interface DialogProps {
  /** Abre/fecha o modal (controlado). */
  open: boolean;
  /** Título do modal (rotulado por `aria-labelledby`). */
  title: string;
  /** Linha de apoio sob o título. */
  subtitle?: string;
  /** Fecha (botão ✕, backdrop e `Esc`). */
  onClose: () => void;
  /** Conteúdo do formulário. */
  children: ReactNode;
  /** Rodapé com as ações (padrão: "Cancelar" / "Salvar"). */
  footer?: ReactNode;
}
```

Regras do primitivo:

- `<div className="dialog__backdrop">` + `<div className="dialog__card"
  role="dialog" aria-modal="true" aria-labelledby={id}>`.
- `Esc` fecha; o clique no backdrop fecha; o `✕` fecha.
- O foco entra no modal ao abrir (primeiro campo) e o `Tab` fica dentro dele.
- O `overflow` do `settings__body` é travado enquanto o modal está aberto.

## 3. Core (Rust)

### 3.1 `domain/assistant.rs`

```rust
/// Id do agente **protegido** (o Padrão): nunca editado nem apagado.
pub const PROTECTED_MODE_ID: &str = DEFAULT_MODE_ID; // "default"

pub struct AssistantMode {
    pub id: String,
    pub name: String,
    pub category: String,
    pub system_prompt: String,
    pub suggestion_prompt: String,
    /// Prompt do resumo (usado pelo `SummaryService` quando o agente está
    /// ativo — BR-076/BR-084).
    #[serde(default)]
    pub summary_prompt: String,
    pub context: String,
    pub icon: String,
    pub color: String,
    pub source_id: Option<String>,
}
```

Mudanças em `validate_mode`:

| Regra | Antes | Depois |
|---|---|---|
| id reservado | rejeita **todos** os ids embutidos | rejeita **só** `PROTECTED_MODE_ID` (`default`) |
| "ao menos um prompt" | `system_prompt` \| `suggestion_prompt` \| `context` | + `summary_prompt` |
| limite de prompt | 16 384 | 16 384 (idem para `summary_prompt`) |

`validate_custom_modes` segue rejecting ids duplicados na lista.

> Consequência: um agente de fábrica materializado (id `meeting`, …) passa a ser
> um agente **do usuário** — é isso que habilita editar/excluir (BR-075).

### 3.2 `application/prompts/assistant.rs`

```rust
/// Regras gerais de leitura da transcrição (BR-082), injetadas em todo agente.
pub const TRANSCRIPT_CONTRACT: &str = "…";

/// O agente **protegido** (Padrão) — sempre existe, nunca é editado.
pub fn default_mode() -> AssistantMode { /* id "default" */ }

/// Os agentes **de fábrica** (semente da lista do usuário — BR-075).
pub fn factory_modes() -> Vec<AssistantMode> { /* 4 agentes */ }

/// Catálogo efetivo: o protegido + a lista do usuário.
pub fn effective_modes(custom: &[AssistantMode]) -> Vec<AssistantMode> {
    let mut modes = vec![default_mode()];
    modes.extend(custom.iter().cloned());
    modes
}

/// Resolve um agente: o protegido pelo id; qualquer outro, só na lista do usuário.
pub fn resolve_mode(id: &str, custom: &[AssistantMode]) -> Option<AssistantMode> {
    if id == PROTECTED_MODE_ID {
        return Some(default_mode());
    }
    custom.iter().find(|mode| mode.id == id).cloned()
}
```

Os **quatro agentes de fábrica** e o **agente protegido** nascem com
`summary_prompt` = o contrato de resumo já existente (`SUMMARY_SYSTEM_PROMPT`),
para que o card mostre **dado real** e não texto de exemplo.

**Contrato comum de leitura (BR-082):** o `TRANSCRIPT_CONTRACT` é o preâmbulo de
`suggestion_system_prompt` (todos os agentes, inclusive personalizados) e de
`fast_first_system_prompt`. O contrato do agente fica só com o que é específico
da cena; as linhas de ME/THEY/ASR saíram dos prompts de fábrica. O `AskService`
explica ME/THEY no próprio `ASK_RULES`, sem herdar a prioridade do THEY (lá a
pergunta do usuário é a prioridade); o `SUMMARY_SYSTEM_PROMPT` mantém o contrato
profundo e ganha a definição de ME/THEY.

**Fallback de prompt vazio (BR-079):** `effective_suggestion_prompt`,
`effective_system_prompt` e `effective_summary_prompt` devolvem o valor do
agente ou, quando vazio depois do `trim`, o do agente **Padrão**. O
`suggestion_system_prompt` usa os helpers — o `## Domain criteria` passa a ser
sempre incluído (do agente ou do Padrão). O `context` fica fora da regra.

### 3.3 `domain/settings.rs`

```rust
/// Agentes materializados do usuário (a lista "AGENTES CONFIGURADOS").
pub custom_assistant_modes: Vec<AssistantMode>,

/// Agentes de fábrica já materializados na lista do usuário (BR-075).
#[serde(default)]
pub assistant_modes_seeded: bool,

/// Revisão do catálogo de fábrica (BR-083); `0` = settings anteriores.
#[serde(default)]
pub assistant_catalog_revision: u32,
```

- `Settings::default()` → `custom_assistant_modes: Vec::new()`,
  `assistant_modes_seeded: false` e `assistant_catalog_revision: 0`. O domínio
  **não** conhece o catálogo de fábrica (que vive na aplicação) — R1. A
  materialização é da aplicação (§3.4).
- `selected_mode_exists()` → `id == PROTECTED_MODE_ID || está na lista`.
- `sanitized()` → descarta agentes inválidos e, se o selecionado sumiu, volta
  para `default`.
- `validate()` → `validate_custom_modes` (agora permite ids de fábrica).

### 3.4 Semeadura e revisões do catálogo (migração de boot)

Novo caso de uso em
`src-tauri/src/application/assistant_catalog_migration.rs`:

```rust
pub struct SeedOutcome {
    pub added: Vec<String>,
    pub seeded: bool,
    pub upgraded: bool,
}

pub struct AssistantCatalogMigration { settings: Arc<dyn SettingsStore> }

impl AssistantCatalogMigration {
    pub fn seed(&self) -> Result<SeedOutcome> {
        let mut settings = self.settings.load()?;
        let normalized = normalize_legacy_prompts(&mut settings); // BR-082
        if settings.assistant_modes_seeded {
            let upgraded = upgrade_catalog(&mut settings);        // BR-083
            if normalized || upgraded {
                self.settings.save(&settings)?;
            }
            return Ok(SeedOutcome { added: vec![], seeded: true, upgraded });
        }
        let mut added = Vec::new();
        for mode in factory_modes() {
            if !settings.custom_assistant_modes.iter().any(|m| m.id == mode.id) {
                settings.custom_assistant_modes.push(mode.clone());
                added.push(mode.id);
            }
        }
        settings.assistant_modes_seeded = true;
        settings.assistant_catalog_revision = ASSISTANT_CATALOG_REVISION;
        self.settings.save(&settings)?;
        Ok(SeedOutcome { added, seeded: true, upgraded: false })
    }
}
```

Regras:

- A **semeadura** roda **uma vez** (a marca `assistant_modes_seeded`) e não
  sobrescreve um agente que o usuário já tenha com o mesmo id.
- A **revisão do catálogo** roda uma vez por revisão
  (`assistant_catalog_revision < ASSISTANT_CATALOG_REVISION`, definida em
  `application/assistant_catalog.rs`):
  - revisão 1 (**BR-083**): remove `system_design`, sobrescreve
    `technical_interview` com o prompt fundido (preserva `context` e
    `source_id`; carrega o contexto/fonte do system design quando o técnico não
    tinha), adiciona `hr_interview` e troca a seleção ativa de `system_design`
    para `technical_interview`;
  - depois da revisão, edições e exclusões do usuário são respeitadas (um
    `hr_interview` excluído não volta).
- Roda também em **instalação nova**: `Settings::default()` nasce vazio e com
  revisão 0 (§3.3), então o primeiro boot materializa os **4 agentes atuais**
  (Reunião, Entrevista Técnica, LeetCode e Entrevista com RH) e grava a revisão.
- Um `assistant_mode_id` de agente semeado continua válido; só a revisão troca
  `system_design` pelo técnico fundido.

Wiring: `lib.rs` (composition root) instancia e chama `seed()` no boot, ao
lado da migração do agente Padrão (BR-059); `outcome.upgraded` loga
"catálogo de agentes atualizado".

### 3.5 Defaults materializados (para o "Restaurar Padrões")

O default do **domínio** não tem os agentes de fábrica (§3.3). A aplicação
expõe o default **materializado**, usado pelo `settings_defaults`:

```rust
// application/assistant_catalog.rs
pub fn default_settings() -> Settings {
    let mut settings = Settings::default();
    settings.custom_assistant_modes = factory_modes();
    settings.assistant_modes_seeded = true;
    settings.assistant_catalog_revision = ASSISTANT_CATALOG_REVISION;
    settings
}
```

### 3.6 Resumo por agente (BR-076/BR-084)

O `SummaryService` carrega as settings, resolve o agente **ativo**
(`assistant_mode_id`; id ausente → Padrão) e usa
`effective_summary_prompt(&mode)`:

- **primeira geração / regeneração manual** (`current = None`): system prompt =
  `summary_prompt` do agente; vazio depois do `trim` → `SUMMARY_SYSTEM_PROMPT`
  do Padrão (BR-079);
- **atualização incremental** (`Some(previous)`): system prompt =
  `SUMMARY_UPDATE_SYSTEM_PROMPT`; o resumo corrente continua sendo a base da
  estrutura. As seções existentes (inclusive as marcadas como "não
  mencionado") são preservadas;
- trocar o agente no meio da sessão não reestrutura o resumo já gerado; a nova
  estrutura vale na próxima regeneração manual ou sessão.

O agente **Entrevista com RH** tem `summary_prompt = HR_SUMMARY_PROMPT`, com as
seções **Overall summary**, **What I answered**, **What the recruiter said
about the company**, **Benefits and salary expectations**, **Main questions and
answers** e **Brief opinion and insight on how I did**; seções sem informação
recebem "Not discussed so far in the session." (traduzido).

A **revisão de catálogo 2** (BR-084) atualiza o `summary_prompt` do RH já
semeado **somente** quando ele ainda é o contrato geral antigo; prompt editado
ou vazio é preservado, e um RH excluído não volta.

### 3.7 Sugestão: passes rápido e completo em paralelo (BR-088)

No `SuggestionService::generate`, quando `fast_first` está ligado:

- o **rascunho** roda em uma thread de escopo (`std::thread::scope`) com o
  `complete_for` (não-streaming);
- o **completo** roda no thread atual com `complete_streaming_for`;
- os dois começam juntos; o rascunho não bloqueia mais a completa;
- um `AtomicBool` `full_emitted` é marcado no primeiro delta do completo. O
  rascunho que terminar depois disso é **descartado** (não emite `quick`);
- uma falha no rascunho só gera log; a completa segue;
- o `useSuggestions` mantém um `Set` de `generation_id` cuja completa já
  chegou e ignora uma fase `quick` atrasada da mesma geração (defesa dupla);
- o custo é uma chamada concorrente ao provider — sujeita a rate limit; o
  rascunho é descartável por definição.

## 4. IPC

| Command | Entrada | Saída | Mudança |
|---|---|---|---|
| `assistant_modes_list` | — | `Vec<AssistantMode>` | Devolve `effective_modes(&settings.custom_assistant_modes)` (protegido + lista) |
| `settings_defaults` | — | `Settings` | **novo**: default **materializado** da aplicação (`default_settings()` — §3.5) |
| `settings_get` / `settings_set` | — | — | Ganham o campo `summary_prompt` e `assistant_modes_seeded` (serde) |

`assistant_modes_list` deixa de concatenar builtins + custom (o que geraria
duplicata quando um agente de fábrica é materializado) e passa a usar
`effective_modes`.

## 5. "Restaurar Padrões"

Hoje o botão monta `defaultSettings()` **no frontend**. Com os agentes de
fábrica semeados, o frontend não tem os contratos (vêm do Rust). O botão passa
a pedir ao core:

```ts
const defaults = await settingsDefaults();          // command novo (§3.5)
setDraft({ ...defaults, shortcuts: defaultBindings() });
```

`Settings::default()` (Rust) e `defaultSettings()` (TS) já espelham campo a
campo (ver
[`../../../src/core/domain/settings.ts`](../../../src/core/domain/settings.ts));
a task 02 confere a paridade e alinha o que divergir. O `settings_defaults`
parte do default do domínio e acrescenta os agentes de fábrica — a paridade é
sobre os **demais campos**.

## 6. Estado do frontend

Sem estado novo no `SettingsPage`: o painel trabalha sobre o **rascunho**
(`draft`), como o resto da tela (BR-047).

| Estado | Onde | Observação |
|---|---|---|
| Lista de agentes | `draft.custom_assistant_modes` | a fonte da verdade do painel |
| Agente ativo | `draft.assistant_mode_id` | só para a badge "Ativo" (informativa) |
| Agente Padrão | `assistant_modes_list()` (catálogo) | card do topo, read-only; fallback estático com as constantes do domínio se o catálogo não carregar |
| Modal | `AssistantsSettings` local | `{ mode: AssistantMode \| null, mode: "create" \| "edit" }` |

O modal grava no rascunho (`update("custom_assistant_modes", next)`); quem
persiste é o "Salvar Alterações" global.

O card do Padrão também recebe a badge "Ativo" quando
`draft.assistant_mode_id === PROTECTED_MODE_ID`.

## 7. CSS — mapa de classes

Tokens: `--oh-accent`, `--oh-text-strong`, `--oh-text-muted`, `--oh-border`,
`--oh-border-strong`, `--oh-surface-container`, `--oh-agent-*` (BR-037).

### 7.1 Barra de abas em pílulas (BR-078)

```css
.settings__tabs {
  display: flex;
  flex-wrap: wrap;
  gap: 0.3rem;
  padding: 0.55rem 1.1rem;
  border-bottom: 1px solid var(--oh-border);
}

.settings__tab {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  font: inherit;
  font-size: 0.75rem;
  color: var(--oh-text-muted);
  background: transparent;
  border: 1px solid transparent;
  border-radius: 999px;
  padding: 0.35rem 0.7rem;
  cursor: pointer;
}

.settings__tab--active {
  color: var(--oh-accent);
  border-color: color-mix(in srgb, var(--oh-accent) 45%, transparent);
  background: color-mix(in srgb, var(--oh-accent) 10%, transparent);
}
```

O sublinhado (`border-bottom: 2px solid transparent`) sai; o `margin-bottom: -1px`
também (a pílula não encosta na borda da barra).

### 7.2 Painel de agentes

| Classe | Elemento |
|---|---|
| `.settings__agents` | wrapper do painel |
| `.agent-card` | card de agente (borda + raio) |
| `.agent-card--active` | agente em uso (borda ciano) |
| `.agent-card--default` | card do agente Padrão (topo) |
| `.agent-card__head` | linha do topo do card |
| `.agent-card__tile` | quadrado do ícone (borda na cor do agente) |
| `.agent-card__name` | nome do agente |
| `.agent-card__badges` | badges ("Ativo", "Primário") |
| `.agent-card__actions` | ações (Editar / duplicar / excluir) |
| `.agent-card__actions .settings__button--small` | "Editar" (lápis + texto) |
| `.agent-card__actions .icon-button` | duplicar e excluir (botões de ícone do mockup) |
| `.agent-card__prompts` | grade de 2 colunas (1 col. em telas estreitas) |
| `.prompt-panel` | painel de prompt (fundo levemente elevado) — **mesmo formato em todos os cards** (o mockup mostra três níveis ilustrativos; a app usa um só) |
| `.prompt-panel__title` | "Prompt de Sugestão" (ícone `BoltIcon`) / "Prompt de Resumo" (ícone `SummaryIcon`) |
| `.prompt-panel__text` | texto truncado (`-webkit-line-clamp: 3`; o texto completo fica no modal) |
| `.settings__agents-title` | "AGENTES CONFIGURADOS (n)" — uppercase, `letter-spacing`, `--oh-text-muted` (mockup) |
| `.agent-cta` | "Criar Novo Assistente de IA" — linha/card de borda sutil, ícone `PlusIcon` + texto em negrito à esquerda e a nota "Configure personas sob medida para suas reuniões" em `--oh-text-muted` à direita |

A cor do agente entra no card por `--agent-color` (mesmo padrão do
`AgentBadge`/`agent-color--*`): o tile usa `color-mix` do token para borda e
fundo, e o ícone herda `currentColor` (BR-037 — sem hex solto).

### 7.3 Modal

| Classe | Elemento |
|---|---|
| `.dialog__backdrop` | fundo escurecido (fixo, cobre a janela) |
| `.dialog__card` | o modal (card do design system) |
| `.dialog__header` / `.dialog__title` / `.dialog__subtitle` | cabeçalho |
| `.dialog__body` | formulário (rolável) |
| `.dialog__footer` | ações ("Cancelar" / "Salvar") |

## 8. Acessibilidade

- A barra de abas mantém `role="tablist"` / `role="tab"` / `aria-selected`.
- O card do agente Padrão é um `article` com `aria-label` — **sem** botões de
  edição/exclusão (não há nada a fazer nele).
- "Editar", "Duplicar" e "Excluir" são botões com `aria-label` explícito
  ("Editar agente X", "Duplicar agente X", "Excluir agente X"), pois os de
  ícone não têm texto visível.
- O `Dialog` tem `role="dialog"`, `aria-modal="true"` e `aria-labelledby`.
- A badge "Ativo" é texto (não só cor) — a cor do agente é **coadjuvante**.
- A cor do agente continua sendo o **token** do design system, nunca hex
  (BR-037).

## 9. Matriz de testes

### 9.1 Rust (`cargo test`)

| Caso | Onde |
|---|---|
| `validate_mode` aceita um agente com só `summary_prompt` | `domain/assistant.rs` |
| `validate_mode` rejeita id `default` (protegido) | `domain/assistant.rs` |
| `validate_mode` aceita id de fábrica (`meeting`) | `domain/assistant.rs` |
| `resolve_mode("default", [])` devolve o agente protegido | `application/prompts/assistant.rs` |
| `resolve_mode("meeting", [])` devolve `None` (excluído de vez) | `application/prompts/assistant.rs` |
| `effective_modes` = protegido + lista, sem duplicata | `application/prompts/assistant.rs` |
| semeadura acrescenta os 4 de fábrica uma única vez | `application/assistant_catalog_migration.rs` |
| semeadura não sobrescreve agente do usuário com o mesmo id | idem |
| revisão do catálogo funde Entrevista Técnica + System Design e adiciona RH | idem |
| revisão não ressuscita um agente excluído depois dela | idem |
| revisão 2 troca o prompt de resumo do RH apenas se ainda for o geral | idem |
| `SummaryService` usa o `summary_prompt` do agente ativo | `application/summary_service.rs` |
| prompt de resumo vazio cai no do Padrão | idem |
| atualização incremental usa o prompt incremental | idem |
| `HR_SUMMARY_PROMPT` tem as seis seções do debrief | `application/prompts/summary.rs` |
| passes rápido e completo começam juntos; rascunho atrasado é descartado | `application/suggestion_service.rs` |
| `useSuggestions` ignora `quick` atrasada da mesma geração | `core/application/useSuggestions.test.tsx` |
| `default_settings()` traz os 4 agentes de fábrica | `application/assistant_catalog.rs` |
| `Settings::default()` nasce **vazio** (domínio puro) | `domain/settings.rs` |
| `sanitized()` volta para `default` se o selecionado sumiu | `domain/settings.rs` |
| `assistant_modes_list` devolve o catálogo efetivo | `ipc/suggestion_commands.rs` |
| `settings_defaults` devolve o default materializado | `ipc/settings_commands.rs` |

> Os literais `AssistantMode { … }` dos testes existentes
> (`domain/settings.rs`, `domain/assistant.rs`, `application/prompts/assistant.rs`,
> `application/suggestion_service.rs`, `application/assistant_mode_migration.rs`)
> ganham `summary_prompt` na task 01.

### 9.2 Frontend (`pnpm test`)

| Caso | Onde |
|---|---|
| a aba mostra o Padrão no topo, sem ações | `AssistantsSettings.test.tsx` |
| o Padrão mostra "Primário" + "Ativo" quando é o agente em uso | idem |
| a contagem de "AGENTES CONFIGURADOS (n)" bate | idem |
| "Editar" abre o modal com os valores do agente | idem |
| o modal salva as alterações no rascunho | idem |
| "Criar Novo Assistente de IA" abre o modal vazio | idem |
| o modal recusa nome vazio / nenhum prompt | idem |
| "Excluir" remove o agente da lista | idem |
| excluir o agente ativo volta a seleção para `default` | idem |
| o card mostra os dois prompts (sugestão e resumo) | `AgentCard.test.tsx` |
| `Dialog` fecha com `Esc` e no backdrop | `Dialog.test.tsx` |
| a aba ativa da barra é a pílula | `SettingsPage.test.tsx` |
| "Restaurar Padrões" pede os defaults ao core | `SettingsPage.test.tsx` |
| cadências e perguntas prontas na "Geral & HUD" | `CadenceSettings.test.tsx` |

> Fixtures TS de `AssistantMode` (`AgentSelector.test.tsx`) ganham
> `summary_prompt` na task 04.

## 10. Compatibilidade (`settings.json`)

| Situação | Efeito |
|---|---|
| Arquivo antigo (sem `summary_prompt`) | `serde(default)` ⇒ `""`; `sanitized()` normaliza |
| Arquivo antigo (sem `assistant_modes_seeded`) | `false` ⇒ a migração semeia os 4 de fábrica |
| `assistant_mode_id` = `meeting` (agente de fábrica) | Continua válido: antes da semeadura o id de fábrica é aceito; depois, o agente está na lista (ISS-060) |
| `assistant_mode_id` = id de agente **custom** | Intacto |
| `custom_assistant_modes` com ids repetidos | `validate_custom_modes` rejeita (já existia) |

## 11. Seletor global de agentes (overlay) — BR-080

### 11.1 Geometria da janela do overlay

O overlay é uma janela Tauri ancorada no topo útil do monitor; a barra ocupava
o primeiro pixel. Para o seletor flutuar acima:

| Peça | Valor |
|---|---|
| `AGENT_SELECTOR_BAND` (`domain/hud_layout.rs`) | **34 px** lógicos (26 da pílula + 6 de respiro + 2 de folga) |
| Altura da janela | `AGENT_SELECTOR_BAND + densidade.window_height()` |
| Altura inicial (`tauri.conf.json`) | 118 (84 + 34, densidade padrão) |
| Largura | a da **pilha** (`.overlay__stack`), medida pela UI |

O topo da janela continua em `y = work_area.y`; a barra desce 34 px dentro da
janela. A âncora (`top_center` / `top_right` / livre) não muda.

```
janela (118 px)
├── 26 px  .agent-switcher (pílula)
├──  6 px  gap
├── 52 px  .overlay__bar
├──  6 px  gap (quando o banner do ASR existe)
├── 20 px  banner/erro
└──  8 px  padding inferior
```

O menu do seletor **não vive mais dentro do overlay**: é uma janela Tauri
própria (`menu`, largura 300, altura dinâmica pelo conteúdo), posicionada pelo
core **fora do notch** — abaixo da barra ou acima do pill, a 6 px. Abrir/fechar
não muda o tamanho nem a posição do notch. O pill no overlay envia o `DOMRect`
do botão e o retângulo da pilha (BR-072/ISS-062,
`notch/tasks/20-task.md`).

### 11.2 Componente e CSS

- `src/features/overlay/AgentSwitcher.tsx`: pílula + popover (mesma interação do
  antigo `AgentSelector`: setas, `Home`/`End`, `Esc`, clique fora).
- `--agent-color` no root `.agent-switcher` pinta o ícone (BR-037).
- Classes: `.overlay__stack`, `.agent-switcher`, `.agent-switcher__button`,
  `__icon`, `__name`, `__divider`, `__hint`, `__popover`, `__item`,
  `__item--on`, `__item-name`, `__empty`.
- O `AgentSelector` da janela de Sugestões foi **removido** (componente, teste
  e CSS `.agent-selector*`).

### 11.3 IPC

| Command | Efeito |
|---|---|
| `settings_set_assistant_mode` (**novo**) | carrega as settings, troca **só** `assistant_mode_id`, valida, salva e emite `settings_changed` |

Não é um `settings_set` completo de propósito: o overlay fica sempre montado e
um snapshot poderia sobrescrever uma edição feita em outra janela.

### 11.4 Tempo real

- `Overlay` já usa `useSettings` (assina `settings_changed`) → o seletor recebe
  o novo `assistant_mode_id` na hora.
- `useAssistantModes(settings.custom_assistant_modes)` recarrega o catálogo
  quando os agentes mudam (criar/editar/excluir nas Configurações).
- `SettingsPage` ganhou uma assinatura de `settings_changed` **só** para
  sincronizar `assistant_mode_id` (badge "Ativo"); o resto do rascunho fica
  intocado para não atrapalhar edição/autosave da opacidade.

### 11.5 Testes

| Caso | Onde |
|---|---|
| pílula mostra nome, ícone na cor e "Alternar agente" | `AgentSwitcher.test.tsx` |
| menu lista, marca o ativo e escolhe | idem |
| `Esc`, setas/`Home`/`End`, clique fora | idem |
| `onOpenChange` ao abrir/fechar | idem |
| fallback do Padrão sem catálogo | idem |
| Sugestões não tem mais o seletor | `SuggestionsWindow.test.tsx` |
| Settings sincroniza o agente de outra janela | `SettingsPage.test.tsx` |

## 12. Tooltips de ajuda dos campos (BR-081)

### 12.1 Componente

`src/components/InfoTooltip.tsx`:

```tsx
<InfoTooltip label="Prompt de Sugestão" text="…" />
```

- gatilho: `<button type="button">` com `InfoIcon` e
  `aria-label="Ajuda: <campo>"`; abre no hover **e** no foco;
- balão: `role="tooltip"` + `id` ligado por `aria-describedby`, renderizado em
  **portal** (`document.body`) com `position: fixed` — não sofre com o
  `overflow` do `.settings__body`/`.dialog__body`;
- posição decidida por `tooltipPosition` (`src/core/domain/tooltip.ts`, função
  **pura** e testada): prefere abaixo; sem espaço, abre acima; centraliza no
  gatilho e prende nas quatro bordas da janela (inclusive no painel da
  direita);
- recalcula ao rolar/redimensionar (o balão acompanha o gatilho);
- `pointer-events: none` no balão (não bloqueia cliques).

### 12.2 Markup dos campos

O botão de ajuda **não** pode ficar dentro do `<label>` (um `<button>` dentro
de um label vira o controle rotulado). O padrão é:

```tsx
<div className="settings__field">
  <div className="settings__field-label">
    <label htmlFor="agent-suggestion-prompt">Prompt de Sugestão</label>
    <InfoTooltip label="Prompt de Sugestão" text="…" />
  </div>
  <textarea id="agent-suggestion-prompt" … />
</div>
```

`getByLabelText` continua funcionando pelo `htmlFor`/`id`.

### 12.3 Onde

| Local | Campos |
|---|---|
| Modal de edição | Prompt de Sugestão, Prompt de Resumo, Categoria, Prompt de sistema, Contexto fixo |
| Card (painéis) | Prompt de Sugestão, Prompt de Resumo |

### 12.4 CSS

`.settings__field-label` (rótulo + ícone), `.info-tooltip`,
`.info-tooltip__button`, `.info-tooltip__bubble`, `.prompt-panel__help`.
O balão usa `max-width: min(22rem, 60vw)` e `z-index: 60` (acima dos inputs e
do popover do seletor, que é 40).
