# Arquitetura — Design System "Obsidian HUD"

Fonte única dos **tokens visuais** do GhostWhisper (BR-035). Pedido do usuário em
2026-10-19; a **task [07](../tasks/07-task.md)** aplica estes tokens na UI.

> **Regra de leitura (BR-016):** este arquivo é o dono dos valores. Componentes,
> tasks e PRD **citam** os tokens daqui; não repetem hex/tamanho.

---

## 1. Conceito

HUD discreto sobre o desktop: vidro escuro (ink) com **blur só nas cascas**
(notch, painel, popovers) e **nunca por bolha/item** — protege o desempenho já
documentado em [`../../chat/architecture/chat-ui.md`](../../chat/architecture/chat-ui.md)
§3 (sem `backdrop-filter` por bolha; ISS-020). Acentos luminosos são **sinais**,
não decoração:

| Sinal | Cor | Uso |
|---|---|---|
| Cyan elétrico | `#00f2fe` | foco, borda ativa, transcrição ao vivo, medidores |
| Violeta neon | `#8b5cf6` | IA (sugestões, resumo em geração, agentes) |
| Esmeralda | `#10b981` | gravação/saúde da transcrição, sucesso, ação primária |
| Coral | `#f43f5e` | erro, microfone mudo, encerrar/parar |

## 2. Cores (tokens canônicos)

Os valores abaixo são os **tokens estruturados** fornecidos no design (o YAML).
Onde a prosa do design divergia (ex.: `#0B0F17` vs `#0f131c`), **vale o token**
— a divergência está registrada no §8.

| Token | Valor | Uso |
|---|---|---|
| `--oh-surface` / `--oh-surface-dim` | `#0f131c` | fundo base |
| `--oh-surface-bright` | `#353942` | realce de superfície |
| `--oh-surface-container-lowest` | `#0a0e16` | poços/insets |
| `--oh-surface-container-low` | `#181c24` | cartão baixo |
| `--oh-surface-container` | `#1c2028` | cartão padrão |
| `--oh-surface-container-high` | `#262a33` | hover/elevado |
| `--oh-surface-container-highest` | `#31353e` | popover ativo |
| `--oh-surface-variant` | `#31353e` | variante de superfície |
| `--oh-on-surface` | `#dfe2ee` | texto padrão |
| `--oh-on-surface-variant` | `#b9cacb` | texto secundário |
| `--oh-outline` | `#849495` | contorno visível |
| `--oh-outline-variant` | `#3a494b` | divisor/contorno sutil |
| `--oh-primary` | `#e0fdff` | texto sobre cyan |
| `--oh-on-primary` | `#00373a` | texto no primary |
| `--oh-primary-container` | `#00f2fe` | **cyan de acento** |
| `--oh-on-primary-container` | `#006a70` | texto no cyan |
| `--oh-secondary` | `#d0bcff` | texto violeta claro |
| `--oh-on-secondary` | `#3c0091` | texto no violeta |
| `--oh-secondary-container` | `#571bc1` | **violeta de acento** |
| `--oh-on-secondary-container` | `#c4abff` | texto no violeta |
| `--oh-tertiary` | `#e1ffec` | texto esmeralda claro |
| `--oh-on-tertiary` | `#003824` | texto no esmeralda |
| `--oh-tertiary-container` | `#67f4b7` | **esmeralda de acento** |
| `--oh-on-tertiary-container` | `#006e4b` | texto no esmeralda |
| `--oh-error` | `#ffb4ab` | texto de erro |
| `--oh-on-error` | `#690005` | texto no erro |
| `--oh-error-container` | `#93000a` | fundo de erro |
| `--oh-on-error-container` | `#ffdad6` | texto no erro |
| `--oh-background` | `#0f131c` | canvas |
| `--oh-on-background` | `#dfe2ee` | texto no canvas |

### 2.1 Aliases semânticos (o que os componentes usam)

| Alias | Valor/token | Uso |
|---|---|---|
| `--oh-text` | `var(--oh-on-surface)` | texto padrão |
| `--oh-text-strong` | `#f8fafc` (95%) | títulos/valores |
| `--oh-text-muted` | `#94a3b8` (70%) | microcopy |
| `--oh-border` | `rgba(255,255,255,0.10)` | hairline padrão |
| `--oh-border-strong` | `rgba(255,255,255,0.14)` | topo do gradiente |
| `--oh-accent` | `var(--oh-primary-container)` | foco/live/cyan |
| `--oh-ai` | `#8b5cf6` | IA/violeta |
| `--oh-success` | `#10b981` | gravação/sucesso |
| `--oh-danger` | `#f43f5e` | erro/parar |
| `--oh-warning` | **não existe** no sistema | estado "pausado" usa `--oh-outline` |
| `--oh-focus-ring` | `0 0 0 1px rgba(0,242,254,.35)` | foco |

> **Decisão de produto:** a ação primária "Começar a ouvir" mantém o **verde**
> (BR-032), agora no **esmeralda `#10b981`** do sistema. O azul de "ligado" cede
> lugar a **cyan** (foco/live) e **violeta** (IA), conforme o papel.

## 3. Tipografia

Geist (UI) + JetBrains Mono (metadados/tempos/atalhos/código). Empacotadas
**localmente** (sem CDN — o app é local-only), com fallbacks do sistema.

| Token | Família | Tam./Linha | Peso | Tracking |
|---|---|---|---|---|
| `headline-xl` | Geist | 24/32 | 600 | -0.02em |
| `headline-lg` | Geist | 18/26 | 600 | -0.015em |
| `headline-md` | Geist | 15/22 | 500 | -0.01em |
| `body-lg` | Geist | 14/20 | 400 | -0.005em |
| `body-md` | Geist | 13/18 | 400 | 0 |
| `body-sm` | Geist | 12/16 | 400 | 0 |
| `label-md` | Geist | 12/16 | 500 | 0.01em |
| `label-sm` | Geist | 11/14 | 500 | 0.02em |
| `code-sm` | JetBrains Mono | 11/14 | 400 | -0.01em |
| `status-badge` | JetBrains Mono | 10/12 | 600 | 0.04em |

- **Números tabulares** obrigatórios em timers, contadores e tempos (`font-variant-numeric: tabular-nums`).
- **Fontes:** `Geist` (SIL OFL 1.1) e `JetBrains Mono` (SIL OFL 1.1); incluir o
  arquivo de licença em `src/assets/fonts/`. `font-display: swap`.

## 4. Raios e espaçamento

| Token | Valor |
|---|---|
| `--oh-radius-sm` | `0.5rem` |
| `--oh-radius` | `1rem` |
| `--oh-radius-md` | `1.5rem` |
| `--oh-radius-lg` | `2rem` |
| `--oh-radius-xl` | `3rem` |
| `--oh-radius-full` | `9999px` |
| `--oh-gutter` | `0.75rem` |
| `--oh-margin` | `1rem` |
| `--oh-space-xs` | `0.25rem` |
| `--oh-space-sm` | `0.375rem` |
| `--oh-space-md` | `0.625rem` |
| `--oh-space-lg` | `1rem` |
| `--oh-space-xl` | `1.5rem` |

- Unidade base **4px**; gaps internos do dock em `space-xs`/`space-sm`.
- Margem de **16px** das bordas da tela (já respeitada pelo overlay).

## 5. Elevação, vidro e borda

| Token | Valor |
|---|---|
| `--oh-glass-bar` | `rgba(11,15,23,0.78)` + `blur(28px) saturate(180%)` |
| `--oh-glass-card` | `rgba(19,27,46,0.85)` + `blur(36px)` |
| `--oh-read-surface` | `rgba(15,19,28,0.94)` (leitura: painel/bolhas/configurações) |
| `--oh-shadow-rest` | `0 12px 40px -10px rgba(0,0,0,0.7)` |
| `--oh-glow-focus` | `0 0 24px -4px rgba(0,242,254,0.25), 0 8px 32px -8px rgba(0,0,0,0.6)` |

- **Borda hairline em gradiente:**
  `linear-gradient(180deg, rgba(255,255,255,0.14), rgba(255,255,255,0.03))`,
  aplicada com pseudo-elemento de `padding: 1px` + `mask` (não `border-image`,
  para preservar o raio).
- **Blur só nas cascas** (notch, painel, popovers). **Nunca** em bolha, item de
  lista ou chip (ISS-020).
- **Fallback:** `@supports not (backdrop-filter: blur(1px))` → superfície opaca
  `--oh-surface` com a mesma borda; sem blur.
- **Leitura:** superfícies de texto (painel, bolhas, configurações, histórico)
  usam `--oh-read-surface` (quase opaca) — o vidro é do HUD, não do conteúdo.

## 6. Componentes (mínimo a padronizar)

| Componente | Tratamento |
|---|---|
| **Notch / `ActionBar`** | pill de vidro (`--oh-glass-bar`), borda gradiente, `--oh-shadow-rest`; separadores `rgba(255,255,255,0.06)`; ações trailing com a **engrenagem por último** |
| **Ação primária** (Começar a ouvir) | pill esmeralda (`--oh-success`), texto escuro, glow sutil; badges em mono |
| **Ícones/`IconButton`** | alvo 28–32px circular/`--oh-radius-full`; ghost `rgba(255,255,255,.04)` + borda `.08`; hover `.12`; foco com `--oh-focus-ring` |
| **Abas do painel** | segmented switcher em `--oh-radius-full`; destaque deslizante `rgba(255,255,255,.12)` (`--oh-radius`) |
| **Switcher do notch** | segmented pill; item ativo em `surface-container-highest` + texto **cyan** + hairline; play/pause circular 28px ao lado do item ativo |
| **Drill-hover (dwell)** | fill da esquerda para a direita em cyan suave (`--oh-dwell-fill`) + linha inferior 2px (`--oh-dwell-line`); confirmação em pill de vidro com borda cyan; `popoverIn`; sem som |
| **Waveform** | 4 barras de 2px (raio full, gap 2px) cyan; altura 4–14px pelo nível de áudio; pausado estático/esmaecido |
| **Painel / `PanelFrame`** | `--oh-glass-card` na casca; corpo em `--oh-read-surface`; `--oh-radius-md` |
| **Botões** | primário (cyan hairline + hover gradiente cyan→violeta), ghost, toggle "ligado" em cyan |
| **Status** | gravação: ponto esmeralda 4px + glow 10px; pausado: `--oh-outline` (sem âmbar); erro/parar: coral |
| **Pill de resumo** | fundo violeta suave (`--oh-ai` 14%), borda 40%, spinner; título em `headline-md` |
| **Medidores** (`LevelMeters`) | preenchimento cyan→violeta; trilha `rgba(255,255,255,.10)` |
| **Bolhas** | geometria atual preservada; `--oh-read-surface`/tons; **sem blur**; rótulos `label-sm`; tempo `status-badge` mono |
| **Markdown/código** | `code-sm` (JetBrains Mono); tabelas/código em `--oh-surface-container-low`; links cyan |
| **Inputs/selects** | pill/inset `rgba(0,0,0,.35)`; foco `#00f2fe` 30% + glow |
| **Configurações/Histórico/Sugestões** | mesmos tokens; grupos em `--oh-surface-container-low` + borda hairline |

## 7. Acessibilidade e desempenho

- **Contraste:** texto padrão ≥ 4,5:1 sobre `--oh-read-surface`; microcopy ≥ 3:1.
  Testar com `--oh-text-muted` sobre o painel.
- **Foco visível** em todo controle (cyan + glow), inclusive navegação por teclado
  das abas.
- **`prefers-reduced-motion`:** sem pulsos/transições; spinner estático.
- **`prefers-contrast: more` / `forced-colors`:** remover vidro e usar contornos.
- **Blur:** apenas nas cascas; medir que o custo de composição não reintroduz a
  ISS-020 (re-render por evento de nível).

## 8. Divergências do design (resolvidas aqui)

| Ponto | Prosa do design | Token YAML | Decisão |
|---|---|---|---|
| Fundo/canvas | `#0B0F17` | `#0f131c` | **Token** (`--oh-surface`) |
| Superfície de painel | `#131B2E` | `#181c24`/`#1c2028` | **Tokens** (`container-low`/`container`) |
| Elevado | `#1C263D` | `#262a33`/`#31353e` | **Tokens** (`container-high`/`highest`) |
| Texto alto | `#F8FAFC` (95%) | `#dfe2ee` | `--oh-text` = token; `--oh-text-strong` = `#f8fafc` para títulos |
| Texto médio | `#94A3B8` (70%) | `#b9cacb` | `--oh-text-muted` = `#94a3b8`; `on-surface-variant` fica como token |
| Acentos | cyan/violeta/esmeralda/coral | primary/secondary/tertiary/error | aliases semânticos do §2.1 |
| "Pausado" | não define | — | `--oh-outline` (sem âmbar) |

> A prosa do design é referência de intenção; os **tokens estruturados** são a
> fonte dos valores (consistência com o arquivo `design.md` do usuário).
