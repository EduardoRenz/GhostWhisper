# Tasks — Precision and Performance

Índice das tarefas. Cada tarefa é um **entregável testável**; a próxima só começa
depois da anterior aprovada no teste manual do usuário (PRD §11).

**Legenda de status:** `PENDING` · `IN PROGRESS` · `CONCLUDED` · `CANCELED`

| # | Resumo | Entregável | Status |
|---|--------|-----------|--------|
| 01 | Precisão do ASR: prompt de decodificação, idioma, resampler anti-alias, catálogo de modelos, detecção de GPU | E1 | CONCLUDED |
| 02 | Bolha compacta e sugestão fast-first: streaming, prévias no contexto, insights automáticos, UI colapsável | E2 | CONCLUDED |
| 03 | Áudio limpo e engine rápida: GPU + flash attention, modelo por hardware, AEC, reuso do final, commits | E3 | CONCLUDED |
| 04 | Commit de fala mais rápido (etapa 1): fila com prioridade, prévias na pausa, fechamento por frase, métrica | L1 | IN PROGRESS |
| 05 | Prompt como contexto: default vazio, migração e aviso na UI | L2 | IN PROGRESS |

## Dependências

```
01 (E1) ──▶ 02 (E2) ──▶ 03 (E3) ──▶ 04 (L1)
```

- 01 é independente e não muda comportamento de UI.
- 02 depende de 01 apenas pela ordem do ciclo (não por código).
- 03 depende de 03/01 para o default de modelo (catálogo vem do 01) e de 13/14 do
  MVP (streaming/áudio) para AEC e reuso do final.
- 04 depende de 03 (reuso do final + VAD nativo) e da fila global do ASR; não
  depende do provider em nuvem. É a **etapa 1** do plano de latência; a etapa 2
  (commit otimista + correção) e a 3 (commit por frase) ainda não estão criadas.
- 05 é independente (prompt/idioma) e depende só de 01 para o contrato do
  `whisper_prompt`; a UI usa o mesmo campo.

## Definição de pronto (geral)

- Código compila (`cargo test`, `pnpm build` sem erros).
- Critérios de aceite da task atendidos e **roteiro "Como testar" executado**.
- `tasks/NN-task.md` atualizado para `CONCLUDED` e status em `_tasks.md`.
- `memory.md` deste PRD com entrada `## YYYY-MM-DD — …` (`IDs:`/`Refs:`).
- `business-rules.md` atualizado quando a task criar/alterar regra
  (`BR-027`…`BR-029`, escrita única — R14).
- Nenhuma regressão em `cargo clippy`, `cargo fmt --check`, `pnpm test`,
  `tsc --noEmit`.
