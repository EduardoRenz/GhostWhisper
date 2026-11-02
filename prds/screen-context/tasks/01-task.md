# 01 — Domínio puro: snapshot, poda e orçamentos

- **Status:** CONCLUDED (2026-11-01)
- **Depende de:** —
- **Habilita:** 02, 04, 06
- **Issues:** — · **BR:** BR-105

## Objetivo

Criar o núcleo puro do contexto de tela: tipos de snapshot, DTO da árvore
acessível e a função determinística de **poda** que transforma a árvore crua em
texto com orçamento. Nada de I/O, D-Bus, threads ou UI.

## Entregáveis

- `src-tauri/src/domain/screen_context.rs` (novo):
  - `ScreenContextSnapshot { app, window_title, text, captured_at_ms }`;
  - `AccessibleNode { role, name, value, children, invisible }` (DTO mínimo);
  - constantes `CONTEXT_MAX_CHARS` (2 000), `TITLE_MAX_CHARS` (120),
    `PREVIEW_MAX_CHARS` (300), `CONTEXT_FRESH_MS` (30 000);
  - `extract_text(root, budget) -> String` com as regras do architecture §2.1;
  - `truncate_title(&str) -> String`.
- `src-tauri/src/domain/mod.rs`: registrar o módulo.
- Testes no próprio arquivo (`#[cfg(test)]`).

## Critérios de aceite

- [ ] `extract_text` concatena texto em ordem de leitura.
- [ ] Pula nós `invisible`/`hidden` e roles estruturais puros.
- [ ] Usa `name`/`value` conforme o role (label, entry, heading, link).
- [ ] Deduplica linhas idênticas consecutivas.
- [ ] Respeita o budget com `…`; nunca excede `CONTEXT_MAX_CHARS`.
- [ ] `truncate_title` corta em `TITLE_MAX_CHARS` sem quebrar no meio.
- [ ] Árvore vazia/`None` de children → texto vazio (sem panic).
- [ ] Nenhum `std::fs`, `reqwest`, `zbus`, `atspi` ou relógio de parede.

## Testes

- `extracts_text_in_reading_order`
- `skips_hidden_nodes`
- `uses_value_for_entries_and_name_for_labels`
- `dedupes_consecutive_identical_lines`
- `respects_char_budget`
- `empty_tree_yields_empty_text`
- `truncate_title_respects_limit`
