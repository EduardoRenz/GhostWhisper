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

- **IDs:** BR-023, BR-024, BR-025, BR-026, T01–T08
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
- **T07** histórico de versões do resumo **em memória** (teto 20) na janela:
  hora + título do tópico, leitura congelada com “Voltar ao atual”; `summaryTopic`
  como fallback de título.
- **T08** janelas redimensionáveis: `resizable: true` + mínimo 320×240, alças
  próprias (`ResizeHandles` → `startResizeDragging`) e permissão
  `core:window:allow-start-resize-dragging`; CSS responsivo; tamanho persistido em
  `Settings.summary_size`/`suggestions_size` e reaplicado no boot.

### Verificação
- **282 testes Rust** (+65; 8 `ignored`) · `cargo clippy --all-targets` e
  `cargo fmt --check` limpos · `cargo build` ok.
- **118 testes frontend** (+44) · `tsc --noEmit` limpo · `pnpm build` ok.
- Pacotes: `.deb` (8,65 MiB) e AppImage (86,44 MiB) em
  `src-tauri/target/release/bundle/` (reconstruir após a T08).

### Pendências
- Captura de tela/OCR continua fora de escopo: no modo LeetCode o enunciado vem
  da fala ou da caixa “Contexto extra”.
- Streaming token a token nas janelas e edição manual do resumo ficam para depois.
- O histórico de versões do resumo é **em memória** (não sobrevive a reinícios);
  versões persistidas por sessão ficam para uma PRD/tarefa futura (T07).
- `metadata` da tabela `messages` ainda não guarda o modo — no histórico a
  sugestão fixada aparece com o rótulo genérico.

---

## 2026-10-19 — Resposta vazia do provider: reenvio e diagnóstico

- **IDs:** BR-023, BR-028 · **Refs:** `src-tauri/src/adapters/ai/openai_compatible.rs`,
  `src-tauri/src/adapters/ai/gemini_chat_bot_provider.rs`,
  `src-tauri/src/application/summary_service.rs`

### Contexto
Relato: às vezes o **resumo** falha com "provider retornou uma resposta vazia".
A resposta HTTP era 200, mas o texto extraído ficava vazio — então o resumo abortava.

### Diagnóstico (causas prováveis)
- **Transiente** do provider/proxy (200 com `choices` vazio ou `content` vazio).
- **Modelo de raciocínio** gastou todo o `max_tokens`/`max_completion_tokens`
  antes de escrever (`finish_reason: "length"`) → `content` vazio.
- **Formato de conteúdo** diferente: `content` como **array de partes** (endpoints
  multimodais/proxies) ou resposta no formato legado `choices[0].text`.
- **Bloqueio** (`content_filter`, Gemini `SAFETY`/`RECITATION` ou `blockReason`).

### Correção
- `openai_compatible::extract_text` aceita `content` string, **array de partes** e
  `choices[0].text`; `parse_completion` captura `finish_reason` e `refusal`.
- **Reenvio único** quando o texto vem vazio, com o orçamento de tokens **dobrado**
  (teto de 16k), cobrindo transiente e raciocínio que consumiu o limite.
- Erro final **acionável**: recusa do provider, `finish_reason=length`
  (aumentar tokens/modelo sem raciocínio), `content_filter` ou motivo desconhecido.
- Mesmo tratamento no **Gemini** (`extract_candidate`, `grow_output_budget`,
  `finishReason`/`promptFeedback.blockReason`).

### Verificação
- **343 testes Rust** (+9: extração, orçamento, mensagens e dois testes de reenvio
  com servidor local) · `cargo clippy --all-targets` e `cargo fmt --check` limpos.

### Limite conhecido
- Se o provider devolver vazio de forma **persistente por contexto grande**, o
  reenvio pode não bastar (o resumo usa 24k caracteres por chunk); um fallback de
  chunk menor fica como próximo passo.
