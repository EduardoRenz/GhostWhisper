# 05 — Pill na AskBar (estados, tooltip, a11y)

- **Status:** CONCLUDED (2026-11-01)
- **Depende de:** 03
- **Habilita:** —
- **Issues:** ISS-090 · **BR:** BR-106

## Objetivo

Criar o componente `ContextPill` e inseri-lo na AskBar com os cinco estados do
PRD §3.3, tooltip de prévia, l10n e acessibilidade. Com a flag desligada, a
pill **não renderiza**.

## Entregáveis

- `src/features/overlay/ContextPill.tsx` (novo): props
  `state: "hidden" | "loading" | "active" | "empty" | "unavailable"`,
  `app`, `windowTitle`, `preview`; `aria-live="polite"`, `title`/`aria-label`
  l10n.
- `src/features/overlay/AskBar.tsx`: renderizar `ContextPill` entre
  `askbar__keys` e o divisor do botão de captura; nova prop
  `context?: ScreenContextDto | null`.
- CSS em `src/styles` com tokens `--oh-*` (BR-035): hairline, cyan no estado
  ativo, outline nos neutros.
- `src/features/ask/AskWindow.tsx`: chamar `screen_context_snapshot` na
  montagem (só se a flag estiver ligada) e assinar o evento `screen_context`.
- l10n pt/en das strings dos 5 estados.
- Testes: `ContextPill.test.tsx` (5 estados + aria) e AskBar com/sem flag.

## Critérios de aceite

- [ ] Flag desligada: nenhum DOM de pill (CA-02.1).
- [ ] Os 5 estados renderizam com texto l10n correto (pt/en).
- [ ] Estado ativo: `🌐 app · título` + tooltip com preview ≤ 300 chars.
- [ ] `aria-live` anuncia mudanças de estado.
- [ ] Evento `screen_context` atualiza a pill sem reabrir a barra (CA-02.5).

## Testes

- `renders_no_pill_when_disabled`
- `renders_five_states_with_l10n`
- `tooltip_shows_preview`
- `live_region_announces_state_change`
- `event_updates_pill`
