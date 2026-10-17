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

CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id);
CREATE INDEX IF NOT EXISTS idx_messages_timestamp ON messages(timestamp);
CREATE INDEX IF NOT EXISTS idx_transcriptions_session ON transcriptions(session_id);
CREATE INDEX IF NOT EXISTS idx_transcriptions_timestamp ON transcriptions(timestamp);
CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status);
CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions(user_id);

CREATE TABLE IF NOT EXISTS schema_version (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER DEFAULT (strftime('%s', 'now'))
);

INSERT OR IGNORE INTO schema_version (version) VALUES (1);
