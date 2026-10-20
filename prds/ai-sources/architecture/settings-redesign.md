# Arquitetura — Casca das Configurações

Repaginação da tela de Configurações: cabeçalho próprio, barra de abas com
ícones, conteúdo em *cards* e rodapé fixo com salvamento global.

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

A janela continua com a moldura do sistema (`decorations: true`): o cabeçalho
novo é **dentro** do conteúdo, como no resto do app (Obsidian HUD — BR-035).

## 2. Estrutura da tela

```
<main class="app settings" data-route="settings">
  <header class="settings__header">
    <span class="settings__header-icon">…</span>
    <div>
      <h1 class="settings__header-title">Configurações</h1>
      <p class="settings__header-subtitle">Personalize captura de áudio, IA local, privacidade e atalhos</p>
    </div>
    <kbd class="settings__header-hint">Esc</kbd>
    <button class="settings__header-close" aria-label="Fechar">✕</button>
  </header>

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

- `.settings__header`, `.settings__header-icon`, `.settings__header-title`,
  `.settings__header-subtitle`, `.settings__header-hint`, `.settings__header-close`
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

## 7. Aba "Geral & HUD" (v0.2)

Referência: mockup do usuário (2026-10-20). A aba ocupa o lugar de "Ajustes" e
reúne o **comportamento do HUD** num cabeçalho de seção + grade de 2 colunas.

```
Comportamento do HUD & Janela                         HUD Anchor: Topo Central
┌──────────────────────────────┐ ┌──────────────────────────────┐
│ Opacidade do Overlay   [75%] │ │ Posição de Ancoragem do Notch│
│ transparência da superfície  │ │ ponto de fixação na área útil│
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

### 7.1 Componentes

| Componente | Papel |
|---|---|
| `src/features/settings/HudSettings.tsx` | conteúdo da aba: cabeçalho da seção + grade + cards |
| `SettingsPage.tsx` | troca o bloco inline de "Ajustes" por `<HudSettings …/>` |
| `src/styles/globals.css` | `.settings__grid`, `.settings__section-header`, `.settings__segmented`, `.settings__toggle`, `.settings__badge`, `.settings__range-labels` e o novo estado visual das abas |

### 7.2 Dados e validação

| Campo | Tipo | Padrão | Faixa/valores |
|---|---|---|---|
| `overlay_opacity` | `f32` | `0.75` | `0.2..=1.0` (clamp no load, erro no save) |
| `notch_anchor` | `String` | `top_center` | `top_center` · `top_right` · `free` |
| `hud_density` | `String` | `standard` | `compact` · `standard` · `expanded` |
| `dwell_ms` | `u32` | `500` | `100..=1500` (era 300–2000) |

### 7.3 Janela: ancoragem e densidade

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
