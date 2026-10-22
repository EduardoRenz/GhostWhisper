# 13 — Contrato comum de leitura da transcrição

- **Status:** CONCLUDED
- **Depende de:** 02, 03
- **Habilita:** —
- **Issues:** — · **BR:** BR-082

## Objetivo

Tirar dos prompts de cada agente as regras **gerais e reutilizáveis** do
contexto do app — o texto é uma transcrição de fala (microfone = ME, áudio do
sistema = THEY), com erros de reconhecimento, fragmentos, sobreposição e
atribuição imperfeita — e aplicá-las **sempre** no prompt de sistema, para todo
agente (protegido, de fábrica e personalizado) e para o fast-first.

O campo editável do agente fica só com o que é específico da cena; o contrato
comum não depende de o usuário copiá-lo.

Detalhe de produto: [`../prd.md`](../prd.md) §13. Desenho técnico:
[`../architecture/assistants-panel.md`](../architecture/assistants-panel.md) §3.2.

## Entregáveis

- `src-tauri/src/application/prompts/assistant.rs`
  - `TRANSCRIPT_CONTRACT` (ME/THEY, ASR, atribuição, correção silenciosa,
    proibição de exibir os rótulos);
  - `suggestion_system_prompt` e `fast_first_system_prompt` começam com o
    contrato;
  - remoção das linhas duplicadas de ME/THEY dos prompts de fábrica
    (`DEFAULT_SUGGESTION_PROMPT`, `MEETING_SUGGESTION_PROMPT`,
    `TECHNICAL_INTERVIEW_SUGGESTION_PROMPT`, `LEETCODE_SUGGESTION_PROMPT`,
    `SYSTEM_DESIGN_SUGGESTION_PROMPT` e `MEETING_SYSTEM_PROMPT`);
  - testes: todo agente (incluindo um personalizado) e o fast-first recebem o
    contrato; os prompts de fábrica **não** o repetem.
- `src-tauri/src/application/ask_service.rs`
  - `ASK_RULES` explica `ME:`/`THEY:` e a atribuição imperfeita no bloco
    `<transcript>`; **não** injeta o contrato ao vivo inteiro (lá a pergunta do
    usuário é a prioridade, não a fala do THEY).
- `src-tauri/src/application/assistant_catalog_migration.rs`
  - normalização de boot dos agentes de fábrica **já semeados**: remove os
    trechos antigos que passaram a ser comuns, preserva o resto do texto e as
    edições do usuário; agentes personalizados ficam intactos; idempotente.
- `src-tauri/src/application/prompts/summary.rs`
  - o contrato de resumo mantém a normalização profunda e ganha a definição de
    `ME:`/`THEY:`.

## Critérios de aceite

- [x] Todo agente, inclusive personalizado, recebe o `TRANSCRIPT_CONTRACT` no
      início do system prompt.
- [x] O fast-first também recebe.
- [x] Os prompts de fábrica não repetem ME/THEY/ASR.
- [x] A Busca explica ME/THEY sem herdar a prioridade do THEY.
- [x] O resumo continua com o contrato profundo e entende ME/THEY.
- [x] Agentes de fábrica já semeados perdem as linhas antigas sem perder o
      resto do texto; agentes personalizados não são tocados.

## Testes

- `application::prompts::assistant::tests::every_agent_gets_the_shared_transcript_contract`
- `application::prompts::assistant::tests::the_fast_first_prompt_also_gets_the_shared_contract`
- `application::prompts::assistant::tests::the_factory_prompts_do_not_repeat_the_transcript_contract`
- `application::ask_service::tests::the_ask_explains_the_transcript_tags_without_the_live_priority_rule`
- `application::prompts::summary::tests::system_prompt_has_the_contract_sections`
- `application::assistant_catalog_migration::tests::normalizes_the_legacy_transcript_lines_from_factory_agents`
- `application::assistant_catalog_migration::tests::removes_the_short_transcript_lines_from_factory_prompts`
- `application::assistant_catalog_migration::tests::normalizing_the_legacy_prompts_is_idempotent`
- `cargo test --lib` → **634** testes, 0 falhas (10 ignorados).
