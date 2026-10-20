# Arquitetura — Sessão (diário de perguntas) e feature Busca

Spec técnico da PRD [`../prd.md`](../prd.md). Decisões de produto em
[`../../business-rules.md`](../../business-rules.md); evidência do Perssua em
[`../prd.md`](../prd.md) §9.

---

## 1. Visão geral

```
        ┌──────────────────────── janela `chat` (painel da sessão) ─────────────────────────┐
        │  pill de abas:  Transcrição | Resumo | Sessão                                        │
        │                                                                                       │
        │  ┌── aba Sessão ──────────────────────────────────────────────────────────────────┐  │
        │  │  [ 🔍 Perguntar ]  ← botão no notch abre/foca aqui                            │  │
        │  │  ┌────────────────────────────────────────────────────────────┐                 │  │
        │  │  │ campo de pergunta          📷 print   ⌘V colar    [Perguntar] │                 │  │
        │  │  └────────────────────────────────────────────────────────────┘                 │  │
        │  │  [miniatura da imagem anexada ×]                                               │  │
        │  │  chips: (Resumir tudo até agora) (Listar as ações) …  [+] [editar]             │  │
        │  │  ─────────────────────────────────────────────────────────────                  │  │
        │  │  busca: [ __________ ]                                                        │  │
        │  │  ─────────────────────────────────────────────────────────────                  │  │
        │  │  12:04  Qual o nome do cliente?                            [Pesquisa]           │  │
        │  │        ▸ resposta em Markdown (streaming)                                     │  │
        │  └───────────────────────────────────────────────────────────────────────────────┘  │
        └───────────────────────────────────────────────────────────────────────────────────────┘
```

O **diário** é a lista de entradas da aba. A **Busca** é o composer que as
produz. Um botão no notch (lupa) leva até lá.

## 2. Modelo de dados

### 2.1 Tabelas novas

```sql
-- Âncora estável do FTS: `id` é TEXT (uuid) e não serve de rowid.
CREATE TABLE IF NOT EXISTS journal_entries (
  rowid_pk     INTEGER PRIMARY KEY AUTOINCREMENT,
  id           TEXT NOT NULL UNIQUE,
  session_id   TEXT NOT NULL,
  question     TEXT NOT NULL,
  answer       TEXT,
  asked_at_ms  INTEGER NOT NULL,          -- relativo ao início da sessão (BR-019)
  answered_at_ms INTEGER,
  status       TEXT NOT NULL DEFAULT 'pending'
                 CHECK(status IN ('pending','answered','error','cancelled')),
  request_source TEXT NOT NULL DEFAULT 'search'
                 CHECK(request_source IN ('search','preset','screenshot')),
  provider     TEXT,
  model        TEXT,
  error        TEXT,
  created_at   INTEGER DEFAULT (strftime('%s','now')),
  FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS journal_images (
  id        TEXT PRIMARY KEY,
  entry_id  TEXT NOT NULL,
  mime      TEXT NOT NULL,
  data      BLOB NOT NULL,
  width     INTEGER,
  height    INTEGER,
  FOREIGN KEY (entry_id) REFERENCES journal_entries(id) ON DELETE CASCADE
);

-- FTS5 ancorado por `id` (o `session_id` fica UNINDEXED: o filtro por sessão
-- acontece na tabela base, que tem índice).
CREATE VIRTUAL TABLE IF NOT EXISTS journal_fts USING fts5(
  entry_id UNINDEXED,
  session_id UNINDEXED,
  question,
  answer
);

CREATE TRIGGER IF NOT EXISTS journal_ai AFTER INSERT ON journal_entries BEGIN
  INSERT INTO journal_fts(rowid, entry_id, session_id, question, answer)
  VALUES (new.rowid_pk, new.id, new.session_id, new.question, new.answer);
END;

CREATE TRIGGER IF NOT EXISTS journal_ad AFTER DELETE ON journal_entries BEGIN
  DELETE FROM journal_fts WHERE rowid = old.rowid_pk;
END;

CREATE TRIGGER IF NOT EXISTS journal_au AFTER UPDATE OF question, answer ON journal_entries BEGIN
  UPDATE journal_fts SET question = new.question, answer = new.answer
  WHERE rowid = new.rowid_pk;
END;
```

**Por que `rowid_pk`**: o FTS é uma tabela FTS5 **comum** (guarda o próprio
conteúdo) e o trigger grava `new.rowid_pk` — o mesmo rowid da tabela base. Sem
uma chave `INTEGER PRIMARY KEY` explícita, o `rowid` é implícito e o
`VACUUM` pode renumerá-lo, quebrando o casamento
`t.rowid IN (SELECT rowid FROM ..._fts WHERE ..._fts MATCH ?)` **em
silêncio**. Ver ISS-042.

### 2.2 Índices

| Índice | Tabela | Colunas | Serve |
|---|---|---|---|
| `idx_journal_session_time` | `journal_entries` | `(session_id, asked_at_ms)` | Listar o diário da sessão **já ordenado** (sem sort) |
| `idx_journal_images_entry` | `journal_images` | `(entry_id)` | Cascade do `DELETE` da sessão + busca das imagens de uma entrada |

Planos medidos (`EXPLAIN QUERY PLAN`):

```
-- listar o diário da sessão
SEARCH journal_entries USING INDEX idx_journal_session_time (session_id=?)
-- (sem "USE TEMP B-TREE FOR ORDER BY")

-- buscar no diário (FTS + sessão)
SEARCH j USING INDEX idx_journal_session_time (session_id=?)
LIST SUBQUERY 1
  SCAN journal_fts VIRTUAL TABLE INDEX 0:M4
-- (sem temp b-tree)

-- imagens de uma entrada
SEARCH journal_images USING INDEX idx_journal_images_entry (entry_id=?)
```

### 2.3 Dívida do schema existente (task 10)

O schema atual tem **índices simples** que forçam *temp b-tree* em toda
listagem ordenada e **índices mortos**. Verificação com `EXPLAIN QUERY PLAN`:

| Situação atual | Plano | Com a correção |
|---|---|---|
| `transcriptions WHERE session_id=? ORDER BY timestamp` | `SEARCH … (session_id=?)` + **temp b-tree** | `SEARCH … USING INDEX idx_transcriptions_session_time (session_id=?)` |
| `messages WHERE session_id=? ORDER BY timestamp` | idem | idem |
| `sessions ORDER BY started_at DESC LIMIT n` | **SCAN sessions** + temp b-tree | `SCAN sessions USING INDEX idx_sessions_started_at` |
| `UPDATE sessions … WHERE status='active'` | `SEARCH sessions USING INDEX idx_sessions_status (status=?)` | **manter como está** |

Correção (aplicada idempotentemente a cada boot — `schema.sql` roda em
`conn.execute_batch(SCHEMA)`, sem framework de migração):

```sql
CREATE INDEX IF NOT EXISTS idx_transcriptions_session_time ON transcriptions(session_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_messages_session_time      ON messages(session_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_sessions_started_at         ON sessions(started_at DESC);

DROP INDEX IF EXISTS idx_transcriptions_session;   -- coberto pelo composto
DROP INDEX IF EXISTS idx_transcriptions_timestamp; -- morto
DROP INDEX IF EXISTS idx_messages_session;         -- coberto pelo composto
DROP INDEX IF EXISTS idx_messages_timestamp;       -- morto
DROP INDEX IF EXISTS idx_sessions_user;             -- morto: user_id é sempre NULL (BR-002)
-- idx_sessions_status: MANTER (é usado por end_all_active)
```

## 3. Camadas

```
domain/journal.rs            JournalEntry, JournalImage, JournalStatus, JournalRequestSource
domain/llm.rs                CompletionRequest.images: Vec<CompletionImage>
domain/ai_route.rs           AiCapability::Search

ports/journal_repository.rs  JournalRepository        → LocalJournalRepository / InMemoryJournalRepository
ports/screen_capture.rs      ScreenCapture            → X11ScreenCapture / FakeScreenCapture

application/ask_service.rs   AskService (pergunta → resposta, com streaming)
application/context.rs       (reusado) contexto das falas para a pergunta

adapters/persistence/sqlite/local_journal_repository.rs
adapters/screen/x11_screen_capture.rs
adapters/ai/openai_compatible.rs   (multimodal)
adapters/ai/gemini_chat_bot_provider.rs (multimodal)

ipc/journal_commands.rs      journal_ask / list / get / search / delete / image
ipc/screen_commands.rs       screen_capture
```

`domain` e `application` não conhecem adaptadores (BR-011, R1). O
composition root (`src-tauri/src/lib.rs`) é o **único** lugar que instancia
adaptadores (R2).

## 4. Portas

### 4.1 `JournalRepository`

```rust
pub trait JournalRepository: Send + Sync {
    /// Registra a pergunta (persistida **antes** de chamar a IA).
    fn insert_question(&self, entry: &JournalEntry) -> Result<()>;
    /// Grava a resposta e marca a entrada como respondida.
    fn complete_answer(&self, id: &str, answer: &str, answered_at_ms: i64) -> Result<()>;
    /// Marca a entrada como erro/cancelada.
    fn fail(&self, id: &str, error: &str, status: JournalStatus) -> Result<()>;
    fn get(&self, id: &str) -> Result<Option<JournalEntry>>;
    /// Diário da sessão, em ordem cronológica.
    fn list_by_session(&self, session_id: &str) -> Result<Vec<JournalEntry>>;
    /// Busca full-text (pergunta e resposta) dentro da sessão.
    fn search(&self, session_id: &str, query: &str, limit: usize) -> Result<Vec<JournalEntry>>;
    /// Anexa uma imagem a uma entrada.
    fn attach_image(&self, image: &JournalImage) -> Result<()>;
    /// Imagens de uma entrada.
    fn images_of(&self, entry_id: &str) -> Result<Vec<JournalImage>>;
}
```

Dobras: `InMemoryJournalRepository` (BR-013, R8).

### 4.2 `ScreenCapture`

```rust
pub trait ScreenCapture: Send + Sync {
    /// Captura a tela e devolve a imagem (bytes + mime + dimensões).
    fn capture(&self) -> Result<CapturedImage>;
}
```

Dobras: `FakeScreenCapture`.

## 5. Fluxos

### 5.1 Perguntar (com e sem imagem)

```
UI (aba Sessão)
  └─ journal_ask { question, images?, source }
       └─ ipc/journal_commands.rs
            └─ AskService::ask(question, images, source)
                 1. resolve_session(sessions, None)      ← sessão ativa ou mais recente
                 2. journal.insert_question(entry)        ← persistida ANTES da IA (RNF-06)
                 3. contexto = context::recent_context_live(...)   ← falas finalizadas + prévias
                 4. llm.complete_streaming_for(Capability(Search), request, sink)
                      └─ sink → publisher.emit_journal(JournalUpdate { entry_id, phase, text })
                 5. journal.complete_answer(entry_id, text, at)
```

**Single-flight** (BR-042): uma pergunta por vez. O `AskService` guarda a
entrada em curso; um novo envio durante uma resposta **não** duplica texto.
Se a fonte falhar **antes** de emitir texto, a cadeia tenta a próxima
(BR-045); se falhar **depois**, o erro sobe para a entrada (não troca de
fonte, para não duplicar).

### 5.2 Captura de tela (X11)

```
UI → screen_capture
     └─ X11ScreenCapture::capture()
          1. esconder as janelas do app (overlays)   ← paridade com o Perssua
          2. x11rb: GetImage na root window (ZPixmap)
          3. reescalar para ≤1280 px no lado maior
          4. recomprimir em JPEG q0.86
          5. restaurar as janelas
```

Fallback: **colar imagem** (`Ctrl+V`) no campo da pergunta — funciona também
em Wayland. Se não houver display X11, o erro é explícito.

### 5.3 Perguntas prontas

Vivem nas **settings** (`settings.saved_questions`) — são configuração do
usuário, não dado de sessão (BR-054). O frontend edita e salva via o IPC de
settings que já existe (`saveSettings`); **nenhuma porta nova**.

## 6. Multimodal

`CompletionRequest` ganha `images: Vec<CompletionImage>`:

```rust
pub struct CompletionImage {
    pub mime: String,     // "image/jpeg"
    pub data: Vec<u8>,
}
```

- **OpenAI-compatível** (`openai_compatible.rs`): `content` vira lista —
  `[{"type":"text","text":…},{"type":"image_url","image_url":{"url":"data:image/jpeg;base64,…"}}]`.
- **Gemini** (`gemini_chat_bot_provider.rs`): `parts` ganha
  `{"inline_data":{"mime_type":…,"data":<base64>}}`.

Provider que **não** aceita `image_url` responde erro → o `AskService`
traduz para mensagem clara ("esse modelo não aceita imagem") e a entrada
fica com status de erro (R-04).

## 7. Eventos

`EventPublisher` ganha:

```rust
fn emit_journal(&self, update: &JournalUpdate);

pub struct JournalUpdate {
    pub entry_id: String,
    pub phase: JournalPhase,   // Pending | Streaming | Answered | Error
    pub text: String,          // texto acumulado (streaming) ou final
    pub error: Option<String>,
}
```

Espelha `emit_suggestion` (BR-028) — o frontend substitui o texto da entrada
a cada delta.

## 8. A barra "Perguntar" (janela `ask`)

O **notch** continua sendo a superfície principal (arrastável, ancorado pela
base) e ganha, ao lado da marca, um botão de **lupa** (cor primária + badge
`Ctrl K`). A lupa — **disponível com ou sem transcrição** — abre a barra numa
**janela própria** (`ask`, 1 040 px, no topo, centralizada):

```
┌──────────────────────────────────────────────────────────────────────────┐
│ 🔍  Perguntar ao GhostWhisper sobre a tela ou áudio…  [Ctrl Space] │ ⛶ Capturar Tela Alt C │  🎤  👁  🕘  ⚙ │
└──────────────────────────────────────────────────────────────────────────┘
```

- **Perguntar** (Enter) → `journalAsk` (com o print, se houver) → **a barra
  some** e o painel abre na aba **Sessão**.
- **Capturar Tela** (ou `Alt+C`) → **só anexa**. O print fica pendente
  (`PendingCapture`) e a **prévia** aparece no **canto superior direito**
  (janela `capture`), com o botão de remover. Nada é enviado ainda.
- **Ctrl+K** (`focus_ask`) → abre a barra e foca o campo.
- `Esc` fecha a barra sem perguntar.
- O notch mantém o switcher (Transcrição | Resumo | Sugestões) + timer quando
  há gravação; a lupa fica disponível nos dois estados.

### 8.1 Captura de tela: anexo + prévia + espera do compositor

`window.hide()` apenas **enfileira** o pedido; o X11/compositor leva alguns
quadros para tirar a janela do que está desenhado. Por isso o comando
`screen_capture` faz:

1. esconde todas as janelas do app;
2. espera `HIDE_SETTLE_MS` (200 ms) **e** confirma o `unmap` no Tauri (até
   600 ms);
3. só então `GetImage`; depois restaura as janelas.

Sem essa espera o print sai com o próprio HUD — regressão observada no teste de
2026-10-19.

O print capturado **não é enviado**: ele entra na porta `PendingCapture`
(em memória) e a janela `capture` mostra a prévia no canto superior direito.
O `×` (ou `Esc` na prévia) descarta. Só o **Perguntar** da barra consome o
print (e o `capture_clear` fecha a prévia).

## 9. Frontend

| Arquivo | Papel |
|---|---|
| `src/core/domain/settings.ts` | `PanelTab` += `"session"` |
| `src/core/domain/journal.ts` | Tipos (`JournalEntry`, `JournalStatus`, `JournalRequestSource`) |
| `src/core/application/useJournal.ts` | Lista + streaming + `ask()` |
| `src/features/journal/JournalView.tsx` | A aba Sessão (composer + diário + busca) |
| `src/features/journal/AskComposer.tsx` | Campo, print, anexos, chips |
| `src/features/journal/JournalEntryCard.tsx` | Uma entrada (pergunta + resposta + print) |
| `src/features/journal/SavedQuestionsEditor.tsx` | Editor das perguntas prontas |
| `src/features/panel/PanelTabs.tsx` | Terceira aba |
| `src/features/panel/PanelWindow.tsx` | Monta a aba Sessão |
| `src/features/overlay/AskBar.tsx` | A **barra padrão**: lupa, campo, atalho e "Capturar Tela" |
| `src/features/overlay/NotchTabs.tsx` | Botão **"Perguntar"** (lupa) ao lado de "Resumo" (estado ativo) |
| `src/features/overlay/Overlay.tsx` | `onOpenAsk` → `panelOpenTab("chat", "session")` |
| `src/components/icons.tsx` | `SearchIcon` (lupa, SVG do projeto — BR-008) |
| `src/adapters/ipc/journal.ts` | Adaptador IPC do diário |
| `src/adapters/ipc/screen.ts` | Adaptador IPC da captura |

O botão do notch (US-05):

```tsx
<DwellButton
  triggerId="notch-ask"
  className="notch-tabs__item"
  title="Perguntar à IA"
  onActivate={onOpenAsk}
>
  <SearchIcon className="notch-tabs__icon" size={15} />
  <span className="notch-tabs__label">Perguntar</span>
</DwellButton>
```

Posicionado **entre "Resumo" e "Sugestões"** — o switcher só é renderizado no
estado ativo do notch (BR-032), então o botão **só existe com sessão
iniciada**.

## 10. Persistência e ciclo de vida

- A pergunta é persistida **antes** da chamada à IA (RNF-06): se o app
  cair no meio, a entrada fica `pending` e aparece no diário.
- **Excluir a sessão** exclui o diário e as imagens (cascade — FKs ligadas em
  `connection.rs`).
- **Encerrar a sessão** mantém o diário; "Continuar" (Histórico) reabre a
  sessão e o diário volta junto (BR-019).
- A entrada **nunca** entra na linha temporal da aba Transcrição
  (BR-040): `session_timeline.rs` continua lendo só `transcriptions` +
  `messages` (sugestões).
- O export Markdown ganha a seção **"Perguntas"** — montada no core
  (`application/markdown_export.rs`), nunca na UI (BR-020, BR-016).

## 11. Privacidade

- A imagem e o texto saem **só** para a fonte de IA escolhida pelo usuário
  (BR-002, BR-005). Nada mais.
- **Nenhuma telemetria** (BR-004).
- Chave de API nunca em log, erro ou no diário (BR-003).
- O print é **local** até o envio; depois de gravado no diário, fica no banco
  local (o usuário pode excluir a sessão).

## 12. Referências

- PRD: [`../prd.md`](../prd.md)
- Issues: [`../issues.md`](../issues.md)
- Regras: BR-011, BR-013, BR-015, BR-019, BR-020, BR-024, BR-025, BR-028,
  BR-032, BR-033, BR-036, BR-040, BR-042, BR-045, BR-048, BR-052…BR-058.
- Sessão e histórico: [`../../mvp/architecture/session-history.md`](../../mvp/architecture/session-history.md).
- Linha temporal e export: [`../../chat/architecture/transcription-panel.md`](../../chat/architecture/transcription-panel.md).
