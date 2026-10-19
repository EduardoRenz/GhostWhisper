# 03 — Painel da sessão em abas (Transcrição | Resumo) e aposentadoria de `summary`

- **Status:** CONCLUDED
- **Depende de:** 01
- **Habilita:** 04 (barra inferior), 05
- **Issues:** — · **BR:** BR-033

## Objetivo

Transformar a janela `chat` no **painel da sessão**, com **abas** — **Transcrição**
(o que hoje é a janela `chat`) e **Resumo** (o que hoje é a janela `summary`) — e
**aposentar a janela `summary`**.

## Entregáveis

### Frontend

- `src/features/panel/PanelTabs.tsx` — pill de abas acessível (`role="tablist"`,
  `aria-selected`, navegação por teclado).
- `src/features/panel/PanelWindow.tsx` — host; decide a aba por `Settings.panel_tab`;
  substitui `src/features/chat/ChatWindow.tsx`.
- `src/features/panel/TranscriptionView.tsx` — conteúdo da janela `chat`
  (medidores + `ChatHeaderActions` + `ChatView` + tradução na bolha).
- `src/features/panel/SummaryView.tsx` — conteúdo da janela `summary`
  (`MarkdownView` + Auto/Atualizar/Copiar + histórico), **sem** `PanelFrame`
  próprio; recebe `active` (aba visível) para o automático.
- `src/features/chat/ChatHeaderActions.tsx` — "Resumo" passa a **trocar de aba**
  (não abre janela); "Sugestões" continua abrindo a janela `suggestions`.
- `src/features/summary/SummaryWindow.tsx` — **removido** (vira `SummaryView`).
- `src/features/chat/ChatWindow.tsx`/`.test.tsx` — **removidos** (viram
  `PanelWindow`/`TranscriptionView`).
- `src/App.tsx` — rota `summary` removida; rota `chat` renderiza `PanelWindow`.

### Núcleo (Rust) + config

- `src-tauri/tauri.conf.json` — janela `summary` **removida**; título da janela
  `chat` → `GhostWhisper — Transcrição`.
- `src-tauri/capabilities/default.json` — sem `"summary"`.
- `src-tauri/src/domain/settings.rs` — `PANEL_LABELS` sem `"summary"`; remover
  `summary_position`/`summary_size`; adicionar `panel_tab`
  (`"transcription" | "summary"`, default `"transcription"`); `sanitized()` valida.
- `src-tauri/src/lib.rs` — boot/tracker/tray iteram `PANEL_LABELS`; o item de tray
  "Abrir resumo" abre a janela `chat` na **aba Resumo**.
- `src/core/domain/settings.ts` + `src/adapters/ipc/panels.ts` + `src/adapters/ipc/settings.ts`
  — espelhar `panel_tab`; `PanelLabel` sem `"summary"`.

## Critérios de aceite

- [ ] A janela `chat` mostra a pill **Transcrição | Resumo**; clicar troca a aba
      sem reiniciar transcrição nem rolagem (CA-03.6).
- [ ] A aba ativa é lembrada entre aberturas/reinícios (`panel_tab`) (CA-03.3).
- [x] Não há botão "Resumo" duplicado no cabeçalho da Transcrição — o resumo é a aba
      ao lado (revisão de 2026-10-19).
- [ ] Nenhuma referência à janela `summary` resta (`tauri.conf.json`,
      `capabilities`, `PANEL_LABELS`, `App.tsx`, `panels.ts`) (CA-03.4).
- [ ] O **automático** do resumo só chama o LLM com a **aba Resumo ativa**;
      "Atualizar agora" funciona em qualquer aba (CA-05.2).
- [ ] Sem chave, a aba Resumo mostra o erro amigável com atalho para Configurações
      (CA-05.4).
- [ ] `Settings` sem `summary_*` carrega configs antigas (serde ignora órfãos) e
      `sanitized()` não quebra.
- [ ] Posição/tamanho da janela `chat` continuam sendo lembrados (`chat_*`).
- [ ] `cargo test`, `cargo clippy`, `cargo fmt --check`, `pnpm test`,
      `tsc --noEmit`, `pnpm build` verdes.

## Como testar

1. Inicie uma sessão e fale; abra a janela pelo ícone de Transcrição.
2. Alternar **Transcrição ⇄ Resumo**: o conteúdo muda, a conversa **não** reinicia
   e a rolagem do resumo é preservada.
3. Confirme que **não existe** botão "Resumo" no cabeçalho da Transcrição (a troca é
   pela aba) nem botão "Sugerir".
4. Feche e reabra o app: a **aba ativa** deve voltar a mesma.
5. Confirme que **não existe** mais janela/menu de "Resumo" separado (o tray aponta
   para a janela `chat` na aba Resumo).
6. Deixe a aba **Transcrição** ativa por um tempo: o resumo **não** deve disparar
   chamadas automáticas; volte à aba Resumo e confirme que retoma.

Ponto de parada: usuário aprova as abas e a remoção da janela `summary`.

## Riscos / rollback

- Perder o resumo ao aposentar `summary` → conteúdo preservado em `SummaryView` +
  `meeting_summaries`/`sessions.overview` (BR-023) intactos.
- Custos do resumo automático → gate pela aba ativa (CA-05.2).
- Config antiga com `summary_position/size` → ignorada pelo serde; documentado.

## Verificação

- Automática: `cargo test` · `clippy` · `fmt` · `pnpm test` · `tsc` · `pnpm build`.
- Manual: os 6 passos acima.
- Observações do usuário: (aguardando)

## Resultado (2026-10-18)

- Painel em abas na janela `chat`: `features/panel/{PanelTabs,PanelWindow,
  TranscriptionView,SummaryView}`; `TranscriptionView` substitui `ChatWindow` e
  `SummaryView` substitui `SummaryWindow` (ambos removidos com seus testes).
- `ChatHeaderActions` passou a ter **só** o atalho **Sugestões** — o botão "Resumo"
  duplicado e o "Sugerir" saíram (revisão de 2026-10-19).
- Janela `summary` **removida**: `tauri.conf.json`, `capabilities`, `App.tsx`
  (rota) e `panels.ts` (`PanelLabel`); `PANEL_LABELS` = `["suggestions","chat",
  "history"]`; `Settings` sem `summary_*` e com `panel_tab` (sanitizado); tray
  "Abrir resumo" abre `chat` e emite `panel_tab`.
- O automático do resumo só chama o LLM com a **aba Resumo ativa** (`opened &&
  active`); as duas vistas ficam montadas (inativa `hidden`).- Testes: `PanelWindow.test.tsx` (3); Rust `panel_tab`/legacy/acessores de layout.
- **Feedback do resumo (2026-10-19):** pill "Gerando resumo completo…" durante a
  chamada e "Processando o restante do resumo…" com `hasMore`; o `subject` aparece
  como título acima do Markdown (`SummaryView.test.tsx`, 3 casos).
- Verificação: `cargo test` **334 ✓** (8 ignored) · clippy/fmt ✓ · `pnpm test` 155 ✓.

> Nota: `PanelFrame.test.tsx` passou a usar `label="chat"` (o label `summary` não
> existe mais).
