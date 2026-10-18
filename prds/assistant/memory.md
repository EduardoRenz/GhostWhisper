# Histórico — PRD Assistant

Histórico deste PRD no formato fixo do
[`rules.md`](../../.agents/rules.md) §6.2 (busca por palavra-chave, ID e período).

| Objetivo | Comando |
|----------|---------|
| Palavra-chave | `rg -n -i 'resumo' prds/assistant/memory.md` |
| Entrada do dia | `rg -n '^## 2026-10-18' prds/assistant/memory.md` |
| Por ID | `rg -n 'ISS-022\|BR-023' prds/assistant/memory.md` |

---

## 2026-10-18 — PRD Assistant: resumo vivo e sugestões por agentes

- **IDs:** BR-023, BR-024, BR-025, BR-026, T01–T06
- **Refs:** [`prd.md`](prd.md), [`issues.md`](issues.md),
  [`architecture/assistant-windows.md`](architecture/assistant-windows.md),
  [`tasks/_tasks.md`](tasks/_tasks.md)

### Como a PRD nasceu
O usuário pediu duas superfícies novas inspiradas no Perssua: uma **janela de
resumo** que se atualiza sozinha e uma **janela de sugestões** com seleção de
agentes (Reunião, Entrevista Técnica, LeetCode, System Design e personalizados).
As duas devem gerar e renderizar Markdown, com código formatado e colorido.

### O que foi extraído do Perssua
- `electron/prompts/notetakerSummaryPrompt.js` — contrato completo do resumo.
- `build/static/js/main.f180d025.js` — prompt incremental do resumo, envelope
  `SUBJECT:`/`CONTENT:` e intervalo de 30 s.
- `electron/flavors.js` — Note Taker e Cody (system/realtime/fast-first).
- `build/static/js/assistantsWindow.8c21ae6c.js` — campos de assistente
  personalizado.

### Decisões (respostas do usuário)
- Janelas **sem moldura, arrastáveis**, sempre no topo e com posição lembrada.
- Sugestões automáticas **efêmeras** + botão **Fixar**.
- **Editor completo** de agentes personalizados nesta PRD.
- Prompts **adaptados** (contrato, não cópia literal), com atribuição.

### Desvios conscientes do Perssua (R12)
- Janelas de SO separadas em vez do modal/Notch interno.
- Áudio-only: LeetCode usa enunciado falado + “Contexto extra” (sem OCR).
- Sempre tentar responder; `...` só sem transcrição/ininteligível.
- Big O em negrito em vez de `<span style="color: green">`.
- Resumo automático só com a janela aberta (controle de custo).

### Entregue
- **T01** domínio de agentes (`domain/assistant.rs`), `MeetingSummary`
  (`domain/overview.rs`), campos novos em `Settings` (modo, cadências, posições),
  contratos de prompt adaptados (`prompts/summary.rs`, `prompts/assistant.rs`),
  `application/context.rs` e a tabela/repositório `meeting_summaries`.
- **T02** `SummaryService` (contrato completo + incremental com cursor/chunks) e
  `SuggestionService` (agente, últimos momentos, `since_ms`, fixar), IPC
  (`summary_get/generate`, `suggestion_generate/pin`, `assistant_modes_list`,
  `llm_insight`) e o `InsightService`/`InsightGenerator` aposentados.
- **T03** `MarkdownView` (react-markdown + GFM + highlight.js, sem HTML bruto),
  janela `summary` (sem moldura, arrastável, automático de 30 s, cópia e resumo
  parcial) e `useSummary`; botões no overlay/tray.
- **T04** janela `suggestions` com seletor de agentes, automático de 20 s,
  contexto extra, “Fixar”, histórico curto e `useSuggestions`; bolhas da IA
  renderizam Markdown.
- **T05** aba **Assistente** nas configurações: agente padrão, cadências e editor
  completo de agentes personalizados (criar/editar/duplicar/excluir).
- **T06** resumo no export Markdown e no detalhe do histórico; BR-023 a BR-026;
  README e ponteiros em `mvp/overlay.md` e `chat/chat-ui.md`.

### Verificação
- **280 testes Rust** (+63; 8 `ignored`) · `cargo clippy --all-targets` e
  `cargo fmt --check` limpos · `cargo build` ok.
- **104 testes frontend** (+30) · `tsc --noEmit` limpo · `pnpm build` ok.
- Pacotes: `pnpm tauri build` gerou `.deb` (8,65 MiB) e AppImage (86,44 MiB) em
  `src-tauri/target/release/bundle/`.
### Pendências
- Captura de tela/OCR continua fora de escopo: no modo LeetCode o enunciado vem
  da fala ou da caixa “Contexto extra”.
- Streaming token a token nas janelas e edição manual do resumo ficam para depois.
- `metadata` da tabela `messages` ainda não guarda o modo — no histórico a
  sugestão fixada aparece com o rótulo genérico.
