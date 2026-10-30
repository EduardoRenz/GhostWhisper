# 24 — Histórico: busca por título, exclusão em lote e título automático

- **Status:** CONCLUDED
- **Depende de:** 17 (histórico), PRD Sessão 03/15 (resumo/perguntar)
- **Habilita:** —

## Objetivo

Deixar a janela **Histórico** utilizável com muitas sessões: achar pelo título,
limpar várias de uma vez e garantir que toda sessão encerrada saia com título
mesmo sem o usuário abrir a aba Resumo.

## Contexto

- A lista carrega as 50 sessões mais recentes (`HISTORY_LIMIT`) e não tinha
  busca: achar a reunião de semanas atrás exigia rolar e abrir uma por uma.
- Excluir era **uma por vez**, só no detalhe.
- O título (`sessions.subject`) só era preenchido pelo `SummaryService` —
  quando a aba Resumo era aberta (ou quando o Perguntar disparava o refresh).
  Sessões sem resumo ficavam "Sessão sem título" para sempre.

## Entregáveis

### Busca por título (BR-103)

- `domain/search_text.rs`: `fold_text` (NFD sem diacríticos + minúsculas),
  `query_tokens` e `like_pattern` (escapa `%`, `_` e `\`).
- `SessionRepository::search_by_subject(query, limit)`:
  - SQLite registra `ghw_fold(text)` na conexão (`rusqlite` feature
    `functions`) e usa um `LIKE … ESCAPE '\'` por palavra (AND), ordenado por
    `started_at DESC`;
  - a dobra em memória usa a **mesma** função do domínio.
- IPC `db_search_sessions(query, limit)` + `searchSessions()` no adaptador.
- `useSessionHistory(activeSessionId, query)`: debounce de 250 ms, descarte de
  respostas fora de ordem e `listSessions` de volta quando a busca esvazia.
- UI: input com placeholder l10n, contador de resultados, aviso próprio para
  "sem resultado".

### Exclusão em lote (BR-103)

- Modo **Selecionar** na toolbar; checkbox por linha (com `aria-label` do
  título), "Selecionar todas" apenas das linhas visíveis e contador.
- "Excluir (n)" com **confirmação em dois cliques**, botões desabilitados e
  "Excluindo…" durante a operação.
- `useSessionHistory.removeMany(ids)`: exclusões **sequenciais** (cada uma pode
  parar a sessão ativa e descartar a gravação), um `refresh()` no fim e aviso
  de sucesso/parcial.
- Mudar a pesquisa limpa a seleção (nada é excluído "fora da tela").

### Resumo/título no encerramento (BR-104)

- `application/summary_refresh.rs`: o single-flight/cooldown que vivia dentro do
  `AskService` virou `SummaryRefresh`, com `spawn` (Perguntar) e
  `spawn_if_missing` (encerramento), além do evento `session_updated`.
- Porta `SessionEndObserver` + `SessionService::with_session_end`: o `stop`
  avisa o observador **depois** de encerrar a sessão, sem esperar nada.
- `SummaryService`: o título só é aplicado quando a sessão ainda não tem
  `subject` — a sessão é **relida** antes do `rename` (o usuário pode ter
  renomeado durante a chamada). Sem trechos novos e com resumo gravado, o
  `subject` é aplicado sem chamar o LLM (backfill).
- `lib.rs`: uma instância de `SummaryRefresh` para o app inteiro (Perguntar e
  encerramento no mesmo single-flight).
- `useSessionHistory` assina `session_updated` e recarrega em silêncio.

## Critérios de aceite

- [x] Buscar "reuniao" encontra "Reunião" (caixa e acento ignorados).
- [x] Consulta com duas palavras exige as duas no título.
- [x] `%` e `_` digitados são texto literal.
- [x] Busca vazia volta à lista normal; sem resultado há aviso dedicado.
- [x] Selecionar várias linhas e excluir remove todas, com confirmação.
- [x] Mudar a pesquisa limpa a seleção.
- [x] Sessão encerrada sem resumo ganha resumo e título em background.
- [x] Sessão com resumo e sem título ganha o título sem nova chamada ao LLM.
- [x] Título escolhido pelo usuário nunca é sobrescrito.
- [x] O `stop` não espera o LLM; falha do resumo fica em log.
- [x] A janela de Histórico acha/atualiza o novo título sozinha.

## Verificação

- Rust: `cargo test` **877 ✓** (11 ignorados) · `clippy --all-targets -D
  warnings` ✓ · `cargo fmt --check` ✓. Testes novos: fold/`LIKE` escapado,
  busca SQLite (acento, wildcard literal, ordem, limite) e dobra em memória;
  `SummaryRefresh` (gera quando falta resumo/título, pula sem fala, single-flight,
  cooldown, evento com o título), `SessionService` (stop notifica, pausa não),
  `SummaryService` (backfill sem LLM, renomeação concorrente preservada).
- Frontend: `pnpm test` **542 ✓** · `tsc --noEmit` ✓. Testes novos em
  `HistoryPanel.test.tsx`: busca com debounce, aviso de sem resultado, exclusão
  em lote com confirmação, limpeza da seleção e atualização pelo evento.

## Notas

- A busca varre `sessions` com uma função escalar (sem índice): com o volume
  típico de sessões é irrelevante; se crescer, indexar a coluna normalizada.
- Longas sessões continuam processando até 3 chunks por chamada: o título sai
  do primeiro; o resumo pode ficar parcial (`has_more`), como antes.
- Sem chave/fonte de IA, o encerramento não gera título (falha só em log).
- Excluir a sessão **ativa** dispara o `stop` e a geração agendada; se a exclusão
  acontecer antes da chamada, o `rename`/`set_overview` não encontram a linha e a
  falha fica contida (warn).
