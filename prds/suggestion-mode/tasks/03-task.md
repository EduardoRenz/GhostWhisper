# 03 — Serviço de gate + cache da última classificação

- **Status:** CONCLUDED (2026-10-26)
- **Depende de:** 01, 02
- **Habilita:** 04, 07, 08, 10
- **Issues:** — · **BR:** BR-093, BR-094, BR-095

## Objetivo

Orquestrar a classificação de blocos e a decisão de gerar: ler falas, montar
blocos, chamar o classificador, publicar o evento, guardar a última classificação
por sessão e aplicar limiar/allowlist/cooldown/dedupe.

## Entregáveis

- `src-tauri/src/ports/suggestion_gate.rs` (novo):
  - `SuggestionGate` (`classify_latest`, `last`, `pending`, `take_pending`,
    `should_generate`, `mark_generated`, `reset`);
  - `SuggestionGateConfig` (montada pela task 04) e `SuggestionGateError`;
  - `GateDecision`/`GateSkipReason` vêm do domínio.
- `src-tauri/src/application/suggestion_gate_service.rs` (novo):
  - implementação com `Arc<dyn SuggestionCueClassifier>`,
    `Arc<dyn TranscriptionsRepository>`, `Arc<dyn EventPublisher>`,
    `Arc<dyn Clock>` e `SuggestionGateConfig`;
  - cache `Mutex<HashMap<String, ClassificationOutcome>>`;
  - pending decision por sessão;
  - reset ao trocar de sessão.
- `src-tauri/src/ports/repositories.rs`:
  - `list_recent_by_session(session_id, since_ms, limit)`.
- `src-tauri/src/adapters/persistence/sqlite/local_transcriptions_repository.rs`
  e `.../memory/in_memory_transcriptions_repository.rs`: implementar.
- `src-tauri/src/application/mod.rs`: registrar.
- `src-tauri/src/state.rs`: `Arc<dyn SuggestionGate>` — **adiado para a task
  10** (o gate ainda não tem classificador concreto; não faz sentido injetar um
  placeholder no composition root).

## Critérios de aceite

- [x] `classify_latest(session, false)` escolhe o bloco **fechado mais recente**
      com ME + THEY; sem bloco elegível, retorna erro tipado/estado vazio.
- [x] `classify_latest(session, true)` usa o bloco **aberto** (sonda).
- [x] O request inclui até `context_blocks` blocos anteriores.
- [x] O outcome é normalizado e publicado (`Ready`) antes de retornar.
- [x] `last(session)` devolve a última classificação (mesmo negativa) ou `None`.
- [x] `should_generate` aplica: enabled, sessão, `cue.can_suggest`, allowlist,
      confiança, cooldown, `block_id` não repetido, `generating=false`.
- [x] Cooldown vencido com decisão positiva vira `pending`; gera no próximo
      tick sem reclassificar.
- [x] Bloco novo substitui a pendência; decisão negativa limpa.
- [x] Erro/timeout guarda estado de erro e **não** apaga a última válida.
- [x] Trocar de sessão limpa cache/pendência.
- [x] Nenhum log de texto; só `block_id`, cue, confidence, latency, backend.

## Testes

- `classifies_newest_closed_block_with_both_speakers`
- `does_not_classify_same_block_twice`
- `force_open_probes_the_open_block`
- `includes_previous_blocks_as_context`
- `last_classification_survives_no_suggestion`
- `classifier_error_keeps_last_valid_outcome`
- `skips_when_cue_disabled_or_below_confidence`
- `skips_cooldown_and_duplicate_block`
- `pending_generates_after_cooldown_without_reclassifying`
- `new_block_replaces_pending_decision`
- `generating_blocks_and_reset_clears_state`
- `list_recent_filters_finals_and_keeps_chronological_order`
- `list_recent_filters_and_keeps_the_last_n_in_order`

## Como testar

```bash
cargo test --lib application::suggestion_gate_service
cargo test --lib adapters::persistence
```

## Verificação (2026-10-26)

- `cargo test --lib suggestion_gate_service` → **12 passed**.
- `cargo test --lib list_recent` → **2 passed**.
- `cargo test --lib` → **743 passed**, 10 ignored.
- `cargo clippy --lib -- -D warnings` e `cargo fmt --check` limpos.
- `allow(dead_code)` temporário na porta/serviço até o scheduler da task 10.

## Notas

- Não chamar `SuggestionService` aqui; a geração fica no scheduler (task 10)
  para manter o serviço de gate focado em decisão.
- `Clock` é obrigatório para `classified_at_ms` e cooldown; não usar
  `SystemTime::now()` no serviço.
- O cache é memória por sessão (não-objetivo persistir no POC).
- A configuração vem de `SuggestionGateConfig`; a task 04 é quem traduz as
  settings (modo, sensibilidade, blocos) para esse struct.
