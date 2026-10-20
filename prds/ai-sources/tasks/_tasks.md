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

## Ordem e dependências

```
01 ─▶ 02 ─▶ 03 ─┬─▶ 04 ─┬─▶ 06 ─┬─▶ 08 ─┐
                │       └─▶ 05 ──┘        ├─▶ 09
                └─────────────────────────┘
07 ────────────────────────────────────▶ 09
```

## Definição de pronto (geral)
- Critérios de aceite da tarefa atendidos.
- `cargo test`, `cargo clippy`, `cargo fmt --check` limpos.
- `pnpm test`, `tsc --noEmit` e `pnpm build` limpos (quando houver UI).
- Arquivo da tarefa atualizado para `CONCLUDED` e `_tasks.md` refletindo o status.
- `memory.md` do PRD atualizado com `IDs:`/`Refs:`.
