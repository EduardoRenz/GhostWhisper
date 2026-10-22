# 03 — IPC: catálogo efetivo e defaults do core

- **Status:** CONCLUDED
- **Depende de:** 02
- **Habilita:** 05
- **Issues:** — · **BR:** BR-074, BR-075

## Objetivo

Expor o **catálogo efetivo** de agentes (protegido + lista do usuário) e os
**defaults do core**, para que a UI não precise reproduzir os contratos de
fábrica no frontend.

Detalhe de produto: [`../prd.md`](../prd.md) §8.6. Desenho técnico:
[`../architecture/assistants-panel.md`](../architecture/assistants-panel.md) §4/§5.

## Entregáveis

- `src-tauri/src/ipc/suggestion_commands.rs`
  - `assistant_modes_list` passa a devolver `effective_modes(&settings.custom_assistant_modes)`
    (hoje concatena builtins + custom, o que duplicaria o agente de fábrica
    materializado).
- `src-tauri/src/ipc/settings_commands.rs`
  - novo command `settings_defaults` → `application::assistant_catalog::default_settings()`
    (default do domínio + agentes de fábrica + atalhos padrão).
- `src-tauri/src/lib.rs`
  - registra `settings_defaults` no `generate_handler!`.
- `src/adapters/ipc/settings.ts`
  - `settingsDefaults(): Promise<Settings | undefined>`.
- `src/features/settings/SettingsPage.tsx`
  - "Restaurar Padrões" usa `settingsDefaults()` (com `defaultBindings()` por
    cima) em vez do `defaultSettings()` local.

## Critérios de aceite

- [ ] `assistant_modes_list` devolve o agente Padrão **primeiro**, seguido da
      lista do usuário, **sem id repetido**.
- [ ] `settings_defaults` devolve os 4 agentes de fábrica com os prompts
      completos.
- [ ] "Restaurar Padrões" na janela de Configurações devolve os agentes de
      fábrica ao rascunho.
- [ ] A tela de sugestões (que consome `assistant_modes_list`) continua
      listando o agente Padrão + os agentes do usuário.

## Testes

- `ipc/suggestion_commands.rs`: catálogo efetivo (ordem + sem duplicata).
- `ipc/settings_commands.rs`: `settings_defaults` traz os agentes de fábrica.
- `SettingsPage.test.tsx`: "Restaurar Padrões" pede os defaults ao core.
- `cargo test`, `pnpm test`.
