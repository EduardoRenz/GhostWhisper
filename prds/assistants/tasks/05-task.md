# 05 — Painel "Assistentes" (cards do mockup)

- **Status:** CONCLUDED
- **Depende de:** 03, 04
- **Habilita:** 06
- **Issues:** — · **BR:** BR-074

## Objetivo

Substituir o formulário da aba "Assistente" pelo **painel de cards** do
mockup: o agente Padrão no topo (informativo), a seção "AGENTES CONFIGURADOS
(n)", um card por agente e o CTA "Criar Novo Assistente de IA".

Detalhe de produto: [`../prd.md`](../prd.md) §5/§8.1. Desenho técnico:
[`../architecture/assistants-panel.md`](../architecture/assistants-panel.md) §2/§7.2.

## Entregáveis

- `src/features/settings/AssistantsSettings.tsx` (novo)
  - card do agente **Padrão** (do catálogo efetivo): nome, badge "Primário",
    frase de papel e **sem** ações; com fallback estático
    (`DEFAULT_MODE_NAME`, `DEFAULT_AGENT_ICON`, `DEFAULT_AGENT_COLOR`) quando o
    catálogo não carregar;
  - badge "Ativo" no card do Padrão quando `assistant_mode_id === "default"`;
  - título "AGENTES CONFIGURADOS (n)" com a contagem real;
  - um `AgentCard` por agente de `settings.custom_assistant_modes`, **todos no
    mesmo formato** (o mockup tem três níveis ilustrativos);
  - CTA "Criar Novo Assistente de IA" (abre o modal em modo de criação);
  - mantém a badge "Ativo" **informativa** (compara com
    `settings.assistant_mode_id`) — o painel **não** troca o agente ativo.
- `src/features/settings/AgentCard.tsx` (novo)
  - tile do ícone na cor do agente, nome, badges, ações (Editar / duplicar /
    excluir) e a leitura dos **dois** prompts (sugestão e resumo), truncada;
  - "Editar" com `PencilIcon` + texto (`.settings__button--small`); duplicar e
    excluir como `.icon-button` (`CopyIcon` / `TrashIcon`);
  - `aria-label` explícito nas ações de ícone.
- `src/features/settings/AssistantSettings.tsx` — **removido** (o conteúdo que
  sobra vai para a task 07).
- `src/features/settings/AssistantSettings.test.tsx` — **removido junto** (a
  cobertura migra para `AssistantsSettings.test.tsx`/`CadenceSettings.test.tsx`).
- `src/features/settings/SettingsPage.tsx`
  - rótulo da aba: "Assistente" → **"Assistentes"**;
  - `TABS[3].id` continua `"assistant"` (a URL `?tab=assistant` segue valendo).
- `src/styles/globals.css`
  - `.settings__agents`, `.agent-card` (+ `--active`, `--default`),
    `.agent-card__head`, `.agent-card__tile`, `.agent-card__name`,
    `.agent-card__badges`, `.agent-card__actions`, `.agent-card__prompts`,
    `.prompt-panel` (+ `__title`, `__tag`, `__text`), `.settings__agents-title`,
    `.agent-cta`.

## Critérios de aceite

- [ ] A aba se chama "Assistentes" e mostra o Padrão no topo, com a badge
      "Primário" e **sem** botão de editar/excluir.
- [ ] A contagem em "AGENTES CONFIGURADOS (n)" bate com a lista.
- [ ] Cada card mostra ícone, nome, "Ativo" quando for o agente em uso,
      "Prompt de Sugestão" e "Prompt de Resumo".
- [ ] O card do agente ativo recebe o destaque visual (borda ciano).
- [ ] "Editar", "Duplicar" e "Excluir" abrem as ações corretas.
- [ ] Excluir o agente ativo devolve a seleção para `default`.
- [ ] O CTA "Criar Novo Assistente de IA" abre o modal de criação.

## Testes

- `src/features/settings/AssistantsSettings.test.tsx` (novo): card do Padrão
  sem ações; contagem; badge "Ativo"; exclusão volta para `default`.
- `src/features/settings/AgentCard.test.tsx` (novo): mostra os dois prompts;
  `aria-label` das ações.
- `SettingsPage.test.tsx`: rótulo da aba.
- `pnpm test`, `tsc --noEmit`.
