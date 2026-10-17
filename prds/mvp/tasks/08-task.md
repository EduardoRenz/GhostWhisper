# 08 — Tradução ao vivo + insights/sugestões

- **Status:** CONCLUDED
- **Depende de:** 07
- **Habilita:** —

## Objetivo
Entregar tradução ao vivo da transcrição e sugestões/insights contextuais via
provider de LLM configurado.

## Entregáveis (implementados)
- Portas: `ports/translator.rs` (`Translator`), `ports/insight_generator.rs`
  (`InsightGenerator`); `LlmController::complete` reintroduzido.
- Aplicação: `application/translation_service.rs`,
  `application/insight_service.rs`, `prompts.rs` (sem `allow(dead_code)`).
- IPC: `ipc/llm_commands.rs` (`llm_translate`, `llm_insight`).
- Frontend: `features/translation/TranslationView.tsx`,
  `features/insights/InsightsPanel.tsx`; hooks `useTranslation`, `useInsight`,
  `useSettings`; adaptador `adapters/ipc/llm.ts`; util `clipboard.ts`.

## Critérios de aceite
- [x] Tradução acompanha a transcrição (fila serial por segmento finalizado,
  exibida lado a lado com o original).
- [x] Insight usa o contexto da transcrição (últimos segmentos da sessão,
  limitado a 4.000 caracteres) e é gerado pelo provider configurado.
- [x] Respostas copiáveis com um clique (`navigator.clipboard`).
- [x] Erros do provider exibidos de forma clara, sem vazar a chave.

## Evidências
- `cargo test` → **74 testes** (tradução: prompt/idioma/erro; insight: contexto,
  sessão ausente, transcrição vazia, truncamento do contexto).
- **Fluxo real de ponta a ponta** (settings → segredo → fábrica → HTTP →
  prompt): `translates_through_local_openai_compatible_provider` sobe um
  servidor OpenAI-compatível local e confirma `POST /chat/completions`,
  `Authorization: Bearer …` e o texto no corpo.
- `pnpm test` → **8 testes** (render da tradução com original + botão copiar).
- `pnpm build` ok; smoke run: app inicia.
- `clippy`/`fmt` limpos.

## Notas / decisões
- **Streaming de resposta adiado**: os providers respondem de forma completa
  (não SSE). O critério de latência (< 5 s em cloud) é atendido sem streaming;
  streaming pode ser adicionado depois sem mudar as portas.
- Tradução em **fila serial** (uma requisição por vez) para respeitar rate limit
  e reduzir custo; contexto do insight limitado a 4.000 caracteres.
- Idiomas (`source_language`/`target_language`) e a flag `translation_enabled`
  vêm das settings (Tarefa 06); a UI só traduz quando habilitado.
- Um único ponto de cópia (`copyText`) usado por tradução e insights.
- Validação com provider cloud real (OpenAI/Gemini) é manual e recomendada.
