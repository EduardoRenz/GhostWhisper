# Tasks — Precision and Performance

Índice das tarefas. Cada tarefa é um **entregável testável**; a próxima só começa
depois da anterior aprovada no teste manual do usuário (PRD §11).

**Legenda de status:** `PENDING` · `IN PROGRESS` · `CONCLUDED` · `CANCELED`

| # | Resumo | Entregável | Status |
|---|--------|-----------|--------|
| 01 | Precisão do ASR: prompt de decodificação, idioma, resampler anti-alias, catálogo de modelos, detecção de GPU | E1 | IN PROGRESS |
| 02 | Bolha compacta e sugestão fast-first: streaming, prévias no contexto, insights automáticos, UI colapsável | E2 | PENDING |
| 03 | Áudio limpo e engine rápida: GPU + flash attention, modelo por hardware, AEC, reuso do final, commits | E3 | PENDING |

## Dependências

```
01 (E1) ──▶ 02 (E2) ──▶ 03 (E3)
```

- 01 é independente e não muda comportamento de UI.
- 02 depende de 01 apenas pela ordem do ciclo (não por código).
- 03 depende de 03/01 para o default de modelo (catálogo vem do 01) e de 13/14 do
  MVP (streaming/áudio) para AEC e reuso do final.

## Definição de pronto (geral)

- Código compila (`cargo test`, `pnpm build` sem erros).
- Critérios de aceite da task atendidos e **roteiro "Como testar" executado**.
- `tasks/NN-task.md` atualizado para `CONCLUDED` e status em `_tasks.md`.
- `memory.md` deste PRD com entrada `## YYYY-MM-DD — …` (`IDs:`/`Refs:`).
- `business-rules.md` atualizado quando a task criar/alterar regra
  (`BR-027`…`BR-029`, escrita única — R14).
- Nenhuma regressão em `cargo clippy`, `cargo fmt --check`, `pnpm test`,
  `tsc --noEmit`.
