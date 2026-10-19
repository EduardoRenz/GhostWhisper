# Arquitetura — Identidade do agente (ícone + cor)

Especificação da **identidade visual** dos agentes de sugestão: cada agente ganha
um **ícone** (SVG do projeto) e uma **cor** distinta, usados no seletor, na pill
do título e no histórico. Contrato em **BR-037**.

---

## 1. Domínio

`domain::assistant::AssistantMode` (Rust) e `core/domain/assistant.ts` (TS)
ganham dois campos:

```rust
/// Ícone do agente (id do catálogo; desconhecido → `sparkle`).
#[serde(default = "default_agent_icon")]
pub icon: String,
/// Cor do agente (token do design system; desconhecido → `slate`).
#[serde(default = "default_agent_color")]
pub color: String,
```

- Ambos são **opcionais** (`serde(default)`): settings/agentes antigos carregam
  sem quebra e recebem o default.
- `validate_mode` passa a checar: ícone ∈ catálogo, cor ∈ paleta.

## 2. Catálogo de ícones (ids)

| id | Desenho | Agente padrão |
|---|---|---|
| `sparkle` | faísca | fallback de qualquer agente |
| `users` | pessoas | `meeting` |
| `braces` | chaves/código | `technical_interview` |
| `code` | código (`</>`) | `leetcode` |
| `layers` | camadas/blocos | `system_design` |
| `book` | livro | — (personalizados) |
| `briefcase` | maleta | — (personalizados) |
| `terminal` | terminal | — (personalizados) |

- Os SVGs são **desenhados no projeto** (BR-008) e ficam em
  `src/components/AgentIcon.tsx` (catálogo id → componente).
- Ícone desconhecido (agente antigo/customizado) → `sparkle`.

## 3. Paleta de cores (tokens do design system, BR-035)

| token | variável CSS | uso padrão |
|---|---|---|
| `cyan` | `--oh-agent-cyan` | `meeting` |
| `violet` | `--oh-agent-violet` | `technical_interview` |
| `emerald` | `--oh-agent-emerald` | `leetcode` |
| `coral` | `--oh-agent-coral` | `system_design` |
| `slate` | `--oh-agent-slate` | neutro — fallback e personalizados sem cor escolhida |

- As cores dos agentes são **tokens próprios** (`--oh-agent-*`, em
  `styles/tokens.css`), distintas dos papéis semânticos do tema
  (`--oh-accent`, `--oh-ai`, `--oh-success`, `--oh-danger`): a identidade do
  agente não pode depender do papel da cor.
- O **domínio** guarda o **nome do token** (nunca hex): a UI resolve para a
  variável CSS e o tema continua sendo a fonte da verdade.
- A **UI** aplica a cor em: pill do agente, ícone do seletor e borda do item
  ativo do popover.

## 4. Agentes embutidos (fixos)

| id | nome | ícone | cor |
|---|---|---|---|
| `meeting` | Reunião | `users` | `cyan` |
| `technical_interview` | Entrevista Técnica | `braces` | `violet` |
| `leetcode` | LeetCode / Algoritmos | `code` | `emerald` |
| `system_design` | System Design | `layers` | `coral` |

- `application::prompts::assistant::builtin_modes()` preenche ícone e cor.
- O **snapshot** na sugestão (`mode_icon`/`mode_color`) garante que o histórico
  mostre a identidade **daquele** agente, mesmo que o usuário troque depois.

## 5. Agentes personalizados

- Ao criar/duplicar um agente nas Configurações: **ícone** (catálogo) e **cor**
  (paleta) escolhidos; default = `sparkle` + `slate`.
- A tela de Configurações mostra a prévia (ícone + pill com a cor).
- Agentes personalizados **antigos** (sem os campos) continuam válidos: entram
  com `sparkle`/`slate` via `serde(default)` + `sanitized()`.

## 6. Mapeamento no frontend

```ts
// core/domain/assistant.ts
export const AGENT_ICON_IDS = ["sparkle","users","braces","code","layers","book","briefcase","terminal"] as const;
export const AGENT_COLOR_TOKENS = ["cyan","violet","emerald","coral","slate"] as const;
export const isAgentIconId = (icon: string): boolean => ...;
export const isAgentColorToken = (color: string): boolean => ...;
// Aceita agentes antigos (sem os campos) e cai no padrão.
export const agentIconId = (mode: { icon?: string | null }): string =>
  mode?.icon && isAgentIconId(mode.icon) ? mode.icon : "sparkle";
export const agentColorToken = (mode: { color?: string | null }): string =>
  mode?.color && isAgentColorToken(mode.color) ? mode.color : "slate";
```

- `src/components/AgentIcon.tsx`: `GLYPHS: Record<string, Glyph>` (a faísca
  reaproveita o desenho do notch — BR-016).
- `src/components/AgentBadge.tsx`: pill com ícone + nome na cor do agente.

## 7. Testes

- **Rust:** default de ícone/cor em agente antigo; `validate_mode` recusa ícone e
  cor desconhecidos; `builtin_modes()` devolve os 4 com ícone/cor válidos.
- **TS:** `agentIconId`/`agentColorToken` com fallback; catálogo completo.
- **UI:** pill renderiza ícone + nome + cor; popover lista todos os agentes.
