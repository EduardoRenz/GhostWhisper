# Tasks — PRD Fontes de IA

Índice de tarefas deste PRD. Cada tarefa tem um arquivo `NN-task.md`.

**Legenda de status:** `PENDING` · `IN PROGRESS` · `CONCLUDED` · `CANCELED`

| # | Resumo | Status |
|---|--------|--------|
| 01 | Domínio das fontes de IA e das rotas | CONCLUDED |
| 02 | Migração das settings e das chaves legadas | CONCLUDED |
| 03 | Serviço de fontes, saúde e circuit breaker | CONCLUDED |
| 04 | Roteador com fallback, latência e evento | CONCLUDED |
| 05 | Adapter DeepSeek e erros tipados de provider | CONCLUDED |
| 06 | IPC das fontes e wiring no composition root | CONCLUDED |
| 07 | Casca da tela de Configurações | CONCLUDED |
| 08 | Aba "Fontes de IA": cards, ordem e chaves | CONCLUDED |
| 09 | Demais abas, rotas por escopo e docs | CONCLUDED |
| 10 | Domínio e settings do HUD (opacidade, ancoragem, densidade) | CONCLUDED |
| 11 | Janela do notch: ancoragem e densidade | CONCLUDED |
| 12 | UI: aba "Geral & HUD" e acabamento visual | CONCLUDED |
| 13 | Overlay consome opacidade/densidade, docs e verificação | CONCLUDED |
| 14 | Redesign da aba "Fontes de IA" | PENDING |
| 15 | Opacidade global das janelas em tempo real + autosave | CONCLUDED |
| 16 | Escala de fonte global + bônus do texto de IA | CONCLUDED |
| 17 | Reordenar fontes de IA no arraste (WebKitGTK, ISS-086) | CONCLUDED |
| 18 | Circuit breaker rígido (1 falha) e prazos curtos por escopo | CONCLUDED |

## Revisão v0.2 (2026-10-20)

O mockup do usuário pede uma aba **"Geral & HUD"** no lugar de "Ajustes", com
opacidade do overlay, ancoragem e densidade do notch, e o dwell-hover movido
para lá (BR-062). A casca ganha abas com sublinhado e grade de 2 colunas. As 5
abas atuais continuam; a reorganização de 6 abas do mockup fica fora (PRD §10.2).

## Revisão v0.3 (2026-10-20)

A aba **"Fontes de IA"** recebe o mesmo tratamento visual de "Geral & HUD":
cabeçalho de seção, cards de fonte com hierarquia (modelo, estado, chave e
ações) e formulário de adição em grid. É uma entrega **somente de frontend e
CSS**; não altera autenticação, backend, `Settings`, keyring nem o contrato de
fallback (PRD §11).

## Ordem e dependências

```
01 ─▶ 02 ─▶ 03 ─┬─▶ 04 ─┬─▶ 06 ─┬─▶ 08 ─┐
                │       └─▶ 05 ──┘        ├─▶ 09
                └─────────────────────────┘
07 ────────────────────────────────────▶ 09

09 ─▶ 10 ─┬─▶ 11 ─┐
          └─▶ 12 ─┴─▶ 13 ─▶ 14
```

- A **10** é a base da v0.2 (campos + geometria pura) e não muda UI.
- A **11** (janela) e a **12** (tela) são paralelas e dependem só da 10.
- A **13** fecha: overlay, BR-062, docs e verificação.
- A **14** é independente do HUD no backend: só consome o padrão visual da 13
  e pode ser feita em uma única entrega de frontend/CSS.
- A **15** estende a opacidade da 13 para todas as janelas, com preview e
  autosave; depende da casca visual da 13/BR-064.
- A **17** é um ajuste de runtime sobre a **08** (ISS-086): não muda contrato
  de settings nem backend, só faz o arraste reordenar no WebKitGTK.
- A **18** endurece a **03/04** (BR-045): uma falha já abre o circuito e os
  prazos passam a depender do escopo. Não muda settings nem IPC.

> 10–13 implementadas em 2026-10-20; a **validação visual/manual** do usuário
> (mockup e notch real) é o último passo antes de considerar a v0.2 encerrada.

## Definição de pronto (geral)
- Critérios de aceite da tarefa atendidos.
- `cargo test`, `cargo clippy`, `cargo fmt --check` limpos.
- `pnpm test`, `tsc --noEmit` e `pnpm build` limpos (quando houver UI).
- Arquivo da tarefa atualizado para `CONCLUDED` e `_tasks.md` refletindo o status.
- `memory.md` do PRD atualizado com `IDs:`/`Refs:`.
