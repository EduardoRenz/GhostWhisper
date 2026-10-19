# 04 — Seletor de agente em popover (ícone + cor, largura total)

- **Status:** CONCLUDED
- **Depende de:** 01
- **Habilita:** 07
- **Issues:** — · **BR:** BR-037

## Objetivo

Trocar o `<select>` de agente por um **botão de largura total** (ícone + nome +
pill **ATIVO** + chevron) que abre um **popover** com a lista de agentes
(ícone, nome e cor), fluido e navegável por teclado.

## Escopo

- `AgentSelector`: botão full-width + popover próprio (reaproveitando o padrão
  de popover do `ChannelMenu`/drill-hover).
- Lista: agentes embutidos + personalizados, com `AgentIcon` na cor do agente,
  nome, marca do ativo.
- Largura **fluida** (acompanha a janela; sem largura fixa que “pule” para fora).
- Teclado: `Enter`/`Space` abre, setas navegam, `Escape` fecha, foco preso
  enquanto aberto.
- A escolha continua persistida (`settings.assistant_mode_id`) e vale para o
  automático do core.

## Critérios de aceite

- [x] O seletor ocupa **100%** da largura disponível.
- [x] Mostra ícone + nome do agente selecionado + pill ATIVO + chevron.
- [x] O popover lista todos os agentes com ícone e cor.
- [x] Trocar de agente persiste e muda a sugestão seguinte.
- [x] Fecha ao clicar fora / `Escape`; navegável por teclado.
- [x] `pnpm test`, `tsc`, `pnpm build` verdes.

## Como testar

1. Clicar no seletor: o popover abre com a lista; escolher outro agente.
2. Conferir que a pill ATIVO e o ícone acompanham.
3. Fechar com `Escape` e clicando fora; navegar por teclado.
4. Abrir de novo: a seleção persiste.

## Realizado (2026-10-19)

- `features/suggestions/AgentSelector.tsx`: botão **full-width** (ícone na cor do
  agente + nome + pill **ATIVO** + “Alternar agente” + chevron) e popover com
  todos os agentes (ícone, nome, marca do ativo).
- Padrão de popover o mesmo do `ChannelMenu` (fecha ao clicar fora / `Escape`),
  **mais** navegação por teclado (`↑`/`↓`, `Home`, `End`) e `role="menu"`.
- A escolha continua persistida em `settings.assistant_mode_id`.
- O `<select>` antigo saiu do cabeçalho.
- **Testes**: 5 novos (TS).
