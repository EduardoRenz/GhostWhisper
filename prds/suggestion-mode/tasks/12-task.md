# 12 — Caminho de fine-tuning: manifesto, dataset, eval e registry

- **Status:** PENDING (evolução; não bloqueia o MVP)
- **Depende de:** 06, 09, 11
- **Habilita:** —
- **Issues:** — · **BR:** BR-100

## Objetivo

Deixar o caminho pronto para um modelo **fine-tuned** entrar no produto sem
reescrever a integração, o model manager ou a UI. Esta task **não treina** nada:
ela define e implementa o contrato de artefato, o dataset de avaliação e o
processo de ativação/rollback.

## Entregáveis

- **Manifesto estável** (já em 06) com:
  - `model_id`, `model_version`, `base_checkpoint`, `sha256`, `size_bytes`,
    `license`, `calibration`, `metrics`, `eval_dataset_sha256`.
- **Schema de exemplo para treino/avaliação**
  (`poc/laya-classifier/fixtures/schema.md`):
  - `state` (bloco + contexto), `label_cue`, `label_should_suggest`,
    `locale`, `source` (`human|synthetic|session`), `consent` (bool);
  - nenhum texto de sessão real sem opt-in explícito.
- **Harness de avaliação comparativo** (`src-tauri/examples/laya_eval.rs`):
  - aceitar `--model-dir` (base) e `--candidate-dir` (fine-tuned);
  - rodar o mesmo fixture set nos dois;
  - comparar acurácia/FP/p95 e gerar tabela para o `memory.md`;
  - gate: candidato só ativa se não regride além da margem definida.
- **Calibração**:
  - script/documentação para ajustar temperatura/limiar por cue;
  - registrar `calibration` no manifesto;
  - validar no fixture set antes de ativar.
- **Registry local**:
  - `manifest.json` por versão + `current` (já no 06);
  - “Ativar”/“Rollback” no model manager;
  - manter a versão anterior instalada.
- **Documentação**:
  - `architecture/fine-tuning.md` com o fluxo:
    export → eval → calibration → manifest → release → download → activate →
    rollback;
  - política de dados: opt-in, local, anonimizado, sem upload automático
    (BR-100);
  - como o modelo fine-tuned usa o mesmo `laya-onnx` e a mesma pergunta `cue`.
- **UI**:
  - model manager mostra “base” vs “fine-tuned” e a versão;
  - “Ativar” só aparece se o candidato passou no eval;
  - rollback em um clique.

## Critérios de aceite

- [ ] Um artefato fine-tuned com o mesmo layout ONNX e `manifest.json` é
      carregado sem mudança de código.
- [ ] O eval compara base vs candidato no mesmo fixture set e reporta as
      métricas.
- [ ] A ativação exige eval aprovado e calibração registrada.
- [ ] Rollback para a versão anterior funciona.
- [ ] Nenhum dado de conversa é enviado para treino sem opt-in explícito.
- [ ] `memory.md` registra a versão, o dataset hash e as métricas do modelo
      ativado.

## Testes

- `fine_tuned_manifest_loads_with_same_engine`
- `eval_compares_base_and_candidate`
- `activation_requires_passing_eval`
- `rollback_restores_previous_version`
- `training_example_requires_consent`
- `no_session_text_without_opt_in`

## Como testar

```bash
# 1) avaliar base vs candidato
cargo run --example laya_eval --features laya --release -- \
  --base ~/models/laya-multilingual-base \
  --candidate ~/models/laya-multilingual-ft-2027.01 \
  --fixtures poc/laya-classifier/fixtures/blocks.jsonl

# 2) ativar no app
# Fontes de IA & Modelos → Modelo de classificação → Gerenciar modelos →
# escolher versão aprovada → Ativar
```

## Notas

- O fine-tuning em si (treino, dados, GPU) é um PRD futuro; aqui só o contrato.
- O modelo base continua sendo o fallback; nunca remover a versão anterior
  durante a ativação.
- O dataset de avaliação deve ser versionado/hashado; mudanças no fixture set
  exigem recalibrar o limiar.
- O `laya-onnx` carrega qualquer ONNX que siga o layout; a pergunta `cue` é o
  contrato de produto que não pode mudar sem uma nova `model_version`.
