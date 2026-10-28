# 16 — Remover o backend “Regras locais” (palavras-chave)

- **Status:** PENDING
- **Depende de:** 10, 15
- **Habilita:** —
- **Issues:** ISS-078 · **BR:** BR-094/BR-098 · **R23**
- **Refs:** [`../issues.md`](../issues.md) ISS-078,
  [`../../business-rules.md`](../../business-rules.md) BR-094

## Objetivo

Remover a última decisão de classificação baseada em **listas de
palavras-chave/idioma** (R23): o backend “Regras locais”. O caminho efetivo
passa a ser o Laya (modelo multilíngue + prompt), com a guarda lexical de
`already_answered` (Jaccard, independente de idioma).

## Escopo

- Remover “Regras locais” do seletor de backend (`SuggestionsActivationBlock`)
  e do fallback de build sem Laya; sem modelo, **não sugerir** (exibir o aviso).
- Migrar `suggestion_classifier_backend = "heuristic"` para `"laya"` (settings e
  usuários existentes); o valor antigo não deve reativar keywords.
- Remover do domínio `heuristic_cue`, `is_question_text`, `invites_comment`,
  `is_small_talk`, `is_thinking_filler` e as listas correlatas.
- Manter `is_repeated_question` (Jaccard), `they_target`,
  `target_across_blocks`, `answered_in_block` e os helpers estruturais.
- Ajustar/remover testes e docs que descrevem o backend heurístico; atualizar o
  PRD (default e riscos) e a UI de configurações.
- Não criar substituto por palavras-chave; se a acurácia do Laya for baixa, o
  caminho é fine-tuning (ISS-062/task 12), não listas.

## Critérios de aceite

- [ ] Nenhum caminho de produção classifica por lista de palavras/idioma.
- [ ] `suggestion_classifier_backend` não oferece mais “Regras locais”.
- [ ] Sem Laya, a UI mostra o motivo e **não** gera sugestão.
- [ ] `already_answered` continua via Jaccard.
- [ ] PRD, settings, testes e UI atualizados; `cargo test`/`pnpm test` verdes.
