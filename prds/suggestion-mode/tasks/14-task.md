# 14 — Janela de 180 caracteres no `already_answered`

- **Status:** CONCLUDED
- **Depende de:** 13 (taxonomia por intenção e histórico de perguntas)
- **Habilita:** —
- **Issues:** ISS-076 · **BR:** BR-101 (revisada)
- **Refs:** [`../issues.md`](../issues.md) ISS-076,
  [`../../business-rules.md`](../../business-rules.md) BR-101,
  [`../architecture/suggestion-gating.md`](../architecture/suggestion-gating.md) §3

## Objetivo

Fazer o `already_answered` voltar a detectar perguntas longas repetidas. O
histórico guarda o título com ≤ 180 caracteres; a comparação precisa usar a
**mesma janela** dos dois lados.

## Causa (sessão real de 2026-10-27 18:19)

A pergunta de *load balancer* foi feita três vezes, com a resposta do ME no meio.
O classificador devolveu `comment`/`question` nas três. O título guardado tinha
180 caracteres; a pergunta atual chegava com 400+. O Jaccard caía para ~0,51
(limiar 0,60) e a repetição nunca era detectada.

> A guarda efetiva de `already_answered` é o **heurístico** (o Laya zero-shot é
> descartado para não suprimir pergunta válida). Corrigir o heurístico restaura a
> função no caminho efetivo do app.

## Entregáveis

- `src-tauri/src/domain/suggestion_gate.rs`:
  - `QUESTION_TITLE_MAX_CHARS = 180`, usado por `question_title` e por
    `is_repeated_question`;
  - `is_repeated_question` trunca a pergunta atual na mesma janela antes do
    Jaccard e da comparação exata.
- Testes:
  - domínio: pergunta longa repetida (falharia antes) e pergunta diferente;
  - serviço: fluxo completo pergunta → sugestão gerada → resposta do ME em bloco
    separado → pergunta repetida = `already_answered`.

## Critérios de aceite

- [x] Pergunta longa repetida (400+ caracteres) com título de 180 vira
      `already_answered`.
- [x] Pergunta diferente continua `question`.
- [x] O histórico (`suggestion_answered_questions`) continua funcionando.
- [x] `cargo test`, `cargo clippy -D warnings`, `pnpm test` e `tsc` verdes.

## Resultado (2026-10-27)

- Janela única de 180 caracteres nos dois lados da comparação.
- Testes anti-regressão que falhariam antes: Jaccard longo + gate ponta a ponta.
- `cargo test --lib` **837 ✓** (11 ignorados) · clippy `-D warnings` ✓ ·
  `pnpm test` **520 ✓** · `tsc --noEmit` ✓.
