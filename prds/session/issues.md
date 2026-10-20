# Issues — PRD Sessão (diário de perguntas)

Registry de bugs e pendências observadas. Convenções em
[`../../.agents/rules.md`](../../.agents/rules.md) (`ISS-NNN` é canônico por
arquivo de PRD; a numeração **continua** a dos PRDs anteriores para o ID ser
único no repositório). Próximo ID livre: **ISS-043**.

| ID | Resumo | Severidade | Status |
|---|---|---|---|
| ISS-041 | FTS ancorado no `rowid` da tabela base — quebra num `VACUUM` | Média | ABERTO |
| ISS-042 | Busca FTS com texto do usuário quebra a consulta (sem escape) | Média | ABERTO |

---

## ISS-041 — FTS ancorado no `rowid` da tabela base quebra num `VACUUM`

- **Severidade:** Média · **Status:** ABERTO (pré-existente; não bloqueia a PRD)
- **Sintoma:** as buscas full-text (`TranscriptionsRepository::search` e a
  nova busca do diário) casam a linha da tabela base com a linha do FTS por
  `rowid`:

  ```sql
  SELECT … FROM transcriptions t
   WHERE t.rowid IN (SELECT rowid FROM transcriptions_fts WHERE transcriptions_fts MATCH ?1)
  ```

  O trigger `transcriptions_ai` grava `new.rowid` — o rowid **implícito** da
  tabela base (o `id` é `TEXT PRIMARY KEY`, não `INTEGER PRIMARY KEY`). O
  SQLite **renumera rowids implícitos** num `VACUUM`. Depois disso, o
  casamento passa a apontar para a **linha errada** — a busca devolve
  segmentos que **não** contêm o termo, e continua parecendo funcionar.
- **Causa raiz:** a âncora entre o FTS e a tabela base é o `rowid`, que é
  **derivado** e mutável, em vez de uma chave estável.
- **Evidência no Perssua:** o mesmo padrão (`getSessionsPage` →
  `messages_fts MATCH` + `id IN (SELECT session_id FROM messages_fts …)`).
  Lá o FTS casa pelo **`session_id`**, que é uma coluna `UNINDEXED` do FTS —
  ou seja, também depende do `rowid` interno do FTS para casar com a linha.
- **Correção proposta:** ancorar pelo **id** (chave estável), usando a coluna
  `id`/`segment_id` que o FTS já guarda:

  ```sql
  -- hoje
  t.rowid IN (SELECT rowid FROM transcriptions_fts WHERE transcriptions_fts MATCH ?1)
  -- corrigido
  t.id   IN (SELECT segment_id FROM transcriptions_fts WHERE transcriptions_fts MATCH ?1)
  ```

  Plano medido — **mesmo custo**:

  ```
  SEARCH t USING INDEX sqlite_autoindex_transcriptions_1 (id=?)
  LIST SUBQUERY 1
    SCAN transcriptions_fts VIRTUAL TABLE INDEX 0:M3
  ```

  E as **tabas novas** já nascem ancoradas: `journal_entries` tem
  `rowid_pk INTEGER PRIMARY KEY AUTOINCREMENT` (âncora estável) e o
  `journal_fts` casa por `entry_id`.
- **Arquivos:** `adapters/persistence/sqlite/local_transcriptions_repository.rs`,
  `adapters/persistence/schema.sql`, `adapters/persistence/sqlite/local_journal_repository.rs`.
- **Task:** [`tasks/10-task.md`](tasks/10-task.md)

---

## ISS-042 — Busca FTS com texto do usuário quebra a consulta

- **Severidade:** Média · **Status:** ABERTO
- **Sintoma:** a busca do diário (e a de transcrições) passa a string do
  usuário **crua** para o `MATCH` do FTS5. Uma pergunta com aspas, `*`, `:`
  ou `-` é **sintaxe FTS**, não texto: o SQLite devolve
  `fts5: syntax error near …` e a consulta inteira falha — a lista de
  resultados fica **vazia**, sem aviso.
- **Causa raiz:** falta de **escape** da entrada do usuário antes do `MATCH`.
- **Evidência no Perssua:** `DatabaseManager.getSessionsPage` faz exatamente
  isso (`ftsQuery = search.trim()`, sem escape) — é a armadilha que **não**
  deve ser copiada (BR-015 decide *como* resolver, não *copiar* defeito).
- **Correção proposta:** escapar a query antes do `MATCH` — envolver em
  aspas e duplicar as aspas internas (`"` → `""`), que é a forma de buscar
  **termos literais** no FTS5:

  ```rust
  fn escape_fts(query: &str) -> String {
      format!("\"{}\"", query.trim().replace('"', "\"\""))
  }
  ```

  Ou, se a busca for por **palavras** (e não frase exata), tokenizar e
  unir com `AND`.
- **Arquivos:** `adapters/persistence/sqlite/local_journal_repository.rs`,
  `adapters/persistence/sqlite/local_transcriptions_repository.rs`.
- **Task:** [`tasks/08-task.md`](tasks/08-task.md)
