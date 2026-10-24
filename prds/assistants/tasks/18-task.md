# 18 — Debrief técnico: "em aberto" só com o que ficou sem resposta

- **Status:** CONCLUDED
- **Depende de:** 17
- **Habilita:** —
- **Issues:** — · **BR:** BR-090

## Objetivo

Ajuste no `TECHNICAL_SUMMARY_PROMPT` (BR-090): a seção **Open questions** era
demasiada exigente — Marcava perguntas "respondidas parcialmente" como em
aberto. Responder (mesmo que melhorável) não é pergunta em aberto.

## Mudanças

- **Open questions:** só o que ficou **sem resposta** (ou admitiu não saber);
  nota apontando respostas melhoráveis para o sub-bloco da avaliação.
- **Overall assessment of my performance:** sub-bloco **What to detail
  better** — respostas que existiram mas podiam ser mais afiadas (mecanismo,
  números, trade-offs) + a versão de 1–2 linhas que ME poderia dar.
- **Gaps and what to cover next:** saiu o item "resposta vaga" (era a mesma
  informação em outra seção).

## Entregáveis

- `src-tauri/src/application/prompts/summary.rs` — os três textos.
- `src-tauri/src/application/assistant_catalog.rs` +
  `assistant_catalog_migration.rs` — `ASSISTANT_CATALOG_REVISION = 4`;
  `upgrade_catalog_to_revision_4` troca os trechos textualmente (idempotente,
  edição preservada).

## Critérios de aceite

- [x] "Open questions" não pede mais "answered only partially".
- [x] "What to detail better" existe no contrato e é apontado nas duas seções.
- [x] Instalação na revisão 3 recebe o afinamento no boot; editado fica intacto.
- [x] Segunda execução não mexe de novo (idempotente).

## Testes

- `application::prompts::summary::tests::technical_summary_prompt_has_the_required_sections`
- `application::assistant_catalog_migration::tests::revision_4_refines_the_technical_summary_contract`
- `application::assistant_catalog_migration::tests::revision_4_preserves_an_edited_technical_summary_prompt`
- `application::assistant_catalog_migration::tests::revision_4_is_idempotent`
- `cargo test --lib` → **676** testes, 0 falhas (10 ignorados); `clippy` limpo.
