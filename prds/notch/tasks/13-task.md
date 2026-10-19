# 13 — Histórico no trailing e ícone de recolher

- **Status:** CONCLUDED
- **Depende de:** 12 (layout atual do notch)
- **Habilita:** —
- **Issues:** — · **BR:** BR-032 (revisão 2026-10-19), BR-036

## Objetivo

Dois ajustes visuais no notch:

1. O item **Histórico** com rótulo sai do switcher; vira **ícone no grupo da
   direita, antes de Configurações** (ordem da imagem: ocultar · Histórico · gear).
2. O botão **ocultar** passa a usar o ícone de **recolher** do protótipo: duas
   setas diagonais apontando para o centro (como o `close_fullscreen` do Material).

## Escopo

- `NotchTabs`: removem-se o item Histórico e as props `historyOpen`/
  `onToggleHistory`; ficam Transcrição · play/pause · Resumo · Sugestões.
- `ActionBar`: o grupo trailing passa a ser **ocultar · Histórico · Configurações**
  nos dois estados (repouso e ativo); a engrenagem continua o último botão.
- `icons.tsx`: novo `CollapseIcon` (duas setas diagonais); `ChevronDownIcon`
  permanece exportado (sem uso no notch).
- Testes: ordem do trailing nos dois estados, Histórico fora do switcher, ícone
  incluído no catálogo.
- Docs: PRD/CA-10.1/10.6/10.7, `notch-ui.md`, BR-032 e `memory.md`.

## Critérios de aceite

- [ ] O notch ativo não tem mais "Histórico" no switcher.
- [ ] Histórico é um `IconButton` no grupo da direita, antes da engrenagem.
- [ ] No repouso, a ordem é ocultar · Histórico · Configurações.
- [ ] Ocultar usa o ícone de duas setas diagonais (imagem de referência).
- [ ] A engrenagem continua o último botão redondo.
- [ ] `pnpm test`, `tsc`, `pnpm build`, `cargo test/clippy/fmt` verdes.

## Como testar

Com e sem sessão, conferir o grupo da direita (ocultar, Histórico, engrenagem),
clicar em Histórico para alternar a janela e no recolher para ocultar o overlay.
