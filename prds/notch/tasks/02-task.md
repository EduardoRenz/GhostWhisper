# 02 — Notch em dois estados (repouso/ativo) por ícones

- **Status:** CONCLUDED
- **Depende de:** 01
- **Habilita:** 05
- **Issues:** ISS-034 · **BR:** BR-032

## Objetivo

Reformular a `ActionBar` em **dois estados** — **repouso** (sem sessão) e **ativo**
(com sessão) — com todos os controles **por ícone** e a ação primária
**"Começar a ouvir"** em **verde** com o **atalho em badges**.

## Entregáveis

- `src/features/overlay/ActionBar.tsx` — estado derivado de `listening`:
  - **Repouso:** alça · marca · **engrenagem** · **"Começar a ouvir"** (verde +
    `ShortcutBadges` do atalho `toggle_session`) · ocultar.
  - **Ativo:** alça · marca · status (bolinha + texto) · transporte · canais ·
    pílula de tempo · ícones Transcrição/Sugestões/Histórico · stealth (olho) ·
    engrenagem · ocultar.
- `src/features/overlay/Overlay.tsx` — passa os toggles por ícone; mantém
  `usePanelOpen("chat"|"history")`; o mesmo handler de Configurações.
- `src/features/session/SessionControls.tsx` — ação parada com rótulo
  **"Começar a ouvir"** + badges do atalho.
- `src/styles/globals.css` — token `--ghostwhisper-primary` (verde), estilos do
  `IconButton` e do estado verde da ação; remover estilos de botão de texto que
  saírem de uso.
- `src/features/overlay/ActionBar.test.tsx` — repouso × ativo; "não há seletor de
  canais em repouso"; engrenagem abre Configurações; badges presentes.

## Critérios de aceite

- [ ] Sem sessão, a barra exibe **só** os controles da CA-01.1.
- [ ] Com sessão, além dos controles da sessão, aparecem os ícones de janela,
      stealth, engrenagem e ocultar (CA-01.3).
- [ ] "Começar a ouvir" é **verde**, inicia a captura e mostra o atalho atual em
      badges; mudar o atalho nas Configurações muda os badges (CA-07.2/07.3).
- [ ] Toggle de janela aberta aparece destacado (CA-02.3).
- [ ] Todo botão tem `title` + `aria-label`; foco visível (RNF-04).
- [ ] Nenhum `audio_level` assinado na barra (RNF-02).
- [ ] `pnpm test`, `tsc --noEmit`, `pnpm build` verdes.

## Como testar

1. Abra o app **sem** sessão: o notch deve mostrar marca, engrenagem, "Começar a
   ouvir" (verde + `Ctrl Shift S`) e ocultar — nada de canais/transporte/relógio.
2. Clique em "Começar a ouvir": a barra muda para o **estado ativo**.
3. Passe o mouse em cada ícone: o `title` deve explicar a ação; `Tab` deve mostrar
   foco visível.
4. Abra/fecha as janelas: o ícone correspondente acende.
5. Troque o atalho de sessão nas Configurações: os badges devem refletir o novo.

Ponto de parada: usuário aprova o notch nos dois estados.

## Riscos / rollback

- Perder acesso a uma ação ao esconder por ícone → tooltips + acessibilidade
  (testados); nada é removido do **estado ativo**.
- Regressão de click-through → manter `data-interactive` apenas nos controles.

## Verificação

- Automática: `pnpm test` · `tsc --noEmit` · `pnpm build`.
- Manual: os 5 passos acima.
- Observações do usuário: (aguardando)

## Resultado (2026-10-18)

- `ActionBar` em **dois estados** (`data-state`): **repouso** (arraste, marca,
  engrenagem, "Começar a ouvir" verde + badges e ocultar) e **ativo** (status,
  transporte, canais, pílula de tempo, ícones Transcrição/Sugestões, stealth,
  engrenagem e ocultar). O **Histórico** fica na extremidade direita **nos dois
  estados** (revisão de 2026-10-19: acessível sem sessão).
- `Overlay` calcula `startAccelerator` do binding `toggle_session` e passa o toggle
  de Sugestões.
- CSS: `.overlay__start` (verde), `.overlay__group`, `.overlay__drag svg`.
- Testes: `ActionBar.test.tsx` (5) — repouso × ativo, badges, engrenagem, ação verde.
- Verificação: `pnpm test` **155 ✓** · `tsc` ✓ · `pnpm build` ✓.
