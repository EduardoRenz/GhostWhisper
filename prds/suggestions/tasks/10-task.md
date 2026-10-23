# 10 — Contexto de sessão recuperada (ISS-048)

- **Status:** CONCLUDED
- **Depende de:** 02, 03
- **Habilita:** —
- **Issues:** ISS-048 · **BR:** BR-089

## Objetivo

Corrigir o falso “transcrição vazia” ao gerar sugestão numa sessão já gravada e
continuada/recuperada. A janela do contexto ao vivo deve usar a **conversa**
como referência, não o relógio de parede: quando a última fala está fora dos
últimos `window_ms` (sessão pausada/retomada horas depois), ancora na última
fala.

## Entregáveis

- `src-tauri/src/application/context.rs`
  - `live_window_anchor` — último segmento com texto quando não há fala dentro
    da janela;
  - `recent_context_live` e `context_since` usam a âncora.
- `src-tauri/src/application/suggestion_service.rs`
  - teste de regressão com clock horas à frente do início da sessão.
- `prds/suggestions/issues.md` — ISS-048.

## Critérios de aceite

- [x] Sessão recuperada com transcrição antiga gera sugestão (não diz vazia).
- [x] Sessão ativa com fala recente continua ancorada no agora.
- [x] A Busca também usa a última fala como âncora quando não há fala recente.
- [x] Sem nenhuma fala utilizável, o erro de transcrição vazia continua.

## Testes

- `application::context::tests::live_context_anchors_on_the_last_speech_after_a_long_pause`
- `application::context::tests::context_since_anchors_on_the_last_speech_after_a_long_pause`
- `application::suggestion_service::tests::recovered_sessions_use_the_last_speech_as_context`
- `cargo test --lib` → **661** testes, 0 falhas (10 ignorados).
- `pnpm test` → **454** testes; `tsc --noEmit` limpo.
