# 14 — Entrevista Técnica funde System Design; novo Entrevista com RH

- **Status:** CONCLUDED
- **Depende de:** 02, 03, 13
- **Habilita:** —
- **Issues:** — · **BR:** BR-083

## Objetivo

Reduzir a granularidade do catálogo: **Entrevista Técnica** passa a cobrir
também **System Design** (clarify → size → design, QPS, ASCII, componentes
genéricos), e entra um agente novo, **Entrevista com RH**, focado em:

1. listar perguntas ainda não feitas/respondidas que o RH valoriza (vaga,
   planos, benefícios, time, cultura, crescimento, remuneração);
2. sugerir aprofundamento quando ME foi vago, ancorado em algo que o RH disse
   ou no cargo (ex.: vaga de tech lead → exemplos e desafios de liderança);
3. sugerir **STAR** quando a pergunta for comportamental.

Instalações já semeadas recebem a mudança **uma vez**, sem perder contexto nem
a fonte de IA do agente técnico.

Detalhe de produto: [`../prd.md`](../prd.md) §7.2/§7.3. Desenho técnico:
[`../architecture/assistants-panel.md`](../architecture/assistants-panel.md) §3.4.

## Entregáveis

- `src-tauri/src/application/prompts/assistant.rs`
  - `TECHNICAL_SYSTEM_PROMPT` e `TECHNICAL_INTERVIEW_SUGGESTION_PROMPT`
    incorporam o system design (ordem da entrevista, QPS, ASCII, componentes
    genéricos, glossário);
  - `HR_SYSTEM_PROMPT` e `HR_SUGGESTION_PROMPT` (perguntas em aberto,
    aprofundamento, STAR);
  - `factory_modes()`: Reunião, Entrevista Técnica, LeetCode e Entrevista com
    RH; `system_design` sai;
  - `NEVER_ASK_BACK_RULE` deixa claro que a sugestão pode conter uma pergunta
    que **ME** faz ao outro participante.
- `src-tauri/src/domain/assistant.rs`
  - `BUILTIN_MODE_IDS` troca `system_design` por `hr_interview`.
- `src-tauri/src/domain/settings.rs`
  - `assistant_catalog_revision: u32` (`serde(default)` = 0).
- `src-tauri/src/application/assistant_catalog.rs`
  - `ASSISTANT_CATALOG_REVISION = 1`; `default_settings()` grava a revisão.
- `src-tauri/src/application/assistant_catalog_migration.rs`
  - `upgrade_catalog`: remove `system_design`, sobrescreve
    `technical_interview` (preservando `context`/`source_id`; carrega
    contexto/fonte do system design quando o técnico não tinha), adiciona
    `hr_interview` e troca a seleção ativa;
  - `SeedOutcome.upgraded`; revisão idempotente; exclusões posteriores são
    respeitadas.
- `src-tauri/src/application/suggestion_service.rs`
  - remove `system_design` do orçamento técnico; RH usa o orçamento de reunião.
- `src/core/domain/assistant.ts`
  - espelho de `BUILTIN_MODE_IDS`.
- `lib.rs`
  - loga quando o catálogo é atualizado.

## Critérios de aceite

- [x] O catálogo tem **Entrevista Técnica** (código + system design) e
      **Entrevista com RH**; não há mais **System Design** separado.
- [x] O prompt de Entrevista Técnica cobre LeetCode/código e system design.
- [x] O prompt de RH cobre perguntas em aberto, aprofundamento e STAR.
- [x] Instalação existente: `system_design` some, `technical_interview` vira o
      fundido, `hr_interview` entra e a seleção ativa é corrigida.
- [x] Contexto e `source_id` do técnico são preservados; o contexto/fonte do
      system design é carregado quando o técnico não tinha.
- [x] Agentes personalizados não são tocados; um RH excluído depois da revisão
      não volta.
- [x] "Restaurar Padrões" usa o catálogo novo e a revisão atual.

## Testes

- `application::prompts::assistant::tests::the_catalog_covers_the_five_agents`
- `application::prompts::assistant::tests::the_merged_technical_agent_covers_coding_and_system_design`
- `application::prompts::assistant::tests::the_hr_agent_covers_open_questions_deepening_and_star`
- `application::assistant_catalog::tests::the_defaults_carry_the_factory_agents`
- `application::assistant_catalog_migration::tests::upgrades_the_legacy_catalog_once`
- `application::assistant_catalog_migration::tests::catalog_upgrade_respects_a_later_deletion`
- `domain::assistant::tests::only_the_protected_id_is_reserved`
- `domain::settings::tests::legacy_settings_get_assistant_defaults`
- `cargo test --lib` → **638** testes, 0 falhas (10 ignorados).
- `pnpm test` → **422** testes; `tsc --noEmit` limpo.
