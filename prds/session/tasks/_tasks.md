# Tasks — PRD Sessão (diário de perguntas)

Índice de tarefas deste PRD. Cada tarefa tem um arquivo `NN-task.md`.

**Legenda de status:** `PENDING` · `IN PROGRESS` · `CONCLUDED` · `CANCELED`

| # | Resumo | Status |
|---|--------|--------|
| 01 | Domínio do diário + porta `JournalRepository` + schema (tabelas, FTS ancorado, índices) | PENDING |
| 02 | Aba "Sessão" no painel + botão "Perguntar" (lupa) no notch | PENDING |
| 03 | `AskService`: perguntar com contexto da sessão, streaming e persistência | PENDING |
| 04 | UI do diário: lista, streaming, estados de erro e composer | PENDING |
| 05 | Perguntas prontas: settings, chips e editor | PENDING |
| 06 | Captura de tela: porta `ScreenCapture` + adaptador X11 + colar imagem | PENDING |
| 07 | Multimodal: `CompletionRequest.images` + providers + anexar o print | PENDING |
| 08 | Pesquisa no diário (FTS com escape) + perguntas no export Markdown | PENDING |
| 09 | Documentação (BRs/ISS/memory), verificação e pacotes | PENDING |
| 10 | Dívida de índices do schema + ancoragem do FTS por `id` | PENDING |

## Ordem e dependências

```
01 ─▶ 03 ─▶ 04 ─▶ 08 ─┐
02 ─▶ 04 ─▶ 05 ────────┤
06 ─▶ 07 ───────────────┴─▶ 09
10 ────────────────────────▶ 09
```

- A **01** cria o modelo (domínio, porta, tabelas) e é pré-requisito de tudo
  que persiste.
- A **02** dá a casca (aba + botão do notch) e é independente do núcleo.
- A **03** implementa o caso de uso da pergunta (contexto, streaming,
  persistência) — depende da 01.
- A **04** monta a UI do diário e o composer — depende da 03 (e da 02, que
  cria a aba).
- A **05** acrescenta as perguntas prontas **dentro** do composer da 04.
- A **06** implementa a captura de tela (porta + X11 + colar imagem) — não
  depende da 01.
- A **07** torna o pedido **multimodal** e liga o print à pergunta — depende
  da 06 (tem o que enviar).
- A **08** fecha o ciclo: busca no diário (com escape do `MATCH`) e o
  export Markdown com as perguntas.
- A **10** é a dívida do schema (índices compostos + ancoragem do FTS) —
  toca `schema.sql`, então vai **antes** da 09 (que fecha a documentação) e
  pode ser feita a qualquer momento depois da 01.
- A **09** fecha: BRs, ISS, `memory.md`, verificação e pacotes.

## Entregáveis por camada

| Task | `domain` | `application` | `ports` | `adapters` | `ipc` | frontend |
|---|---|---|---|---|---|---|
| 01 | `domain/journal.rs` | — | `ports/journal_repository.rs` | `local_journal_repository.rs` + `in_memory_…` | — | — |
| 02 | — | — | — | — | — | `PanelTabs`, `PanelWindow`, `NotchTabs`, `Overlay`, `icons` |
| 03 | `domain/llm.rs` (imagens), `domain/ai_route.rs` (`Search`) | `application/ask_service.rs` | — | — | `ipc/journal_commands.rs` | — |
| 04 | `core/domain/journal.ts` | `core/application/useJournal.ts` | — | — | `adapters/ipc/journal.ts` | `features/journal/*` |
| 05 | `domain/settings.rs` (`saved_questions`) | — | — | `file_settings_store` (já existe) | `ipc/settings_commands` (já existe) | `SavedQuestionsEditor` |
| 06 | — | — | `ports/screen_capture.rs` | `adapters/screen/x11_screen_capture.rs` | `ipc/screen_commands.rs` | `adapters/ipc/screen.ts` |
| 07 | `domain/llm.rs` | — | — | `openai_compatible.rs`, `gemini_chat_bot_provider.rs` | — | `AskComposer` (anexo) |
| 08 | — | `application/markdown_export.rs` | `ports/journal_repository.rs` (`search`) | — | `ipc/journal_commands.rs` (`search`) | `JournalView` (busca) |
| 10 | — | — | — | `schema.sql` + repositórios | — | — |

> Nenhum adaptador é instanciado fora de `src-tauri/src/lib.rs` (R2).
