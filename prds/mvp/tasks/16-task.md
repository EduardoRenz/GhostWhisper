# 16 — Sessão contínua (pausar/retomar/continuar) e linha temporal

- **Status:** CONCLUDED
- **Depende de:** 05, 13, 14
- **Habilita:** 17 (histórico/sugestões) e 18 (export Markdown)

## Objetivo
O usuário relatou: *"toda vez que eu pauso e continuo a gravação, a aplicação
está apagando as mensagens e reiniciando"*. Causa: `SessionService::start`
encerrava a sessão anterior e criava **um id novo**, e a UI
(`useLiveTranscription`) **zerava a lista** quando o `session_id` mudava — a
linha do tempo antiga ficava no banco, invisível. Faltava o **conceito de
sessão** no ciclo de captura.

Entregar: pausar/retomar **na mesma sessão**, continuar uma sessão existente,
nova sessão, e **tempo por mensagem** (linha temporal cumulativa).

## Escopo (decidido em 2026-10-18)
- Ações no overlay: **Pausar · Retomar · Nova sessão · Parar** (padrão Perssua).
- Histórico (lista/detalhe) e export Markdown **não** entram aqui (tasks 17/18).
- Sem busca de texto no overlay por ora (evita roubar foco do app da reunião).

## Referência (BR-015, REA do Perssua 0.27.5)
- `transcriptionPreview`: `pause: "Pausar"`, `resume: "Retomar"`,
  `paused: "Pausado — o áudio não está sendo capturado"` → **pausar mantém a
  sessão**; só a captura é suspensa.
- `DatabaseManager.resumeSession(id)`: volta `status='active'` e limpa `ended_at`;
  `endAllActiveSessions(userId)` encerra sessões órfãs no boot.
- `sessionDetailPage.resumeSession: "Continuar sessão"`.
- Sessões encerradas são **retomáveis**: as mensagens continuam na mesma sessão.

## Entregáveis
- `ports/session_controller.rs`: `pause()`, `resume()`, `continue_session(id, channels)`.
- `ports/repositories.rs`: `SessionRepository::{resume, end_all_active}`.
- `application/session_service.rs`: estado `paused` **de runtime** (o banco
  continua `active`, respeitando o `CHECK` do schema → **sem migração**);
  `started_at_ms` preservado ao retomar/continuar.
- `application/transcription_pipeline.rs`: `with_time_origin(ms)` — os
  timestamps passam a ser relativos ao **início da sessão**, não ao início da
  captura; id do segmento inclui a origem (corrige colisão de id ao continuar).
- `ports/event_publisher.rs`: `SessionStateEvent` ganha `paused`.
- `ipc/session_commands.rs`: `session_pause`, `session_resume`,
  `session_continue`.
- `lib.rs`: encerra sessões órfãs no boot (`end_all_active`).
- Frontend: `useSessionState` (pause/resume/continue), `SessionControls`
  (Pausar/Retomar/Nova sessão/Parar), `useLiveTranscription` deixa de zerar e
  **carrega os finais persistidos** da sessão, `TranscriptView` mostra `mm:ss`,
  `formatClock`/`mergeSegments` puros em `core/domain/transcript.ts`.

## Critérios de aceite
- [x] Pausar mantém a sessão aberta (mesmo id) e para a captura; o texto em tela
      permanece.
- [x] Retomar volta a capturar **na mesma sessão**, com o relógio continuando
      de onde estava (sem reiniciar em 0).
- [x] "Nova sessão" encerra a atual e cria outra; a lista em tela é substituída
      pelo histórico (vazio) da nova sessão.
- [x] Continuar uma sessão encerrada reativa o mesmo id e mantém
      `started_at` (linha temporal contínua).
- [x] Cada linha mostra o tempo (`mm:ss`) desde o início da sessão.
- [x] Segmentos de sessões diferentes nunca colidem (id único por
      sessão+origem+turno).
- [x] Sessões órfãs (app fechado durante a gravação) são encerradas no boot.
- [x] `cargo test`/`clippy`/`fmt` e `pnpm test`/`tsc` verdes.

## Verificação (2026-10-18)
- **175 testes Rust** (+13) · 7 `ignored` · `clippy`/`fmt` limpos.
  - `SessionService`: pausa mantém a sessão (`status = active`, `ended_at` nulo) e
    para a captura; pausa idempotente e sem sessão não publica estado; retomar
    reusa o id e os canais; retomar sem pausa é no-op; parar funciona pausado;
    "nova sessão" encerra a pausada; continuar reativa e preserva `started_at`;
    continuar encerra a ativa anterior; continuar id desconhecido falha; falha de
    áudio ao continuar devolve a sessão para `ended`.
  - Pipeline: `with_time_origin` desloca `start_ms`/`end_ms` (0 → 60.000) e os
    ids não colidem entre capturas; id do turno no formato novo.
  - Repositórios (SQLite + in-memory): `resume` limpa `ended_at` preservando
    `started_at`; `end_all_active` encerra só as ativas e relata a contagem.
- **40 testes frontend** (+17) · `tsc` ✓ · `pnpm build` ✓.
  - `formatClock` (`mm:ss`, `h:mm:ss`, negativo), `mergeSegment` (final substitui
    prévia, ordem, teto), `mergeHistory` (sem duplicar, final ganha da prévia,
    ordena e corta).
  - `useLiveTranscription`: carrega o histórico persistido da sessão retomada,
    não mistura sessões diferentes, esvazia sem sessão.
  - `SessionControls`: botões por estado (parado/gravando/pausado) e canais.
  - `TranscriptView`: mostra o tempo por linha.
- Fluxo real de decodificação/streaming inalterado (e2e de ASR continuam
  `ignored` por dependerem de modelo/rede; rodam com `HEAVY_E2E`).

## Pendências
- Histórico (lista/detalhe/busca) e sugestões persistidas: task 17.
- Export Markdown: task 18.
- O atalho/tray "iniciar/encerrar sessão" encerra a sessão pausada (não retoma);
  comportamento documentado, sem ação dedicada.
