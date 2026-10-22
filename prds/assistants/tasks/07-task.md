# 07 — "Geral & HUD": cadências e perguntas prontas

- **Status:** CONCLUDED
- **Depende de:** 04
- **Habilita:** 09
- **Issues:** — · **BR:** BR-074

## Objetivo

Liberar a aba "Assistentes" só para o painel de agentes: as **cadências**
(resumo/sugestões automáticas + intervalos) e as **perguntas prontas** saem de
lá e viram uma **seção nova na aba "Geral & HUD"** (decisão do usuário,
[`../prd.md`](../prd.md) §5.1).

Detalhe de produto: [`../prd.md`](../prd.md) §4.1. Desenho técnico:
[`../architecture/assistants-panel.md`](../architecture/assistants-panel.md) §1.

## Entregáveis

- `src/features/settings/CadenceSettings.tsx` (novo)
  - toggle "Resumo automático" + select "Intervalo do resumo";
  - toggle "Sugestões automáticas" + select "Intervalo das sugestões";
  - lista de **perguntas prontas** (input por pergunta, botão "Excluir") e o
    botão "Nova pergunta pronta";
  - respeita `MAX_SAVED_QUESTIONS` e `MAX_SAVED_QUESTION_CHARS`.
- `src/features/settings/HudSettings.tsx`
  - **não** muda de responsabilidade: continua só com o HUD.
- `src/features/settings/SettingsPage.tsx`
  - a aba "general" passa a renderizar `<HudSettings/>` **e** `<CadenceSettings/>`
    (seções irmãs, na ordem), em vez de só o HUD.
- `src/styles/globals.css`
  - `.settings__section` (mesmas regras de `.settings__hud`: `flex` em coluna
    com `gap`) para a seção nova; o cabeçalho reaproveita
    `.settings__section-header` / `__title` / `__note`; ajustes na lista de
    perguntas prontas.

## Critérios de aceite

- [ ] A aba "Assistentes" **não** tem mais cadências nem perguntas prontas.
- [ ] A aba "Geral & HUD" tem a seção nova com as quatro preferências e a
      lista de perguntas prontas.
- [ ] O comportamento não muda: toggles, intervalos, limite de perguntas e
      exclusão de pergunta em branco seguem iguais.
- [ ] O botão "Salvar Alterações" global continua persistindo tudo (BR-047).

## Testes

- `src/features/settings/CadenceSettings.test.tsx` (novo): toggle de resumo,
  intervalo, adicionar/excluir pergunta pronta, limite de 20.
- `HudSettings.test.tsx`: a seção aparece na aba.
- `pnpm test`, `tsc --noEmit`.
