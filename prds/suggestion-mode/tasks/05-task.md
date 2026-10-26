# 05 — Backend heurístico local (offline)

- **Status:** CONCLUDED (2026-10-26)
- **Depende de:** 01, 02
- **Habilita:** 09, 10
- **Issues:** — · **BR:** BR-094

## Objetivo

Implementar o backend `heuristic` do `SuggestionCueClassifier` usando apenas as
funções puras do domínio. Ele funciona offline, sem modelo, e serve de fallback
quando o Laya não está disponível.

## Entregáveis

- `src-tauri/src/adapters/ai/heuristic_suggestion_cue_classifier.rs` (novo):
  - implementa `SuggestionCueClassifier`;
  - `backend_id() -> "heuristic"`;
  - usa `heuristic_cue` + `ConversationBlock`;
  - `confidence` determinística:
    - `question` 0,90;
    - `hesitation` 0,80;
    - `silence` 0,75;
    - `no_suggestion` 0,60;
    - `insufficient_context` 0,40.
  - `evidence`: última fala relevante (THEY para pergunta; ME para hesitação/
    silêncio), truncada em 160 chars;
  - `focus`: rótulo curto por cue;
  - `latency_ms`: 0 (sem I/O).
- `src-tauri/src/adapters/ai/mod.rs`: registrar.
- Testes de fixture pt/en.

## Critérios de aceite

- [x] `question` quando THEY contém `?` ou `？` (inclusive no meio da frase).
- [x] `hesitation` com fillers (`um`, `uh`, `hmm`, `tipo`, `é`, `ahn`),
      repetição adjacente, autocorreção (`na verdade`, `espera`, `I mean`) ou
      reticências após 4 s.
- [x] `silence` quando o bloco fechou, o último turno é ME e o gap é ≥ 3 s.
- [x] `no_suggestion` em conversa fluente sem pergunta/sinal.
- [x] `insufficient_context` para < 3 palavras ou sem os dois falantes.
- [x] Não usa `std::time` nem I/O; `latency_ms = 0`.
- [x] Não gera `lost_thread`/`invitation` (exclusivos da IA).

## Testes

- `detects_question_with_they_evidence`
- `detects_hesitation_with_me_evidence`
- `detects_silence_after_me`
- `fluent_exchange_is_no_suggestion`
- `short_block_is_insufficient_context_and_abstains`
- `backend_id_is_heuristic`
- `demo_prints_heuristic_classifications` (demo end-to-end com o gate)

## Como testar

```bash
cargo test --lib adapters::ai::heuristic_suggestion_cue_classifier
# demo end-to-end (imprime cue/decisão de 4 blocos):
cargo test --lib demo_prints_heuristic_classifications -- --nocapture
```

## Verificação (2026-10-26)

- `cargo test --lib heuristic_suggestion_cue_classifier` → **6 passed**.
- Demo end-to-end imprime `question`, `hesitation`, `silence` e
  `no_suggestion` com `Generate`/`Skip(NoSuggestion)`.
- `cargo test --lib` → **750 passed**, 10 ignored.
- `cargo clippy --lib -- -D warnings` e `cargo fmt --check` limpos.
- `allow(dead_code)` temporário até o scheduler da task 10 escolher o backend.

## Correção (2026-10-26)

- A sonda aberta entrega bloco só de ME; o heurístico agora avalia hesitação
  **antes** de exigir os dois falantes e reconhece incerteza explícita
  (“não tenho certeza”, “deixa eu pensar”). Ver ISS-061.
- Testes: `heuristic_detects_uncertainty_in_me_only_block` e
  `heuristic_me_only_without_markers_is_insufficient`.

## Notas

- A heurística é de propósito conservadora: é melhor `no_suggestion` do que
  interromper à toa.
- O backend é selecionável em Fontes de IA & Modelos → card “Modelo de
  classificação” → Modelo selecionado (`Regras locais`).
- O texto de fixture não deve conter dados reais.
