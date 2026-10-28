# 19 — Debounce estrutural: setup de THEY antes da pergunta real

- **Status:** CONCLUDED
- **Depende de:** 15/17 (classificação), 18 (histórico)
- **Habilita:** —
- **Issues:** ISS-083 · **BR:** BR-094 (revisada) · **R23**
- **Refs:** [`../issues.md`](../issues.md) ISS-083,
  [`../../business-rules.md`](../../business-rules.md) BR-094

## Objetivo

Não gerar sugestão para uma fala de **setup** de THEY (“vou pedir licença para
fazer algumas perguntas…”) antes de a pergunta real chegar, sem palavra-chave.

## Entregáveis

- Porta `TranscriptActivity` (sessão + canal; memória) e adaptador
  `InMemoryTranscriptActivity`.
- O pipeline publica a atividade no store; o gate lê **só o canal System/THEY**.
- `SuggestionGateConfig.they_quiet_ms` (produção: **3 s**):
  - bloco fechado só classifica depois do silêncio mínimo;
  - se THEY está falando (`TranscriptActivity`), segura;
  - ME falando **não** segura (o usuário costuma responder em seguida).
- `Gate::reset` limpa a atividade da sessão.
- Testes:
  - bloco fechado recente → `NoBlock`;
  - `System` falando → `NoBlock`; `Microphone` falando → classifica;
  - store acompanha a fala até o final.

## Critérios de aceite

- [x] Setup seguido de pergunta real não gera sugestão para o setup.
- [x] ME falando não bloqueia a classificação de THEY.
- [x] Bloco fechado tem o debounce configurável.
- [x] `cargo test`, clippy, fmt e frontend verdes.

## Resultado (2026-10-28)

- Store de atividade + debounce de 3 s + guarda do canal THEY.
- `cargo test --lib` **855 ✓** (11 ignorados) · frontend **526 ✓** ·
  clippy/fmt/tsc ✓.
