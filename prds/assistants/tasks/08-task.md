# 08 — Casca: barra de abas em pílulas

- **Status:** CONCLUDED
- **Depende de:** 04
- **Habilita:** 09
- **Issues:** — · **BR:** BR-078

## Objetivo

Trocar a barra de abas das Configurações do **sublinhado** para o estilo
**pílula** do mockup (aba ativa com borda e texto ciano), em todas as abas.

Detalhe de produto: [`../prd.md`](../prd.md) §4.1/§5.1. Desenho técnico:
[`../architecture/assistants-panel.md`](../architecture/assistants-panel.md) §7.1.

## Entregáveis

- `src/styles/globals.css`
  - `.settings__tabs`: `gap` maior, `padding` vertical e borda inferior
    mantida;
  - `.settings__tab`: `border: 1px solid transparent`, `border-radius: 999px`,
    `padding` menor — **sem** `border-bottom` de 2px e **sem**
    `margin-bottom: -1px`;
  - `.settings__tab--active`: `color: var(--oh-accent)`, borda e fundo em
    `color-mix` do accent;
  - `.settings__tab:hover`: mantém o estado legível.
- `src/features/settings/SettingsPage.tsx`
  - a marcação da barra segue a mesma (`role="tablist"` / `role="tab"` /
    `aria-selected`) — **sem** mudança de markup.

## Critérios de aceite

- [ ] A aba ativa é uma pílula com borda ciano e texto ciano.
- [ ] As abas inativas são texto neutro, sem sublinhado.
- [ ] A navegação por teclado e o `aria-selected` continuam iguais.
- [ ] Nenhuma outra tela é afetada (o estilo é da casca das Configurações).

## Testes

- `src/features/settings/SettingsPage.test.tsx`: a aba ativa recebe
  `settings__tab--active` (o teste já cobre a marcação).
- Verificação visual manual: a barra com 6 abas em ~1020px de largura não
  quebra em duas linhas; em janelas estreitas o `flex-wrap` continua valendo.
- `pnpm test`, `pnpm build`.
