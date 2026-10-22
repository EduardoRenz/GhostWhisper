# 01 — Domínio do agente: `summary_prompt` e catálogo protegido

- **Status:** CONCLUDED
- **Depende de:** —
- **Habilita:** 02, 03
- **Issues:** — · **BR:** BR-075, BR-076

## Objetivo

Dar ao modelo de agente o campo **prompt de resumo** (persistido, ainda não
consumido — BR-076) e reorganizar o catálogo do core em **agente protegido**
(o Padrão) + **agentes de fábrica** (semente). É a base do BR-075: só o id
`default` fica reservado; os ids de fábrica passam a ser ids válidos na lista
do usuário, para que possam ser editados e excluídos.

Detalhe de produto: [`../prd.md`](../prd.md) §7. Desenho técnico:
[`../architecture/assistants-panel.md`](../architecture/assistants-panel.md) §3.1.

## Entregáveis

- `src-tauri/src/domain/assistant.rs`
  - `AssistantMode.summary_prompt: String` com `#[serde(default)]`;
  - `PROTECTED_MODE_ID` (o `default`);
  - `validate_mode`: rejeita **só** o id protegido (hoje rejeita todos os ids
    embutidos); o "ao menos um prompt" passa a contar `summary_prompt`;
    limite de 16 384 para o campo novo;
  - `validate_custom_modes` inalterado (ids duplicados seguem rejeitados).
- `src-tauri/src/application/prompts/assistant.rs`
  - `default_mode()` (o agente protegido, id `default`);
  - `factory_modes()` (os 4 agentes de fábrica) — o catálogo atual, renomeado,
    com `summary_prompt` semeado do contrato de resumo existente
    (`SUMMARY_SYSTEM_PROMPT`);
  - `effective_modes(custom)` = protegido + lista do usuário;
  - `resolve_mode(id, custom)`: `default` → protegido; qualquer outro id → só
    na lista do usuário;
  - `builtin_modes()` sai (ou vira apelido de `factory_modes()`), conforme o
    uso no resto do código;
  - `find_mode` deixa de ter uso (o `resolve_mode` não consulta mais os
    builtins) — sai ou vira helper de teste;
  - `is_builtin_id` **continua** significando "id de fábrica" (não é mais usado
    para reservar id; a reserva passa a ser só o `PROTECTED_MODE_ID`).
- `src-tauri/src/application/prompts/summary.rs`
  - `SUMMARY_SYSTEM_PROMPT` passa a ser referenciável pelos agentes de fábrica
    (mesmo módulo, sem dependência circular).
- Literais `AssistantMode { … }` dos testes existentes ganham `summary_prompt`:
  `domain/settings.rs`, `domain/assistant.rs`,
  `application/prompts/assistant.rs`, `application/suggestion_service.rs` e
  `application/assistant_mode_migration.rs`.

## Critérios de aceite

- [ ] `AssistantMode` tem `summary_prompt` e `settings.json` antigo continua
      carregando (`serde(default)` ⇒ `""`).
- [ ] Um agente com **só** `summary_prompt` preenchido passa na validação.
- [ ] Um agente com id `default` é **rejeitado** na lista do usuário.
- [ ] Um agente com id `meeting` (de fábrica) é **aceito** na lista do usuário.
- [ ] `resolve_mode("default", [])` devolve o agente protegido.
- [ ] `resolve_mode("meeting", [])` devolve `None` (agente de fábrica excluído
      não volta por fallback).
- [ ] `effective_modes` não devolve id duplicado quando a lista do usuário tem
      um agente de fábrica materializado.

## Testes

- `domain/assistant.rs`: validação com `summary_prompt`; id protegido
  rejeitado; id de fábrica aceito; `summary_prompt` acima do limite rejeitado.
- `application/prompts/assistant.rs`: `resolve_mode` (protegido / de fábrica /
  desconhecido); `effective_modes` sem duplicata.
- `cargo test`, `cargo clippy`, `cargo fmt --check`.
