# 04 — Roteador com fallback, latência e evento

- **Status:** CONCLUDED
- **Depende de:** 03
- **Habilita:** 05, 06

## Objetivo

Transformar o `LlmRouter` em uma **cadeia**: a fonte primária responde; se
falhar, a próxima fonte habilitada assume. Registrar latência e estado, e
avisar a UI quando houve fallback.

## Entregáveis

- `application/llm_router.rs`:
  - `complete`/`complete_streaming` percorrem a cadeia a partir da fonte
    resolvida pelo escopo;
  - fonte sem chave ou com circuito aberto é **pulada** (sem gastar timeout);
  - sucesso ⇒ registra saúde (latência) e emite `llm_source_used`;
  - falha ⇒ registra a falha e tenta a próxima; erro só quando **todas** as
    candidatas falharam (mensagem agregada, **sem** segredos);
  - `validate` valida **uma** fonte, sem fallback.
- `ports/llm_controller.rs`: `complete_for(&AiRouteScope, &CompletionRequest)`
  (default delega a `complete`).
- `ports/event_publisher.rs`: `emit_llm_source_used(&LlmSourceUsage)` +
  `LlmSourceUsage { source_id, label, is_fallback, latency_ms }`.
- `adapters/events/tauri_event_publisher.rs`: evento `llm_source_used`.
- `application/summary_service.rs`, `application/suggestion_service.rs`,
  `application/translation_service.rs`: passam a chamar `complete_for` com o
  escopo (Capability / Agent) — **sem rota, o comportamento é o de hoje**.

## Critérios de aceite

- [x] Falha na primária ⇒ a segunda responde e o evento marca `is_fallback`.
- [x] Todas as fontes falhando ⇒ erro agregado sem vazar chave.
- [x] Fonte com circuito aberto é pulada.
- [x] `complete_for` com escopo sem rota usa a primária.
- [x] `complete_for` com rota de agente usa a fonte vinculada.
- [x] O streaming **não** troca de fonte depois de emitir texto.
- [x] `cargo test` verde.
