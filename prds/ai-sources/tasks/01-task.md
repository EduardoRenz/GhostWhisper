# 01 — Domínio das fontes de IA e das rotas

- **Status:** CONCLUDED
- **Depende de:** —
- **Habilita:** 02, 03, 05

## Objetivo

Modelar a **fonte de IA** (provedor + modelo + chave, com id estável) e a
**rota por escopo**, e encaixar as duas em `Settings`. Adicionar `DeepSeek` ao
catálogo de provedores.

## Entregáveis

- `domain/ai_source.rs`:
  - `AiSource { id, kind, label, model, base_url, enabled }`;
  - `AiSourceState` (`Unknown`/`Ready`/`Degraded`/`Failed`) e
    `AiSourceHealth { source_id, state, consecutive_failures, latency_ms,
    last_error, is_fallback, open_until_ms }`;
  - `AiSource::secret_key()` → `llm.source.{id}`;
  - `AiSource::effective_base_url()`;
  - `validate_source(&AiSource)` (modelo obrigatório, `base_url` http(s) quando
    `custom`, id não vazio, rótulo ≤ 120 chars);
  - `validate_sources(&[AiSource])` (ids únicos).
- `domain/ai_route.rs`:
  - `AiCapability` (`Summary`/`Suggestions`/`Translation`);
  - `AiRouteScope { Default, Capability(..), Agent(String) }`;
  - `AiRoute { scope, source_id }`;
  - `resolve_route(routes, scope) -> Option<&str>` (Agent > Capability >
    Default).
- `domain/settings.rs`:
  - `ProviderKind::DeepSeek` (`as_str`, `default_model` = `deepseek-chat`,
    `default_base_url` = `https://api.deepseek.com/v1`, `all()`);
  - `Settings.ai_sources: Vec<AiSource>` e `Settings.ai_routes: Vec<AiRoute>`;
  - `provider`/`model`/`base_url` → campos legados de leitura
    (`serde(rename + skip_serializing)`);
  - `sanitized()` monta a lista padrão quando `ai_sources` está vazia;
  - `validate()` valida as fontes e as rotas.
- `domain/assistant.rs`: `AssistantMode.source_id: Option<String>`.
- Espelho no frontend: `src/core/domain/settings.ts` (`AiSource`,
  `AiSourceHealth`, `AiRoute`, `PROVIDER_MODELS`).

## Critérios de aceite

- [x] `ProviderKind::DeepSeek` aparece no catálogo com base e modelo padrão.
- [x] `AiSource` valida modelo obrigatório e `base_url` http(s) para `custom`.
- [x] Duas fontes com o mesmo `id` são rejeitadas.
- [x] `resolve_route` prefere `Agent` sobre `Capability` sobre `Default`.
- [x] `Settings` com `ai_sources` vazia e campos legados cria uma fonte.
- [x] `Settings` legado serializa **sem** `provider`/`model`/`base_url`.
- [x] `cargo test` verde.
