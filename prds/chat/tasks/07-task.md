# 07 — Janela de Chat própria (movível e redimensionável)

- **Status:** CONCLUDED
- **Depende de:** 01
- **Habilita:** 08

## Objetivo

Tirar a conversa ao vivo do painel do overlay e colocá-la numa **janela própria**,
no mesmo molde de `summary`/`suggestions` (sem moldura, sempre no topo, arrastável,
redimensionável, com posição e tamanho lembrados). O overlay deixa de expandir: um
botão **Chat** esconde/mostra a janela.

O Perssua mantém a conversa dentro da própria superfície (modal/Notch) — este é o
mesmo desvio consciente já registrado para as janelas de assistente
(BR-026, BR-015/R12). Os medidores de nível migram para a janela de Chat.

## Entregáveis

**Backend**

- `src-tauri/tauri.conf.json`: nova janela `chat` — `resizable: true`,
  `minWidth: 360`, `minHeight: 320`, `width: 560`, `height: 720`,
  `decorations: false`, `transparent: true`, `alwaysOnTop: true`,
  `skipTaskbar: true`, `shadow: false`, `focus: false`, `visible: false`,
  `center: true` (mesmo padrão de `summary`/`suggestions`).
- `src-tauri/capabilities/default.json`: `chat` na lista `windows` (a permissão
  `core:window:allow-start-resize-dragging` já existe).
- `src-tauri/src/ipc/panel_commands.rs`: `PANEL_LABELS` →
  `["summary", "suggestions", "chat"]`.
- `src-tauri/src/domain/settings.rs`: `chat_position` / `chat_size` (padrão
  `None`), clamp no mínimo (`MIN_PANEL_WIDTH`/`MIN_PANEL_HEIGHT`) e testes
  (round-trip, clamp, Settings legado).
- `src-tauri/src/lib.rs`: o boot e o tracker de layout passam a **iterar os
  rótulos** de painel (incluindo `chat`) em vez do `match` fixo de dois labels.

**Frontend**

- `src/adapters/ipc/panels.ts`: `PanelLabel` ganha `"chat"`.
- `src/App.tsx`: rota `chat` (+ `?window=chat` no `vite dev`).
- Novo `src/features/chat/ChatWindow.tsx`: `PanelFrame` + `usePanelOpen("chat")` +
  `useSessionState` + `useLiveTranscription` + `useInsight`/`useInsightFeed` +
  `useTranslation`/`useSettings` + `LevelMeters` + `ChatView`.
- Novo `src/features/chat/ChatHeaderActions.tsx`: o cabeçalho (Sugerir / Resumo /
  Sugestões) sai do `Overlay` e passa a ser compartilhado.
- `src/features/overlay/Overlay.tsx`: botão **Chat** (toggle via `usePanelOpen`);
  saem o `.overlay__panel`, as abas Conversa/Histórico, o `HistoryPanel`, os
  `LevelMeters`, `showLive` e a superfície `expanded-tall`; `Surface` vira
  `"actionbar" | "hidden"`.
- `src/adapters/ipc/overlay.ts`, `src-tauri/src/ipc/overlay.rs`,
  `src-tauri/src/adapters/overlay/tauri_overlay_window.rs` e a geometria: removem
  o caso `expanded-tall`.
- `src/styles/globals.css`: limpeza das regras do painel expandido; o que sobra
  vira o corpo da janela de Chat.

## Critérios de aceite

- [ ] A conversa ao vivo (bolhas finalizadas, fala em andamento e sugestões)
      roda na janela `chat`, com a mesma linha temporal da 01–03.
- [ ] O botão **Chat** alterna esconder/mostrar a janela; o ✕ do cabeçalho esconde
      (não destrói) e reabrir mantém o histórico da sessão.
- [ ] A janela é arrastável pelo cabeçalho e redimensionável por bordas/cantos
      (mínimo 360×320); **posição e tamanho voltam** ao reabrir o app.
- [ ] Os medidores de nível ficam na janela de Chat, ligados aos canais da sessão.
- [ ] O cabeçalho abre Resumo e Sugestões como hoje; **Sugerir** pede a sugestão.
- [ ] O overlay **não expande mais**; `Surface` só `actionbar`/`hidden` e o
      click-through segue preservado (RNF-02).
- [ ] `cargo test`/`clippy`/`fmt` e `pnpm test`/`tsc`/`build` limpos.

## Verificação

- Frontend: `ChatWindow.test.tsx` (bolhas + pendentes + toggle de painel),
  `App.test.tsx` (rota `chat`), `Overlay.test.tsx` (sem painel; botão Chat).
- Rust: `Settings` (clamp do `chat_size`), boot/tracker.

## Notas

- Wayland nativo: o resize por alça depende do compositor e pode não funcionar; em
  X11/XWayland funciona (BR-007).

## Resultado (2026-10-18)

- Janela `chat` declarada no `tauri.conf.json` e liberada em `capabilities`
  (além de `chat` no `PANEL_LABELS`, agora com os quatro painéis).
- `Settings.chat_position`/`chat_size` (Rust + TS) com clamp; boot e tracker
  passaram a iterar `domain::settings::PANEL_LABELS` (menos duplicação).
- `ChatWindow` + `ChatHeaderActions`; `App` roteia `chat` (e `?window=chat`).
- Overlay reduzido à ActionBar: `Surface` = `actionbar | hidden` (Rust e TS);
  saíram o painel expansível, as abas, os medidores do overlay e o `expanded-tall`.
- Testes: `ChatWindow.test.tsx` (2), `App.test.tsx` (rotas `chat`/`history`),
  `Settings` (acessores por rótulo + clamp). **331 Rust** · **138 frontend**.
