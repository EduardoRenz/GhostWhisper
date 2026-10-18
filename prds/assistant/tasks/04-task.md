# 04 — Janela de sugestões, agentes e “Fixar”

- **Status:** CONCLUDED
- **Depende de:** 02, 03
- **Habilita:** 05

## Objetivo

Entregar a janela de sugestões com seleção de agentes, atualização automática,
contexto extra e fixação na conversa.

## Entregáveis

- Rota `suggestions` em `App.tsx` / `resolveRoute` (`?window=suggestions`).
- `tauri.conf.json` + capabilities + `suggestions_open`.
- `adapters/ipc/suggestions.ts` (`assistantModesList`, `suggestionGenerate`,
  `suggestionPin`).
- `useAssistantModes`: lista embutidos/personalizados, seleção persistida.
- `useSuggestions(sessionId, modeId)`: gera no open e ao trocar de agente;
  intervalo de 20 s; `since_ms`/`context_until_ms`; `skipped`; uma requisição em
  voo; histórico curto em memória; `fixar`.
- `features/suggestions/SuggestionsWindow.tsx`: seletor de agente, automático,
  “Sugerir agora”, “Fixar”, “Copiar”, lista das últimas sugestões, Markdown,
  caixa “Contexto extra”.
- `ChatBubble` renderiza Markdown quando a bolha é de sugestão da IA.
- Botões **Sugestões** no overlay e no tray.
- Evento `insight` carrega `mode_id`; o chat usa o agente selecionado.

## Critérios de aceite

- [x] A janela abre, lista agentes e gera a primeira sugestão.
- [x] Trocar de agente gera nova sugestão; a escolha persiste.
- [x] Automático só chama o LLM com fala nova; “Sugerir agora” força.
- [x] LeetCode devolve Big O + código colorido a partir de pergunta falada.
- [x] “Fixar” grava a sugestão no chat e no export; automáticas não aparecem.
- [x] As bolhas da IA renderizam listas/código.
- [x] Testes de hook, janela e Markdown; `pnpm test`/`tsc`/`build` verdes.
