# 18 — Circuit breaker rígido e prazos curtos (BR-045)

- **Status:** CONCLUDED
- **Depende de:** 03, 04
- **BR:** BR-045 · **Rules:** R16, R17

## Objetivo

O produto exige **resposta rápida de IA durante a reunião**. O circuit breaker
antigo esperava 3 falhas e o adaptador esperava até 60 s antes de desistir — na
prática, uma fonte pendurada segurava a sugestão e o fallback nunca chegava a
tempo.

## Mudanças

- `application/ai_source_health.rs`: `FAILURE_THRESHOLD` **3 → 1**. Uma falha
  (timeout, 5xx, 429, resposta vazia) já abre o circuito por **60 s**; a
  próxima chamada começa na fonte reserva. `COOLDOWN_MS` continua 60 s. Com o
  limiar em 1 o estado `Degraded` fica reservado (não é produzido).
- `application/llm_timeouts.rs` (novo): prazo por escopo de rota —
  `FAST_TIMEOUT` **15 s** (Default, agente, `Capability::Suggestions`) e
  `LONG_TIMEOUT` **60 s** (`Summary`, `Translation`).
- `domain/llm.rs`: `CompletionRequest.timeout: Option<Duration>` +
  `with_timeout` (sobrescreve) e `with_default_timeout` (só se vazio).
- `application/llm_router.rs`: aplica o prazo do escopo em `complete_for` e
  `complete_streaming_for`; a validação usa o prazo rápido.
- `application/ai_source_service.rs`: validação da chave com prazo rápido.
- `adapters/ai/openai_compatible.rs` e `gemini_chat_bot_provider.rs`:
  `request_timeout(&request)` no lugar da constante fixa (o teto de 60 s vira
  o padrão de quem não define prazo).

## Critérios de aceite

- [x] Uma falha basta para a reserva assumir na chamada seguinte.
- [x] A fonte com o circuito aberto **não** é criada/tentada de novo (60 s).
- [x] Cooldown de 1 min e retomada em meio-aberto (sucesso fecha, falha reabre).
- [x] Caminho rápido com 15 s; resumo/tradução com 60 s; prazo explícito do
      chamador vence a política.
- [x] Testes anti-regressão falham com o limiar antigo (3) e passam com 1.
- [x] `cargo test` (861 ✓), `cargo clippy --all-targets` e `cargo fmt --check`
      limpos; `pnpm test` e `tsc --noEmit` limpos.

## Notas

- Um modelo lento (raciocínio) passa a ser demovido por 1 min na primeira
  estourada de prazo — é o comportamento pedido: a sugestão ao vivo não espera.
- Sondagem em meio-aberto: a cada 60 s a primária é tentada uma vez; se
  falhar, reabre por mais 60 s.
