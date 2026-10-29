# 21 — Resposta rápida não some com modelo rápido/streaming

- **Status:** CONCLUDED
- **Depende de:** 16
- **Habilita:** —
- **Issues:** ISS-062 · **BR:** BR-088

## Objetivo

Com os passes rápido e completo em paralelo, a resposta completa pode chegar
(e até terminar) antes do rascunho — especialmente em modelos com streaming
rápido, como DeepSeek e Mercury. A rápida era descartada nesse caso, então o
bloco **Resposta rápida** sumia de forma intermitente. A rápida passa a ser
independente: sempre preenche o bloco próprio da mesma geração; só uma `quick`
de geração já substituída é ignorada.

## Mudanças

- `src-tauri/src/application/suggestion_service.rs`
  - removida a flag `full_emitted` e o descarte do rascunho atrasado;
  - a `quick` é publicada quando o passe rápido termina, mesmo depois da
    completa.
- `src/core/application/useSuggestions.ts`
  - aceita `quick` atrasada da **mesma** geração;
  - ignora `quick` de uma geração que já foi substituída por uma completa mais
    nova, usando `generation_id` + `at_ms`;
  - o filtro do subscribe não descarta mais toda `quick` cuja completa já
    chegou.
- Documentação: BR-088, PRD §13.1, architecture §3.7 e task 16 (revisão).

## Critérios de aceite

- [x] Modelo com completa rápida/streaming ainda mostra a resposta rápida.
- [x] A rápida atrasada da mesma geração não limpa a completa.
- [x] A rápida não reabre o aviso “Gerando resposta completa…” quando a
      completa já chegou.
- [x] Uma `quick` de geração antiga não sobrescreve a sugestão atual.
- [x] Uma falha no rascunho continua sem derrubar a completa.

## Testes

- `application::suggestion_service::tests::a_late_quick_draft_still_fills_the_quick_block`
- `application::suggestion_service::tests::fast_first_emits_a_short_draft_before_the_full_suggestion`
- `core/application/useSuggestions.test.tsx` — “mostra a rápida que chega depois
  da completa da mesma geração”.
- `core/application/useSuggestions.test.tsx` — “ignora a rápida de uma geração
  antiga depois de uma completa mais nova”.
- `cargo test --lib` → **855** testes, 0 falhas (11 ignorados).
- `cargo test --lib --features laya` → **858** testes, 0 falhas (24 ignorados).
- `pnpm test` → **527** testes; `tsc --noEmit` limpo; `clippy` limpo.
