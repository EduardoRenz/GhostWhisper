# 02 — Porta do classificador + evento + fake

- **Status:** CONCLUDED (2026-10-26)
- **Depende de:** 01
- **Habilita:** 03, 05, 06, 07
- **Issues:** — · **BR:** BR-094

## Objetivo

Definir o contrato do classificador (pedido, resposta, erros), o evento de
classificação para a UI e a dobra de teste `FakeSuggestionCueClassifier`.

## Entregáveis

- `src-tauri/src/ports/suggestion_cue_classifier.rs` (novo):
  - `ClassificationRequest` (sessão, bloco, contexto, locale, mode_id,
    `is_open_probe`);
  - `ClassificationOutcome` (cue, should_suggest, confidence, evidence, focus,
    backend, model, latency_ms, block range, turn_count, classified_at_ms,
    abstained);
  - `ClassifierError` (`Disabled`, `Unavailable`, `Timeout`,
    `InvalidResponse`, `Other`);
  - `SuggestionCueClassifier` (`classify`, `backend_id`).
- `src-tauri/src/ports/event_publisher.rs`:
  - `SuggestionClassifierState` (`Paused`, `NoSession`, `Listening`,
    `Analyzing`, `Ready`, `Error`);
  - `SuggestionClassificationEvent`;
  - `emit_suggestion_classification` com default no-op.
- `src-tauri/src/adapters/events/{tauri,null}_event_publisher.rs`: implementar
  o evento (`suggestion_classification` no Tauri).
- `src-tauri/src/ports/mod.rs`: registrar.
- `FakeSuggestionCueClassifier` em testes (fila de resultados/erros + requests
  registrados).

## Critérios de aceite

- [x] O trait é `Send + Sync` e síncrono.
- [x] `ClassificationOutcome` é `Serialize` e todos os campos têm tipo estável.
- [x] `confidence` é normalizada para `0.0..=1.0` no adaptador; o contrato não
      confia no backend.
- [x] `evidence` é truncada em 160 caracteres; `focus` em 240.
- [x] `should_suggest` é coerente com `cue.can_suggest()` no `normalized()`.
- [x] O fake registra cada request e devolve o próximo resultado/erro.
- [x] O evento não inclui transcrição completa (só evidence curta).
- [x] `NullEventPublisher` e todos os test doubles compilam com o novo método.

## Testes

- `returns_queued_outcomes_and_errors_in_order`
- `records_each_request`
- `empty_queue_is_unavailable`
- `backend_id_is_configurable`
- `request_carries_block_and_context`
- `outcome_normalizes_confidence_and_negative_cue`
- `outcome_serializes_snake_case_fields`
- `classification_event_serializes_snake_case`

## Como testar

```bash
cargo test --lib ports::suggestion_cue_classifier
cargo test --lib adapters::events
```

## Verificação (2026-10-26)

- `cargo test --lib suggestion_cue_classifier` → **7 passed**.
- `cargo test --lib event_publisher` → **1 passed**.
- `cargo clippy --lib -- -D warnings` e `cargo fmt --check` limpos.
- `allow(dead_code)` temporário nos tipos do evento e na porta: serão
  consumidos pelo gate na task 03 (TODOs no código).

## Notas

- O evento carrega `outcome: Option<...>` + `error: Option<String>`; a UI usa
  `state` para escolher o rótulo.
- Não adicionar `reason` livre: a explicação é determinística por cue + evidence.
- O `backend_id` é para a UI/settings; nunca contém segredo.
