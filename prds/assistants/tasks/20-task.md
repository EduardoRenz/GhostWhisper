# 20 — Resumo do Padrão livre; Reunião e LeetCode com contrato próprio

- **Status:** CONCLUDED
- **Depende de:** 19
- **Habilita:** —
- **Issues:** — · **BR:** BR-091

## Objetivo

O resumo do agente **Padrão** não deve impor a pauta de uma reunião de empresa
(Overview, Decisões confirmadas, Metas, Métricas, Action Items, Questões em
aberto) quando a conversa é outra coisa. O contrato fixo passa a ser o do agente
**Reunião**; o Padrão ganha um contrato **livre e adaptativo**; o **LeetCode**
deixa o contrato de reunião e recebe o debrief técnico que a Entrevista Técnica
já usa.

## Mudanças

- **`DEFAULT_SUMMARY_PROMPT`** (novo, agente Padrão): detecta o tipo de conversa,
  abre com um parágrafo curto e escolhe **2–4 títulos `##`** conforme o
  conteúdo — sem pauta fixa, sem seção vazia, sem "não mencionado". Mantém os
  invariantes (idioma, não inventar, preservar números, distinguir decisão de
  proposta/hipertese/recomendação).
- **Rename** `SUMMARY_SYSTEM_PROMPT` → `MEETING_SUMMARY_PROMPT` (conteúdo
  idêntico): o contrato de reunião deixa de ser "o geral".
- **`default_mode()`** usa o contrato livre; **`effective_summary_prompt`**
  (vazio após `trim`) herda o do **Padrão** (BR-079).
- **LeetCode** passa a usar `TECHNICAL_SUMMARY_PROMPT`.
- **Revisão de catálogo 6:** troca o `summary_prompt` do `leetcode` já semeado
  **somente** quando ainda é o contrato de reunião; edição/vazio preservado;
  Reunião não é tocada. O Padrão não precisa de migração (vive no core).
- **`SUMMARY_UPDATE_SYSTEM_PROMPT` não muda:** é compartilhado com RH/Técnica
  (que dependem das seções "Not discussed so far") e só preserva a organização
  que o resumo já tem.

## Entregáveis

- `src-tauri/src/application/prompts/summary.rs` — contratos + testes.
- `src-tauri/src/application/prompts/assistant.rs` — fábrica, fallback e testes.
- `src-tauri/src/application/assistant_catalog.rs` +
  `assistant_catalog_migration.rs` — `ASSISTANT_CATALOG_REVISION = 6` e
  `upgrade_catalog_to_revision_6`.
- `src-tauri/src/application/summary_service.rs` — testes do contrato efetivo.

## Critérios de aceite

- [x] O contrato do Padrão não contém "Required output structure", "Confirmed
      Decisions", "Success Metrics" nem a lista fixa de seções.
- [x] O contrato do Padrão escolhe 2–4 títulos pelo conteúdo e proíbe seção
      vazia / placeholder.
- [x] Reunião mantém a pauta (texto preservado) e LeetCode usa o debrief técnico.
- [x] `effective_summary_prompt` vazio herda o contrato do Padrão.
- [x] Instalação na revisão 5 recebe a troca do LeetCode no boot; editado fica
      intacto e Reunião não muda.
- [x] Instalação na revisão 0 (caminho rev0 → 6) também converge.
- [x] Segunda execução não mexe de novo (idempotente).
- [x] Todos os contratos cabem no limite de 16 384 caracteres do campo.

## Testes

- `application::prompts::summary::tests::default_summary_prompt_is_free_form`
- `application::prompts::summary::tests::meeting_summary_prompt_keeps_the_required_sections`
- `application::prompts::summary::tests::summary_contracts_fit_the_agent_field_limit`
- `application::prompts::assistant::tests::each_agent_carries_the_summary_contract_of_its_scene`
- `application::prompts::assistant::tests::effective_prompts_prefer_the_agent_and_fall_back_to_the_default`
- `application::summary_service::tests::an_empty_summary_prompt_falls_back_to_the_default`
- `application::assistant_catalog_migration::tests::revision_6_*`
- `cargo test --lib` → **701** testes, 0 falhas (10 ignorados); `clippy` e
  `fmt --check` limpos; `pnpm test` → **485** testes.
