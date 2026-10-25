# 17 — Casco padrão das janelas: header, arraste e raio

- **Status:** CONCLUDED
- **Depende de:** 10, 16; redesign das Configurações (ai-sources T07/T12)
- **Habilita:** novas janelas com o mesmo casco
- **Issues:** — · **BR:** BR-040, BR-043, BR-064

## Objetivo

Padronizar o cabeçalho de **todas as janelas internas** conforme o mockup de
2026-10-21: grip de arraste no canto superior esquerdo, selo de ícone + título
(e subtítulo), ações e fechar no canto direito. As janelas de Configurações
deixam a moldura nativa do SO e usam o mesmo casco dos painéis. O raio externo
passa a ser o dos balões de conversa (`--oh-radius-sm`, 8 px), com um token
próprio (`--oh-window-radius`).

## Entregáveis

- **`WindowHeader`** (`src/components/WindowHeader.tsx`):
  - ordem fixa: `DragHandle` → selo de ícone 32 px → título/subtítulo →
    `actions`/`hint` → fechar;
  - usa `CloseIcon` e mantém os `aria-label`s;
  - `headingLevel` permite `h1` em Configurações e `h2` nos painéis.
- **`PanelFrame`** usa o `WindowHeader`; `title`/`subtitle`/`icon`/`titleExtra`
  substituem o `heading` livre. Chat (ícone por aba), Sugestões e Histórico
  passam seus ícones e o badge LIVE vai para `titleExtra`.
- **Configurações**: shell `panel-shell settings-shell` + `ResizeHandles` +
  `.settings` com borda/raio/sombra; `WindowHeader` com grip, `Esc` e fechar;
  `panel_start_drag` no grip e `settings_close` no fechar.
- **`tauri.conf.json`**: janela `settings` com `decorations: false`,
  `transparent: true` e `shadow: false`.
- **CSS**:
  - `--oh-window-radius: var(--oh-radius-sm)` em `tokens.css`;
  - `.window-header` (padding 16/8 px, min-height 56 px), título com ellipsis e
    ações com `flex-wrap`;
  - `.panel` e `.settings` usando o token; `.app` também;
  - em painéis estreitos, as ações quebram e o rótulo “Auto-sugerir” some
    (o anel + contagem permanecem) para o título não truncar.
- Sem mudança nas densidades do HUD (`compact`/`standard`/`expanded`).

## Critérios de aceite

- [x] Chat, Sugestões, Histórico e Configurações têm o mesmo header.
- [x] Grip no canto superior esquerdo em todas; fechar no canto direito.
- [x] Título com margem mínima de 16 px e sem colar na borda.
- [x] Configurações sem moldura nativa, arrastável e redimensionável.
- [x] Raio externo padrão de 8 px via `--oh-window-radius`.
- [x] Fechar/arrastar e resize preservados; `pnpm test`, `tsc`, `build`.

## Como testar

1. Abra Chat, Sugestões, Histórico e Configurações.
2. Confirme o grip à esquerda e o `×` à direita; arraste cada janela pelo grip.
3. Redimensione Chat/Sugestões/Histórico pelas bordas e Configurações pelas
   alças do shell.
4. Confira os cantos menos arredondados e o título com respiro.
5. Em Configurações, confirme que não há mais a moldura do SO.

## Verificação (2026-10-21)

- `pnpm test` **359 ✓** (57 arquivos) · `tsc --noEmit` ✓ · `pnpm build` ✓.
- `cargo test --lib` **545 ✓** (9 ignorados) · `clippy`/`rustfmt` ✓.
- Validação visual no navegador (`?window=...`): header padrão, raio de 8 px,
  margem do título e quebra das ações em 1020/560/540/520/360 px.
- AppImage/deb gerados (09:53). A conferência visual **dentro do AppImage**
  nesta sessão ficou bloqueada por uma falha de pintura do WebKitGTK que afeta
  também o build anterior e páginas carregadas direto no MiniBrowser (o DOM e o
  bundle foram validados); a configuração da janela sem moldura está no
  `tauri.conf.json` e o shell/resize no código.

## Revisão (2026-10-24) — arraste pelo header inteiro

- O `WindowHeader` passou a iniciar o arraste em **toda área inerte do header**
  (grip, selo, título, subtítulo, espaços e dica `Esc`); o grip continua
  visível como indicação, mas não é mais o único ponto de arraste.
- Controles (botões, abas, links, campos, `role=...`, `contenteditable` e
  `data-no-window-drag`) preservam o clique; só o botão primário arrasta.
- Regra isolada em `shouldStartHeaderDrag`
  (`src/components/windowHeaderDrag.ts`), com testes de unidade e de componente.
- O critério do grip no canto esquerdo segue válido como affordance visual; o
  arraste agora não depende dele.
- **ISS-063:** o primeiro clique em janela inativa só a ativava; arraste e
  resize agora partem do **`mousedown`** e o core usa
  `gtk_window_begin_move_drag`/`begin_resize_drag` com timestamp X11 válido
  (`window_commands.rs`), em vez do `start_dragging` com timestamp zero. O
  tooltip nativo do grip/alças foi removido. Isso substitui o antigo
  `panel_start_drag`.
