# 16 — Sugestão: rascunho e resposta completa em paralelo

- **Status:** CONCLUDED
- **Depende de:** 09, 13
- **Habilita:** —
- **Issues:** — · **BR:** BR-088

## Objetivo

Tirar o passe rápido (`fast-first`) do caminho crítico da resposta completa. Os
dois passes passam a rodar **em paralelo**: a completa começa imediatamente e o
rascunho aparece se chegar antes. Se o rascunho chegar depois que a completa já
começou a falar, ele é descartado — no core e na UI.

## Entregáveis

- `src-tauri/src/application/suggestion_service.rs`
  - `QuickPass` com os dados do rascunho;
  - `std::thread::scope` para o rascunho + completo no thread atual;
  - `AtomicBool` `full_emitted` marcado no primeiro delta do completo;
  - rascunho atrasado não emite; falha do rascunho só loga.
- `src-tauri/src/application/test_support.rs`
  - atrasos artificiais no rascunho e no completo para testar a ordem.
- `src/core/application/useSuggestions.ts`
  - `Set` de gerações com completa recebida;
  - `quick` atrasada da mesma geração é ignorada (não reabre a fase rápida nem
    dispara o aviso de completa pendente).
- Documentação: BR-088, PRD §13.1/§9, architecture §3.7.

## Critérios de aceite

- [x] O rascunho não bloqueia o início da resposta completa.
- [x] Com o completo mais lento, o rascunho aparece antes (fase `quick`).
- [x] Rascunho que chega depois do primeiro delta do completo é descartado.
- [x] Falha no rascunho não derruba a completa.
- [x] A UI ignora uma `quick` atrasada da mesma geração.

## Testes

- `application::suggestion_service::tests::fast_first_emits_a_short_draft_before_the_full_suggestion`
- `application::suggestion_service::tests::a_late_quick_draft_is_discarded_when_the_full_already_spoke`
- `core/application/useSuggestions.test.tsx` — “ignora a rápida atrasada depois
  da completa da mesma geração”.
- `cargo test --lib` → **658** testes, 0 falhas (10 ignorados).
- `pnpm test` → **454** testes; `tsc --noEmit` limpo.

## Limite conhecido

Duas chamadas concorrentes ao mesmo provider podem esbarrar em rate limit. O
rascunho é descartável: se ele falhar, a completa continua. Se o provider
limitar agressivamente, a alternativa é desligar o `fast-first` no manual.
