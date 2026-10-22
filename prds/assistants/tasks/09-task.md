# 09 — Regressão, testes e fechamento

- **Status:** CONCLUDED
- **Depende de:** 06, 07, 08
- **Habilita:** —
- **Issues:** — · **BR:** BR-074, BR-075, BR-076, BR-077, BR-078

## Objetivo

Fechar a entrega: caçar o que quebrou com a mudança de modelo (agentes de
fábrica materializados, `summary_prompt` novo, painel novo, casca nova),
rodar a suíte inteira e registrar o histórico da PRD.

## Entregáveis

- `src/features/settings/SettingsPage.test.tsx`
  - ajusta as expectativas do rótulo da aba ("Assistente" → "Assistentes");
  - cobre o "Restaurar Padrões" usando os defaults do core.
- `src/core/application/useAssistantModes.ts`
  - confere que a lista devolvida pelo core é consumida sem duplicata.
- `src/core/domain/assistant.ts`
  - confere que `isBuiltinMode` deixou de ser usado pela UI (o painel usa
    `isProtectedMode`) e que nada mais depende de `BUILTIN_MODE_IDS` para
    decidir editabilidade.
- `src/features/suggestions/AgentSelector.tsx`
  - a tela de sugestões continua listando o agente Padrão + os agentes do
    usuário (BR-037) e trocando o agente ativo.
- `prds/business-rules.md`
  - as BRs 074–078 entram com o status final.
- `prds/assistants/memory.md`
  - entrada da entrega (IDs/Refs) e o que ficou para a próxima.
- `prds/assistants/issues.md`
  - registra o que aparecer (numeração continua: próximo livre **ISS-060**).

## Critérios de aceite

- [ ] `cargo test`, `cargo clippy`, `cargo fmt --check` limpos.
- [ ] `pnpm test`, `tsc --noEmit`, `pnpm build` limpos.
- [ ] Sem regressão na tela de sugestões: o seletor de agente mostra o Padrão e
      a lista do usuário, e a troca de agente ativo continua funcionando.
- [ ] Um `settings.json` antigo (sem `summary_prompt` nem
      `assistant_modes_seeded`) abre o app sem erro e recebe os agentes de
      fábrica.
- [ ] Um agente excluído pelo usuário não reaparece depois de reiniciar.
- [ ] `memory.md` e `issues.md` atualizados.

## Testes

- Regressão do `settings.json` antigo (carga + semeadura).
- Regressão da exclusão definitiva (não volta no boot).
- `cargo test`, `pnpm test`, `pnpm build`.
