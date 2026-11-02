# Tasks — Contexto de tela (AT-SPI)

Índice das tarefas do PRD [`../prd.md`](../prd.md). Cada tarefa é um entregável
testável; a próxima só começa depois da anterior aprovada no teste manual do
usuário (PRD §13).

**Legenda de status:** `PENDING` · `IN PROGRESS` · `CONCLUDED` · `CANCELED`

| # | Resumo | Fase | Status |
|---|--------|------|--------|
| 01 | Domínio puro: snapshot, poda e orçamentos | Fundação | CONCLUDED |
| 02 | Porta `ScreenContextSource` + fakes + evento | Fundação | CONCLUDED |
| 03 | Settings, card em Geral & HUD e l10n | Contrato | CONCLUDED |
| 04 | Adaptador AT-SPI (foco universal, cache, limites) | Adaptador | CONCLUDED |
| 05 | Pill na AskBar (estados, tooltip, a11y) | UI | CONCLUDED |
| 06 | Injeção no prompt + integração do serviço | Integração | CONCLUDED |
| 07 | Hardening, teste manual e docs | Fechamento | IN PROGRESS |

## Fases

```
Fase 1 — Fundação (sem UI, sem I/O)
  01 ─▶ 02

Fase 2 — Contrato
  02 ─▶ 03

Fase 3 — Adaptador
  03 ─▶ 04

Fase 4 — UI
  03 ─▶ 05

Fase 5 — Integração
  04 ─▶ 06 ─▶ 07
```

Observação: 04 e 05 podem correr em paralelo depois de 03; 06 depende de 04.
