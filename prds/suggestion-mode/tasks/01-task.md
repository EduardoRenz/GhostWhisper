# 01 — Domínio: blocos, cues, heurística e decisão pura

- **Status:** CONCLUDED (2026-10-26)
- **Depende de:** —
- **Habilita:** 02, 03, 05
- **Issues:** — · **BR:** BR-092, BR-093, BR-094

## Objetivo

Criar o núcleo puro do modo de sugestão: tipos de **bloco de conversa**, **cue**,
**configuração de blocos** e as funções determinísticas de montagem, heurística e
decisão. Nada de I/O, threads, HTTP ou UI.

## Entregáveis

- `src-tauri/src/domain/suggestion_gate.rs` (novo):
  - `SuggestionMode` (`Auto`, `Classifier`);
  - `SuggestionCue` (`Question`, `Hesitation`, `LostThread`, `Invitation`,
    `Silence`, `NoSuggestion`, `InsufficientContext`);
  - `BlockTurn`, `ConversationBlock`, `BlockConfig`;
  - `build_blocks(...)`;
  - `heuristic_cue(...)` + `HeuristicConfig`;
  - `GateTick`, `GateDecision`, `GateSkipReason`, `decide(...)`.
- `src-tauri/src/domain/mod.rs`: registrar o módulo.
- Testes no próprio arquivo (`#[cfg(test)]`).

## Critérios de aceite

- [x] `build_blocks` ignora segmentos não finais e vazios.
- [x] Fecha bloco por **gap de silêncio**, **duração máxima** e **nº de turnos**.
- [x] O último bloco fica `is_open = true` até o gap ser observado por `now_ms`.
- [x] `id` é determinístico para o mesmo conjunto de segmentos.
- [x] `is_classifiable` exige bloco fechado + ME + THEY + mínimo de caracteres.
- [x] `heuristic_cue`:
  - `question` por `?`/`？` em fala de THEY;
  - `hesitation` por filler/repetição/autocorreção/trailing em ME;
  - `silence` quando o último turno é ME e o gap ≥ `silence_ms`;
  - `no_suggestion` no restante;
  - `insufficient_context` para bloco curto/sem os dois falantes.
- [x] `decide` retorna `Generate` só com enabled/sessão/cue permitida/confiança/
      cooldown/dedupe/generating válidos.
- [x] Sem `std::fs`, `reqwest`, `ureq`, `tauri` ou relógio de parede no arquivo.

## Testes

- `closes_block_on_silence_gap`
- `closes_block_on_max_duration`
- `closes_block_on_max_turns`
- `last_block_stays_open_until_gap`
- `block_id_is_deterministic`
- `mode_and_cue_strings_are_stable`
- `classifiable_requires_both_speakers_and_min_chars`
- `ignores_non_final_empty_and_other_session`
- `heuristic_detects_question_pt_en`
- `heuristic_detects_hesitation_fillers_and_self_correction`
- `heuristic_detects_trailing_off_after_long_speech`
- `heuristic_detects_silence_after_me`
- `heuristic_returns_no_suggestion_for_fluent_exchange`
- `heuristic_returns_insufficient_context_for_short_or_one_sided_block`
- `heuristic_never_returns_lost_thread_or_invitation`
- `decide_skips_disabled_or_without_session`
- `decide_skips_no_suggestion_and_disallowed_cue`
- `decide_skips_below_confidence_or_invalid_confidence`
- `decide_skips_generating_duplicate_and_cooldown`
- `decide_generates_when_all_conditions_hold`

## Como testar

```bash
cargo test --lib domain::suggestion_gate
```

## Verificação (2026-10-26)

- `cargo test --lib domain::suggestion_gate` → **20 passed**, 0 failed.
- `cargo test --lib` → **721 passed**, 10 ignored.
- `cargo clippy --lib -- -D warnings` e `cargo fmt --check` limpos.
- `#![allow(dead_code)]` temporário no módulo: o gate só é consumido nas tasks
  02/03; remover quando o scheduler estiver ligado (TODO no arquivo).

## Notas

- Preferir `&[TranscriptSegment]` + `now_ms` para manter a função pura; a
  aplicação passa o relógio.
- Não usar `unwrap()` em produção.
- A heurística é o **backend offline**; o classificador de IA usa os mesmos
  tipos, mas não as mesmas regras.
