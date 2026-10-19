# Tasks — PRD Translation

Índice das tarefas deste PRD. Cada tarefa é um entregável testável.

**Legenda de status:** `PENDING` · `IN PROGRESS` · `CONCLUDED` · `CANCELED`

| # | Resumo | Entregável | Status |
|---|--------|-----------|--------|
| 01 | Tradução sob demanda na bolha (botão + toggle), painel duplicado removido | T1 | IN PROGRESS |
| 02 | Gate por idioma detectado (não oferecer tradução quando o idioma já é o destino) | T2 | PENDING |

## Dependências

```
01 (T1) ──▶ 02 (T2)
```

- 01 entrega a superfície (botão/toggle) e pode ser testada isolada.
- 02 precisa do canal de idioma do ASR e usa o botão da 01 para decidir se aparece.

## Definição de pronto (geral)

- Compila (`pnpm build`, `cargo test`).
- Critérios de aceite da task e roteiro "Como testar" executados.
- `tasks/NN-task.md` → `CONCLUDED`; `_tasks.md` atualizado.
- `memory.md` deste PRD com entrada do dia; `business-rules.md` quando criar regra.
