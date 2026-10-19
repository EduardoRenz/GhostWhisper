# Tasks — PRD Sugestões

Índice das tarefas deste PRD. Cada tarefa tem um arquivo `NN-task.md`. A execução
só começa após a revisão da PRD pelo usuário.

**Legenda de status:** `PENDING` · `IN PROGRESS` · `CONCLUDED` · `CANCELED`

| # | Resumo | Status |
|---|--------|--------|
| 01 | Identidade do agente: ícone + cor no domínio, settings e UI | CONCLUDED |
| 02 | Sugestão em duas fases: título, resposta rápida e resposta completa | CONCLUDED |
| 03 | Cabeçalho novo: faísca, título, LIVE, cadência circular e Sugerir agora | CONCLUDED |
| 04 | Seletor de agente em popover (ícone + cor, largura total) | CONCLUDED |
| 05 | Blocos de resposta rápida/completa + metadados + caption de código | CONCLUDED |
| 06 | Histórico em accordion: seção, itens, scroll até o bloco e pill do agente | CONCLUDED |
| 07 | Acabamento, usabilidade, validação visual, docs e pacote | CONCLUDED |

## Dependências

```
01 ─▶ 02 ─┬─▶ 03 ─┐
          ├─▶ 04 ─┤
          ├─▶ 05 ─┼─▶ 07
          └─▶ 06 ─┘
```

- 01 (identidade) precede tudo: os ícones/cores aparecem no seletor (04), na
  pill do título e no histórico (06).
- 02 (modelo em duas fases) é a base de 03 (evento de cadência usa o mesmo
  contrato de eventos), 05 (blocos) e 06 (histórico com título + snapshot).
- 07 fecha a PRD (acabamento, validação, docs e pacote).

## Definição de pronto (geral)

- Critérios de aceite da tarefa atendidos e roteiro "Como testar" executado.
- `cargo test`, `cargo clippy`, `cargo fmt --check` limpos.
- `pnpm test`, `tsc --noEmit` e `pnpm build` limpos.
- Arquivo da tarefa atualizado para `CONCLUDED` e `_tasks.md` refletindo o status.
- `memory.md` deste PRD atualizado com `IDs:`/`Refs:` (BR-017).
- `business-rules.md` atualizado quando a tarefa criar/alterar regra
  (`BR-037`…`BR-039`, escrita única — R14).
