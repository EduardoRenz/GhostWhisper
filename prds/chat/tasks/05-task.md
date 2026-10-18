# 05 — Documentação, verificação e pacotes

- **Status:** CONCLUDED
- **Depende de:** 04
- **Habilita:** —

## Objetivo

Fechar a PRD: regras de negócio, ponteiros na documentação existente, verificação
completa e pacotes.

## Entregáveis

- `prds/business-rules.md`: **BR-021** (conversa em bolhas intercaladas por tempo;
  sugestões na mesma linha temporal) e **BR-022** (transporte por ícones e desvios),
  referenciadas por ID (R14).
- `README.md`: seção de uso com as bolhas, o transporte e a confirmação de
  encerramento.
- `prds/mvp/architecture/overlay.md`: ponteiro para a nova tech spec (sem duplicar).
- `prds/chat/memory.md`: entradas do dia com `IDs:`/`Refs:`.
- Verificação: `cargo test` + `clippy` + `fmt`; `pnpm test` + `tsc` + `pnpm build`;
  binário release subindo; `.deb` + AppImage.

## Critérios de aceite

- [ ] Nenhuma referência duplicada (R14): docs apontam para a tech spec.
- [ ] Todos os testes verdes e pacotes reconstruídos.
- [ ] `_tasks.md` com todas as tarefas `CONCLUDED`.

## Resultado (2026-10-18)

- **BR-021** e **BR-022** registradas em `prds/business-rules.md` (com os desvios
  conscientes do Perssua); README e `prds/mvp/architecture/overlay.md` apontam para
  [`../architecture/chat-ui.md`](../architecture/chat-ui.md) em vez de duplicar
  (R14).
- Verificação: **217 testes Rust** (8 `ignored`) + clippy/fmt limpos; **74 testes
  frontend** + `tsc --noEmit` limpo + `pnpm build`; binário release subindo; `.deb`
  e AppImage reconstruídos.
