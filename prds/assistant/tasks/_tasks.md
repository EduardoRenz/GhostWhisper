# Tasks — PRD Assistant

Índice de tarefas deste PRD. Cada tarefa tem um arquivo `NN-task.md`.

**Legenda de status:** `PENDING` · `IN PROGRESS` · `CONCLUDED` · `CANCELED`

| # | Resumo | Status |
|---|--------|--------|
| 01 | Domínio, prompts, Settings e persistência do resumo | CONCLUDED |
| 02 | Serviços de resumo/sugestão, IPC e eventos | CONCLUDED |
| 03 | Markdown compartilhado e janela de resumo | CONCLUDED |
| 04 | Janela de sugestões, agentes e “Fixar” | CONCLUDED |
| 05 | Settings: agentes personalizados e cadências | CONCLUDED |
| 06 | Export/histórico, documentação, verificação e pacotes | CONCLUDED |
| 07 | Histórico de versões do resumo (em memória) | CONCLUDED |
| 08 | Janelas redimensionáveis e responsivas | CONCLUDED |

## Ordem e dependências

```
01 ─▶ 02 ─┬─▶ 03 ─┬─▶ 05 ─▶ 06
          │       └─▶ 07
          │       └─▶ 08
          └─▶ 04 ─▶ 05
```

## Definição de pronto (geral)
- Critérios de aceite da tarefa atendidos.
- `cargo test`, `cargo clippy`, `cargo fmt --check` limpos.
- `pnpm test`, `tsc --noEmit` e `pnpm build` limpos (quando houver UI).
- Arquivo da tarefa atualizado para `CONCLUDED` e `_tasks.md` refletindo o status.
- `memory.md` do PRD atualizado com `IDs:`/`Refs:`.
