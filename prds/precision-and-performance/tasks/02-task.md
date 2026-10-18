# 02 — Bolha compacta e sugestão fast-first · Entregável E2

- **Status:** PENDING
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

- [ ] Primeira sugestão (fast-first) é publicada antes do texto completo e cabe em
      ~3 linhas no painel padrão.
- [ ] Deltas de streaming chegam à UI e o texto cresce progressivamente; provider
      sem suporte cai no caminho não-streaming sem erro visível.
- [ ] Sua sugestão automática roda com a janela **fechada**, a cada fala nova,
      com debounce; nunca há duas requisições em voo (a antiga é superada).
- [ ] O contexto inclui a fala em andamento (prévia) e continua tratando a
      transcrição como **dado**, nunca instrução.
- [ ] O cue no chat é colapsado em 1–3 linhas, com expandir/recolher acessível.
- [ ] Sugestão automática continua **efêmera**; só **Fixar** entra no export
      (BR-023).
- [ ] Toggle e intervalo nas configurações; pausar a sessão para as chamadas.
- [ ] `pnpm test`, `tsc --noEmit`, `pnpm build`, `cargo test`, `clippy` ✓.

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

## Verificação (preencher ao concluir)

- `t1` antes/depois: …
- Linhas da bolha: …
- Chamadas por minuto: …
- Observações do usuário: …
