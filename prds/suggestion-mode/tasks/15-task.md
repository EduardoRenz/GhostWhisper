# 15 — Pergunta longa sem `?`: contexto inteiro + prompt multilíngue

- **Status:** CONCLUDED
- **Depende de:** 14 (histórico de perguntas)
- **Habilita:** 16
- **Issues:** ISS-077 · **BR:** BR-094 (revisada) · **R23/R24**
- **Refs:** [`../issues.md`](../issues.md) ISS-077,
  [`../../business-rules.md`](../../business-rules.md) BR-094,
  [`../architecture/suggestion-gating.md`](../architecture/suggestion-gating.md) §2

## Objetivo

Fazer o **Laya** classificar como `question` perguntas longas que o Whisper não
pontuou com `?`, que vieram em vários turnos e que o `build_blocks` dividiu no
limite de tempo — **sem** listas de palavras-chave ou regras de idioma (R23).

## Causa (sessões reais)

- `606de489…` (18:55, *load balancer*): pergunta em 3 turnos, sem `?`, com o
  interrogativo no meio; o alvo era só o último fragmento.
- `0e882d0a…` (19:41, *arquitetura*): pergunta em 4 turnos; a divisão em 45 s
  deixava o último turno como alvo isolado.

O caminho Laya era sobrescrito pela guarda heurística de palavras-chave e o
estado do prompt separava contexto/alvo; o modelo via um fragmento.

## Entregáveis

- **Contexto**: `target_across_blocks` junta blocos **contíguos** (divisão por
  tempo, não por pausa) e o alvo leva a pergunta inteira; teste garante 4 turnos
  de THEY.
- **Prompt/estado do Laya**: conversa inteira (contexto + bloco atual, sem
  `[CONTEXTO]/[ALVO]`), prompt curto em inglês **genérico** (sem exemplos ou
  palavras de idioma), opções com descrições genéricas. O alvo/idioma do
  conteúdo é o da conversa; nada depende de uma língua.
- **Caminho efetivo**: remove a guarda heurística do Laya; `question`/`comment`/
  `no_action` vêm do modelo. Só `already_answered` usa Jaccard de tokens
  (independente de idioma, R23). Laya indisponível → não sugerir.
- **Testes reais com Laya** (R24, `#[ignore]` + `LAYA_ONNX_DIR`):
  - `regression_0e882d0a_architecture_question_is_question`;
  - `regression_606de489_load_balancer_question_is_question`;
  - `regression_repeated_architecture_question_is_already_answered`.
- **Remoção das inclusões de palavras-chave** (`ANYWHERE`, `LEADING_PHRASES`,
  `IMPERATIVES`): revertidas; a regra R23 foi adicionada às rules.

## Critérios de aceite

- [x] Arquitetura (`0e882d0a…`) → `question` com Laya real.
- [x] Load balancer (`606de489…`) → `question` com Laya real.
- [x] Pergunta repetida após resposta do ME → `already_answered` (Jaccard).
- [x] O alvo leva os 4 turnos de THEY; o prompt vê a conversa inteira.
- [x] Nenhuma lista de palavras/idioma no caminho Laya; sem fallback por keyword.
- [x] Testes reais Laya verdes; suite unitária/clippy/fmt verdes.

## Resultado (2026-10-27)

- `target_across_blocks` + estado de conversa inteira + prompt genérico.
- `SettingsSuggestionCueClassifier`: sem guarda de keywords; Jaccard só para
  `already_answered`; Laya indisponível retorna erro (não sugere).
- 3 testes de integração reais com Laya verdes; o relatório das fixtures antigas
  segue como diagnóstico (a acurácia bruta do modelo é ISS-062; fine-tuning).
- `cargo test --lib` **841 ✓** (11 ignorados) · `cargo test --features laya --
  --ignored laya_integration` **5 ✓** · clippy `-D warnings` ✓ · fmt ✓.
- Débito separado: “Regras locais” keyword-based (ISS-078/task 16).
