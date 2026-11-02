# 02 — Porta `ScreenContextSource` + fakes + evento

- **Status:** CONCLUDED (2026-11-01)
- **Depende de:** 01
- **Habilita:** 03, 04, 06
- **Issues:** — · **BR:** BR-105, BR-106

## Objetivo

Definir o contrato de leitura do contexto (porta) e as dobras de teste,
mais o DTO/IPC do evento `screen_context`. Sem adaptador real ainda.

## Entregáveis

- `src-tauri/src/ports/screen_context_source.rs` (novo):
  - trait `ScreenContextSource { fn snapshot(&self) -> Result<Option<ScreenContextSnapshot>>; fn name(&self) -> &'static str; }`
  - `FakeScreenContextSource` (devolve snapshot fixo/`None`, conta chamadas) e
    `FailingScreenContextSource` (erro de mecanismo).
- `src-tauri/src/ports/mod.rs`: registrar.
- DTO de evento `ScreenContextDto { available, app, window_title, preview,
  captured_at_ms }` (core → front; preview ≤ 300 chars, cortado no core).
- `src/core/domain/screenContext.ts` (tipos TS espelhando o DTO).

## Critérios de aceite

- [ ] Porta pequena (2 métodos), sem `atspi`/`zbus` no módulo.
- [ ] Fakes cobrem: snapshot OK, `None`, erro.
- [ ] DTO nunca carrega `text` completo (só `preview`).
- [ ] Teste unitário do fakes: `fake_records_calls` e
      `failing_source_returns_error`.

## Testes

- `fake_returns_configured_snapshot`
- `failing_source_surfaces_error`
- `dto_preview_is_capped` (corte no serviço, task 06 — aqui só o tipo)
