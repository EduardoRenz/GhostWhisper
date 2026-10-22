# 04 — Frontend: espelho do domínio, `Dialog` e ícones novos

- **Status:** CONCLUDED
- **Depende de:** —
- **Habilita:** 05, 06, 07, 08
- **Issues:** — · **BR:** BR-008, BR-076, BR-077

## Objetivo

A base do frontend da nova aba: o **tipo** de agente espelhado (com
`summary_prompt`), o **primitivo de modal** que o app não tinha e os **ícones
novos** do mockup.

Detalhe de produto: [`../prd.md`](../prd.md) §4.1. Desenho técnico:
[`../architecture/assistants-panel.md`](../architecture/assistants-panel.md) §2.1/§7.3.

## Entregáveis

- `src/core/domain/assistant.ts`
  - `AssistantMode.summary_prompt: string`;
  - `PROTECTED_MODE_ID` (`"default"`) e `isProtectedMode(id)`;
  - `agentSummaryPrompt(mode)` (fallback vazio, sem hex/valor solto);
  - mantém `AGENT_ICON_IDS`, `AGENT_COLOR_TOKENS` e os limites (BR-037);
  - `BUILTIN_MODE_IDS`/`isBuiltinMode` continuam como o catálogo dos ids de
    fábrica (o painel passa a usar `isProtectedMode`, não `isBuiltinMode`).
- `src/core/domain/settings.ts`
  - `Settings.assistant_modes_seeded: boolean` no tipo + `defaultSettings()`
    (lista vazia, marca `false` — espelho do default do domínio).
- `src/components/Dialog.tsx` (novo)
  - `Dialog({ open, title, subtitle, onClose, children, footer })`;
  - `role="dialog"`, `aria-modal="true"`, `aria-labelledby`;
  - `Esc` fecha, clique no backdrop fecha, `✕` fecha;
  - foco entra no modal ao abrir e o `Tab` fica preso nele;
  - trava o scroll do `settings__body` enquanto estiver aberto.
- `src/components/icons.tsx`
  - `PencilIcon` (editar), `StarIcon` (agente Padrão), `PlusIcon` (criar) —
    SVG do projeto, `currentColor`, viewBox 24 (BR-008).
- `src/components/icons.test.tsx`
  - registra os três ícones no contrato (`aria-hidden`, `focusable`, viewBox).
- `src/styles/globals.css`
  - classes do modal: `.dialog__backdrop`, `.dialog__card`, `.dialog__header`,
    `.dialog__title`, `.dialog__subtitle`, `.dialog__body`, `.dialog__footer`.

## Critérios de aceite

- [ ] `Dialog` abre, fecha por `Esc`, por backdrop e pelo `✕`.
- [ ] O `Tab` não escapa do modal enquanto ele estiver aberto.
- [ ] Os três ícones novos herdam `currentColor` e passam no teste de contrato.
- [ ] `AssistantMode` do TypeScript tem `summary_prompt` e o compilador passa
      (`tsc --noEmit`).

## Testes

- `src/components/Dialog.test.tsx` (novo): abre/fecha, `Esc`, backdrop, foco.
- `src/components/icons.test.tsx`: ícones novos.
- `src/features/suggestions/AgentSelector.test.tsx`: acrescenta
  `summary_prompt` na fixture de `AssistantMode`.
- `pnpm test`, `tsc --noEmit`.
