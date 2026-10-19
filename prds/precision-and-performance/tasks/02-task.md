# 02 — Bolha compacta e sugestão fast-first · Entregável E2

- **Status:** CONCLUDED
- **Depende de:** 01 (ciclo), 08/17/22 do MVP e PRD assistant (sugestões, chat)
- **Issues:** ISS-023 · **BR nova:** BR-028
- **Refs:** BR-015, BR-023, BR-025, BR-026,
  [`../architecture/precision-and-performance.md`](../architecture/precision-and-performance.md) §3/§5

## Objetivo

Fazer a sugestão **aparecer rápida e curta** (≈3 linhas) como no Perssua, e
funcionar mesmo com a janela de sugestões fechada — o que o usuário percebe como
"mais rápido".

## Entregáveis

- `src-tauri/src/application/prompts/assistant.rs` — variantes **fast-first** dos
  contratos (curtas, "be correct over complete"), separadas dos contratos completos.
- `src-tauri/src/ports/llm_controller.rs` — caminho de **streaming** com callback
  de delta, com implementação padrão não-streaming (compatível com os adaptadores
  atuais).
- `src-tauri/src/adapters/ai/openai_compatible.rs` (+ `gemini_chat_bot_provider.rs`)
  — SSE quando suportado; fallback transparente para resposta única.
- `src-tauri/src/application/suggestion_service.rs` — dois passes: fast-first
  (`max_tokens` 300–420, temperatura 0) e completo; `supersede` de requisições
  antigas; `SuggestionLimits` inalterados no passe completo.
- `src-tauri/src/application/context.rs` — novo contexto que **inclui prévias**
  (falas em andamento, marcadas), sem quebrar `recent_context` (usado pelo que já
  existe).
- `src-tauri/src/application/suggestion_scheduler.rs` (novo) — dispara sugestão
  automática por fala nova, com debounce e no máximo 1 requisição em voo; toggle e
  intervalo vêm de `Settings`.
- `src-tauri/src/ipc/llm_commands.rs` + `ports/event_publisher.rs` — eventos
  `suggestion_delta` (streaming) e `insight_draft` (fast-first efêmero).
- `src/core/application/useSuggestions.ts` + `useInsightFeed.ts` — consomem os
  deltas; `src/features/suggestions/SuggestionsWindow.tsx` mostra o rascunho.
- `src/features/chat/ChatBubble.tsx` + `src/styles/globals.css` — cue colapsado em
  1–3 linhas (clique expande) e seções colapsáveis.
- `prds/business-rules.md` — **BR-028** (fast-first + streaming + auto contínuo).

## Critérios de aceite

- [x] Primeira sugestão (fast-first) é publicada antes do texto completo
      (contrato compacto, teto de 420 tokens, temperatura 0).
- [x] Deltas de streaming chegam à UI e o texto cresce progressivamente; provider
      sem `text/event-stream` cai no caminho não-streaming sem erro visível.
- [x] Sugestão automática roda **sem a janela aberta**, a cada fala nova, com
      debounce (700 ms) e teto de frequência (mín. 3 s entre chamadas).
- [x] O contexto inclui a fala em andamento (prévia) e a transcrição continua
      tratada como **dado**, nunca instrução.
- [x] A bolha da IA no chat começa colapsada em ~3 linhas, com "mostrar mais".
- [x] Sugestão automática continua **efêmera**; só **Fixar** entra no export.
- [x] Toggle **Auto** persiste em `Settings.suggestion_auto_refresh` e desliga o
      automático do core; pausar a sessão para as chamadas (sem fala nova ⇒ sem
      chamada).
- [x] `pnpm test` (121 ✓), `tsc --noEmit` ✓, `pnpm build` ✓, `cargo test`
      (315 ✓), `clippy -D warnings` ✓, `fmt --check` ✓.
- [ ] Teste manual do usuário: `t1` da primeira bolha, nº de linhas e automático
      com a janela fechada.

## Como testar

Automatizado:

```bash
cargo test --lib
pnpm test && pnpm exec tsc --noEmit && pnpm build
```

Manual (sessão real, com chave de provider configurada):

1. Manter a janela de **Sugestões fechada** e iniciar uma conversa/vídeo.
2. Medir `t1` = tempo entre a fala do interlocutor e a primeira bolha de sugestão;
   comparar com o Perssua (alvo < 2–3 s).
3. Contar as linhas da bolha com o painel no tamanho padrão (alvo ≤ 3) e expandir
   para ler o restante.
4. Falar sem parar por ~1 min e verificar que o texto aparece gradualmente e que
   não há fila de requisições (logs sem chamadas sobrepostas).
5. Confirmar que nada apareceu no export sem clicar em **Fixar**.

Ponto de parada: usuário aprova a sensação de velocidade/tamanho antes de E3.

## Riscos / rollback

- Streaming aumenta complexidade do IPC → manter fallback e evento único de
  "sugestão pronta".
- Auto contínuo eleva custo → fast-first curto + supersede + teto de 1 em voo +
  toggle; medir número de chamadas por minuto antes/depois.
- Colapso esconder informação útil → expandir a um clique e lembrar a preferência.

## Verificação

- **Automática:** `cargo test --lib` 315 ✓ (novos: fast-first antes do completo,
  streaming como deltas acumulados, SSE com fallback, agendador puro, contexto
  ao vivo, `latest_final_ms`); `pnpm test` 121 ✓ (novos: evento de sugestão com
  rascunho+completo no mesmo `at_ms`, ignore de outra sessão); `clippy`, `fmt`,
  `tsc`, `pnpm build` ✓.
- **Manual:** o usuário seguiu para a próxima etapa (E3) após validar o
  comportamento geral — fast-first curto, automático com a janela fechada e bolha
  colapsada. Ajustes de UX de tradução que apareceram no teste foram para o PRD
  [`translation`](../../translation/prd.md) (BR-030).
- **Observações do usuário:** "pode seguir para próxima etapa".
