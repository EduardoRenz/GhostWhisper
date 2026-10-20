# 03 — `AskService`: perguntar com contexto da sessão

- **Status:** PENDING
- **Depende de:** 01
- **Habilita:** 04
- **Issues:** — · **BR:** BR-042, BR-045, BR-048, BR-053
- **Refs:** [`../prd.md`](../prd.md) US-02;
  [`../architecture/session-journal.md`](../architecture/session-journal.md) §3, §5.1, §7

## Objetivo

Implementar o caso de uso da **pergunta**: resolver a sessão, montar o
contexto, chamar a IA com **streaming**, persistir pergunta e resposta, e
publicar o progresso.

## Entregáveis

- `src-tauri/src/domain/ai_route.rs`: `AiCapability::Search` (escopo novo de
  rota — BR-048).
- `src-tauri/src/application/ask_service.rs`:
  - `AskService::ask(session_id, question, images, source) -> Result<String>`
    (id da entrada);
  - resolve a sessão (`application/session_lookup.rs::resolve_session` — sem
    id, a **ativa** ou a mais recente);
  - monta o contexto com `application/context.rs`
    (`recent_context_live` — falas finalizadas **e** prévias, como a
    sugestão BR-024/BR-028);
  - persiste a pergunta **antes** da chamada (RNF-06);
  - chama `LlmController::complete_streaming_for(Capability(Search), …)` e
    publica `emit_journal` a cada delta;
  - persiste a resposta ao final e marca `answered`;
  - em erro: marca a entrada `error` com a mensagem e **repropaga**.
- `src-tauri/src/ports/event_publisher.rs`: `emit_journal(&JournalUpdate)`
  com `JournalPhase { Pending, Streaming, Answered, Error }`.
- `src-tauri/src/ports/llm_controller.rs`: nada muda (o escopo novo é só um
  `AiRouteScope`).
- `src-tauri/src/ipc/journal_commands.rs`: `journal_ask { question, images,
  source }`.
- `src-tauri/src/lib.rs`: registrar o `AskService` no composition root.

## Critérios de aceite

- [ ] A pergunta é respondida com o **contexto** da sessão (as falas até
      aquele instante).
- [ ] A entrada aparece no diário **antes** da resposta (status `pending`).
- [ ] O texto chega **em streaming** pelo evento `journal`.
- [ ] **Single-flight**: um segundo envio durante uma resposta em curso não
      duplica texto (BR-042).
- [ ] Falha **antes** de emitir texto → tenta a próxima fonte da cadeia
      (BR-045); falha **depois** → o erro sobe para a entrada, sem trocar de
      fonte (evita duplicar).
- [ ] A rota `Capability(Search)` é respeitada: com rota, usa a fonte
      vinculada; sem rota, a primária (BR-048).
- [ ] `cargo test --lib` verde: casos de sucesso, de erro, de fallback e de
      single-flight (com `FakeChatBotProvider`).

## Notas

- O **contexto** é o mesmo das sugestões (janela ≈3 min, até 40 falas, teto
  de 6 k caracteres — BR-024) — reusar `context.rs` evita duplicar a
  heurística (R14).
- A entrada registra `provider` e `model` que responderam (o Perssua grava
  `analysisRoute` no `metadata` da mensagem) — aqui são **colunas**, mais
  fácil de filtrar.
- Sem sessão nenhuma, a pergunta é **recusada** com erro claro (não há
  para qual `session_id` apontar).
