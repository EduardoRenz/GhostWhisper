# 01 — Domínio, prompts, Settings e persistência do resumo

- **Status:** CONCLUDED
- **Depende de:** —
- **Habilita:** 02

## Objetivo

Criar a fundação pura e persistente: modos de agente, contratos de prompt
adaptados do Perssua, configurações novas e armazenamento do resumo.

## Entregáveis

- `domain/assistant.rs`: `AssistantMode`, `AssistantCategory` (ou string),
  `builtin_modes()`, `find_mode(id, custom)`, validação (nome obrigatório, IDs
  embutidos reservados, prompts ≤ 16.384 chars).
- `domain/overview.rs`: `MeetingSummary { session_id, subject, markdown,
  summarized_from_ms, summarized_until_ms, updated_at_ms, has_more }`.
- `domain/settings.rs`: `assistant_mode_id`, `custom_assistant_modes`,
  `summary_auto_refresh`, `summary_interval_secs`, `suggestion_auto_refresh`,
  `suggestion_interval_secs`, `summary_position`, `suggestions_position`;
  defaults, `sanitized` (clamp de intervalos, fallback de modo) e `validate`.
- `application/context.rs` (puro): `build_full_context`, `build_recent_context`,
  `take_chunks`, filtros de falas finais e limites por caracteres.
- `application/prompts/summary.rs`: contrato completo adaptado, prompt inicial
  com envelope `SUBJECT:`/`CONTENT:` e prompt incremental.
- `application/prompts/assistant.rs`: contratos Reunião, Entrevista Técnica,
  LeetCode, System Design e fallback de modo personalizado.
- `ports/meeting_summaries_repository.rs` + adaptadores SQLite/em memória.
- `schema.sql`: tabela `meeting_summaries`.
- `SessionRepository::set_overview(id, overview)` (SQLite + memória); rename
  continua separado.
- Testes de domínio, prompts (regras críticas), contexto, settings e repositório.

## Critérios de aceite

- [x] Os 4 modos embutidos têm id, nome, categoria e prompts não vazios.
- [x] Modo personalizado inválido (sem nome ou id reservado) é rejeitado.
- [x] `Settings` antigo sem os campos novos carrega com defaults válidos.
- [x] Contexto de resumo/sugestão respeita os limites e ignora prévias.
- [x] Repositório grava, lê e remove o resumo em cascata com a sessão.
- [x] Prompts contêm as regras de grounding, idioma e formato.
- [x] `cargo test` verde.
