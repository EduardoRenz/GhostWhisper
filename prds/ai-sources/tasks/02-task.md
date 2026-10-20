# 02 — Migração das settings e das chaves legadas

- **Status:** CONCLUDED
- **Depende de:** 01
- **Habilita:** 03, 06

## Objetivo

Fazer o upgrade silencioso de quem já usa o GhostWhisper: as settings com
`provider`/`model`/`base_url` viram **uma** fonte de IA, e a chave guardada no
keyring (`llm.openai`) passa a existir também como `llm.source.{id}`.

## Entregáveis

- `application/ai_source_migration.rs`:
  - `AiSourceMigration::new(settings, secrets)` + `migrate()`;
  - passo 1: `ai_sources` vazia + `legacy_provider` presente ⇒ cria uma
    `AiSource` (id novo, `kind`/`model`/`base_url` legados) e salva;
  - passo 2: para cada fonte migrada, se `llm.source.{id}` não existe e
    `llm.{kind}` existe ⇒ **copia** o valor (a chave legada **não** é apagada);
  - **idempotente**: rodar duas vezes não muda nada.
- Chamada no boot: `lib.rs` → `build_app_state` (antes de montar o
  `LlmRouter`).
- `domain/ai_source.rs`: gerador de id de fonte (`new_source_id()` — uuid v4).

## Critérios de aceite

- [x] Settings antigas viram uma fonte com o mesmo provider/modelo.
- [x] A chave legada é copiada para `llm.source.{id}` e continua no keyring.
- [x] Rodar a migração de novo não duplica fontes nem chaves.
- [x] Settings já migradas não são tocadas.
- [x] `cargo test` verde.
