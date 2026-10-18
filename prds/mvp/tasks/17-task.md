# 17 — Histórico de sessões e sugestões da IA na linha temporal

- **Status:** CONCLUDED
- **Depende de:** 05, 16
- **Habilita:** 18 (export Markdown)

## Objetivo
Dar acesso ao que já estava gravado — **75 sessões / 120 transcrições** no banco do
autor, sem nenhum caminho na UI (US-08 "rever sessões anteriores" e CU-07 nunca
entregues) — e parar de perder as **sugestões da IA**, que não eram persistidas em
lugar nenhum (a tabela `messages` existia desde a task 05 e estava morta).

## Escopo (decidido com o usuário em 2026-10-18)
- Histórico **dentro do overlay expandido** (aba `Ao vivo | Histórico`), sem janela
  nova; superfície `expanded-tall` (900×560) quando a aba está aberta.
- **Sem busca livre** por ora: um `<input>` no overlay exige roubar o foco do app
  da reunião (IPC de foco dedicado). A busca FTS5 fica para depois.
- Sugestões aparecem com o tempo e ficam gravadas na sessão (o export da task 18
  depende disso).

## Referência (BR-015, REA do Perssua 0.27.5)
- `allSessionsPage`: "Todas as Sessões", busca por título/resumo/conteúdo,
  paginação, `untitled: "Sessão sem título"`.
- `sessionDetailPage`: painel com abas Detalhes/Chat/Transcrição; ações
  `resumeSession: "Continuar sessão"`, copiar resumo/chat/transcrição, excluir,
  renomear título.
- `DatabaseManager.resumeSession` / `getSessionsPage(excludeActive)`.

## Entregáveis
- `domain/message.rs` (`MessageRole`, `ConversationMessage`) — usa a tabela
  `messages` do schema, com os triggers FTS5 **já existentes** (sem migração).
- `ports/repositories.rs` → `MessagesRepository`; adaptadores
  `LocalMessagesRepository` (SQLite) e `InMemoryMessagesRepository` (dobra).
- `application/session_timeline.rs` — **puro**: funde falas finais + mensagens
  `assistant` numa linha temporal ordenada (empate determinístico: fala primeiro,
  depois texto).
- `application/insight_service.rs` — injeta `MessagesRepository` + `Clock`,
  **persiste** a sugestão com o tempo desde o início da sessão e devolve
  `InsightSuggestion { message_id, session_id, text, at_ms }`.
- `ports/insight_generator.rs` — nova assinatura; `EventPublisher::emit_insight`
  passa a emitir a sugestão (não só o texto), para o atalho global.
- `SessionRepository::{rename, delete}` (SQLite + in-memory).
- IPC: `db_session_messages`, `db_session_timeline`, `session_rename`,
  `session_delete`. `AppState` ganha `messages` e `clock`.
- Frontend: `useSessionHistory`/`useSessionTimeline`, `useInsight` (lista com
  tempo, persistida + eventos), `features/history/HistoryPanel.tsx` (lista +
  detalhe), `InsightsPanel` com `[mm:ss]` por sugestão, abas no overlay.

## Correções de higiene incluídas
- **`PRAGMA foreign_keys = ON`** na inicialização do SQLite: o schema declara
  `ON DELETE CASCADE` desde a task 05, mas sem o pragma a FK era aceita e
  **ignorada** — excluir uma sessão deixaria transcrições/mensagens órfãs.
- `session_delete` encerra a captura antes de apagar (se for a sessão em
  andamento) e `delete` remove os filhos explicitamente (não depende do pragma).

## Critérios de aceite
- [x] Histórico lista as sessões (título, data, duração, contagem) com a sessão em
      andamento marcada.
- [x] Detalhe mostra a **linha temporal** com falas e sugestões, cada item com
      `mm:ss`.
- [x] "Continuar sessão" reabre a sessão escolhida (mesmo id, BR-019).
- [x] Renomear (título vazio é recusado) e excluir (com falas + sugestões).
- [x] Sugestão pedida no painel **e** por atalho é persistida e aparece na lista,
      sem duplicar.
- [x] Nenhuma migração de schema; nada de rede.
- [x] `cargo test`/`clippy`/`fmt` e `pnpm test`/`tsc`/`build` verdes.

## Verificação (2026-10-18)
- **191 testes Rust** (+16): `session_timeline` (intercalação por tempo, ignora
  prévias e papéis não-`assistant`, empate determinístico, rótulos);
  `InsightService` (persiste com o relógio da sessão, sessão desconhecida falha,
  nada é gravado quando a geração falha); `LocalMessagesRepository`
  (ordem temporal, isola sessões); `SessionRepository` (rename, delete em
  cascata + FTS limpo, `resume`, `end_all_active`).
- **58 testes frontend** (+18): `HistoryPanel` (lista, detalhe com tempos,
  continuar, renomear, exportar pelo caminho do diálogo, cancelar, copiar,
  excluir, vazio); `InsightsPanel` (gravadas com tempo, pedir nova, copiar,
  sugestão do atalho sem duplicar); domínio (`sessionTitle`, `formatDateTime`,
  `sessionDuration`, `timelineLabel`, `toSuggestion`).
- `tsc` ✓ · `pnpm build` ✓ · `clippy`/`fmt` limpos.

## Pendências
- Busca livre no histórico (FTS5 já existe no core: `db_search_transcriptions`)
  exige um IPC de foco para o `<input>` não roubar o foco da reunião.
- Paginação do histórico (hoje: 50 mais recentes).
- Renomear/apagar direto na lista (hoje só no detalhe, exceto "Continuar").
