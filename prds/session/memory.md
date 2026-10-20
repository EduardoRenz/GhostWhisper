# Memory — PRD Sessão (diário de perguntas)

Histórico cronológico deste PRD (entradas `## YYYY-MM-DD — Título`, com linha
`IDs:`; convenções em [`../../.agents/rules.md`](../../.agents/rules.md)).

---

## 2026-10-19 — PRD criada (aba Sessão + feature Busca)

- **IDs:** BR-052, BR-053, BR-054, BR-055, BR-056, BR-057, BR-058, ISS-041,
  ISS-042 · **Refs:** [`prd.md`](prd.md),
  [`architecture/session-journal.md`](architecture/session-journal.md),
  [`tasks/_tasks.md`](tasks/_tasks.md)

### Pedido

O usuário pediu para **analisar o código do Perssua** e preparar uma PRD que
adicionasse o conceito de **"Sessão"**: uma **aba** que vai acumulando, em um
**diário**, tudo que o usuário perguntar em uma feature **nova de busca**, onde
se pode **perguntar**, **tirar print de tela** e **salvar perguntas prontas**.

### Decisões fechadas com o usuário (2026-10-19)

| Tema | Decisão |
|---|---|
| Âmbito do diário | **Por sessão** de transcrição (`session_id`) |
| Persistência | **Tabela nova** `journal_entries` + `journal_fts` próprios (não mexer em `messages`) |
| Escopo da v1 | **perguntar + print + perguntas prontas** (sem busca web, sem busca em documentos) |
| Captura de tela | **X11 via `x11rb` `GetImage`** (sem ferramenta externa) |
| Botão "Perguntar" | **No notch, ao lado de "Resumo"**, ícone de **lupa**, só com sessão iniciada |

### Evidência do Perssua (REA no `Perssua-0.27.5-x86_64.AppImage`)

- A sessão do Perssua **é** um diário: `messages` (`role` user/assistant/
  system) + `transcriptions` + `subject`/`overview`, com `messages_fts`
  (FTS5 com triggers) para busca.
- O **Ask** é uma superfície única com taxonomia `requestType` /
  `requestSource` (`ask`, `followup`, `word`, `transcription`, `clipboard`,
  `screenshot`).
- O **print** usa `capture-screenshot` → JPEG q80 ≤1920×1080, reencodeado por
  preset (`balanced` = 1280/q0.86, o default do Ask); **1 imagem por
  request**; as janelas do app são escondidas durante a captura; **colar
  imagem** também anexa.
- As **perguntas prontas** são as "Prompt Suggestions": máx. **4 × 140
  chars**, em `localStorage["perssua.textInput.promptSuggestions"]`, com
  editor (editar/resetar/salvar) e chips que disparam a pergunta. Defaults:
  *"Summarize everything so far"*, *"List action items"*, *"List main ideas
  discussed"*, *"What can I say?"*.
- O "**Document sources**" do Perssua é **MCP** (Drive/Docs/Sheets/Notion/
  Obsidian) — **não** é busca no filesystem local. O "**Web**" é uma chamada
  separada, só na barra flutuante e só para Gemini/OpenAI.
- A busca de sessões do Perssua é `getSessionsPage` (FTS `MATCH` + `LIKE` em
  subject/overview), página de 20, debounce 180 ms, **sem ranking**.

### Desvios consciente (R12)

1. Perguntas prontas **20 × 200** (o Perssua limita por paywall; aqui não há
   login — BR-002).
2. FTS ancorado por **`id`**, não por `rowid` (o do Perssua quebra num
   `VACUUM` — ISS-041).
3. **Sem "Ask" flutuante**: o composer vive na aba Sessão e o notch ganha
   **um botão** (o overlay é `actionbar | hidden` — BR-032).
4. **Sem busca web nem busca em documentos** na v1.

### Achado colateral — índices do schema

A análise do banco mostrou **índices simples** forçando *temp b-tree* em toda
listagem ordenada (`transcriptions`/`messages` por `session_id ORDER BY
timestamp`), **full scan** em `sessions ORDER BY started_at` e **três
índices mortos** (`idx_messages_timestamp`, `idx_transcriptions_timestamp`,
`idx_sessions_user` — `user_id` é sempre `NULL`, BR-002). `idx_sessions_status`
**é** usado (pelo `end_all_active`) e **fica**. A correção está na task 10,
junta da ancoragem do FTS (ISS-041).

### Pendências

- Wayland nativo: sem captura pelo X11 (BR-007) — o caminho é **colar
  imagem**, com erro explícito quando não há display X11.
- Busca web / busca em documentos: PRD própria, se e quando for pedido.
