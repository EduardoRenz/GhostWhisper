# 07 — Botão de câmera no notch

- **Status:** CONCLUDED
- **Depende de:** 06
- **Habilita:** 08
- **Issues:** — · **BR:** BR-065, BR-066
- **Refs:** [`../prd.md`](../prd.md) US-01 (CA-01.1…CA-01.9);
  [`../architecture/recording.md`](../architecture/recording.md) §6

## Objetivo

Dar ao usuário o controle da gravação **dentro da sessão**, com um botão
discreto ao lado de Sugestões, cujo estado reflete o backend.

## Entregáveis

- `src/features/overlay/RecordingButton.tsx`:
  - 4 estados (`off`, `recording`, `paused`, `unavailable`);
  - `aria-pressed`, `aria-label` e `title` contextuais;
  - `data-interactive`;
  - pulso do ponto `REC` com `prefers-reduced-motion`;
  - ícone de câmera desenhado no `components/icons.tsx` (BR-008, sem asset de
    terceiros).
- `src/features/overlay/NotchTabs.tsx`: novo item **depois de Sugestões**, com
  divisor; só renderiza com `listening` (CA-01.2).
- `src/features/overlay/ActionBar.tsx`: repassa `recording` + `onToggleRecording`.
- `src/core/application/useRecording.ts`: assina `recording_state` e expõe
  `toggle`, `status`, `targetLabel`.
- `src/core/domain/shortcuts.ts` + `domain/shortcuts.rs`: ação
  `recording_toggle` (**Ctrl+Shift+R**), com badge no *tooltip*.
- `src/styles/*`: tokens de perigo (`--oh-danger`) para o vermelho, sem cor
  hardcoded.

## Critérios de aceite

- [ ] O botão é o **último** item da pílula, depois de Sugestões, como no mockup.
- [ ] Gravando: vermelho + ponto pulsante; pausado: vermelho vazado + *play*;
      desligado: neutro.
- [ ] Clique alterna iniciar → pausar → retomar; o estado vem do evento, não de
      `useState`.
- [ ] Clique **secundário** (botão direito / *chevron*) abre o seletor de
      janelas (task 03) antes de iniciar; o principal usa a janela ativa.
- [ ] Sem sessão, o botão não existe.
- [ ] `Ctrl+Shift+R` dispara a mesma ação e aparece no catálogo de atalhos.
- [ ] O clique não pausa a sessão nem abre o menu de canais.
- [ ] `pnpm test` cobre os 4 estados e o clique.

## Verificação

- `pnpm test` · `pnpm build` · teste manual com overlay em X11 (click-through
  fora do botão)

## Notas

- O mockup mostra a câmera dentro da pílula, à direita de "Sugestões"; manter o
  espaçamento `--oh-space-*` da barra.
- Durante `finalizing`, o botão fica desabilitado (CA-01.6).
