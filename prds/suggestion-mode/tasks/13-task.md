# 13 — Taxonomia por intenção, contexto do agente e histórico

- **Status:** CONCLUDED (2026-10-26)
- **Depende de:** 05, 06, 10, 11
- **Habilita:** 12 (fine-tuning na taxonomia nova)
- **Issues:** ISS-062, ISS-064 · **BR:** BR-093, BR-094, BR-101

## Objetivo

Substituir a taxonomia antiga (silêncio/hesitação/perda do fio/convite) por
uma decisão por **intenção**:

- `question` — pergunta a ser respondida;
- `comment` — ponto/opinião a ser comentado;
- `already_answered` — pergunta repetida do histórico recente;
- `no_action` — conversa casual, explicação ou fala sem oportunidade;
- `insufficient_context` — contexto curto/abstenção.

Além disso, o classificador passa a receber o **agente selecionado** e as
**últimas 5 perguntas respondidas** (configurável 0–20), e lê pelo menos 8
blocos anteriores.

## Entregáveis

- [x] Remover `hesitation`, `lost_thread`, `invitation`, `silence` e
  `no_suggestion`; migrar `invitation` → `comment` e `no_suggestion` →
  `no_action` no parse de settings antigas.
- [x] `ClassificationRequest.prior_questions: Vec<PriorQuestion>` com título
  curto, block id e instante.
- [x] Histórico por sessão no `SuggestionGateService`; toda cue acionável
  (`question`/`comment`) que gerou sugestão entra (`mark_generated`); `reset`
  limpa. O título é a fala completa de THEY (pergunta pode estar fragmentada).
- [x] `suggestion_answered_questions` (padrão 5, faixa 0–20) e
  `suggestion_context_blocks` (piso 8, teto 20; janela 5 min/120 segmentos).
- [x] Heurístico novo: pergunta, convite **explícito** de comentário, small
  talk/explicação, resposta no mesmo bloco e similaridade Jaccard ≥ 0,60 para
  repetição. Opinião solta de THEY vira `no_action`.
- [x] Prompt do Laya com agente + histórico; taxonomia de 5 opções.
- [x] Guarda determinística no seletor: já respondida vence o Laya; abstenção/
  `no_action` cede às regras quando há sinal positivo; `already_answered`
  inventado sem histórico cai nas regras; bloco terminando em ME nunca aceita
  decisão positiva do Laya.
- [x] Remover a sonda de hesitação; em vez dela, classificar o bloco aberto
  300 ms após a última fala finalizada (conversa contínua).
- [x] Fixtures de 20 blocos (4 por cue), incluindo `prior_questions` e
  `already_answered`.
- [x] Teste de integração com Laya real no **caminho efetivo** (Laya + guarda).
- [x] AppImage Vulkan + Laya + modelo.

## Testes

- `heuristic_detects_question_pt_en`
- `heuristic_detects_comment_invitation`
- `heuristic_marks_a_repeated_question_as_already_answered`
- `heuristic_marks_a_question_answered_in_the_same_block`
- `heuristic_returns_no_action_for_casual_or_explanation`
- `generated_question_enters_the_answered_history`
- `repeated_question_overrides_a_positive_laya_decision`
- `a_generated_comment_also_enters_the_answered_history`
- `repeated_comment_invitation_is_already_answered`
- `a_mere_statement_is_not_a_comment_invitation`
- `short_me_noise_does_not_hide_the_they_question`
- `thinking_filler_does_not_block_the_they_question`
- `an_open_block_is_classified_after_the_bubble_settles`
- `short_me_noise_does_not_block_the_they_question`
- `a_laya_comment_without_an_explicit_invitation_is_ignored`
- `laya_integration_marks_a_repeated_mutex_question_as_already_answered`
- `me_last_turn_is_not_a_suggestion_opportunity`
- `a_block_ending_in_me_never_uses_a_positive_laya_decision`
- `an_already_answered_block_enters_history_without_generation`
- `laya_question_includes_the_agent_context` (atualizado para histórico)
- `laya_integration_reports_each_situation`
- `laya_integration_enforces_the_gate` (com `LAYA_ENFORCE_GATE=1`)

## Resultado

```text
Heurístico (20 fixtures): decisão 100% · cue 100% · FP 0%
Laya bruto:               decisão 60% · cue 30% · FP 25%
Laya + guarda (app):      decisão 100% · cue 100% · FP 0% · p95 599 ms
```

Todas as 20 fixtures ficaram corretas no caminho efetivo. O gate bruto do
modelo continua reprovado (ISS-062); o gate do caminho efetivo passa.

## Como testar

```bash
cd src-tauri
cargo test --lib
cargo test --lib --features laya
LAYA_ENFORCE_GATE=1 LAYA_ONNX_DIR=$PWD/resources/laya/multilingual \
  cargo test --lib laya_integration --features laya -- --ignored --nocapture
cd ..
pnpm test
pnpm tauri:build:laya
```
