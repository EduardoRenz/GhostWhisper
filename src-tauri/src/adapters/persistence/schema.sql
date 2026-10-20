-- GhostWhisper — schema de conversas.
-- Base: formato observado no Perssua, estendido com colunas de transcrição
-- (end_ms, is_final) e uma tabela FTS dedicada para transcrições.

CREATE TABLE IF NOT EXISTS sessions (
  id TEXT PRIMARY KEY,
  started_at INTEGER NOT NULL,
  ended_at INTEGER,
  provider TEXT NOT NULL,
  model TEXT,
  transcription_model TEXT,
  user_id TEXT,
  assistant_name TEXT,
  assistant_id TEXT,
  subject TEXT,
  overview TEXT,
  status TEXT DEFAULT 'active' CHECK(status IN ('active', 'ended')),
  created_at INTEGER DEFAULT (strftime('%s', 'now'))
);

CREATE TABLE IF NOT EXISTS messages (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  role TEXT NOT NULL CHECK(role IN ('user', 'assistant', 'system')),
  content TEXT NOT NULL,
  timestamp INTEGER NOT NULL,
  metadata TEXT,
  created_at INTEGER DEFAULT (strftime('%s', 'now')),
  FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS transcriptions (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  speaker TEXT NOT NULL CHECK(speaker IN ('ME', 'THEY')),
  text TEXT NOT NULL,
  timestamp INTEGER NOT NULL,
  end_ms INTEGER NOT NULL DEFAULT 0,
  is_final INTEGER NOT NULL DEFAULT 1,
  created_at INTEGER DEFAULT (strftime('%s', 'now')),
  FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

-- Resumo vivo da sessão (documento Markdown + cursor incremental). O conteúdo
-- também é espelhado em `sessions.overview` para o export/histórico.
CREATE TABLE IF NOT EXISTS meeting_summaries (
  session_id TEXT PRIMARY KEY,
  subject TEXT,
  content TEXT NOT NULL,
  summarized_from_ms INTEGER NOT NULL DEFAULT 0,
  summarized_until_ms INTEGER NOT NULL DEFAULT 0,
  updated_at_ms INTEGER NOT NULL,
  FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
  id UNINDEXED,
  session_id UNINDEXED,
  content
);

CREATE VIRTUAL TABLE IF NOT EXISTS transcriptions_fts USING fts5(
  segment_id UNINDEXED,
  session_id UNINDEXED,
  content
);

-- Diário de perguntas da sessão (aba "Sessão", BR-052).
--
-- `rowid_pk` é a âncora estável do FTS: `id` é TEXT (uuid) e não serve de
-- rowid, e o `rowid` implícito é renumerado num `VACUUM` — o casamento por
-- rowid passaria a apontar para a linha errada, em silêncio (ISS-041).
CREATE TABLE IF NOT EXISTS journal_entries (
  rowid_pk INTEGER PRIMARY KEY AUTOINCREMENT,
  id TEXT NOT NULL UNIQUE,
  session_id TEXT NOT NULL,
  question TEXT NOT NULL,
  answer TEXT,
  asked_at_ms INTEGER NOT NULL,
  answered_at_ms INTEGER,
  status TEXT NOT NULL DEFAULT 'pending'
    CHECK(status IN ('pending', 'answered', 'error', 'cancelled')),
  request_source TEXT NOT NULL DEFAULT 'search'
    CHECK(request_source IN ('search', 'preset', 'screenshot')),
  provider TEXT,
  model TEXT,
  error TEXT,
  created_at INTEGER DEFAULT (strftime('%s', 'now')),
  FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

-- Imagens anexadas (prints de tela) — fora da linha para a listagem do
-- diário não carregar bytes.
CREATE TABLE IF NOT EXISTS journal_images (
  id TEXT PRIMARY KEY,
  entry_id TEXT NOT NULL,
  mime TEXT NOT NULL,
  data BLOB NOT NULL,
  width INTEGER,
  height INTEGER,
  created_at INTEGER DEFAULT (strftime('%s', 'now')),
  FOREIGN KEY (entry_id) REFERENCES journal_entries(id) ON DELETE CASCADE
);

-- FTS do diário. O `entry_id` (UNINDEXED) é a chave de negócio: a busca casa
-- por ele, nunca pelo rowid do FTS (ISS-041).
CREATE VIRTUAL TABLE IF NOT EXISTS journal_fts USING fts5(
  entry_id UNINDEXED,
  session_id UNINDEXED,
  question,
  answer
);

CREATE TRIGGER IF NOT EXISTS messages_ai AFTER INSERT ON messages BEGIN
  INSERT INTO messages_fts(rowid, id, session_id, content)
  VALUES (new.rowid, new.id, new.session_id, new.content);
END;

CREATE TRIGGER IF NOT EXISTS messages_ad AFTER DELETE ON messages BEGIN
  DELETE FROM messages_fts WHERE rowid = old.rowid;
END;

CREATE TRIGGER IF NOT EXISTS messages_au AFTER UPDATE OF content ON messages BEGIN
  UPDATE messages_fts SET content = new.content WHERE rowid = new.rowid;
END;

CREATE TRIGGER IF NOT EXISTS transcriptions_ai AFTER INSERT ON transcriptions BEGIN
  INSERT INTO transcriptions_fts(rowid, segment_id, session_id, content)
  VALUES (new.rowid, new.id, new.session_id, new.text);
END;

CREATE TRIGGER IF NOT EXISTS transcriptions_ad AFTER DELETE ON transcriptions BEGIN
  DELETE FROM transcriptions_fts WHERE rowid = old.rowid;
END;

CREATE TRIGGER IF NOT EXISTS transcriptions_au AFTER UPDATE OF text ON transcriptions BEGIN
  UPDATE transcriptions_fts SET content = new.text WHERE rowid = new.rowid;
END;

-- Diário: o FTS acompanha a entrada pela âncora estável (`rowid_pk`).
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

-- Índices compostos: a listagem por sessão é **ordenada** (BR-058). Com o
-- índice simples, o SQLite fazia *temp b-tree* para ordenar.
CREATE INDEX IF NOT EXISTS idx_messages_session_time
  ON messages(session_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_transcriptions_session_time
  ON transcriptions(session_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_sessions_started_at ON sessions(started_at DESC);
-- `status` é usado por `end_all_active` (UPDATE ... WHERE status = 'active'):
-- este índice **fica**.
CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status);

-- Índices simples substituídos pelos compostos acima (o simples é coberto).
-- `idx_sessions_user` é **morto**: `user_id` é sempre NULL (sem login — BR-002).
DROP INDEX IF EXISTS idx_messages_session;
DROP INDEX IF EXISTS idx_messages_timestamp;
DROP INDEX IF EXISTS idx_transcriptions_session;
DROP INDEX IF EXISTS idx_transcriptions_timestamp;
DROP INDEX IF EXISTS idx_sessions_user;

-- Diário: listagem por sessão já ordenada (sem temp b-tree) e cascade das
-- imagens (BR-058).
CREATE INDEX IF NOT EXISTS idx_journal_session_time ON journal_entries(session_id, asked_at_ms);
CREATE INDEX IF NOT EXISTS idx_journal_images_entry ON journal_images(entry_id);

CREATE TABLE IF NOT EXISTS schema_version (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER DEFAULT (strftime('%s', 'now'))
);

INSERT OR IGNORE INTO schema_version (version) VALUES (2);
