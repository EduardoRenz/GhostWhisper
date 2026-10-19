# 02 — Sugestão em duas fases (título + rápida + completa)

- **Status:** CONCLUDED
- **Depende de:** 01
- **Habilita:** 03, 05, 06
- **Issues:** — · **BR:** BR-038

## Objetivo

A sugestão deixa de ser “um texto que se substitui” e passa a ter **título**,
**resposta rápida** e **resposta completa**, com fases explícitas no modelo e nos
eventos.

## Escopo

- **Porta (Rust):** `Suggestion` ganha `phase` (`quick` | `full`) e o snapshot da
  identidade do agente (`mode_icon`, `mode_color`).
- **Serviço:** o passe *fast-first* **não muda de contrato** (continua o texto
  curto de hoje) — passa a ser publicado com `phase = quick`, para a UI guardá-lo
  em bloco próprio; o passe completo continua Markdown e recebe a instrução de
  metadados de código/complexidade.
- **Eventos:** `emit_suggestion` publica a fase; a janela separa os blocos.
- **Frontend:** `useSuggestions` mantém `quick` e `full` **separados** (a rápida
  não some quando a completa chega); `core/domain/suggestion.ts` com o tipo.
- **Persistência:** `messages.content` continua com o Markdown completo (sem
  migração); a resposta rápida é efêmera.
- **IPC:** `suggestion_generate`/`suggestion_pin` ajustados ao novo payload.
- **Sem** campo de título no modelo (o título é derivativo — task 05).

## Critérios de aceite

- [x] O evento distingue a fase rápida e a completa.
- [x] A resposta rápida continua visível depois que a completa chega.
- [x] A sugestão fixada/persistida mantém o Markdown completo.
- [x] O contrato de prompt do agente **não** muda (só o envelope do evento).
- [x] Rust/TS verdes.

## Como testar

1. Iniciar sessão, falar e esperar a sugestão: ver o bloco rápido e depois o
   completo.
2. Fixar a sugestão: conferir que a linha temporal guarda o Markdown completo.
3. Conferir que o título aparece no cartão (derivativo).

## Realizado (2026-10-19)

- **Porta (Rust)**: `SuggestionPhase` (`quick` | `full`, serde lowercase) e
  `Suggestion` com `phase` + snapshot `mode_icon`/`mode_color` (BR-037).
- **Serviço**: o *fast-first* publica com `phase = Quick`; o passe completo
  (streaming + final) com `phase = Full`. **O contrato de prompt não mudou**
  (nenhum JSON novo).
- **Pin**: resolve a identidade do agente no momento da fixação.
- **Frontend**: `Suggestion` espelha a fase; `useSuggestions` separa `quick`
  (bloco próprio) de `current`/`history`; a rápida só convive com a completa da
  **mesma** geração (`at_ms`).
- **Janela**: cartões “Resposta rápida” / “Resposta completa”; `select` saiu —
  o histórico passa a expandir **no lugar** (não substitui o topo).
- **Testes**: `cargo test` 360 ✓, `pnpm test` 202 ✓, `tsc --noEmit` ✓.
