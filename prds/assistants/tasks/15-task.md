# 15 — Resumo por agente ativo + debrief da Entrevista com RH

- **Status:** CONCLUDED
- **Depende de:** 02, 03, 14
- **Habilita:** —
- **Issues:** — · **BR:** BR-076, BR-084

## Objetivo

Colocar em prática o `summary_prompt` por agente: o `SummaryService` usa o
contrato do agente **ativo**; vazio depois do `trim` usa o do **Padrão**
(BR-079). A primeira geração e a regeneração manual aplicam a estrutura; as
atualizações incrementais preservam a estrutura do resumo corrente.

O primeiro contrato específico é o da **Entrevista com RH**: um debrief com
resumo geral, o que ME respondeu, o que o RH disse da empresa, benefícios e
pretensões, outras perguntas/respostas e uma opinião de entrevistador. Seções
sem informação dizem que o assunto não foi mencionado até o momento.

Detalhe de produto: [`../prd.md`](../prd.md) §13.4. Desenho técnico:
[`../architecture/assistants-panel.md`](../architecture/assistants-panel.md) §3.6.

## Entregáveis

- `src-tauri/src/application/prompts/summary.rs`
  - `HR_SUMMARY_PROMPT` com as seções obrigatórias;
  - `SUMMARY_UPDATE_SYSTEM_PROMPT` passa a exigir a preservação de todas as
    seções existentes e proíbe inventar fatos/benefícios/números.
- `src-tauri/src/application/prompts/assistant.rs`
  - `factory_modes()` do RH usa `HR_SUMMARY_PROMPT`;
  - `effective_summary_prompt` deixa de ser `dead_code`.
- `src-tauri/src/application/summary_service.rs`
  - resolve o agente ativo (`resolve_mode` + fallback Padrão);
  - primeira geração/manual usa `effective_summary_prompt`;
  - atualização incremental mantém o `SUMMARY_UPDATE_SYSTEM_PROMPT`.
- `src-tauri/src/application/assistant_catalog.rs` / migration
  - revisão de catálogo **2**: atualiza o `summary_prompt` do RH já semeado
    somente quando ainda é o contrato geral antigo; prompt editado/vazio é
    preservado e um RH excluído não volta.
- `src/features/settings/AgentEditorDialog.tsx` / `AgentCard.tsx`
  - tooltips do Prompt de Resumo deixam de dizer "não consumido".

## Critérios de aceite

- [x] O resumo usa o prompt do agente ativo; vazio usa o do Padrão.
- [x] A primeira geração/regeneração manual aplica a estrutura do agente.
- [x] As atualizações incrementais preservam a estrutura existente.
- [x] Trocar de agente no meio da sessão não reestrutura o resumo já gerado;
      a nova estrutura vale na próxima regeneração/sessão.
- [x] O resumo do RH tem as seis seções pedidas e marca seções vazias.
- [x] A revisão 2 atualiza o RH já semeado sem apagar edições do usuário.

## Testes

- `application::summary_service::tests::uses_the_active_agents_summary_prompt`
- `application::summary_service::tests::an_empty_summary_prompt_falls_back_to_the_default`
- `application::summary_service::tests::updates_incrementally_from_the_cursor`
- `application::prompts::summary::tests::hr_summary_prompt_has_the_required_sections`
- `application::assistant_catalog_migration::tests::revision_2_gives_hr_its_summary_contract`
- `application::assistant_catalog_migration::tests::revision_2_preserves_an_edited_hr_summary_prompt`
- `cargo test --lib` → **643** testes, 0 falhas (10 ignorados).
- `pnpm test` → **422** testes; `tsc --noEmit` limpo.
