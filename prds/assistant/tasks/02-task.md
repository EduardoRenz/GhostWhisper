# 02 — Serviços de resumo/sugestão, IPC e eventos

- **Status:** CONCLUDED
- **Depende de:** 01
- **Habilita:** 03, 04

## Objetivo

Implementar os casos de uso que chamam o LLM e expor os comandos Tauri.

## Entregáveis

- `application/summary_service.rs` (`SummaryGenerator`):
  - resolução da sessão (ativa/pedida/mais recente);
  - primeira geração com contrato completo + envelope;
  - atualização incremental com cursor e até 3 chunks por tick;
  - `manual` regenera do zero;
  - parse de `SUBJECT:`/`CONTENT:`, normalização de cercas externas;
  - persistência em `meeting_summaries` + espelho em `sessions.overview` e
    `subject` quando vazio;
  - `updated=false`/`has_more` quando não há conteúdo novo.
- `application/suggestion_service.rs` (`SuggestionGenerator`):
  - resolução do modo (embutido/personalizado);
  - contexto dos últimos 3 min (≤40 falas / ≤6k chars);
  - `since_ms` → `skipped=true` sem chamar o LLM;
  - `<last_suggestion>` e `<additional_context>` no pedido;
  - normalização do Markdown;
  - `persist=true` grava em `messages` e devolve `message_id`;
  - `pin` grava sem chamar o LLM.
- Substituir `InsightService`/`InsightGenerator` por `SuggestionService`/
  `SuggestionGenerator`; manter o comando `llm_insight`, o atalho e o evento
  `insight` funcionando (modo selecionado, persistido).
- `ipc/summary_commands.rs`: `summary_get`, `summary_generate`.
- `ipc/suggestion_commands.rs`: `suggestion_generate`, `suggestion_pin`,
  `assistant_modes_list`.
- `state.rs`/`lib.rs`: novas portas e wiring.
- Testes com `FakeLlmController`: prompts, cursor, chunks, skip, pin, erros.

## Critérios de aceite

- [x] Resumo inicial de uma sessão nova é persistido e devolve `subject`.
- [x] Segundo tick sem fala nova não chama o LLM.
- [x] Sessão longa avança em chunks e mantém `summarized_from_ms`.
- [x] `manual` regenera e reseta o cursor.
- [x] Sugestão sem fala retorna erro claro; sem fala nova retorna `skipped`.
- [x] `persist=true` e `pin` aparecem em `messages` com o tempo da sessão.
- [x] O comando `llm_insight` continua funcionando para o chat/atalho.
- [x] `cargo test` e `clippy` verdes.
