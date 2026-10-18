# 03 — Markdown compartilhado e janela de resumo

- **Status:** CONCLUDED
- **Depende de:** 02
- **Habilita:** 04, 05

## Objetivo

Criar o renderer de Markdown seguro e a primeira janela separada, com atualização
automática do resumo.

## Entregáveis

- Dependências: `react-markdown`, `remark-gfm`, `rehype-highlight`,
  `highlight.js`.
- `components/MarkdownView.tsx`:
  - headings, listas, tabelas GFM, blockquotes, código inline;
  - blocos cercados com linguagem, highlight.js e botão **Copiar**;
  - sem HTML bruto; links externos via `tauri-plugin-opener`;
  - `memo` e variante compacta para bolhas.
- Rotas `summary` em `App.tsx` / `resolveRoute` (incluindo `?window=summary`).
- `tauri.conf.json`: janela `summary` sem moldura, transparente, always-on-top,
  skip-taskbar, `visible: false`; capabilities.
- IPC `summary_open`, `panel_hide`, `panel_start_drag`; eventos
  `panel_opened`/`panel_hidden`; fechar esconde.
- `adapters/ipc/summary.ts`, `adapters/ipc/panels.ts`.
- `useSummary(sessionId)`: carrega o gravado, gera no open, intervalo de 30 s,
  uma requisição em voo, `has_more` → tick curto, cancela em troca de sessão.
- `features/summary/SummaryWindow.tsx`: cabeçalho arrastável, status, “Atualizar
  agora”, automático on/off, “Copiar”, Markdown, estados vazio/erro/sem chave.
- Botões **Resumo** no overlay e no tray.
- Estilos de Markdown no tema escuro.

## Critérios de aceite

- [x] Abrir pelo overlay/tray mostra a janela e o resumo da sessão.
- [x] Atualiza a cada 30 s apenas com fala nova; esconder para o automático.
- [x] Código aparece colorido e o botão copia o bloco.
- [x] Tabelas/listas/headings são renderizados; HTML bruto não executa.
- [x] A janela arrasta e lembra a posição.
- [x] Testes de Markdown, hook e janela; `pnpm test`/`tsc`/`build` verdes.
