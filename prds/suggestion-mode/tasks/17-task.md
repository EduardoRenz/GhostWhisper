# 17 — Última pergunta: contexto do agente e alvo entre blocos

- **Status:** CONCLUDED
- **Depende de:** 15 (prompt/conversa inteira)
- **Habilita:** —
- **Issues:** ISS-080 · **BR:** BR-094 (revisada) · **R23/R24**
- **Refs:** [`../issues.md`](../issues.md) ISS-080,
  [`../architecture/suggestion-gating.md`](../architecture/suggestion-gating.md) §2

## Objetivo

Fazer o Laya classificar a última pergunta de uma fala longa dividida em blocos,
com a janela de Sugestões aberta, **sem** palavras-chave/idioma.

## Causa (sessão `5984de1d…`, 22:02)

- `selected_mode_context` passava ao Laya o **`suggestion_prompt`** (prompt de
  geração, 500 chars), enviesando o cue.
- `target_across_blocks` desistia quando o bloco atual tinha ME; a pergunta de
  arquitetura (t5/t6/t7) tinha um “deixa eu pensar” no fim e o alvo ficava só no
  t7.

## Entregáveis

- `selected_mode_context`: apenas `Agent: nome (categoria)` + `Agent context`
  definido pelo usuário; **sem** `suggestion_prompt`.
- `target_across_blocks`: junta blocos contíguos quando o **alvo começa no
  primeiro turno do bloco**; um ME não significativo depois do alvo não impede;
  um ME antes do alvo (resposta) impede.
- Testes reais com Laya (`LAYA_ONNX_DIR`, `#[ignore]`):
  - `regression_5984de1d_architecture_question_is_question`;
  - `regression_5984de1d_friend_money_question_is_question`;
  - os 5 testes Laya anteriores continuam verdes.
- Testes de domínio do merge com filler final, resposta significativa e pausa.
- O gate de janela permanece: classifica só com a janela aberta (escolha do
  usuário); nenhum flush por keyword.

## Critérios de aceite

- [x] Arquitetura (`5984de1d…`) → `question` com Laya real.
- [x] Amigo/dinheiro (`5984de1d…`) → `question` com Laya real.
- [x] O prompt do classificador não contém o `suggestion_prompt` de geração.
- [x] O alvo junta a pergunta entre blocos contíguos com filler final.
- [x] Nenhuma palavra-chave/idioma; R23/R24 respeitadas.
- [x] `cargo test`, suíte Laya, clippy, fmt, `pnpm test` e `tsc` verdes.

## Resultado (2026-10-27)

- Contexto do agente reduzido ao que é de classificação; teste garante que
  “live technical-interview copilot” não vaza para o classificador.
- Merge estrutural do alvo entre blocos.
- `cargo test --lib` **844 ✓** (11 ignorados) · Laya **7 ✓** · frontend
  **521 ✓** · clippy/fmt/tsc ✓.
