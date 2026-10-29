# 16 — Sugestão: rascunho e resposta completa em paralelo

- **Status:** CONCLUDED
- **Revisão:** 2026-10-28 — o descarte do rascunho atrasado foi substituído
  pela [`21-task.md`](21-task.md) (BR-088): a rápida agora é independente da
  completa.
- **Depende de:** 09, 13
- **Habilita:** —
- **Issues:** — · **BR:** BR-088

## Objetivo

Tirar o passe rápido (`fast-first`) do caminho crítico da resposta completa. Os
dois passes passam a rodar **em paralelo**: a completa começa imediatamente e o
rascunho aparece se chegar antes. Na revisão de 2026-10-28 (task 21), o rascunho
que chega depois deixou de ser descartado: ele ainda preenche a Resposta rápida;
só uma `quick` de geração antiga é ignorada.

## Entregáveis

- `src-tauri/src/application/suggestion_service.rs`
  - `QuickPass` com os dados do rascunho;
  - `std::thread::scope` para o rascunho + completo no thread atual;
  - ~~`AtomicBool` `full_emitted` marcado no primeiro delta do completo~~
    (removido na task 21);
  - rascunho atrasado ainda emite; falha do rascunho só loga.
- `src-tauri/src/application/test_support.rs`
  - atrasos artificiais no rascunho e no completo para testar a ordem.
- `src/core/application/useSuggestions.ts`
  - `Set` de gerações com completa recebida para reconhecer a mesma geração;
  - `quick` atrasada da mesma geração é aceita; a de geração antiga é ignorada
    (revisão da task 21).
- Documentação: BR-088, PRD §13.1/§9, architecture §3.7.

## Critérios de aceite

- [x] O rascunho não bloqueia o início da resposta completa.
- [x] Com o completo mais lento, o rascunho aparece antes (fase `quick`).
- [x] Rascunho que chega depois do primeiro delta do completo **não bloqueia**
      a completa (revisado na task 21: ele ainda preenche a Resposta rápida).
- [x] Falha no rascunho não derruba a completa.
- [x] A UI não deixa uma `quick` de geração antiga sobrescrever a atual
      (revisado na task 21: a `quick` da mesma geração é aceita).

## Testes

- `application::suggestion_service::tests::fast_first_emits_a_short_draft_before_the_full_suggestion`
- `application::suggestion_service::tests::a_late_quick_draft_still_fills_the_quick_block`
  (revisão 2026-10-28; antes: descarte da rápida atrasada).
- `core/application/useSuggestions.test.tsx` — “mostra a rápida que chega
  depois da completa da mesma geração” e “ignora a rápida de uma geração antiga
  depois de uma completa mais nova”.
- `cargo test --lib` → **658** testes, 0 falhas (10 ignorados).
- `pnpm test` → **454** testes; `tsc --noEmit` limpo.

## Limite conhecido

Duas chamadas concorrentes ao mesmo provider podem esbarrar em rate limit. O
rascunho é descartável: se ele falhar, a completa continua. Se o provider
limitar agressivamente, a alternativa é desligar o `fast-first` no manual.
