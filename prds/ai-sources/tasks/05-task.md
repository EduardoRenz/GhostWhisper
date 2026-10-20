# 05 — Adapter DeepSeek e erros tipados de provider

- **Status:** CONCLUDED
- **Depende de:** 03
- **Habilita:** 06, 08

## Objetivo

Cadastrar o **DeepSeek** como provedor de primeira classe (BR-046) e tipar os
erros de status do provider, para que o circuit breaker classifique por
**código** (401/403/429/5xx) em vez de parsear a mensagem.

## Entregáveis

- `adapters/ai/deepseek_chat_bot_provider.rs`:
  - reusa `openai_compatible::chat_completion` / `chat_completion_streaming`;
  - `base_url` = `https://api.deepseek.com/v1`;
  - `id()` = `deepseek:{model}`.
- `adapters/ai/openai_compatible.rs`:
  - `temperature_supported` passa a excluir `deepseek-reasoner`;
  - status de erro passa a usar `GhostWhisperError::ProviderStatus`.
- `adapters/ai/gemini_chat_bot_provider.rs`: idem (status tipado).
- `adapters/ai/default_chat_bot_provider_factory.rs`: braço `DeepSeek`.
- `adapters/ai/mod.rs`: módulo novo.

## Critérios de aceite

- [x] `DeepSeek` entra no catálogo de provedores e no catálogo de modelos.
- [x] O adapter chama `POST {base_url}/chat/completions` com
      `Authorization: Bearer`.
- [x] `deepseek-reasoner` não recebe `temperature`.
- [x] Um `401` chega ao chamador como `ProviderStatus { code: 401, … }`.
- [x] A mensagem de erro não contém a chave.
- [x] `cargo test` verde.
