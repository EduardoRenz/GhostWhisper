# 02 — Settings: semeadura dos agentes de fábrica e resolução

- **Status:** CONCLUDED
- **Depende de:** 01
- **Habilita:** 03, 05
- **Issues:** — · **BR:** BR-075

## Objetivo

Fazer dos agentes de fábrica **agentes do usuário**: materializá-los **uma
única vez** na lista `custom_assistant_modes` (que passa a ser a fonte da
verdade do painel), com edição definitiva e exclusão definitiva. O agente
Padrão segue protegido e sempre presente.

Detalhe de produto: [`../prd.md`](../prd.md) §7.2/§7.3. Desenho técnico:
[`../architecture/assistants-panel.md`](../architecture/assistants-panel.md) §3.3/§3.4.

## Entregáveis

- `src-tauri/src/domain/settings.rs`
  - `assistant_modes_seeded: bool` (`#[serde(default)]` = `false`);
  - `Settings::default()` continua **puro**: `custom_assistant_modes: Vec::new()`
    e a marca `false` (o `domain` não conhece o catálogo de fábrica — R1);
  - `selected_mode_exists()` → id protegido **ou** presente na lista (deixa de
    usar `is_builtin_id`);
  - `sanitized()` → descarta agentes inválidos e volta para `default` quando o
    selecionado não existe mais;
  - `validate()` → aceita ids de fábrica na lista do usuário.
- `src-tauri/src/application/assistant_catalog_migration.rs` (novo)
  - `AssistantCatalogMigration { settings: Arc<dyn SettingsStore> }`;
  - `seed()`: se `!assistant_modes_seeded`, acrescenta os agentes de fábrica
    que **faltam** (por id), marca a semente e salva; devolve `SeedOutcome`;
  - dobra de teste com `InMemorySettingsStore`.
- `src-tauri/src/application/assistant_catalog.rs` (novo)
  - `default_settings()`: o default do domínio + `factory_modes()` +
    `assistant_modes_seeded = true` (usado pelo `settings_defaults` da task 03).
- `src-tauri/src/lib.rs` (composition root)
  - instancia a migração e chama `seed()` no boot, junto da migração do
    agente Padrão (BR-059).
- Paridade de defaults: conferir `Settings::default()` (Rust) contra
  `defaultSettings()` (TS) e alinhar o que divergir — o "Restaurar Padrões"
  passa a usar o default do core (task 03).

## Critérios de aceite

- [ ] Instalação nova já tem os 4 agentes de fábrica na lista do usuário.
- [ ] Instalação existente recebe os 4 agentes de fábrica **uma única vez**,
      sem duplicar e sem sobrescrever agente do usuário com o mesmo id.
- [ ] `assistant_mode_id` continua válido depois da semeadura (o id do agente
      de fábrica é preservado).
- [ ] Agente de fábrica excluído **não volta** na próxima carga nem no boot.
- [ ] Excluir o agente ativo devolve a seleção para `default`.
- [ ] "Restaurar Padrões" devolve os agentes de fábrica.

## Testes

- `application/assistant_catalog_migration.rs`: semeia uma vez; não duplica;
  não sobrescreve; é idempotente.
- `domain/settings.rs`: `default()` **sem** agentes (domínio puro);
  `selected_mode_exists`; `sanitized()` volta para `default`.
- `cargo test`, `cargo clippy`, `cargo fmt --check`.
