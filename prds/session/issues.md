# Issues — PRD Sessão (diário de perguntas)

Registry de bugs e pendências observadas. Convenções em
[`../../.agents/rules.md`](../../.agents/rules.md) (`ISS-NNN` é canônico por
arquivo de PRD; a numeração **continua** a dos PRDs anteriores para o ID ser
único no repositório). Próximo ID livre: **ISS-056**.

| ID | Resumo | Severidade | Status |
|---|---|---|---|
| ISS-041 | FTS ancorado no `rowid` da tabela base — quebra num `VACUUM` | Média | FIXED |
| ISS-042 | Busca FTS com texto do usuário quebra a consulta (sem escape) | Média | FIXED |
| ISS-055 | Barra “Perguntar” recentraliza e pula depois da captura de tela | Média | FIXED |

---

## ISS-041 — FTS ancorado no `rowid` da tabela base quebra num `VACUUM`

- **Severidade:** Média · **Status:** FIXED
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
- **Correção aplicada:** as tabelas novas nascem ancoradas (`journal_entries`
  tem `rowid_pk INTEGER PRIMARY KEY`) e as duas buscas existentes passaram a
  casar pelo **id** (`journal_fts.entry_id` e `transcriptions_fts.segment_id`),
  nunca pelo `rowid` do FTS.
- **Task:** [`tasks/10-task.md`](tasks/10-task.md)

---

## ISS-042 — Busca FTS com texto do usuário quebra a consulta

- **Severidade:** Média · **Status:** FIXED
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
- **Correção aplicada:** `adapters/persistence/sqlite/fts.rs::escape_query`
  transforma a query do usuário em **termos literais** unidos por `AND`
  (aspas duplicadas). Vale para o diário e para a busca de transcrições.
- **Task:** [`tasks/08-task.md`](tasks/08-task.md)

---

## ISS-055 — Barra “Perguntar” recentraliza e pula depois da captura de tela

- **Severidade:** Média · **Status:** FIXED
  ([`tasks/12-task.md`](tasks/12-task.md))
- **Sintoma:** depois de capturar a tela, quando a barra “Perguntar” voltava a
  ficar visível, ela **pulava** para o topo centralizado — mesmo que o usuário
  a tivesse arrastado. Interações em outras janelas também podiam reposicioná-la
  porque ela não tinha geometria persistida própria.
- **Causa raiz:** `show_ask_window` chamava `center_ask_window` **antes e depois**
  do `show` sempre que a janela estava escondida. O fluxo de captura esconde
  todas as janelas e, ao terminar, o `AskWindow` chama `askOpen()` para
  garantir a volta/foco; isso disparava a centralização. A barra não tinha
  `ask_position`, então a posição real não era lembrada nem protegida de
  snapshots antigos de settings.
- **Correção (task 12):**
  - `show_ask_window` reaplica a **mesma `ask_position` salva** antes e depois
    do `show`; nunca recentraliza quando há posição salva. O `set_position`
    inicial do WM em `0,0` é corrigido antes de o usuário interagir.
  - novo `ask_position` em `Settings` (Rust/TS), restaurado no boot e
    persistido antes de esconder/capturar; sem posição salva, a primeira
    colocação continua centralizada no topo.
  - tracker do core persiste a posição enquanto a barra está visível;
    `Settings::preserve_window_geometry` inclui `ask_position`, então um save
    de outra janela não sobrescreve a posição real.
  - a captura de tela passou a guardar `(posição, tamanho)` de **cada** janela
    visível antes do `hide` e a reaplicá-los antes/depois do `show`, então o
    retorno da captura não depende da posição que o WM escolher.
  - regra geral BR-072: nenhuma janela se move para outro lugar sozinha; só
    arraste explícito, colocação inicial, âncora/densidade ou popover ancorado.
- **Follow-up do teste real (2026-10-21):** o primeiro ajuste (só persistir e
  remover a centralização) não bastou no WM do usuário: uma janela oculta era
  mapeada em `0,0` ao reaparecer. A correção complementar reaplica a posição
  salva no `show` e restaura a geometria exata no ciclo da captura.
- **Arquivos:** `src-tauri/src/ipc/ask_commands.rs`,
  `src-tauri/src/domain/settings.rs`, `src-tauri/src/lib.rs`,
  `src/core/domain/settings.ts`.
- **Task:** [`tasks/12-task.md`](tasks/12-task.md)
- **Dano em dados existentes:** nenhum; `ask_position` ausente vira `None` e a
  barra centraliza uma única vez no próximo boot.
