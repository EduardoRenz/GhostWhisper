# 12 — Ajuda nos campos de prompt (tooltips)

- **Status:** CONCLUDED
- **Depende de:** 06
- **Habilita:** —
- **Issues:** — · **BR:** BR-081

## Objetivo

Explicar, na própria UI, o que cada campo de prompt do agente faz, onde é usado
hoje e o que acontece quando fica vazio. O gatilho é um ícone "i" com tooltip
(hover e teclado), no modal de edição e nos painéis do card.

Detalhe de produto: [`../prd.md`](../prd.md) §13. Desenho técnico:
[`../architecture/assistants-panel.md`](../architecture/assistants-panel.md) §12.

## Entregáveis

- `src/components/InfoTooltip.tsx` (novo)
  - botão focável com `aria-label="Ajuda: <campo>"` e `aria-describedby`;
  - balão `role="tooltip"`, visível no hover e no `focus-within`.
- `src/components/icons.tsx`
  - `InfoIcon` (círculo com "i", SVG do projeto — BR-008).
- `src/features/settings/AgentEditorDialog.tsx`
  - rótulos com ajuda em: Prompt de Sugestão, Prompt de Resumo, Categoria,
    Prompt de sistema e Contexto fixo;
  - campos passam a usar `<label htmlFor>` + `id` (o botão de ajuda não pode
    ficar dentro do `<label>`).
- `src/features/settings/AgentCard.tsx`
  - ajuda nos painéis "Prompt de Sugestão" e "Prompt de Resumo".
- `src/styles/globals.css`
  - `.settings__field-label`, `.info-tooltip*`, `.prompt-panel__help`.

## Critérios de aceite

- [ ] O ícone "i" aparece em cada campo de prompt (e na Categoria).
- [ ] Hover e foco mostram o balão; `Esc`/blur escondem (CSS).
- [ ] O texto do balão explica o uso **de hoje** de cada campo.
- [ ] O Prompt de Resumo deixa explícito que ainda não é consumido.
- [ ] Os campos continuam acessíveis por `getByLabelText`.

## Testes

- `src/components/InfoTooltip.test.tsx`.
- `AssistantsSettings.test.tsx`: os 5 botões de ajuda no modal.
- `AgentCard.test.tsx`: os 2 botões de ajuda nos painéis.
- `icons.test.tsx`: `InfoIcon` no contrato.
- `pnpm test`, `tsc --noEmit`.
