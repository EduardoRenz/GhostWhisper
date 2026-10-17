# 06 — Settings + providers de LLM (BYO key)

- **Status:** CONCLUDED
- **Depende de:** 00, 05
- **Habilita:** 07, 08

## Objetivo
Permitir configurar provider e chave de API (guardada no **keyring**) e implementar
a camada de chamadas aos providers.

## Entregáveis (implementados)
- Domínio: `domain/settings.rs` (`Settings`, `ProviderKind`), `domain/llm.rs`
  (`ProviderConfig`, `CompletionRequest`, `Completion`).
- Portas: `ports/{chat_bot_provider,llm_provider_factory,settings_store,
  secret_store,llm_controller}.rs`.
- Adaptadores de settings: `adapters/settings/file_settings_store.rs`
  (`settings.json`, sem segredos) + dobra em memória.
- Adaptadores de segredos: `adapters/secrets/keyring_secret_store.rs`
  (Secret Service, serviço `ghostwhisper`) + `in_memory_secret_store.rs` (fallback).
- Adaptadores de IA: `adapters/ai/{openai_compatible,openai_chat_bot_provider,
  gemini_chat_bot_provider,openrouter_chat_bot_provider,custom_chat_bot_provider,
  default_chat_bot_provider_factory}.rs` + `fake_chat_bot_provider.rs` (teste).
- Aplicação: `application/prompts.rs`, `application/llm_router.rs`.
- IPC: `ipc/settings_commands.rs` (`settings_providers/get/set/set_provider_key/
  has_provider_key/clear_provider_key/validate_key/provider_label/
  secrets_persistent`).
- Frontend: `core/domain/settings.ts`, `adapters/ipc/settings.ts`,
  `features/settings/SettingsPage.tsx`; janela `settings` no `tauri.conf.json`.

## Critérios de aceite
- [x] Chave salva e recuperada do keyring; nunca em texto puro.
- [x] "Validar chave" retorna sucesso/erro claro para cada provider.
- [x] Endpoint custom (`base_url` + modelo) funciona no formato OpenAI.
- [x] Nenhuma chave é exposta em logs.

## Evidências
- `cargo test` → **55 testes** (4 `ignored`, incl. keyring real).
- **Keyring real** (`cargo test --lib keyring -- --ignored`): set/get/delete
  contra o Secret Service do ambiente → `ok`.
- **Endpoint custom real**: teste sobe um servidor HTTP local (loopback) e
  confirma `POST /chat/completions`, cabeçalho `Authorization: Bearer …`, o
  modelo no corpo e a extração da resposta; o caso 401 vira erro **sem** vazar a
  chave.
- `settings.json` verificado sem campos de segredo (teste do `FileSettingsStore`).
- `cargo clippy`/`cargo fmt` limpos; `pnpm test` (4) e `pnpm build` ok.
- Smoke run: app inicia (Secret Service detectado como disponível).

## Notas / decisões
- **HTTP com `ureq`** (síncrono), não `reqwest` (assíncrono): mantém os comandos
  Tauri síncronos e reaproveita a dependência já usada no download de modelos.
  A fábrica (`DefaultChatBotProviderFactory`) isola os adaptadores; a aplicação
  (`LlmRouter`) só conhece portas.
- A chave do **Gemini** vai em cabeçalho `x-goog-api-key` (nunca na query) para
  não vazar em logs/erros.
- Fallback seguro (BR-003): sem Secret Service, usa `InMemorySecretStore` e
  avisa que a chave vale apenas na sessão.
- `application/prompts.rs` é consumido pelos casos de uso de insights/tradução
  (Tarefa 08); por isso está marcado `#![allow(dead_code)]` até lá.
