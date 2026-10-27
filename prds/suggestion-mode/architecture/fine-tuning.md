# Arquitetura — Caminho de fine-tuning do classificador

Especificação do contrato que permite trocar o modelo base do Laya por um modelo
**fine-tuned** sem mudar o app, o model manager ou a UI. O treino em si **não**
faz parte deste PRD; aqui ficam o artefato, o dataset de avaliação, a calibração
e o processo de ativação/rollback. Contrato em **BR-100**.

---

## 1. Princípio

O app não sabe se o modelo é base ou fine-tuned. Ele conhece apenas:

- um diretório ONNX com o layout do `laya-onnx`;
- a pergunta `cue` com 7 opções;
- um `manifest.json` versionado.

Trocar o modelo é trocar o ponteiro `current` para outro diretório/manifesto. Se
o candidato não passar no eval, o ponteiro não muda.

---

## 2. Artefato

Layout obrigatório (o mesmo do modelo base):

```
laya-multilingual-<version>/
  model.onnx
  model.onnx.data
  rl_agent_config.json
  tokenizer/
    tokenizer.json
    tokenizer_config.json
  manifest.json
```

`manifest.json`:

```json
{
  "schema_version": 1,
  "model_id": "laya-multilingual",
  "model_version": "2027.01.0-ft",
  "base_checkpoint": "convaiinnovations/laya-multilingual",
  "format": "onnx",
  "onnx_layout": "fused",
  "size_bytes": 1288490188,
  "license": "Apache-2.0",
  "files": [
    { "path": "model.onnx", "url": "https://…/model.onnx", "size_bytes": 0, "sha256": "…" },
    { "path": "model.onnx.data", "url": "https://…/model.onnx.data", "size_bytes": 0, "sha256": "…" },
    { "path": "rl_agent_config.json", "url": "https://…/rl_agent_config.json", "size_bytes": 0, "sha256": "…" },
    { "path": "tokenizer/tokenizer.json", "url": "https://…/tokenizer.json", "size_bytes": 0, "sha256": "…" }
  ],
  "calibration": {
    "method": "temperature",
    "temperature": [1.0, 1.0, 1.0],
    "ece": 0.08
  },
  "metrics": {
    "cue_accuracy": 0.84,
    "should_suggest_precision": 0.88,
    "should_suggest_recall": 0.81,
    "false_positive_rate": 0.11,
    "latency_p95_ms_cpu": 620
  },
  "eval_dataset_sha256": "…",
  "created_at": "2027-01-10T00:00:00Z"
}
```

Regras:

- `model_version` é imutável; qualquer mudança de pesos/labels gera versão nova.
- Cada `files[]` tem `sha256`; o model manager verifica antes de instalar. Não
  há tar/zip: o download é arquivo a arquivo.
- `eval_dataset_sha256` liga o modelo ao conjunto de avaliação usado.
- `base_checkpoint` é informativo; o app só usa o diretório ONNX.
- A licença do modelo derivado deve ser compatível com Apache-2.0 e registrada.

---

## 3. Pergunta `cue` (contrato de produto)

A pergunta é estável:

```
question, hesitation, lost_thread, invitation, silence,
no_suggestion, insufficient_context
```

- Mudar rótulos, descrições ou a ordem **exige** nova `model_version` e novo
  eval; a ordem é posicional no `laya-onnx`.
- O modelo fine-tuned deve ser treinado/avaliado com a mesma instrução e as
  mesmas descrições de opção do modelo base; caso contrário, o resultado não é
  comparável.
- O app não envia `noul`; a decisão `should_suggest` é derivada de cue +
  confiança + abstenção.

---

## 4. Dataset de avaliação e treino

Schema de exemplo (JSONL):

```json
{
  "state": {
    "locale": "pt-BR",
    "assistant": "default",
    "block": "ME: ...\nTHEY: ...",
    "context": "blocos anteriores"
  },
  "label_cue": "question",
  "label_should_suggest": true,
  "source": "human",
  "consent": true
}
```

- `source`: `human` (rotulado à mão), `synthetic` (gerado), `session`
  (sessão real).
- `consent`: obrigatório `true` para qualquer `source = session`.
- Sessões reais **não** são exportadas automaticamente. A exportação é uma ação
  explícita do usuário, local, com anonimização (nomes, e-mails, IDs, números).
- O dataset de avaliação é versionado/hashado; o de treino pode ser maior, mas o
  eval é o mesmo para base e candidato.
- Dados de treino e eval não vão para telemetria/backend (BR-002/BR-004).

---

## 5. Avaliação e calibração

Harness `src-tauri/examples/laya_eval.rs`:

- aceita `--model-dir` (base) e `--candidate-dir` (fine-tuned);
- roda o mesmo fixture set nos dois;
- métricas: acurácia de `should_suggest`, P/R/F1 por cue, matriz de confusão,
  falso positivo, p50/p95;
- imprime JSON + tabela Markdown para o `memory.md`.

Gate de ativação (proposta):

| Métrica | Base | Candidato |
|---|---|---|
| `should_suggest` accuracy | ≥ 0.80 | ≥ 0.80 |
| falso positivo | ≤ 0.15 | ≤ 0.15 |
| regressão vs. base | — | ≤ 0.02 em accuracy |
| p95 CPU | ≤ 1 s | ≤ 1 s |

Calibração:

- ajustar temperatura por (tipo de pergunta, nº de opções) no conjunto de
  validação;
- registrar `calibration` no manifesto;
- validar o limiar da sensibilidade (0,85/0,70/0,55) no fixture set;
- se o ECE não melhorar, não ativar.

---

## 6. Ativação e rollback

1. Modelo candidato é publicado com manifesto + SHA-256.
2. `ClassifierModelManager` baixa/verifica/instala em
   `<version>/` sem tocar no `current`.
3. Eval roda (offline) sobre o fixture set; resultado é anexado ao manifesto/
   `memory.md`.
4. “Ativar” só habilita se o gate passou.
5. `current` é trocado atomicamente; a versão anterior permanece instalada.
6. “Rollback” troca `current` de volta; não baixa nada.
7. “Apagar” remove versões não ativas; a ativa e a anterior ficam.

O app não reinicia o modelo em uso no meio de uma sessão; a troca vale para a
próxima sessão ou próximo boot (definir na task 10/12).

---

## 7. Registry local

```
<app_data>/models/laya/
  manifests/
    2026.10.0.json
    2027.01.0-ft.json
  2026.10.0/
  2027.01.0-ft/
  current            # arquivo texto com a versão ativa
```

- O `manifest.json` é a fonte de verdade de versão/métricas.
- O `current` é um ponteiro atômico; nunca aponta para `.part`/diretório
  incompleto.
- O model manager lista versões, ativa, remove e faz rollback.
- O `memory.md` do PRD registra a avaliação de cada versão ativada.

---

## 8. Privacidade e consentimento

- Inferência local; nenhum texto de conversa sai da máquina no runtime.
- Download do modelo é a única rede e é explícito.
- Exportação de sessão para treino é opt-in, local e anonimizada.
- Nenhum dataset de usuário é enviado para um servidor de treino neste PRD.
- O manifesto não contém dados de sessão; só métricas agregadas.

---

## 9. O que fica para o PRD de fine-tuning

- coleta/rotulagem em escala;
- treino (dados, GPU, hiperparâmetros, RLCD/temperatura);
- governança de dataset e retenção;
- publicação automatizada do artefato;
- comparação estatística entre versões;
- feedback “útil/não útil” na UI.

Este documento garante que, quando esse PRD existir, o app já aceita o resultado
sem refatoração.
