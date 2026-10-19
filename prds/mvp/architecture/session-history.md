# Arquitetura — Sessão, linha temporal e histórico

Detalha o **conceito de sessão** do GhostWhisper: o ciclo de captura
(pausar/retomar/continuar), como o tempo de cada mensagem é calculado e o que
falta para histórico/export. Decisão de produto em BR-019; referência no Perssua
em BR-015.

---

## 1. Modelo

- **Sessão** = unidade de continuidade. Um `id` (UUID) acumula transcrições e,
  adiante, sugestões. Duas tabelas persistem a linha temporal:
  - `transcriptions` — falas (`speaker`, `text`, `timestamp` = ms desde o início
    da sessão, `end_ms`, `is_final`);
  - `messages` — sugestões/insumos da IA (`role`, `content`, `timestamp`);
  - `sessions` — metadados (`started_at`, `ended_at`, `subject`, `overview`).
- O **banco não tem estado de pausa**: o `CHECK(status IN ('active','ended'))` do
  schema é respeitado, e a pausa vive só em runtime (BR-019 → **sem migração**).

## 2. Ciclo de vida

| Ação | `sessions` | Captura | Id | Lista em tela |
|---|---|---|---|---|
| Iniciar | cria (active) | liga | novo | vazia (carrega histórico do id) |
| Pausar | segue `active` | desliga | o mesmo | **preservada** |
| Retomar | segue `active` | religa | o mesmo | preservada |
| Nova sessão | encerra a anterior, cria outra | religa | novo | do id novo |
| Parar | encerra (`ended`) | desliga | — | limpa |
| Continuar (histórico) | reativa (`active`, `ended_at = NULL`) | liga | o mesmo | histórico do id |

`sessions.started_at_ms` **nunca** é reescrito ao retomar/continuar — é a âncora
da linha temporal.

**Sessões órfãs:** fechar o app gravando deixa a sessão `active` no banco. No
boot o composition root chama `SessionRepository::end_all_active` (transposto de
`DatabaseManager.endAllActiveSessions`) para o histórico não mentir.

## 3. Tempo (a linha temporal)

- O pipeline acumula o áudio do turno e publica com `start_ms`/`end_ms`.
- `TranscriptionPipeline::with_time_origin(origin_ms)` desloca todo timestamp —
  `origin_ms = agora_ms - session.started_at_ms` no início de cada captura.
  Sessão nova usa `0`; retomar/continuar continua o relógio (a pausa vira um
  **buraco real** na linha do tempo, que é o comportamento desejado: "o tempo em
  que a mensagem foi gerada").
- O relógio de parede é recuperável: `session.started_at_ms + start_ms`.
- **UI:** `formatClock(start_ms)` → `mm:ss` (ou `h:mm:ss`).

### Ids dos segmentos

`{session_id}-{origin_ms}-t{turn}`. O `turn` reinicia em 0 a cada captura, então
a origem entra no id para que continuar uma sessão **não colida** com os
segmentos já gravados (chave primária) nem substitua a linha errada na UI.
Prévia e final do mesmo turno compartilham o id (a UI troca o texto, não duplica).

## 4. Frontend

- `useLiveTranscription(sessionId)`:
  1. ao entrar/trocar de sessão, descarta o que é de outra sessão e **recarrega**
     `db_session_transcriptions` (BR-019);
  2. mescla o histórico com os eventos ao vivo por id (`mergeHistory`), com o
     **final ganhando da prévia**;
  3. ignora eventos de outra sessão (sem contaminar a lista nova).
- `mergeSegment`/`mergeHistory`/`formatClock` são **puras** (`core/domain/transcript.ts`)
  e testadas sem DOM.
- Superfície: `Pausar · Retomar · Nova sessão · Parar` na ActionBar, com pílula de
  status (`○ parado`, `● gravando`, `❚❚ pausado`) e medidores zerados na pausa.

## 5. Sessão × Pipeline

A pausa **encerra os pipelines** (a sessão permanece). Retomar reconstrói os
pipelines (novo contexto do ASR) com a mesma sessão e a origem de tempo correta.
Custo aceito: o modelo não fica residente durante uma pausa longa; em troca, o
estado da aplicação fica simples (nada de pipeline "meio vivo").

## 6. Histórico, sugestões e export (tasks 17/18)

- **Sugestões** são mensagens da sessão (`messages`, papel `assistant`) com o
  tempo desde o início da sessão; entram na linha temporal ao vivo, no histórico e
  no export (BR-020).
- **Linha temporal**: `application/session_timeline.rs` funde falas finais e
  sugestões numa lista ordenada (puro e determinístico — o export depende disso).
- **UI**: a lista e o detalhe vivem na janela própria `history` (task 08 do PRD
  chat), com a conversa em bolhas em ambas; antes eram a aba `Ao vivo | Histórico`
  do painel expandido (`expanded-tall`). Sem busca livre ainda (exigiria um IPC
  de foco para o `<input>` não roubar o foco da reunião).
- **Export**: `application/markdown_export.rs` (puro) +
  `ipc/export_commands.rs`; o front escolhe o destino com o diálogo nativo
  (`tauri-plugin-dialog`) e o core escreve.
- **Exclusão**: `PRAGMA foreign_keys = ON` na conexão (o schema declara
  `ON DELETE CASCADE` desde a task 05 e a FK era aceita e ignorada sem ele) e os
  filhos são removidos explicitamente.
