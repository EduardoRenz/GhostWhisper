# 05 — Blocos de resposta rápida/completa + metadados + caption de código

- **Status:** CONCLUDED
- **Depende de:** 01, 02
- **Habilita:** 07
- **Issues:** — · **BR:** BR-038

## Objetivo

A sugestão atual vira um cartão com **título**, **bloco de resposta rápida** e
**bloco de resposta completa**, cada um com título, cor e estilo próprios, mais o
chip de metadados e o cabeçalho/rodapé dos blocos de código.

## Escopo

- `SuggestionCard`: tag “Sugestão Atual · Há 12s” (tempo relativo que se
  atualiza), **título derivativo** (helper puro `suggestionTitle`, mesmo padrão
  do `summaryTopic`), **pill do agente** e os dois blocos.
- `QuickAnswerBlock`: título “Resposta rápida”, cor própria (violeta/`--oh-ai`),
  borda esquerda, texto em citação/ênfase.
- `FullAnswerBlock`: título “Resposta completa”, cor própria, chip de metadados
  (“Markdown · Explicativo”) e o Markdown renderizado.
- `MarkdownView`: **caption** de bloco de código (cabeçalho `linguagem — arquivo`
  e rodapé `O(n) · O(1)`), quando o modelo emitir as linhas de metadados
  acordadas.
- Estilos: tokens do design system (BR-035); **sem blur** em bolha/bloco
  (ISS-020).
- Ações **copiar/fixar** viram botões compactos no cartão (não no cabeçalho).

## Critérios de aceite

- [x] A resposta rápida e a completa têm títulos, cores e estilos distintos.
- [x] A rápida continua visível quando a completa chega.
- [x] A completa mostra o chip de metadados.
- [x] Blocos de código exibem cabeçalho/rodapé quando há metadados.
- [x] A tag mostra o tempo relativo (“Há 12s”) e se atualiza.
- [x] Copiar/fixar são ícones compactos no cartão.
- [x] `pnpm test`, `tsc`, `pnpm build` verdes.

## Como testar

1. Gerar uma sugestão: ver a tag, o título, a pill do agente e os dois blocos.
2. Esperar a completa: a rápida continua lá, o chip e o código aparecem.
3. Copiar e fixar pelo cartão.

## Realizado (2026-10-19)

- `core/domain/suggestion.ts` (novo): `suggestionTitle()` (reuso do padrão do
  resumo: `Subject:` → heading → 1ª linha, com corte curto), `parseSuggestionMeta()`
  (Big O / arquivo), `stripSuggestionMeta()` e `suggestionKind()` (chip).
- `features/suggestions/SuggestionCard.tsx` (novo): tag “Sugestão Atual · Há 12s”
  (relativo, atualiza a cada 1 s), **título derivativo**, `AgentBadge` (ícone +
  cor do agente), bloco de **resposta rápida** (borda própria, itálico) e bloco de
  **resposta completa** (chip “Markdown · Código/Explicativo” + Markdown).
- `MarkdownView`: caption do código — cabeçalho `linguagem — arquivo` e
  complexidade à direita (`**Big O:**` → “O(n) time · O(1) space”); as linhas de
  metadados saem do corpo quando há código.
- Prompts dos agentes técnicos: linha `**File:** <arquivo>` (o caption usa).
- Copiar/fixar viraram **ícones compactos** no cartão (`CopyIcon`/`PinIcon`).
- **Testes**: 4 + 6 novos (TS).
