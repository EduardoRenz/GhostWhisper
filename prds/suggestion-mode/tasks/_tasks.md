# Tasks — Modo de Sugestão

Índice das tarefas do PRD [`../prd.md`](../prd.md). Cada tarefa é um entregável
testável; a próxima só começa depois da anterior aprovada no teste manual do
usuário (PRD §16).

**Legenda de status:** `PENDING` · `IN PROGRESS` · `CONCLUDED` · `CANCELED`

| # | Resumo | Fase | Status |
|---|--------|------|--------|
| 01 | Domínio: blocos, cues, heurística e decisão pura | Fundação | CONCLUDED |
| 02 | Porta do classificador + evento + fake | Fundação | CONCLUDED |
| 03 | Serviço de gate + cache da última classificação | Fundação | CONCLUDED |
| 04 | Settings, migração, tipos e IPC | Contrato | CONCLUDED |
| 05 | Backend heurístico local (offline) | Classificadores | CONCLUDED |
| 06 | Modelo Laya (`laya-onnx`) + model manager + avaliação | Classificadores | CONCLUDED |
| 07 | Cabeçalho adaptativo (timer vs. classificador) | UI | CONCLUDED |
| 08 | Faixa “Última classificação” | UI | CONCLUDED |
| 09 | Bloco de ativação em “Fontes de IA & Modelos” | UI | CONCLUDED |
| 10 | Integração no agendador (modo classificador) | Integração | CONCLUDED |
| 11 | Hardening, docs, pacote e teste manual | Fechamento | CONCLUDED |
| 12 | Caminho de fine-tuning: manifesto, dataset, eval, registry | Evolução | PENDING |
| 13 | Taxonomia por intenção, contexto do agente e histórico | Revisão | CONCLUDED |
| 14 | Janela de 180 caracteres no `already_answered` (ISS-076) | Revisão | CONCLUDED |
| 15 | Pergunta longa sem `?`: contexto inteiro + prompt (ISS-077) | Revisão | CONCLUDED |
| 16 | Remover o backend “Regras locais” keyword (ISS-078) | Revisão | PENDING |
| 17 | Última pergunta: contexto do agente e alvo entre blocos (ISS-080) | Revisão | CONCLUDED |
| 18 | `already_answered`: repetição deslocada/fragmento (ISS-081) | Revisão | CONCLUDED |
| 19 | Debounce estrutural: setup de THEY (ISS-083) | Revisão | CONCLUDED |

## Fases

```
Fase 1 — Fundação (sem UI)
  01 ─▶ 02 ─▶ 03

Fase 2 — Contrato
  03 ─▶ 04

Fase 3 — Classificadores
  04 ─▶ 05
  04 ─▶ 06   (06 depende de 02/03/04; 05 é fallback)

Fase 4 — UI
  03/04 ─▶ 07 ─▶ 08
  04/05/06/08 ─▶ 09

Fase 5 — Integração
  03/04/05/06/07/08 ─▶ 10

Fase 6 — Fechamento
  10 ─▶ 11

Fase 7 — Revisão de taxonomia
  10/11 ─▶ 13 ─▶ 14 ─▶ 15 ─▶ 17 ─▶ 18 ─▶ 19
  15 ─▶ 16

Fase 8 — Evolução (fine-tuning na taxonomia nova)
  06/13 ─▶ 12
```

## Dependências detalhadas

- **01** é independente: só domínio puro.
- **02** depende de 01 (tipos de bloco/cue).
- **03** depende de 01/02 (gate + fake + repositório).
- **04** depende de 03 (tipos/eventos para o IPC/settings). Pode andar em
  paralelo com 05 se os tipos estiverem congelados.
- **05** depende de 01/02; é o backend que permite testar o modo classificador
  sem Laya.
- **06** depende de 02/03/04; entrega o adaptador, o model manager e a
  avaliação. Se o gate da POC falhar, 05 continua sendo o backend padrão do
  modo classificador.
- **07** depende de 02/03/04 (evento, last, settings).
- **08** depende de 03/04/07 (mesmo hook/estado).
- **09** depende de 04/05/06/08 (bloco de ativação, model manager, faixa).
- **10** depende de 03/04/05/06/07/08; é o único que mexe no agendador.
- **11** fecha tudo.
- **13** revisa a taxonomia em 05/06/10/11: 5 cues, agente, histórico e
  contexto ampliado. Não muda o modo automático.
- **12** depende de 06/09/11/13; é evolução (fine-tuning), não bloqueia o MVP.

## Ordem de execução recomendada

1. **01 → 02 → 03** (fundação; nada de UI ainda).
2. **05** (heurístico) para provar o gate sem rede — executado antes de 04.
3. **04** (settings/migração/tipos/IPC) com o gate já testável.
4. **06** (modelo Laya + model manager) em paralelo com **07/08** (UI com
   heurístico).
5. **09** (bloco em Fontes de IA & Modelos) quando os campos de 04/05/06
   estiverem estáveis.
6. **10** (agendador) por último na integração.
7. **11** (hardening/docs/pacote).
8. **13** (taxonomia por intenção; substitui a de 7 cues).
9. **12** (fine-tuning) quando o modelo base estiver estável.

## Definição de pronto (geral)

- Critérios de aceite da task atendidos e roteiro “Como testar” executado.
- `cargo test`, `cargo clippy -D warnings`, `cargo fmt --check` limpos.
- `pnpm test`, `tsc --noEmit` e `pnpm build` limpos.
- Nenhum texto de transcrição, áudio ou token em log.
- Laya é feature opt-in (`laya-onnx`); o modelo ONNX fica fora do bundle.
- `tasks/NN-task.md` atualizado para `CONCLUDED` e `_tasks.md` refletindo o
  status.
- `memory.md` deste PRD com entrada `## YYYY-MM-DD — …` (`IDs:`/`Refs:`).
- `business-rules.md` atualizado quando a task criar/alterar regra
  (`BR-092`…`BR-101`, escrita única — R14).
- `issues.md` atualizado se a task encontrar bug/limitação (`ISS-059+`).

## Gate da POC (task 06)

A POC só é considerada bem-sucedida se, nas fixtures rotuladas:

- acurácia de `should_suggest` ≥ **80%**;
- falso positivo ≤ **15%**;
- p95 de latência ≤ **1 s** em CPU ou ≤ **300 ms** em GPU;
- model manager baixa/verifica/instala/ativa sem deixar artefato parcial;
- nenhuma amostra com texto de transcrição em log.

Se falhar: o modo `automático` continua padrão; o backend `heuristic` continua
disponível; a POC fica documentada em `memory.md` com a hipótese e o próximo
passo.
