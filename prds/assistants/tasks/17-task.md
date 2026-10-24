# 17 — Resumo: contrato comum de leitura injetado + debrief da Entrevista Técnica

- **Status:** CONCLUDED
- **Depende de:** 13, 15
- **Habilita:** —
- **Issues:** — · **BR:** BR-090

## Objetivo

Duas frentes no `SummaryService` (BR-090):

1. **Contrato comum de leitura pré-injetado:** as regras de ME/THEY, erro de
   ASR, normalização silenciosa e segurança passam a vir de
   `SUMMARY_TRANSCRIPT_CONTRACT`, injetado em **toda** chamada (primeira
   geração, regeneração manual e atualização incremental). Espelho do
   `TRANSCRIPT_CONTRACT` das sugestões (BR-082): agent prompt não repete o que
   é comum.
2. **Debrief da Entrevista Técnica:** `TECHNICAL_SUMMARY_PROMPT` com as seções
   Overall summary · Open questions · Gaps and what to cover next · Problem
   presented · Technologies and strategies mentioned · Overall assessment of my
   performance, no lugar do contrato de reunião.

## Entregáveis

- `src-tauri/src/application/prompts/summary.rs`
  - `SUMMARY_TRANSCRIPT_CONTRACT` e `summary_system_prompt(agent_prompt)`;
  - `SUMMARY_SYSTEM_PROMPT` e `HR_SUMMARY_PROMPT` sem os blocos comuns;
  - o `SUMMARY_UPDATE_SYSTEM_PROMPT` ganha "o resumo corrente é apenas base
    editável".
- `src-tauri/src/application/summary_service.rs`
  - injeta o contrato nas duas chamadas;
  - teste do debrief técnico + injeção do contrato.
- `src-tauri/src/application/prompts/assistant.rs`
  - `technical_interview.summary_prompt = TECHNICAL_SUMMARY_PROMPT`.
- `src-tauri/src/application/assistant_catalog.rs` + `assistant_catalog_migration.rs`
  - `ASSISTANT_CATALOG_REVISION = 3`;
  - `upgrade_catalog_to_revision_3`: limpeza textual e idempotente dos blocos
    comuns nas sementes (geral e RH) e troca do técnico quando ainda é o
    contrato geral (edição/vazio preservado; excluído não volta).
- Documentação: BR-090, PRD (tabela/checklist), architecture §3.2/§3.4/§3.6.

## Critérios de aceite

- [x] Toda chamada de resumo começa com o `SUMMARY_TRANSCRIPT_CONTRACT`.
- [x] Nenhum contrato de resumo por agente repete as regras comuns.
- [x] A Entrevista Técnica gera o debrief com as seis seções.
- [x] Instalações antigas (rev 0/1/2) migram em uma passada (rev0→3).
- [x] Prompt editado ou vazio do usuário é preservado; agente excluído não volta.

## Testes

- `application::prompts::summary::tests::summary_contract_has_the_shared_reading_rules`
- `application::prompts::summary::tests::summary_system_prompt_prepends_the_contract`
- `application::prompts::summary::tests::technical_summary_prompt_has_the_required_sections`
- `application::summary_service::tests::uses_the_technical_agents_summary_prompt`
- `application::assistant_catalog_migration::tests::revision_3_*` (4 testes,
  incluindo o caminho rev0→3 com sementes legadas)
- `cargo test --lib` → **673** testes, 0 falhas (10 ignorados).
