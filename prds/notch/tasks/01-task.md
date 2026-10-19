# 01 — Pacote de ícones SVG + `IconButton` + badges de atalho

- **Status:** CONCLUDED
- **Depende de:** —
- **Habilita:** 02, 03
- **Issues:** ISS-034 · **BR:** BR-032

## Objetivo

Criar a base visual da PRD: um pacote de **ícones SVG próprios** (BR-008), um botão
**só-ícone** acessível e um utilitário **puro** que converte um *accelerator* de
atalho em **badges** (`Ctrl` `Shift` `S`) para exibir ao lado de ações.

## Entregáveis

- `src/components/icons.tsx` — SVGs próprios: `GearIcon`, `TranscriptIcon`,
  `SummaryIcon`, `SparkleIcon`, `HistoryIcon`, `EyeIcon`/`EyeOffIcon`,
  `ChevronDownIcon`, `GripIcon`, `LanguageIcon`.
- `src/components/IconButton.tsx` — botão só-ícone: `title` **e** `aria-label`
  obrigatórios, `data-interactive`, estado `active` (cor de "ligado") e `disabled`.
- `src/components/ShortcutBadges.tsx` — renderiza os badges de um accelerator.
- `src/core/domain/shortcuts.ts` — **funções puras** `parseAccelerator(accel)` e
  `acceleratorBadges(accel)` (normaliza `CommandOrControl`→`Ctrl` no Linux,
  espaços, teclas soltas), com testes.
- `src/components/icons.test.tsx`, `src/components/IconButton.test.tsx`,
  `src/core/domain/shortcuts.test.ts` (casos de badges).

## Critérios de aceite

- [ ] `IconButton` **exige** `aria-label` (tipo TS) e aplica `title`.
- [ ] Clicar/`Enter`/`Espaço` aciona `onClick`; `disabled` bloqueia.
- [ ] `acceleratorBadges("CommandOrControl+Shift+S")` → `["Ctrl","Shift","S"]`.
- [ ] `acceleratorBadges` lida com `Alt`, `Super`/`Meta`, teclas de função e vazio.
- [ ] Nenhuma dependência nova (RNF-05); nenhum asset de terceiros (BR-008).
- [ ] `pnpm test`, `tsc --noEmit` e `pnpm build` verdes.

## Como testar

1. `pnpm test` — testes de `IconButton` e de `acceleratorBadges`.
2. `pnpm storybook`-equivalente: renderizar `IconButton` isolado com hover/foco
   (inspeção visual de foco visível e contraste).

Ponto de parada: revisão visual dos ícones aprovada antes de usar na barra.

## Riscos / rollback

- Ícone ambíguo sem rótulo → `title`/`aria-label` obrigatórios (CA-02.1).
- Mudança de tema clareia/dessatura ícones → usar `currentColor` e herdar do botão.

## Verificação

- Automática: `pnpm test` · `tsc --noEmit` · `pnpm build`.
- Manual: revisão visual dos ícones.
- Observações do usuário: (aguardando)

## Resultado (2026-10-18)

- `components/icons.tsx` (Gear/Transcript/Summary/History/Eye/EyeOff/ChevronDown/
  Grip/Language + `SparkleIcon` reexportado de `TransportIcons`), `IconButton`
  (`aria-label` obrigatório) e `ShortcutBadges`.
- `core/domain/shortcuts.ts`: `parseAccelerator`/`acceleratorBadges` (puros).
- CSS base: `.icon-button`, `.shortcut-badges`, token `--ghostwhisper-primary`.
- Testes: `icons.test.tsx` (4), `IconButton.test.tsx` (5), `shortcuts.test.ts` (+3).
- Verificação: `pnpm test` **155 ✓** · `tsc --noEmit` ✓ · `pnpm build` ✓.
