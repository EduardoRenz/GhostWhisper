# 04 — Settings, migração, tipos e IPC

- **Status:** CONCLUDED (2026-10-26)
- **Depende de:** 03
- **Habilita:** 05, 06, 07, 09, 10
- **Issues:** — · **BR:** BR-092, BR-095, BR-097

## Objetivo

Persistir o modo e a configuração do classificador, migrar settings antigos sem
quebrar ninguém, expor os tipos ao frontend e adicionar os comandos IPC de
consulta/classificação manual.

## Entregáveis

### Rust

- `src-tauri/src/domain/settings.rs`:
  - `suggestion_mode: String` (`auto` | `classifier`; default `auto`) — string
    para tolerar settings antigas e sanitizar sem falhar o parse;
  - `suggestion_show_classification: bool` (default `true`);
  - `suggestion_classifier_backend` (`heuristic` | `laya`);
  - `suggestion_classifier_sensitivity` (`low` | `balanced` | `high`;
    default `balanced`) — o limiar é derivado (0,85/0,70/0,55);
  - `suggestion_classifier_model_id` (default `laya-multilingual`);
  - `suggestion_classifier_model_version` (vazio = ativo);
  - `suggestion_classifier_model_path` (override de dev; vazio = model manager);
  - `suggestion_classifier_auto_download` (default `false`);
  - `suggestion_classifier_model_catalog_url` (default oficial);
  - `suggestion_classifier_checkpoint` (default `multilingual`);
  - `suggestion_classifier_cooldown_secs` (5–120; default 15);
  - `suggestion_classifier_timeout_ms` (10.000–30.000; default 12.000);
  - `suggestion_classifier_fallback` (`skip` | `auto`; default `skip`);
  - `suggestion_block_gap_ms` (1.000–10.000; default 3.000);
  - `suggestion_block_max_ms` (10.000–120.000; default 45.000);
  - `suggestion_block_max_turns` (2–24; default 12);
  - `suggestion_context_blocks` (0–5; default 3);
  - `suggestion_allowed_cues: Vec<String>` (sanitizado para cues positivas);
  - `suggestion_probe_open_block` (default `true`);
  - `suggestion_hesitation_probe_ms` (2.000–10.000; default 4.000);
  - `MIN_SUGGESTION_INTERVAL_SECS = 8`; o resumo continua 10–120;
  - `SUGGESTION_INTERVALS = [8, 15, 30]` fica no TS (presets da UI); valores
    fora dos presets são preservados e a UI mostra um chip “Custom”;
  - `sanitize()`/clamps; JSON antigo sem `suggestion_mode` → `Auto`.
- `src-tauri/src/ports/suggestion_generator.rs`:
  - `SuggestionTrigger::Classifier`;
  - `Suggestion.cue: Option<SuggestionCue>`;
  - `SuggestionRequest` ganha `cue`/`focus` (opcionais).
- `src-tauri/src/ipc/suggestion_commands.rs`:
  - `suggestion_classification_last(session_id)`;
  - `suggestion_classify_now(session_id)` — classifica o último bloco, sem
    gerar sugestão (o “colar exemplo” fica para a task 09);
  - registrar em `lib.rs`.
- `src-tauri/src/lib.rs`: wiring do `Arc<dyn SuggestionGate>` no `AppState`.
- `src-tauri/src/state.rs`: campo novo.

### TypeScript

- `src/core/domain/settings.ts`: campos + defaults idênticos.
- `src/core/domain/assistant.ts`/`suggestion.ts`: `SuggestionMode`,
  `SuggestionCue`, `SuggestionClassification`, `SuggestionTrigger` com
  `classifier`, `Suggestion.cue`.
- `src/adapters/ipc/suggestions.ts`: `classificationLast`, `classifyNow`.
- `src/core/l10n/locales/{pt,en}.ts`: chaves da §8.5 do PRD — **adiado para as
  tasks 07–09**, quando a UI que usa a copy entrar.

## Critérios de aceite

- [x] `Settings` antigo (JSON sem campos novos) carrega com defaults seguros.
- [x] `suggestion_auto_refresh=false` desliga tudo, independente do modo.
- [x] Ao religar, o modo salvo é preservado.
- [x] Todos os campos são clamped/sanitizados; caminho vazio usa
      `LAYA_ONNX_DIR`; checkpoint inválido volta para `multilingual`.
- [x] Cues desconhecidas são descartadas; `no_suggestion` e
      `insufficient_context` nunca entram na allowlist.
- [x] `SuggestionTrigger` serializa `classifier` e a UI compila.
- [x] `suggestion_classification_last` devolve `None` sem sessão/classificação.
- [x] `suggestion_classify_now` publica o evento e **não** cria sugestão.
- [x] Nenhum token no JSON; a única URL é o catálogo de modelos
      (`suggestion_classifier_model_catalog_url`); a inferência é local.

## Testes

- `default_assistant_settings_are_valid`
- `legacy_json_without_suggestion_classifier_fields_uses_defaults`
- `sanitized_falls_back_on_invalid_suggestion_classifier_settings`
- `settings_map_to_the_gate_config`
- `suggestion_trigger_serializes_classifier`
- `cue_focus_builder_keeps_only_non_empty_focus`
- `state::tests::shares_ports_across_clones` (gate injetado)
- `settings.test.ts` e `assistant.test.ts` no frontend

## Como testar

```bash
cargo test --lib domain::settings
cargo test --lib application::suggestion_gate_service
cargo test --lib ports::suggestion_generator
cargo test --lib
pnpm test
pnpm exec tsc --noEmit
```

## Verificação (2026-10-26)

- `cargo test --lib` → **755 passed**, 10 ignored.
- `pnpm test` → **485 passed**; `pnpm exec tsc --noEmit` limpo.
- `cargo clippy --lib -- -D warnings` e `cargo fmt --check` limpos.
- Gate heurístico injetado no `AppState`; os comandos IPC já leem o cache.
- Laya/model manager ainda não escolhidos no composition root (task 06/10).

## Notas

- `suggestion_auto_refresh` continua com o nome antigo para evitar migração
  destrutiva; ele é o interruptor mestre.
- A UI de settings só é entregue na task 09; aqui só o contrato.
- O comando `classify_now` sem `text` usa o último bloco da sessão; o texto
  colado no painel de teste fica para a task 09.
- As settings usam strings nos campos novos para que um JSON antigo/desconhecido
  seja sanitizado em vez de falhar o parse.
