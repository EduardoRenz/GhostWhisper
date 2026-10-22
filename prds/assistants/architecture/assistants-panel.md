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
    /// Prompt do resumo (persistido; consumo em feature futura — BR-076).
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
```

- `Settings::default()` → `custom_assistant_modes: Vec::new()` e
  `assistant_modes_seeded: false`. O domínio **não** conhece o catálogo de
  fábrica (que vive na aplicação) — R1. A materialização é da aplicação (§3.4).
- `selected_mode_exists()` → `id == PROTECTED_MODE_ID || está na lista`.
- `sanitized()` → descarta agentes inválidos e, se o selecionado sumiu, volta
  para `default`.
- `validate()` → `validate_custom_modes` (agora permite ids de fábrica).

### 3.4 Semeadura (migração de boot)

Novo caso de uso em
`src-tauri/src/application/assistant_catalog_migration.rs`:

```rust
pub struct SeedOutcome { pub added: Vec<String>, pub seeded: bool }

pub struct AssistantCatalogMigration { settings: Arc<dyn SettingsStore> }

impl AssistantCatalogMigration {
    /// Materializa os agentes de fábrica na lista do usuário (uma única vez).
    pub fn seed(&self) -> Result<SeedOutcome> {
        let mut settings = self.settings.load()?;
        if settings.assistant_modes_seeded {
            return Ok(SeedOutcome { added: vec![], seeded: true });
        }
        let mut added = Vec::new();
        for mode in factory_modes() {
            if !settings.custom_assistant_modes.iter().any(|m| m.id == mode.id) {
                settings.custom_assistant_modes.push(mode.clone());
                added.push(mode.id);
            }
        }
        settings.assistant_modes_seeded = true;
        self.settings.save(&settings)?;
        Ok(SeedOutcome { added, seeded: true })
    }
}
```

Regras:

- Roda **uma vez** (a marca `assistant_modes_seeded`).
- Roda também em **instalação nova**: `Settings::default()` nasce vazio (§3.3),
  então o primeiro boot materializa os 4.
- Não sobrescreve agente que o usuário já tenha com o mesmo id.
- Não mexe em `assistant_mode_id`: o id do agente semeado é o mesmo, então a
  seleção continua válida.

Wiring: `lib.rs` (composition root) instancia e chama `seed()` no boot, ao
lado da migração do agente Padrão (BR-059).

### 3.5 Defaults materializados (para o "Restaurar Padrões")

O default do **domínio** não tem os agentes de fábrica (§3.3). A aplicação
expõe o default **materializado**, usado pelo `settings_defaults`:

```rust
// application/assistant_catalog.rs
pub fn default_settings() -> Settings {
    let mut settings = Settings::default();
    settings.custom_assistant_modes = factory_modes();
    settings.assistant_modes_seeded = true;
    settings
}
```

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
| Altura da janela | `AGENT_SELECTOR_BAND + densidade.window_height() + popover_extra` |
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

O `popover_extra` continua crescendo **para baixo**: com o menu aberto, a
janela vai a `118 + 240`. O menu é absoluto (`top: calc(100% + 6px)`) e cobre a
barra — comportamento normal de dropdown.

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
