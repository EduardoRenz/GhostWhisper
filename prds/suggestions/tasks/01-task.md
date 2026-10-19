# 01 — Identidade do agente (ícone + cor)

- **Status:** CONCLUDED
- **Depende de:** —
- **Habilita:** 02, 04, 05, 06
- **Issues:** — · **BR:** BR-037

## Objetivo

Cada agente (embutido e personalizado) passa a ter **ícone** e **cor** próprios,
persistidos, validados e refletidos na UI (seletor, pill do título, histórico).

## Escopo

- **Domínio (Rust):** `AssistantMode` ganha `icon` (`String`, default
  `sparkle`) e `color` (`String`, default `slate`); `validate_mode` valida
  catálogo/paleta; `builtin_modes()` preenche os 4 embutidos.
- **Domínio (TS):** `AssistantMode` espelha; `AGENT_ICON_IDS`, `AGENT_COLOR_TOKENS`,
  `agentIconId()`, `agentColorToken()` com fallback.
- **Settings:** criar/duplicar agente escolhe ícone e cor; agentes antigos
  (sem os campos) carregam com default (`serde(default)` + `sanitized()`).
- **UI:** `src/components/AgentIcon.tsx` (catálogo id → SVG do projeto — BR-008) e
  `src/components/AgentBadge.tsx` (pill com ícone + nome na cor do agente).
- Sem mudança de schema do banco.

## Critérios de aceite

- [x] Os 4 agentes embutidos têm ícone e cor definidos (users/cyan,
      braces/violet, code/emerald, layers/coral).
- [x] Agente personalizado pode ter ícone e cor escolhidos; default sparkle/slate.
- [x] Settings antigos (sem os campos) carregam sem erro.
- [x] `validate_mode` recusa ícone/cor desconhecidos.
- [x] `AgentBadge` renderiza ícone + nome com a cor do agente.
- [x] Rust/TS verdes.

## Como testar

1. Configurações → Assistente: ver a prévia (ícone + cor) de cada agente.
2. Criar um agente: escolher ícone e cor; salvar; reabrir.
3. Conferir que um agente antigo (sem os campos) aparece com ícone e cor padrão.

## Realizado (2026-10-19)

- **Domínio Rust** (`domain/assistant.rs`): `AssistantMode` ganhou `icon`/`color`
  (`serde(default)`), catálogo `AGENT_ICON_IDS` (8) e `AGENT_COLOR_TOKENS` (5),
  validação em `validate_mode` e `with_valid_identity()` (normaliza em vez de
  derrubar o agente). `Settings::sanitized()` normaliza antes de descartar.
- **Embutidos** (`application/prompts/assistant.rs`): users/cyan,
  braces/violet, code/emerald, layers/coral.
- **Domínio TS** (`core/domain/assistant.ts`): `icon`/`color`, catálogos,
  `agentIconId()`/`agentColorToken()` com fallback.
- **Componentes**: `components/AgentIcon.tsx` (8 desenhos, faísca reaproveitada
  do notch — BR-016) e `components/AgentBadge.tsx` (pill com `--agent-color`).
- **Tokens**: `--oh-agent-{cyan,violet,emerald,coral,slate}` em `tokens.css`.
- **Settings**: seletor de ícone + paleta de cor no editor de agente; o ícone
  aparece na lista, na cor do agente.
- **Testes**: 5 novos (Rust) + 9 novos (TS) — `cargo test` 356 ✓,
  `pnpm test` 198 ✓, `tsc --noEmit` ✓, clippy/fmt limpos.
