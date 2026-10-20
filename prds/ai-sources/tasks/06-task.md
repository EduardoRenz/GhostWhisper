# 06 — IPC das fontes e wiring no composition root

- **Status:** CONCLUDED
- **Depende de:** 04, 05
- **Habilita:** 08

## Objetivo

Expor o serviço de fontes para o frontend e ligar tudo no `AppState`.

## Entregáveis

- `ipc/ai_source_commands.rs`:
  - `ai_sources_list`, `ai_sources_save`, `ai_sources_health`,
    `ai_source_set_key`, `ai_source_has_key`, `ai_source_clear_key`,
    `ai_source_key_preview`, `ai_source_validate`,
    `ai_sources_restore_defaults`.
- `state.rs`: `AppState.ai_sources: Arc<dyn AiSourceService>`.
- `lib.rs`: registra os comandos; monta `AiSourceService` e injeta no
  `AppState`; roda a migração (T02) antes de montar o `LlmRouter`; o
  `LlmRouter` passa a receber o registro de saúde e o publicador de eventos.
- `src/adapters/ipc/aiSources.ts`: cliente IPC do frontend.

## Critérios de aceite

- [x] Os 9 comandos estão registrados no `generate_handler!`.
- [x] `ai_source_key_preview` devolve a versão mascarada (nunca a chave).
- [x] `ai_sources_save` persiste a **ordem** recebida.
- [x] `cargo test` verde.
