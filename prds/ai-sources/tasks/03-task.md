# 03 — Serviço de fontes, saúde e circuit breaker

- **Status:** CONCLUDED
- **Depende de:** 02
- **Habilita:** 04, 06

## Objetivo

Dar ao núcleo a capacidade de **gerenciar** as fontes (cadastrar, editar,
excluir, reordenar) e de **saber como cada uma está** (conectada, degradada,
em fallback, falhou), com **circuit breaker** por fonte.

## Entregáveis

- `application/ai_source_health.rs`:
  - `AiSourceHealthRegistry` (injetado por construtor; `Arc<Mutex<…>>` interno);
  - `record_success(source_id, latency_ms)`, `record_failure(source_id, error)`,
    `is_open(source_id, now_ms)`, `snapshot()`;
  - política: 3 falhas ⇒ circuito aberto por 60 s; `401/403` ⇒ aberto na hora;
    meio-aberto após o cooldown (1 tentativa) — sucesso fecha, falha reabre;
  - `Clock` como porta de tempo (R10).
- `ports/ai_source_service.rs`: trait `AiSourceService` (list, save, health,
  set_key, has_key, clear_key, key_preview, validate, restore_defaults).
- `application/ai_source_service.rs`: implementação (usa `SettingsStore`,
  `SecretStore`, `ChatBotProviderFactory` e o registro de saúde).
- `error.rs`: variante `ProviderStatus { code, detail }` (detalhe truncado,
  **sem** segredo).
- Dobras de teste: `InMemoryAiSourceService` e `FakeAiSourceService`.

## Critérios de aceite

- [x] `save` grava a lista inteira e a **ordem** é a prioridade.
- [x] A chave de uma fonte vive em `llm.source.{id}` e nunca é devolvida em
      claro (`key_preview` mascara).
- [x] `validate` testa **uma** fonte e devolve a resposta do provider.
- [x] 3 falhas abrem o circuito; a fonte é pulada enquanto aberto.
- [x] `401/403` abre o circuito imediatamente.
- [x] Após o cooldown, a fonte entra em meio-aberto e um sucesso fecha o
      circuito.
- [x] `cargo test` verde.
