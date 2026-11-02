# 06 — Injeção no prompt + integração do serviço

- **Status:** CONCLUDED (2026-11-01)
- **Depende de:** 04 (e 02/01)
- **Habilita:** 07
- **Issues:** ISS-091 · **BR:** BR-105

## Objetivo

Criar o `ScreenContextService` (cache, idade, timeout, evento) e injetar o
bloco `<screen_context>` na pergunta quando a flag estiver ligada e o snapshot
for fresco (≤ 30 s).

## Entregáveis

- `src-tauri/src/application/screen_context_service.rs` (novo):
  - `snapshot_for_ask()` com leitura da flag (desligada → `None` sem tocar na
    porta), timeout 500 ms, cache 5 s;
  - publicação do evento `screen_context` (preview ≤ 300 chars) na abertura e
    em mudança de foco (debounce 500 ms);
  - `CONTEXT_FRESH_MS` gateia o prompt (não a pill).
- `src-tauri/src/application/ask_service.rs::build_prompt`: bloco
  `<screen_context app="…" window="…">` entre `<previous_questions>` e o
  transcript; regra nova no `ASK_RULES`.
- `src-tauri/src/ipc/`: command `screen_context_snapshot` +
  command `screen_context_status` (se não feito na task 03) + registro do
  evento.
- `lib.rs`: ligar serviço + porta + publishers (composition root).
- Testes do serviço com `FakeScreenContextSource` + `FakeClock`.

## Critérios de aceite

- [ ] Flag desligada: `build_prompt` idêntico ao atual (regressão byte a byte).
- [ ] Flag ligada + snapshot ≤ 30 s: bloco presente com app/título/texto.
- [ ] Snapshot > 30 s: bloco ausente (pergunta segue limpa).
- [ ] Print anexado + flag ligada: imagem **e** bloco vão juntos.
- [ ] Preview publicado ≤ 300 chars; `text` completo não sai do core.
- [ ] Timeout da porta → estado `Indisponível` no evento, sem quebrar o ask.
- [ ] Nenhum texto de contexto em log (teste no harness).

## Testes

- `prompt_unchanged_when_flag_disabled`
- `prompt_includes_fresh_context`
- `prompt_skips_stale_context`
- `screenshot_and_context_coexist`
- `event_preview_is_capped`
- `port_error_degrades_to_unavailable`
