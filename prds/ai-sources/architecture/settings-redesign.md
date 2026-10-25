# Arquitetura — Casca das Configurações

Repaginação da tela de Configurações: cabeçalho próprio, barra de abas com
ícones, conteúdo em *cards* e rodapé fixo com salvamento global. A v0.2 cobre a
aba "Geral & HUD" (§9); a v0.3 cobre o redesign da aba "Fontes de IA" (§10).

---

## 1. O que muda

| Antes | Depois |
|---|---|
| 5 abas com texto (`Fontes de IA`, `Transcrição & Áudio`, `Assistente`, `Ajustes`, `Atalhos`) | as **mesmas 5 abas**, com **ícone** + rótulo, na barra superior; **"Ajustes" virou "Geral & HUD"** (v0.2) |
| conteúdo em formulários soltos | conteúdo em **cards** (`.settings__card`); a aba do HUD usa **grade de 2 colunas** (v0.2) |
| `<h1>` "GhostWhisper — Configurações" | cabeçalho próprio: ícone + título + subtítulo + atalho `Esc` + botão `✕` |
| um "Salvar configurações" **por aba** | **um** "Salvar Alterações" no **rodapé fixo** (com "Restaurar Padrões") |
| cada aba carrega e salva as settings | **rascunho único** carregado uma vez e salvo pelo botão global |
| janela 520×640 | janela **1020×720** (mín. 760×560), redimensionável |

A janela é **sem moldura nativa** (`decorations: false`, `transparent: true`):
o cabeçalho é a única barra de título, como no resto do app (Obsidian HUD —
BR-035). O shell transparente de 8 px e `--oh-window-radius` mantêm o raio
padrão; `ResizeHandles` preserva o redimensionamento sem a moldura do SO
(BR-064).

## 2. Estrutura da tela

```
<main class="app panel-shell settings-shell" data-route="settings">
  <ResizeHandles />
  <div class="settings">
    <WindowHeader class="settings__header"
      title="Configurações"
      subtitle="Personalize captura de áudio, IA local, privacidade e atalhos"
      icon="TranscriptIcon" hint="Esc" onDrag="startWindowDrag" onClose="settings_close" />

    <nav class="settings__tabs" role="tablist">…</nav>   <!-- 5 abas com ícone -->

  <div class="settings__body">
    {tab === "sources" && <AiSourcesSettings …/>}
    {tab === "transcription" && …}
    …
  </div>

  <footer class="settings__footer">
    <span class="settings__footer-note">● Todas as alterações são sincronizadas localmente</span>
    <button class="settings__button">Restaurar Padrões</button>
    <button class="settings__button settings__button--primary">Salvar Alterações</button>
  </footer>
</main>
```

- O **tablist** usa `role="tab"`/`aria-selected` (como hoje) — o leitor de tela
  continua anunciando a aba ativa.
- O rodapé é `position: sticky; bottom: 0` dentro da coluna rolável, para
  ficar sempre visível sem sair do fluxo.

## 3. Estado: rascunho único

Hoje cada aba chama `getSettings()` e `saveSettings()` por conta. Com o botão
global, a tela passa a ter **um** rascunho:

```ts
const [draft, setDraft] = useState<Settings>(defaultSettings());
const [dirty, setDirty] = useState(false);
```

- `getSettings()` uma vez no mount.
- Qualquer edição marca `dirty` e mostra "Alterações não salvas".
- "Salvar Alterações" → `saveSettings(draft)` → `dirty = false`.
- "Restaurar Padrões" → diálogo de confirmação → `draft = defaultSettings()`
  (com os atalhos padrão) → `dirty = true` (ainda **não** salvo).
- O evento `settings_changed` (BR-036) recarrega o rascunho quando **outra**
  janela salva — mas só se não houver edição local pendente (senão a UI
  "pula" sob o usuário).

### 3.1 Atalhos: um caso especial

`shortcuts_set` **registra** os atalhos no sistema operacional; não é só um
campo do arquivo de settings. A aba "Atalhos" continua com um botão próprio
("Aplicar atalhos"), porque o registro no SO é uma ação, não um dado. O botão
global salva o **valor** dos atalhos; o botão da aba aplica no SO. A UI diz
isso explicitamente.

## 4. Componentes

| Componente | Papel |
|---|---|
| `src/features/settings/SettingsPage.tsx` | casca (header, tabs, body, footer) e o rascunho |
| `src/features/settings/AiSourcesSettings.tsx` | aba "Fontes de IA": lista de cards + formulário de adição |
| `src/features/settings/ProviderCard.tsx` | um card de fonte (badges, ações, chave) |
| `src/features/settings/AddProviderForm.tsx` | formulário "Adicionar nova fonte" |
| `src/features/settings/AssistantSettings.tsx` | agents (existente, adaptado ao layout de cards) |
| `src/features/settings/AudioTestPanel.tsx` | (existente, sem mudança) |

## 5. Catálogo de modelos por provedor

O mockup mostra um `<select>` de modelo (não texto livre). O frontend ganha um
catálogo em `src/core/domain/settings.ts`:

```ts
export const PROVIDER_MODELS: Record<ProviderKind, string[]> = { … };
```

Cada provedor lista alguns modelos conhecidos; o `select` tem a opção
**"Outro modelo…"**, que revela um input de texto livre (necessário para
`custom` e para modelos novos). O backend **não** valida o nome do modelo contra
o catálogo — a lista é orientativa (o provider é a autoridade).

## 6. Estados de saúde no card

| Badge | Origem |
|---|---|
| `Conectada · Latência 142 ms` | `AiSourceHealth.latency_ms` + estado `Ready` |
| `Falhou` + motivo | estado `Failed` + `last_error` (truncado, sem segredo) |
| `Fallback Ativo` | `is_fallback` do último uso |
| `Primário` | `ai_sources[0]` |
| `Keyring Seguro` / `Só nesta sessão` | `secretsPersistent()` |

A tela assina o evento `llm_source_used` e atualiza o card ao vivo.

## 7. CSS

Tudo em `src/styles/globals.css`, consumindo os tokens `var(--oh-*)`
(BR-035 — não repetir hex/tamanho):

- `.settings__header` (modificador do `WindowHeader`), `.window-header`,
  `.window-header__icon`, `.window-header__title`, `.window-header__subtitle`,
  `.window-header__hint`, `.window-header__close`
- `.settings__tabs`, `.settings__tab`, `.settings__tab--active`,
  `.settings__tab-icon`
- `.settings__body`
- `.settings__card`, `.settings__card-header`, `.settings__card-title`,
  `.settings__card-actions`
- `.settings__footer`, `.settings__footer-note`, `.settings__button--primary`
- `.settings__sources`, `.settings__source`, `.settings__source--primary`,
  `.settings__source--disabled`, `.settings__badge`, `.settings__badge--ok`,
  `.settings__badge--warn`, `.settings__badge--error`, `.settings__badge--muted`

## 8. Testes de frontend

`src/features/settings/SettingsPage.test.tsx` (reescrito) e
`src/features/settings/AiSourcesSettings.test.tsx` (novo):

- as 5 abas existem, com ícone e rótulo;
- o rodapé tem "Salvar Alterações" e "Restaurar Padrões";
- salvar chama `saveSettings` **uma vez** com o rascunho completo;
- "Restaurar Padrões" pede confirmação antes de zerar;
- a lista de fontes mostra os badges de saúde;
- "Tornar Primário" move a fonte para o topo;
- reordenar (drag) altera a ordem enviada no `saveSettings`;
- adicionar fonte cria a entrada no rascunho (ainda não salva);
- trocar a chave pede a nova chave e grava no keyring (IPC), sem salvar as
  settings.

## 9. Aba "Geral & HUD" (v0.2)

Referência: mockup do usuário (2026-10-20). A aba ocupa o lugar de "Ajustes" e
reúne o **comportamento do HUD** num cabeçalho de seção + grade de 2 colunas.

```
Comportamento do HUD & Janela                         HUD Anchor: Topo Central
┌──────────────────────────────┐ ┌──────────────────────────────┐
│ Opacidade das Janelas  [75%] │ │ Posição de Ancoragem do Notch│
│ transparência de todas       │ │ ponto de fixação na área útil│
│ as superfícies (tempo real)  │ │                              │
│ ──────●─────────────         │ │ [Topo Central][Canto][Livre] │
│ Discreto  Equilibrado  Opaco │ │                              │
└──────────────────────────────┘ └──────────────────────────────┘
┌──────────────────────────────┐ ┌──────────────────────────────┐
│ Expandir Notch por Dwell (●) │ │ Tamanho / Densidade do Notch │
│ Tempo de Dwell        [500ms]│ │ espaçamento e espessura      │
│ ──●──────────────────        │ │ [Compacto][Padrão][Expandido]│
│ 100ms (Rápido)   1,5s (Longo)│ │                              │
└──────────────────────────────┘ └──────────────────────────────┘
┌──────────────────────────────────────────────────────────────┐
│ Privacidade — modo stealth (fora da barra de tarefas)   [ ]   │
└──────────────────────────────────────────────────────────────┘
```

- O **cabeçalho da seção** mostra à direita o rótulo vivo `HUD Anchor: …`
  (deriva de `notch_anchor`), como no mockup.
- **Segmentado** (`.settings__segmented`): botão com borda; ativo = borda ciano +
  texto ciano + fundo ciano translúcido.
- **Toggle** (`.settings__toggle`): trilho/pílula; ligado = ciano.
- **Slider** com **badge** do valor atual (`.settings__badge`).
- Os três rótulos do slider de opacidade são **presets** clicáveis? Não — são
  legendas (`Discreto 20%` · `Equilibrado 75%` · `Opaco 100%`). O slider aceita
  qualquer passo de 5 %.
- O card de **Privacidade** (stealth) atravessa as duas colunas.
- A opacidade não é mais exclusiva do notch: o valor vira `--hud-alpha` no
  `document.documentElement` de cada webview e as superfícies de todas as
  janelas usam canais RGB + essa variável. O slider aplica um preview imediato
  (`hud_opacity_preview`) e o autosave grava só a opacidade com debounce; os
  demais campos seguem o "Salvar Alterações" global.

### 9.1 Componentes

| Componente | Papel |
|---|---|
| `src/features/settings/HudSettings.tsx` | conteúdo da aba: cabeçalho da seção + grade + cards |
| `SettingsPage.tsx` | troca o bloco inline de "Ajustes" por `<HudSettings …/>`; agenda o preview/autosave da opacidade |
| `src/core/application/useHudOpacity.ts` | aplica `--hud-alpha` no root da webview, escuta `hud_opacity_preview` e recarrega em `settings_changed` |
| `src/styles/globals.css` | `.settings__grid`, `.settings__section-header`, `.settings__segmented`, `.settings__toggle`, `.settings__badge`, `.settings__range-labels` e as superfícies com opacidade global |

### 9.2 Dados e validação

| Campo | Tipo | Padrão | Faixa/valores |
|---|---|---|---|
| `overlay_opacity` | `f32` | `0.75` | `0.2..=1.0` (clamp no load, erro no save) — nome legado; opacidade global das janelas |
| `notch_anchor` | `String` | `top_center` | `top_center` · `top_right` · `free` |
| `hud_density` | `String` | `standard` | `compact` · `standard` · `expanded` |
| `dwell_ms` | `u32` | `500` | `100..=1500` (era 300–2000) |

### 9.3 Janela: ancoragem e densidade

O adaptador `TauriOverlayWindow` passa a guardar `(density, anchor)`:

- `set_surface(ActionBar)` usa a **altura da densidade**
  (`compact` 62 · `standard` 72 · `expanded` 84 px lógicos).
- Com âncora ≠ `free`, reposiciona pela **área útil do monitor primário**:
  `x = work.x + (work.width − window.width) / 2` ou
  `x = work.right − window.width − margem`; `y = work.y + margem`.
- Com âncora = `free`, mantém a posição lembrada e o comportamento antigo de
  reancorar pela base ao redimensionar.
- `overlay_start_drag` grava `notch_anchor = free` antes de iniciar o arraste.
- `settings_set` reaplica o layout na hora (sem reiniciar).

A geometria pura vive em `domain/hud_layout.rs` (`anchor_position`,
`HudDensity::window_height`) — testável sem I/O.

## 10. Aba "Fontes de IA" (v0.3)

Referência de produto: [`../prd.md`](../prd.md) §11. O mockup do usuário
(2026-10-20) serve para hierarquia e densidade; os dados exibidos são sempre as
fontes reais. Esta revisão não altera backend, IPC, `Settings` nem keyring.

### 10.1 Estrutura da tela

```
AiSourcesSettings
├── section.settings__ai
│   ├── header.settings__section-header
│   │   ├── .settings__section-heading
│   │   │   ├── .settings__section-icon (ServerIcon)
│   │   │   └── h2 + p.settings__section-caption
│   │   └── span.settings__section-note (N fontes · keyring)
│   └── div.settings__sources
│       └── ProviderCard × N
└── AddProviderForm
    └── section.settings__card.settings__card--add
```

O cabeçalho deixa de viver dentro do card da lista. O antigo parágrafo de ajuda
longo vira o subtítulo curto do cabeçalho; a explicação detalhada de fallback
continua disponível nos cards e na documentação.

### 10.2 `ProviderCard`

```
article.settings__source[.settings__source--primary][data-testid]
├── div.settings__source-head
│   ├── span.settings__source-grip          (GripIcon, draggable)
│   ├── span.settings__source-tile          (ícone do provedor)
│   ├── div.settings__source-title
│   │   ├── div.settings__source-title-row  (nome + Pill modelo + pills)
│   │   └── p.settings__source-desc
│   └── div.settings__source-meta
│       ├── span.settings__source-status
│       ├── button.settings__button--ghost  (Tornar primário, se não primária)
│       ├── IconButton (Editar)
│       └── IconButton (Excluir fonte, se canRemove)
├── div.settings__source-footer
│   ├── div.settings__key
│   │   ├── KeyIcon
│   │   ├── code.settings__key-value
│   │   └── span.settings__key-store (LockIcon + Keyring)
│   └── div.settings__source-actions
│       ├── button (Validar chave)
│       ├── button (Trocar chave)
│       └── button (Remover chave)
├── div.settings__source-key-edit (condicional)
└── div.settings__source-edit (condicional)
```

Mapeamento de saúde → chip (`.settings__source-status--*`):

| `AiSourceHealth.state` | Texto | Cor |
|---|---|---|
| `ready` + latência | `Conectada · Latência N ms` | esmeralda |
| `ready` sem latência | `Conectada` | esmeralda |
| `degraded` | `Instável` | âmbar |
| `failed` | `Falhou` (`title` = `last_error`) | vermelha |
| `unknown`/ausente | `Nunca usada` | neutra |

`is_fallback` vira uma pill separada ("Fallback ativo"), não substitui o chip.
A primeira fonte ganha a pill "Primário" e a borda ciano; a fonte sem chave
mostra "Sem chave configurada" e a ação "Adicionar chave".

O drag & drop atual é preservado: o artigo continua `draggable`, com os mesmos
callbacks (`onDragStart`, `onDragOver`, `onDrop`). O grip é apenas o affordance
visual.

### 10.3 Formulário de adição

```
section.settings__card.settings__card--add
├── header.settings__card-header
│   ├── span.settings__card-icon (PlusCircleIcon)
│   ├── h2 + p.settings__card-description
│   └── span.settings__card-note
├── div.settings__add-grid
│   ├── label.settings__field (provedor)
│   ├── label.settings__field (modelo — texto livre, BR-049)
│   └── div.settings__field.settings__field--full (chave opcional)
│       ├── div.settings__add-key (input password)
│       └── p.settings__helper (LockIcon + texto)
└── footer.settings__add-footer
    ├── button (Cancelar)
    └── button.settings__button--primary (Adicionar)
```

Fluxo da chave (usa somente IPC existente):

1. `AddProviderForm` monta a fonte no rascunho e chama
   `onAdd(source, key?)`.
2. `AiSourcesSettings` insere a fonte com `onChange` e, se `key` não estiver
   vazia, chama `setSourceKey(source.id, key)`.
3. `refreshKeys([...sources, source])` atualiza a prévia sem recarregar a lista.
4. O "Salvar Alterações" global continua sendo o único caminho para persistir a
   lista de fontes.

"Cancelar" apenas limpa o formulário local; não remove uma fonte já
adicionada (para isso existe "Excluir fonte" no card).

### 10.4 CSS

Adicionar em `src/styles/globals.css`, sempre com tokens `var(--oh-*)`:

| Classe | Papel |
|---|---|
| `.settings__ai` | coluna da aba: cabeçalho + lista; gap `--oh-space-2` |
| `.settings__section-heading` | ícone + bloco de título do cabeçalho |
| `.settings__section-icon` | tile 2 rem, fundo ciano 10 %, borda ciano 30 % |
| `.settings__section-caption` | subtítulo curto (0,72 rem, muted) |
| `.settings__source-head` | linha do cabeçalho do card |
| `.settings__source-tile` | tile 2 rem do provedor; variante `--secondary` em violeta |
| `.settings__source-title-row` | nome + pills com `flex-wrap` |
| `.settings__source-desc` | descrição em uma linha (0,72 rem) |
| `.settings__source-meta` | coluna à direita: status + ações |
| `.settings__source-status` | chip mono compacto; variantes `--ready`, `--failed`, `--degraded` |
| `.settings__source-footer` | divisor superior + linha da chave/ações |
| `.settings__key` | grupo da chave com `min-width: 0` |
| `.settings__key-value` | chave mascarada em mono, fundo `surface-lowest` |
| `.settings__key-store` | ícone de cadeado + origem |
| `.settings__source-actions` | ações compactas com `flex-wrap` |
| `.settings__button--ghost` | botão neutro sobre `surface-container-high` |
| `.settings__button--compact` | fonte 0,72 rem, padding 0,3/0,6 rem |
| `.settings__add-grid` | grid 1 coluna; 2 colunas a partir de 760 px |
| `.settings__add-key`, `.settings__helper`, `.settings__add-footer` | chave, helper e rodapé |

O card primário usa `border-color: rgba(0, 242, 254, 0.4)` e um glow suave; os
demais usam `var(--oh-border)`. O tile secundário usa `var(--oh-secondary)`.
Não alterar os estilos globais de `.settings__button` para não afetar as outras
abas; os modificadores acima são a extensão.

### 10.5 Ícones novos

Em `src/components/icons.tsx`, no mesmo contrato (`viewBox 24`, `currentColor`,
`aria-hidden`): `KeyIcon`, `LockIcon`, `SyncIcon`, `PencilIcon`, `TrashIcon`,
`PlusCircleIcon`, `ServerIcon`. Adicionar todos ao mapa `ALL_ICONS` de
`src/components/icons.test.tsx` para garantir o contrato de acessibilidade.

### 10.6 Testes

- `ProviderCard.test.tsx`: estados, pills, chave mascarada, ações e edição.
- `AiSourcesSettings.test.tsx`: cabeçalho, contagem/keyring, chave no
  formulário, cancelar/limpar e reordenação.
- `SettingsPage.test.tsx`: continua com 5 abas e o rodapé global; nenhuma
  expectativa nova de autenticação ou cards fora do escopo.

### 10.7 Fora de escopo técnico

Nenhuma mudança em `src-tauri/`, `Settings`, `AiSource`, `SecretStore`,
`AiSourceService` ou `llm_router`. O redesign é composto por componentes React
e CSS; a única chamada nova é o `setSourceKey` já existente.
