# 11 — Seletor global de agentes acima do notch

- **Status:** CONCLUDED
- **Depende de:** 03, 05
- **Habilita:** — (substitui o seletor da janela de Sugestões)
- **Issues:** — · **BR:** BR-080

## Objetivo

Tirar a escolha do agente ativo da janela de Sugestões e dar a ela um lugar
**global**: um seletor compacto e discreto flutuando **acima** da barra do
notch, sempre visível com o overlay. A troca vale na hora em todas as janelas.

Detalhe de produto: [`../prd.md`](../prd.md) §12. Desenho técnico:
[`../architecture/assistants-panel.md`](../architecture/assistants-panel.md) §11.

## Entregáveis

- `src/features/overlay/AgentSwitcher.tsx` (novo)
  - pílula compacta: ícone na cor do agente + nome + divisor + "Alternar
    agente" + chevron;
  - popover com o catálogo efetivo (role `menu`, teclado, `Esc`, clique fora);
  - `data-interactive` (click-through no resto);
  - fallback do nome para o Padrão quando o catálogo ainda não carregou.
- `src/features/overlay/AgentSwitcher.test.tsx` (novo) — 8 casos.
- `src/features/overlay/Overlay.tsx`
  - `.overlay__stack` (seletor + barra) medido para a largura da janela;
  - altura extra agregada: `max(menu de canais, menu de agentes)`;
  - `selectAgent` chama o comando novo.
- `src/features/suggestions/SuggestionsWindow.tsx`
  - seletor removido; `modeId` vem de `settings.assistant_mode_id`.
- `src/features/suggestions/AgentSelector.tsx` + teste — **removidos**.
- `src/features/settings/SettingsPage.tsx`
  - assina `settings_changed` e sincroniza o `assistant_mode_id` (badge
    "Ativo" em tempo real, sem mexer no resto do rascunho).
- `src-tauri/src/domain/hud_layout.rs`
  - `AGENT_SELECTOR_BAND = 34.0`.
- `src-tauri/src/adapters/overlay/tauri_overlay_window.rs`
  - `window_height = faixa + densidade + popover`.
- `src-tauri/tauri.conf.json` — altura inicial do overlay 84 → 118.
- `src-tauri/src/ipc/settings_commands.rs`
  - `settings_set_assistant_mode` (grava só o campo e emite `settings_changed`).
- `src/adapters/ipc/settings.ts` — `setAssistantMode(id)`.
- `src/styles/globals.css`
  - `.overlay__stack`; `.agent-switcher*` no lugar de `.agent-selector*`.

## Critérios de aceite

- [ ] O seletor aparece **acima** da barra, sempre que o overlay está visível.
- [ ] É compacto: pílula de 26 px com ícone, nome, divisor e "Alternar agente".
- [ ] O menu lista Padrão + agentes do usuário e marca o ativo.
- [ ] Escolher um agente grava `assistant_mode_id` e fecha o menu.
- [ ] A troca reflete na hora na janela de Sugestões e na badge "Ativo" das
      Configurações.
- [ ] O seletor sumiu da janela de Sugestões.
- [ ] O menu não é cortado pela janela (altura extra reservada).
- [ ] O resto do overlay continua click-through.

## Testes

- `AgentSwitcher.test.tsx`: pílula, menu, escolha, `Esc`, setas/Home/End,
  `onOpenChange`, fallback do Padrão, catálogo vazio.
- `SuggestionsWindow.test.tsx`: garante que o seletor não está mais lá.
- `SettingsPage.test.tsx`: o agente trocado em outra janela atualiza a badge.
- `cargo test`, `cargo clippy`, `pnpm test`, `tsc --noEmit`.
