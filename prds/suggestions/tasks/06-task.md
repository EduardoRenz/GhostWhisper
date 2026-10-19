# 06 — Histórico em accordion: seção, itens, scroll até o bloco e pill do agente

- **Status:** CONCLUDED
- **Depende de:** 01, 02
- **Habilita:** 07
- **Issues:** — · **BR:** BR-038

## Objetivo

O histórico vira uma **seção colapsável** “Histórico de Sugestões da Sessão” (ícone
+ contador + chevron) com **accordions** por sugestão: ícone do agente, título e
pill com o nome do agente. Expandir **rola até** o bloco e **não** substitui a
sugestão do topo.

## Escopo

- `SuggestionHistory`: seção (ícone + contador “N anteriores” + chevron) e itens
  accordion.
- Item: `AgentIcon` na cor do agente + **título** + `AgentBadge` (nome do agente,
  cor do snapshot) + horário.
- Expandir: abre **no lugar** e faz `scrollIntoView({ block: "nearest" })` no
  bloco; a sugestão atual **permanece no topo** (nada é substituído).
- Ações por item: copiar e fixar (fixar persiste a mensagem da sessão).
- Limite de memória igual ao atual; a lista reinicia quando a sessão muda.
- Nenhum componente fica por cima do bloco (CA-04.6).

## Critérios de aceite

- [x] A seção mostra contador e chevron; colapsa/expande.
- [x] Cada item mostra ícone do agente, título e pill do agente.
- [x] Expandir rola até o item e **não** troca a sugestão do topo.
- [x] Copiar/fixar funcionam no item expandido.
- [x] O histórico é limitado e reinicia com a troca de sessão.
- [x] `pnpm test`, `tsc`, `pnpm build` verdes.

## Como testar

1. Gerar 2–3 sugestões: a seção mostra o contador.
2. Expandir um item: o scroll vai até ele e a sugestão do topo continua no topo.
3. Copiar/fixar no item expandido.
4. Trocar de sessão: o histórico zera.

## Realizado (2026-10-19)

- `features/suggestions/SuggestionHistory.tsx` (novo): seção colapsável
  “Histórico de Sugestões da Sessão” (ícone + contador “N anteriores” +
  chevron) e **accordions** por sugestão (ícone do agente na cor dele + título
  derivativo + pill do agente).
- Expandir abre **no lugar** e rola até o bloco
  (`scrollIntoView({ block: "nearest" })`); a sugestão do topo **não** é
  substituída (o `select` do hook saiu).
- Ações por item: copiar e fixar (fixar persiste a mensagem da sessão).
- Nenhum componente fica por cima do bloco (CA-04.6): o histórico é a última
  faixa do corpo, com espaço próprio.
- **Testes**: 6 novos (TS).
