# 10 — Fallback de prompt vazio: herda o do Padrão

- **Status:** CONCLUDED
- **Depende de:** 01, 05
- **Habilita:** feature de resumo por agente (consome o helper)
- **Issues:** — · **BR:** BR-079

## Objetivo

Garantir que **nenhum agente fique sem contrato**: em qualquer prompt
(`system_prompt`, `suggestion_prompt`, `summary_prompt`), um valor vazio depois
do `trim` usa o prompt correspondente do **agente Padrão**.

Detalhe de produto: [`../prd.md`](../prd.md) §7.4. Desenho técnico:
[`../architecture/assistants-panel.md`](../architecture/assistants-panel.md) §3.2.

## Entregáveis

- `src-tauri/src/application/prompts/assistant.rs`
  - `effective_suggestion_prompt` / `effective_system_prompt` /
    `effective_summary_prompt` (todos com `trim`);
  - `suggestion_system_prompt` passa a usar o contrato do **Padrão** quando o
    `suggestion_prompt` está vazio (o fallback antigo era o de Reunião);
  - o bloco `## Domain criteria` passa a ser **sempre** incluído (do agente ou
    do Padrão).
- `src/features/settings/AgentCard.tsx`
  - prop `defaultMode`; resolve o prompt efetivo e mostra a nota
    "Herdado do agente Padrão" quando o campo do agente está vazio.
- `src/features/settings/AssistantsSettings.tsx`
  - passa o agente Padrão (do catálogo) aos cards.
- `src/styles/globals.css`
  - `.prompt-panel__inherited`.

## Critérios de aceite

- [ ] Um agente com `suggestion_prompt` vazio (ou só espaços) usa o contrato de
      sugestão do **Padrão**, não o de Reunião.
- [ ] Um agente com `system_prompt` vazio usa os critérios do **Padrão** — o
      bloco `## Domain criteria` nunca fica de fora.
- [ ] `effective_summary_prompt` devolve o contrato do Padrão quando o campo
      está vazio (pronto para a feature de resumo).
- [ ] O `context` **não** entra na regra (é contexto, não prompt).
- [ ] O card mostra o prompt efetivo e marca "Herdado do agente Padrão" só
      quando o valor veio do Padrão.
- [ ] A validação "ao menos um prompt/contexto" continua valendo.

## Testes

- `application/prompts/assistant.rs`:
  `custom_mode_without_prompts_inherits_the_default_agent`,
  `effective_prompts_prefer_the_agent_and_fall_back_to_the_default`.
- `src/features/settings/AgentCard.test.tsx`: herda do Padrão (2 painéis) e não
  marca quando o agente tem o próprio prompt.
- `cargo test`, `cargo clippy`, `pnpm test`, `tsc --noEmit`.
