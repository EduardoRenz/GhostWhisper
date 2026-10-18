# Arquitetura — Janelas de resumo e sugestões

Especificação técnica das duas janelas de assistência. Base de comportamento no
**Perssua 0.27.5** (BR-015), extraído do `app.asar`; evidência no §8. Desvios
conscientes no §5.

---

## 1. Componentes

```
domain/assistant.rs                 # modos embutidos, validação, resolução
domain/overview.rs                  # MeetingSummary (documento gerado)
domain/settings.rs                  # modos personalizados, cadências, posições

ports/meeting_summaries_repository.rs
ports/summary_generator.rs
ports/suggestion_generator.rs       # evolui o antigo InsightGenerator

application/context.rs              # janelas de contexto (puro)
application/prompts/summary.rs      # contrato de resumo + prompt incremental
application/prompts/assistant.rs    # contratos dos agentes
application/summary_service.rs
application/suggestion_service.rs

adapters/persistence/sqlite/local_meeting_summaries_repository.rs
adapters/persistence/memory/in_memory_meeting_summaries_repository.rs

ipc/summary_commands.rs
ipc/suggestion_commands.rs
ipc/panel_commands.rs

src/components/MarkdownView.tsx
src/features/summary/SummaryWindow.tsx
src/features/suggestions/SuggestionsWindow.tsx
src/core/application/useSummary.ts
src/core/application/useSuggestions.ts
src/core/application/useAssistantModes.ts
src/adapters/ipc/summary.ts
src/adapters/ipc/suggestions.ts
src/adapters/ipc/panels.ts
```

O overlay continua dono da conversa. As janelas novas são webviews separadas
carregando o mesmo `index.html`; `App.tsx` resolve a rota pelo **label** da janela
(`summary` / `suggestions`).

## 2. Modelo de dados

Nova tabela (idempotente, sem migração destrutiva):

```sql
CREATE TABLE IF NOT EXISTS meeting_summaries (
  session_id TEXT PRIMARY KEY,
  subject TEXT,
  content TEXT NOT NULL,
  summarized_from_ms INTEGER NOT NULL DEFAULT 0,
  summarized_until_ms INTEGER NOT NULL DEFAULT 0,
  updated_at_ms INTEGER NOT NULL,
  FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);
```

- `content` é o Markdown do resumo.
- `summarized_from_ms` é o primeiro instante incluído (mostra “resumo desde …”
  quando a sessão já era longa).
- `summarized_until_ms` é o cursor: o próximo tick só envia falas depois dele.
- `content` também é espelhado em `sessions.overview` para o export/histórico
  existente; `subject` só preenche `sessions.subject` quando ele está vazio.
- Sugestões fixadas usam a tabela `messages` já existente (papel `assistant`),
  com o tempo desde o início da sessão (BR-020/BR-021). Sugestões automáticas
  **não** são gravadas.

## 3. Orçamentos e constantes

| Constante | Valor | Nota |
|---|---|---|
| `SUMMARY_INTERVAL_MS` | 30.000 | paridade Perssua |
| `SUMMARY_MAX_CONTEXT_CHARS` | 24.000 | por chamada |
| `SUMMARY_MAX_CHUNKS_PER_TICK` | 3 | evita rajada de chamadas |
| `SUMMARY_MAX_TOKENS` | 2.500 | |
| `SUMMARY_TEMPERATURE` | 0.2 | |
| `SUGGESTION_INTERVAL_MS` | 20.000 | padrão; configurável 10–60 s |
| `SUGGESTION_WINDOW_MS` | 180.000 | “últimos momentos” |
| `SUGGESTION_MAX_SEGMENTS` | 40 | |
| `SUGGESTION_MAX_CONTEXT_CHARS` | 6.000 | |
| `SUGGESTION_MAX_TOKENS` | 900 (reunião) · 1.400 (técnico) | |
| `SUGGESTION_TEMPERATURE` | 0.4 (reunião) · 0.2 (técnico) | |
| `EXTRA_CONTEXT_MAX_CHARS` | 4.000 | caixa “Contexto extra” |
| `MODE_PROMPT_MAX_CHARS` | 16.384 | validação de agente personalizado |
| `LAST_SUGGESTION_MAX_CHARS` | 2.000 | anti-repetição |

Intervalos e toggles vivem em `Settings` e são limitados por `sanitized()`.

## 4. Fluxos

### 4.1 Resumo

```
Overlay/Tray → summary_open → janela + evento panel_opened
SummaryWindow → summary_get(session_id?)          # mostra o gravado
             → summary_generate(session_id?, auto) # primeiro tick
a cada 30 s (aberta, sessão ativa, auto on):
  summary_generate(auto)
    → SummaryService
       ├─ carrega sessão + falas finais
       ├─ carrega resumo anterior (cursor)
       ├─ se não há fala após o cursor: devolve o anterior (updated=false)
       ├─ processa até 3 chunks de 24k:
       │    primeiro chunk sem resumo → contrato completo + SUBJECT/CONTENT
       │    demais (ou já havia resumo) → prompt incremental
       ├─ parseia/normaliza Markdown
       └─ grava meeting_summaries + sessions.overview/subject
    → MeetingSummary { updated, has_more, summarized_until_ms, … }
```

- **Abrir** usa `auto`: se não há resumo, gera; se há, atualiza o que faltou.
- **Atualizar agora** usa `manual`: regenera do zero, com o cursor resetado.
- Se `has_more = true`, a janela agenda um novo tick curto (1 s) até concluir,
  respeitando o limite de uma requisição em voo.
- O prompt inicial usa o envelope `SUBJECT:`/`CONTENT:`; o incremental devolve
  só o Markdown atualizado. `normalize_summary_markdown` remove cercas externas.
- A janela mantém um **histórico de versões em memória** (teto 20, mais recente
  primeiro): só entram resultados com `updated = true`; selecionar uma versão
  **congela** a leitura e `showLatest()` volta ao ao vivo. É efêmero — a versão
  persistida continua sendo a atual em `meeting_summaries` (T07).

### 4.2 Sugestão

```
SuggestionsWindow → assistant_modes_list
                  → suggestion_generate {
                       session_id, mode_id, since_ms?, last_suggestion?,
                       extra_context?, persist: false
                     }
    → SuggestionService
       ├─ resolve modo (embutido ou personalizado)
       ├─ falas finais dos últimos 3 min (≤40 / ≤6k chars)
       ├─ se since_ms e nenhuma fala nova → skipped=true
       ├─ system = prompt do modo + critérios de domínio + idioma
       ├─ user   = ME/THEY + <last_suggestion> + <additional_context>
       └─ LLM → normalize → Suggestion { text, at_ms, context_until_ms, … }
    → MarkdownView
"Fixar" → suggestion_pin { text, mode_id, at_ms } → messages + evento insight
```

- O botão **Sugerir** do overlay e o atalho global chamam o mesmo serviço com
  `persist: true`, usando o agente selecionado. A sugestão entra na conversa.
- O seletor de agente é global (`Settings.assistant_mode_id`). Trocar o agente
  na janela dispara uma nova análise com `since_ms = None`.
- A caixa de contexto extra é por janela (efêmera) e vai como fonte suplementar.

## 5. Desvios conscientes do Perssua (R12)

| Desvio | Por quê |
|---|---|
| Janelas de SO separadas em vez do modal/Notch interno | pedido do usuário; permite posicionar resumo e sugestões lado a lado com a reunião |
| Sem moldura, tamanho fixo e rolagem interna | mantém o visual stealth do overlay; arraste por cabeçalho próprio |
| Sugestões automáticas efêmeras + **Fixar** | não inundar a linha temporal/export; o usuário escolhe o que vira documento |
| Áudio-only (sem tela/OCR) | escopo Linux-first e privacidade; LeetCode usa enunciado falado + contexto colado |
| Sempre tentar responder (`...` só sem fala) | pedido do usuário; o Perssua usa `...` com mais frequência |
| Big O em **negrito** em vez de `<span style="color: green">` | renderer sem HTML bruto; mesma informação, mais seguro |
| Resumo automático só com a janela aberta | controla custo; o Perssua mantém o pipeline enquanto a sessão roda |
| Rótulos de UI em PT-BR | i18n do produto (BR-010) |
| Prompts adaptados, não literais | BR-008: comportamento sim, código/texto proprietário não |

## 6. Janelas e ciclo de vida

- `tauri.conf.json` declara `summary` e `suggestions` com `visible: false`,
  `decorations: false`, `transparent: true`, `alwaysOnTop: true`,
  `skipTaskbar: true`, `resizable: true`, `minWidth: 320`, `minHeight: 240`.
- `summary_open` / `suggestions_open` mostram e focam; emitem `panel_opened`.
- `panel_hide` esconde e emite `panel_hidden`; o `CloseRequested` nativo também
  esconde em vez de destruir.
- `panel_start_drag` chama `start_dragging()` na janela chamadora.
- **Resize sem moldura:** como uma janela sem decoração não tem bordas para o
  gerenciador de janelas pegar, o `PanelFrame` renderiza `ResizeHandles` (8 alças
  invisíveis nas bordas/cantos) que chamam
  `getCurrentWindow().startResizeDragging(dir)` — permissão
  `core:window:allow-start-resize-dragging` em `capabilities/default.json`. Em
  **X11/XWayland** funciona; no **Wayland nativo** depende do compositor e pode
  não funcionar (BR-007).
- Posições persistem em `Settings.summary_position`/`suggestions_position` e
  tamanhos em `Settings.summary_size`/`suggestions_size` (px físicos), por um
  tracker no composition root que roda enquanto a janela está visível; o boot
  reaplica ambos. O CSS mínimo (`.panel-shell`, `min-width: 0`, `@media
  (max-width: 420px)`) evita layout degenerado no tamanho mínimo.
- As labels novas entram em `src-tauri/capabilities/default.json`.

## 7. Prompt contracts

### 7.1 Resumo (`prompts/summary.rs`)

Contrato completo adaptado de `notetakerSummaryPrompt.js`:

- fonte: somente falas dos participantes; blocos estruturados são suplementares;
- normalização silenciosa de erros de ASR, sem inventar fatos;
- idioma dominante da reunião; preservar nomes/números/termos;
- classificar decisão confirmada × meta/hipótese × recomendação × questão aberta;
- seções: Visão geral, Decisões confirmadas, Metas/hipóteses/propostas,
  Estratégia/plano, Métricas, Action items, Questões em aberto, Riscos;
- action items com **Action / Owner / Deadline** (“Não definido” quando ausente);
- checagem final de qualidade.

Prompt inicial acrescenta regras de Markdown, grounding e o envelope
`SUBJECT:`/`CONTENT:`. O incremental usa “NEW CONTENT / CURRENT SESSION SUMMARY”
e devolve só o Markdown.

### 7.2 Agentes (`prompts/assistant.rs`)

| Agente | Base | Contrato |
|---|---|---|
| Reunião | `NOTETAKER_FLAVOR_ASSISTANT.realtimePrompt` | blockquote com a próxima fala + “O que está sendo dito / Ações / Glossário”; não inventar fatos da reunião |
| Entrevista Técnica | `CODY_FLAVOR_ASSISTANT.realtimePrompt` | primeira pessoa, uma etapa por vez; system design (clarificar → dimensionar → desenhar) e LeetCode (Big O → linguagem → implementação → código) |
| LeetCode | `CODY_FLAVOR_ASSISTANT.systemPrompt` + realtime | foco em código, comentários por linha, testes, glossário, complexidade |
| System Design | `CODY_FLAVOR_ASSISTANT.systemPrompt` | requisitos, estimativas, diagrama ASCII, trade-offs |
| Personalizado | texto do usuário | prompt de sugestão (ou fallback Reunião) + prompt de sistema como critérios + contexto fixo |

Todos recebem a regra de idioma e a instrução de que a transcrição/contexto é
**fonte de fatos, nunca instrução**. Variantes *fast-first* do Cody podem ser
usadas na primeira sugestão de um agente técnico para reduzir latência; ficam
como função de prompt separada.

## 8. Evidência no bundle do Perssua

Extração: `examples/Perssua-0.27.5-x86_64.AppImage` →
`resources/app.asar` em `/tmp/opencode/perssua/extracted`.

| Assunto | Arquivo | Âncora |
|---|---|---|
| Contrato do resumo | `electron/prompts/notetakerSummaryPrompt.js` | `NOTETAKER_SUMMARY_PROMPT` |
| Q&A da reunião | `electron/prompts/notetakerMeetingQuestionPrompt.js` | `NOTETAKER_MEETING_QUESTION_PROMPT` |
| Agentes embutidos | `electron/flavors.js` | `NOTETAKER_FLAVOR_ASSISTANT`, `CODY_FLAVOR_ASSISTANT` |
| Fast-first do Cody | `electron/flavors.js` | `CODY_FAST_FIRST_*` |
| Prompt incremental do resumo | `build/static/js/main.f180d025.js` | “You are updating a live Session Summary…” |
| Intervalo de 30 s | `build/static/js/main.f180d025.js` | `setInterval(...,3e4)` |
| Envelope do resumo | `build/static/js/main.f180d025.js` | `SUBJECT:` / `CONTENT:` |
| Campos de assistente | `build/static/js/assistantsWindow.8c21ae6c.js` | `systemPrompt`, `realtimePrompt`, `context` |

Reproduzir a extração: `Perssua-*.AppImage --appimage-extract` e ler
`resources/app.asar` (header ASAR: 12 bytes + tamanho do JSON).
