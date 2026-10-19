# 10 — Casca "Transcrição ao Vivo": cabeçalho, abas e rodapé

- **Status:** PENDING
- **Depende de:** 09
- **Habilita:** 11
- **Issues:** — · **BR:** BR-040
- **Refs:** [`../prd.md`](../prd.md) §9, [`../architecture/transcription-panel.md`](../architecture/transcription-panel.md) §2

## Objetivo

Aproximar a **casca** do painel da sessão do print de referência anexado pelo
usuário (2026-10-19), sem mudar o comportamento das abas nem o transporte:

- cabeçalho com **ícone de waveform + "Transcrição ao Vivo"**;
- *pill* de abas **Transcrição | Resumo** ocupando a largura toda, com a aba ativa
  em contorno **cyan** (não em fundo preenchido);
- rodapé com **"Idioma de Entrada:"** à esquerda e o **estado real** do motor à
  direita.

Para ficar fiel ao print, a aba Transcrição **não** mostra a linha de medidores de
nível nem o botão de texto "Sugestões" no topo da conversa: o notch já tem o
**waveform** e o acesso às Sugestões. Os medidores continuam existindo como
componente (e no notch), mas saem desta aba.

O layout do print é a fonte desta revisão; a tabela antiga de
[`../architecture/chat-ui.md`](../architecture/chat-ui.md) §3 continua como
evidência histórica da paridade Perssua 0.27.5.

## Entregáveis

- `src/features/panel/PanelWindow.tsx`:
  - cabeçalho próprio (`heading`) com um **`WaveformIcon` estático** (novo SVG do
    projeto em `components/icons.tsx`) em um selo de accent + título
    **"Transcrição ao Vivo"** na aba `transcription`;
  - aba `summary` mantém o título **"Resumo"**;
  - o subtítulo `id · status` sai do cabeçalho (o estado continua no notch e no
    rodapé da Transcrição).
- `src/features/panel/PanelTabs.tsx`:
  - container **100%** da largura, duas colunas iguais;
  - aba ativa com `border`/texto `--oh-accent` e fundo sutil (sem preencher com
    `surface-container-highest`), como no print;
  - `role="tablist"`, teclado e `aria-selected` preservados.
- `src/features/panel/LanguageBar.tsx`:
  - rótulo **"Idioma de Entrada:"** antes do seletor (o `<select>` continua o
    mesmo, com as opções de `SOURCE_LANGUAGE_LABELS`);
  - à direita, **"Whisper {modelo} · {Gravando|Pausado|Sem sessão}"** — dados
    reais de `settings.whisper_model` e `useSessionState`;
  - **não** exibir "Latência: 48ms · WebRTC": não medimos latência e o produto
    não usa WebRTC. O espaço equivalente recebe o estado real (desvio R12).
- `src/features/panel/TranscriptionView.tsx`:
  - sai o `<LevelMeters>` do topo (o waveform do notch continua);
  - sai o `header={<ChatHeaderActions />}`; o `ChatHeaderActions.tsx` pode ser
    removido se ficar sem uso (o atalho Sugestões vive no notch).
- `src/features/panels/PanelFrame.tsx`:
  - no painel `chat`, o grip visível `⋮⋮` some; o arraste continua pelo cabeçalho
    (título/ícone) e o `panel_start_drag` não muda.
- `src/styles/globals.css` / `src/styles/tokens.css`:
  - ajustes de `.panel__header`, `.panel-tabs`, `.language-bar` e do selo do
    título; apenas tokens `--oh-*` (BR-035);
  - nenhum `backdrop-filter` novo.
- Testes: `PanelTabs.test.tsx` (largura/ativo/teclado), `PanelWindow.test.tsx`
  (título e ausência do subtítulo), `LanguageBar.test.tsx` (rótulo, status e erro
  de troca de idioma).

## Critérios de aceite

- [ ] A aba Transcrição mostra o cabeçalho **"Transcrição ao Vivo"** com ícone; a
      aba Resumo mostra **"Resumo"**.
- [ ] As abas ocupam a largura disponível, com a ativa em contorno cyan e a
      inativa mutada; trocar de aba não reinicia transcrição/resumo (BR-033).
- [ ] O rodapé mostra **"Idioma de Entrada:"** + seletor e, à direita, o estado
      real do Whisper — sem número de latência inventado e sem "WebRTC".
- [ ] A aba Transcrição não mostra a linha de medidores nem o botão "Sugestões";
      o header arrasta pelo título/ícone, sem o grip visível.
- [ ] Em 360 px de largura nada estoura; foco visível e navegação por teclado.
- [ ] `pnpm test`, `tsc --noEmit` e `pnpm build` verdes.

## Verificação

- Abrir o painel em `?window=chat`, conferir as duas abas e o rodapé.
- Trocar o idioma em uma sessão ativa: o **próximo** passe usa o novo idioma
  (BR-034) e o estado do rodapé continua correto.
- Redimensionar até o mínimo de 360×320 e conferir a pill de abas.

## Notas

- O ícone do cabeçalho é SVG do projeto (BR-008); não copiar o asset do print.
- O selo/ícone do print é só referência de **hierarquia**; a geometria segue os
  tokens do design system.
