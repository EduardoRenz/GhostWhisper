# 19 — Debrief técnico: autoria das perguntas em aberto é de THEY

- **Status:** CONCLUDED
- **Depende de:** 18
- **Habilita:** —
- **Issues:** — · **BR:** BR-090

## Objetivo

No resumo da simulação do ledger, "Questões em aberto" misturava perguntas de
ME (fragments do ASR no canal do microfone, ex. "O que é") com as do
entrevistador. O contrato passa a fixar a autoria.

## Mudanças

- **Open questions** = perguntas que **THEY** (o entrevistador) fez e ME não
  respondeu; "**Never list questions ME asked**".
- Autoria: pela tag; tags duvidosas → decidir **por conteúdo**; sem falante
  claro ou linha inutilizável → **descartar** em vez de adivinhar.

## Entregáveis

- `src-tauri/src/application/prompts/summary.rs` — parágrafo de "Open
  questions".
- `src-tauri/src/application/assistant_catalog.rs` +
  `assistant_catalog_migration.rs` — `ASSISTANT_CATALOG_REVISION = 5`;
  `upgrade_catalog_to_revision_5` troca o parágrafo textualmente (idempotente).

## Critérios de aceite

- [x] Só perguntas de THEY; pergunta de ME nunca é listada.
- [x] Tags duvidosas resolvidas por conteúdo; linha inutilizável é drogada.
- [x] Semente da revisão 4 recebe o novo texto no boot; edição preservada.

## Testes

- `application::prompts::summary::tests::technical_summary_prompt_has_the_required_sections`
- `application::assistant_catalog_migration::tests::revision_5_attributes_open_questions_to_they`
- `application::assistant_catalog_migration::tests::revision_5_preserves_an_edited_technical_summary_prompt`
- `cargo test --lib` → **678** testes, 0 falhas (10 ignorados); `clippy` limpo.
