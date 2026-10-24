# Tasks — PRD Assistentes

Índice de tarefas deste PRD. Cada tarefa tem um arquivo `NN-task.md`.

**Legenda de status:** `PENDING` · `IN PROGRESS` · `CONCLUDED` · `CANCELED`

| # | Resumo | Status |
|---|--------|--------|
| 01 | Domínio do agente: `summary_prompt` e catálogo protegido | CONCLUDED |
| 02 | Settings: semeadura dos agentes de fábrica e resolução | CONCLUDED |
| 03 | IPC: catálogo efetivo e defaults do core | CONCLUDED |
| 04 | Frontend: espelho do domínio, `Dialog` e ícones novos | CONCLUDED |
| 05 | Painel "Assistentes" (cards do mockup) | CONCLUDED |
| 06 | Modal de agente (criar/editar) | CONCLUDED |
| 07 | "Geral & HUD": cadências e perguntas prontas | CONCLUDED |
| 08 | Casca: barra de abas em pílulas | CONCLUDED |
| 09 | Regressão, testes e fechamento | CONCLUDED |
| 10 | Fallback de prompt vazio: herda o do Padrão | CONCLUDED |
| 11 | Seletor global de agentes acima do notch | CONCLUDED |
| 12 | Ajuda nos campos de prompt (tooltips) | CONCLUDED |
| 13 | Contrato comum de leitura da transcrição | CONCLUDED |
| 14 | Entrevista Técnica funde System Design; novo Entrevista com RH | CONCLUDED |
| 15 | Resumo por agente ativo + debrief da Entrevista com RH | CONCLUDED |
| 16 | Sugestão: rascunho e resposta completa em paralelo | CONCLUDED |
| 17 | Resumo: contrato comum injetado + debrief da Entrevista Técnica | CONCLUDED |
| 18 | Debrief técnico: "em aberto" só sem resposta + What to detail better | CONCLUDED |
| 19 | Debrief técnico: autoria das perguntas em aberto é de THEY | CONCLUDED |

## Implementação (2026-10-22)

Todas as tasks concluídas na ordem prevista (01 → 09). Verificação:

- `cargo test --lib`: **626** testes, 0 falhas (10 ignorados).
- `cargo clippy --all-targets`: sem warnings.
- `pnpm test`: **403** testes, 0 falhas.
- `pnpm exec tsc --noEmit` e `pnpm build`: limpos.
- Conferência visual no navegador (`?window=settings&tab=assistant`) contra o
  mockup: pílulas, card do Padrão, cards com tile/badges/prompts, CTA e modal.
- Regressão de compatibilidade encontrada e corrigida (**ISS-060**): a escolha
  de um agente de fábrica não pode ser zerada na primeira carga depois da
  atualização (o `sanitized` roda antes da semeadura).

## Decisões que moldaram as tasks

1. **O agente Padrão é protegido**: card informativo no topo, sem editar/excluir.
2. **Os outros 4 agentes são de fábrica, mas do usuário**: materializados na
   lista uma única vez (semeadura) e, depois, editáveis e excluíveis de vez.
3. **O painel não troca o agente ativo** — a badge "Ativo" é informativa; a
   troca continua na tela de sugestões.
4. **Cadências e perguntas prontas** migram para uma seção nova na aba
   "Geral & HUD".
5. **Barra de abas em pílulas** (mockup), em todas as abas.

Detalhe de produto: [`../prd.md`](../prd.md). Desenho técnico:
[`../architecture/assistants-panel.md`](../architecture/assistants-panel.md).

## Ordem e dependências

```
01 ─▶ 02 ─▶ 03 ─▶ 05 ─▶ 06 ─┐
          │        └─▶ 04 ──┼─▶ 09
          └───────────────────┘
04 ─▶ 07 ───────────────────┤
04 ─▶ 08 ───────────────────┘
```

- **01–03** são o core (Rust): campo novo, semeadura e IPC.
- **04** é a base do frontend (espelho do tipo, `Dialog`, ícones) e não
  depende do core.
- **05** (painel) consome o catálogo efetivo da **03** e a base da **04**.
- **06** (modal) consome o painel da **05**.
- **07** e **08** são frontend/CSS e podem correr em paralelo.
- **09** fecha: regressão, docs e verificação.

## Definição de pronto (geral)

- Critérios de aceite da tarefa atendidos.
- `cargo test`, `cargo clippy`, `cargo fmt --check` limpos.
- `pnpm test`, `tsc --noEmit` e `pnpm build` limpos (quando houver UI).
- Arquivo da tarefa atualizado para `CONCLUDED` e `_tasks.md` refletindo o status.
- `memory.md` do PRD atualizado com `IDs:`/`Refs:`.
