# Issues — PRD Fontes de IA

Registry de bugs e problemas deste PRD. Convenções em
[`../../.agents/rules.md`](../../.agents/rules.md) (`ISS-NNN` é canônico por
arquivo de PRD; a numeração **continua** a dos PRDs anteriores para o ID ser
único no repositório). Próximo ID livre: **ISS-048**.

| ID | Resumo | Severidade | Status |
|---|---|---|---|
| ISS-039 | Chave não aparece em fonte recém-adicionada (ainda no rascunho) | Alta | FIXED |
| ISS-040 | Card fica em "Nunca usada" mesmo com a fonte em uso | Média | FIXED |

---

## ISS-039 — Chave não aparece em fonte recém-adicionada (ainda no rascunho)

- **Severidade:** Alta · **Status:** FIXED
- **Sintoma:** ao cadastrar uma fonte nova (ex.: DeepSeek) e informar a chave,
  a prévia da chave salva não aparecia no card — a fonte continuava como
  "Sem chave configurada".
- **Causa raiz:** `LocalAiSourceService::secret_key` derivava a chave do
  keyring a partir da fonte **persistida** (`Settings::source(id)`). Uma fonte
  recém-adicionada ainda está só no **rascunho** da tela (o "Salvar
  Alterações" é global), então o comando respondia "fonte de IA desconhecida" e
  a chave nunca era gravada.
- **Correção:** a chave no keyring passa a ser indexada pelo **id** da fonte
  (`domain::ai_source::secret_key_for(id)`), que o frontend gera no rascunho —
  a gestão da chave não depende mais do arquivo de settings. A **validação**
  passou a receber a fonte inteira (`AiSourceService::validate(&AiSource)`),
  para validar uma fonte ainda não salva.
- **Arquivos:** `domain/ai_source.rs`, `ports/ai_source_service.rs`,
  `application/ai_source_service.rs`, `ipc/ai_source_commands.rs`,
  `src/adapters/ipc/aiSources.ts`, `src/features/settings/AiSourcesSettings.tsx`.
- **Task:** [`09-task.md`](tasks/09-task.md) (ajuste pós-entrega)

---

## ISS-040 — Card fica em "Nunca usada" mesmo com a fonte em uso

- **Severidade:** Média · **Status:** FIXED
- **Sintoma:** depois de uma chamada real de IA (sugestão/resumo) ou de um
  "Validar chave" bem-sucedido, o card da fonte continuava com o badge
  **"Nunca usada"**.
- **Causa raiz:** duas. (a) A janela de Configurações **nasce oculta**: o
  React monta no boot e buscava a saúde **uma única vez** — antes de qualquer
  uso —; o evento `llm_source_used` só atualizava quem já estivesse escutando.
  (b) O caminho de **validação** (`AiSourceService::validate`) registrava a
  saúde mas **não emitia** o evento, então a tela não sabia.
- **Correção:** (a) a tela **rebusca a saúde ao ganhar foco**
  (`onWindowFocus`), garantindo que o card reflita o uso real; (b) a validação
  passa a emitir `llm_source_used` (o mesmo evento do roteador).
- **Arquivos:** `src/adapters/ipc/client.ts`,
  `src/features/settings/AiSourcesSettings.tsx`,
  `src-tauri/src/application/ai_source_service.rs`,
  `src-tauri/src/lib.rs`.
- **Task:** [`09-task.md`](tasks/09-task.md) (ajuste pós-entrega)

---

<!--
Modelo:

## ISS-039 — Título curto

- **Severidade:** Alta/Média/Baixa · **Status:** OPEN/FIXED
- **Sintoma:** …
- **Causa raiz:** …
- **Correção:** …
- **Arquivos:** …
- **Task:** [`NN-task.md`](tasks/NN-task.md)
-->
