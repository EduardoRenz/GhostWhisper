# 07 — Histórico de versões do resumo (em memória)

- **Status:** CONCLUDED
- **Depende de:** 03
- **Habilita:** —

## Objetivo

Guardar as versões do resumo geradas durante o uso da janela e permitir voltar a
uma delas, no mesmo estilo do histórico de sugestões: hora + título do tópico.

## Entregáveis

- `useSummary`: `history` (mais recente primeiro, teto `SUMMARY_HISTORY_LIMIT = 20`),
  `viewing`, `select`, `showLatest`; dedupe por `updated_at_ms` + conteúdo e
  **sem empilhar** ticks sem novidade (`updated = false`).
- `SummaryWindow`: subtítulo com o tópico (`summaryTopic`); lista “Resumos
  anteriores (N)” com hora (`HH:MM:SS`), título e “cobre até `mm:ss`”; versão em
  leitura destacada; botão **Voltar ao atual**; **Copiar** copia a versão exibida.
- `summaryTopic` em `core/domain/assistant.ts` (subject → primeiro heading →
  rótulo neutro).
- Estilos do item ativo/meta em `src/styles/globals.css`.

## Critérios de aceite

- [x] Lista as versões anteriores com hora e título do tópico.
- [x] Clicar numa versão **congela** a leitura; novas atualizações continuam
      entrando na lista sem trocar o que está na tela.
- [x] “Voltar ao atual” retorna ao resumo ao vivo.
- [x] Histórico limitado a 20 e limpo ao trocar de sessão.
- [x] Tick automático sem conteúdo novo (`updated = false`) não cria versão.
- [x] Testes de `useSummary`, `SummaryWindow` e `summaryTopic`; `pnpm test`,
      `tsc --noEmit` e `pnpm build` verdes.

## Notas

- Histórico **em memória**: some ao fechar o app (como o das sugestões). A versão
  persistida continua sendo a atual (`meeting_summaries`), espelhada em
  `sessions.overview`.
- Sem mudança de backend/schema.
