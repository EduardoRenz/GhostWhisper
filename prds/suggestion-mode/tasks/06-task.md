# 06 — Modelo Laya (`laya-onnx`) + model manager + avaliação

- **Status:** IN PROGRESS (2026-10-26) — engine, artefato ONNX, model manager e AppImage CONCLUDED; eval real abaixo do gate (ISS-062), fine-tuning pendente
- **Depende de:** 02, 03, 04
- **Habilita:** 09, 10, 12
- **Issues:** — · **BR:** BR-098, BR-099

## Objetivo

Entregar o classificador Laya para **usuário final**: SDK Rust `laya-onnx` dentro
do backend, artefato ONNX versionado com manifesto/checksum e um model manager
que baixa/instala/ativa o modelo sem Python na máquina do usuário.

O checkpoint é **`convaiinnovations/laya-multilingual`** (mmBERT-base, 322M,
1024 tokens, 100+ idiomas, ~1,2 GB em ONNX). O `convaiinnovations/laya` raiz é
English-only e **não é exposto** (o multilingual cobre pt/en e ocupa menos
espaço).

> O link `https://laya.aay.sh/` é o app Laya. Ele **não** é usado: o router de
> eventos do app classifica persona/prioridade de notificações e cria cards.
> O classificador reutilizável é o modelo `convaiinnovations/laya*` + o SDK
> `laya-onnx`.

## Pré-requisito de build (uma vez, não no runtime)

O ONNX ainda não está publicado no HF. Exportar e publicar o artefato:

```bash
# no checkout de github.com/NandhaKishorM/laya
python laya-rust/tools/export_onnx.py --model multilingual --out ./onnx
# publicar os arquivos de ./onnx/multilingual + manifest.json/catalog.json
```

Layout do artefato:

```
laya-multilingual-<version>/
  model.onnx
  model.onnx.data
  rl_agent_config.json
  tokenizer/tokenizer.json
  tokenizer/tokenizer_config.json
  manifest.json
```

## Entregáveis

### Cargo/feature

- `src-tauri/Cargo.toml` (crate via submódulo `vendor/laya`):
  ```toml
  [features]
  laya = ["dep:laya-onnx", "dep:ort"]

  [dependencies]
  laya-onnx = { path = "../vendor/laya/laya-rust", optional = true, default-features = false }
  ort = { version = "=2.0.0-rc.13", optional = true }
  ```
- `rust-version` do projeto subiu para **1.88** (edition 2024 + let-chains).
  O bump ativou lints de MSRV no Clippy; os 6 avisos pré-existentes foram
  corrigidos.
- O build padrão (`cargo build`) continua sem `laya-onnx`; só
  `cargo build --features laya` inclui o backend.

### Domínio/porta do model manager

- `src-tauri/src/domain/classifier_model.rs`:
  - `ClassifierModelManifest` (schema, model_id, version, checkpoint, layout,
    size, license, files[] (path/url/size/sha256), calibration, metrics,
    created_at);
  - `InstalledClassifierModel`.
- `src-tauri/src/ports/classifier_model_manager.rs`:
  - `list_installed`, `active`, `catalog`, `install`, `activate`, `remove`.
- `src-tauri/src/adapters/models/local_classifier_model_manager.rs`:
  - diretório `app_data/models/laya`;
  - download arquivo a arquivo (`files[]`) via `ureq` com `.part`, `Range`
    opcional, progresso; sem tar/zip;
  - verificação SHA-256 por arquivo; `<version>.part/` → `<version>/` atômico;
    ponteiro `current`;
  - evento `suggestion_model_download` (`downloading`, `verifying`,
    `installing`, `ready`, `failed` + bytes);
  - erros tipados (`Unavailable`, `ChecksumMismatch`, `NoSpace`, `Cancelled`).
- `src-tauri/src/adapters/events/tauri_event_publisher.rs`: evento de progresso.
- `src-tauri/src/ipc/suggestion_commands.rs` (ou novo `model_commands.rs`):
  - `classifier_model_catalog`, `classifier_model_installed`,
    `classifier_model_install`, `classifier_model_activate`,
    `classifier_model_remove`.
- `src-tauri/src/lib.rs`: registrar comandos e wiring do manager.

### Adaptador Laya

- `src-tauri/src/adapters/ai/laya_suggestion_cue_classifier.rs` (novo):
  - worker thread dono do `LayaEngine` (carregado uma vez, `Send + Sync`);
  - fila `std::sync::mpsc` + `recv_timeout` (timeout → `ClassifierError::Timeout`);
  - caminho do modelo: `suggestion_classifier_model_path` (dev) ou
    `ClassifierModelManager::active()` (usuário final);
  - `LayaOptions { model_directory, checkpoint: Multilingual, allow_download: false, .. }`;
  - `QuestionSet` com **uma** pergunta `choice` (`cue`, 7 opções);
  - `PredictOptions::new().min_confidence(MinConfidence::scalar(threshold)?)`;
  - `let answer = result.get("cue")`; `let choice = answer.as_choice()`;
  - `confidence = answer.answer_confidence()`;
  - `abstained = answer.abstention() == Some("abstained") ||
    answer.low_confidence()`;
  - `should_suggest = !abstained && confidence >= threshold && cue.can_suggest()`;
  - `evidence` da última fala relevante; `focus` por cue;
  - erros tipados (`Unavailable`, `Timeout`, `InvalidResponse`);
  - `backend_id() = "laya"`;
  - logs só metadados.
- `src-tauri/src/adapters/ai/mod.rs`: registrar com `#[cfg(feature = "laya")]`.
- `src-tauri/src/lib.rs`: selecionar backend; sem a feature, `"laya"` cai para
  `heuristic` com aviso.
- `src-tauri/src/state.rs`: `Arc<dyn ClassifierModelManager>`.

### POC / avaliação

- `src-tauri/examples/laya_eval.rs` (feature-gated):
  - lê `poc/laya-classifier/fixtures/blocks.jsonl`;
  - usa o `LayaEngine` + o mapeamento do adaptador real;
  - calcula acurácia de `should_suggest`, P/R/F1 por cue, matriz de confusão,
    p50/p95;
  - imprime JSON + tabela Markdown para o `memory.md`.
- `poc/laya-classifier/fixtures/blocks.jsonl`:
  - ≥ 24 blocos rotulados (pt/en), balanceados entre as 7 cues;
  - campos `state`, `context`, `label_cue`, `label_should_suggest`.
- `poc/laya-classifier/README.md`:
  - export ONNX + empacotamento/publicação do artefato;
  - estrutura do diretório;
  - como rodar o eval;
  - aviso de que Python só é usado no export.

## Critérios de aceite

- [x] `cargo build` (sem feature) continua funcionando e não baixa modelo.
- [x] `cargo build --features laya` compila com `laya-onnx` e Rust ≥ 1.88.
      Submódulo `vendor/laya` + feature `laya` + pin `ort = rc.13`; o build sem
      a feature continua sem compilar o engine.
- [x] `LayaEngine` carrega o ONNX `laya-multilingual` do caminho ativo.
      Binding `OnnxLayaPredictor` (carga lazy, `allow_download=false`, token
      suprimido); validado até a carga, **sem** o artefato real ainda.
- [x] **Nenhum** processo Python, `laya-serve`, token ou `POST /v1/systemone`
      em runtime.
- [x] Uma única pergunta `choice` (`cue`); sem `noul`.
- [x] `should_suggest` derivado de cue + `answer_confidence` + abstenção.
- [x] Modelo ausente/inválido → `Unavailable`; a UI mostra erro e o heurístico
      continua disponível.
- [x] Model manager lê o catálogo (`catalog.json`), baixa sob confirmação,
      verifica SHA-256, instala atomicamente, ativa e remove versões.
- [x] Download interrompido/corrompido não deixa modelo parcial ativo.
- [x] Sem download silencioso; `auto_download=false` (o manager só baixa em
      `install`; a UI exige confirmação).
- [x] Timeout do worker → `Timeout`; o scheduler não fica preso.
- [x] Nenhum texto de bloco/token em log.
- [ ] O eval Rust reproduz as métricas e respeita o gate: acurácia ≥ 80%,
      FP ≤ 15%, p95 ≤ 1 s CPU / 300 ms GPU.
      **Parcial:** `eval_laya_on_fixtures` (ignorado, feature `laya`) e o
      baseline heurístico prontos; falta o artefato ONNX para rodar o real.
- [ ] Resultado registrado em `memory.md` com checkpoint, hardware, versão do
      `laya-onnx` e `model_version`. **Bloqueado** junto com o engine.

## Testes

- [x] `laya_maps_choice_to_cue_and_confidence` (mapeamento puro)
- [x] `laya_abstention_maps_to_insufficient_context`
- [x] `laya_should_suggest_is_false_for_no_suggestion`
- [x] `laya_low_confidence_abstains`
- [x] `laya_below_threshold_does_not_suggest`
- [x] `laya_unknown_choice_maps_to_insufficient_context`
- [x] `laya_missing_model_is_unavailable`
- [x] `laya_dev_override_wins_over_the_active_model`
- [x] `laya_timeout_returns_timeout_without_blocking`
- [x] `laya_does_not_log_block_text`
- [x] `falls_back_to_the_heuristic_when_laya_is_unavailable`
- [x] `uses_laya_when_selected_and_available`
- [x] `reads_catalog_without_downloading_model_files`
- [x] `installs_and_activates_a_model`
- [x] `rejects_bad_checksum_and_leaves_nothing_active`
- [x] `failed_download_keeps_the_previous_active_version`
- [x] `never_downloads_without_an_explicit_install`
- [x] `refuses_to_remove_the_active_version`
- [x] `lists_installed_versions_with_the_active_flag`
- [x] `rejects_unknown_versions_with_a_typed_error`
- [x] `laya_eval_fixture_shape`
- [x] `eval_heuristic_baseline_on_fixtures` (`#[ignore]`, baseline)
- [x] `eval_laya_on_fixtures` (`#[ignore]`, feature `laya`; exige `LAYA_ONNX_DIR`)
- [ ] Rodar o eval real com o artefato exportado.

### Frontend
- [x] `ClassifierModelManager` (7 testes): recomendado, confirmação antes do
      download, ativo, ativar, remover, erro/retry, progresso.

## Como testar

### Model manager (já funciona)

```bash
pnpm tauri dev
```

Fontes de IA & Modelos → card “Modelo de classificação” → escolher
`Laya (Padrão)` → o cartão do modelo mostra “Nenhum modelo instalado” +
“Baixar e ativar”. Com um catálogo publicado, o fluxo baixa/verifica/instala/
ativa e o progresso aparece na hora.

Sem o ONNX real, gere um artefato falso e aponte a *URL do catálogo* (Avançado)
para ele:

```bash
./poc/laya-classifier/make_dev_artifact.sh
```

O manager aceita caminho absoluto/`file://`, então o fluxo inteiro roda local;
o `model.onnx` falso serve só para UI/progresso.

### Eval (baseline heurístico)

```bash
cd src-tauri
cargo test --lib eval_heuristic_baseline_on_fixtures -- --ignored --nocapture
```

### Build com o engine

```bash
git submodule update --init
pnpm tauri dev --features laya
```

Sem modelo ativo, `Laya (Padrão)` cai em `Regras locais`. Com um modelo
instalado/ativado, reinicie para o engine carregar (a carga é lazy na primeira
classificação).

### Eval real (quando o artefato existir)

```bash
# exportar ONNX uma vez (Python ≥3.10, ~5 GB livres; ver README do POC)
python3 vendor/laya/laya-rust/tools/export_onnx.py --model multilingual --out ./onnx

LAYA_ONNX_DIR=./onnx/multilingual \
  cargo test --lib eval_laya_on_fixtures --features laya -- --ignored --nocapture
```

## Verificação (2026-10-26)

- Rust **814 passed** sem a feature; **817 passed** com `--features laya`
  (11/15 ignored); clippy/fmt limpos nos dois.
- Frontend **518 passed**; `tsc --noEmit` e `pnpm build` limpos.
- Baseline heurístico nas fixtures: acurácia `should_suggest` **92,9%**,
  FP **0%**, FN **10%**, acurácia de cue **53,6%** (28 blocos, 4 por cue).
- O modelo é ~1,2 GB: nunca entra no bundle; fica no model manager.

## Desvios registrados

1. **`laya-onnx` via submódulo**, não crates.io. O crate não está publicado e o
   repo upstream não tem `Cargo.toml` na raiz. Decisão: submódulo
   `vendor/laya` + `path = "../vendor/laya/laya-rust"` (Apache-2.0) atrás da
   feature `laya`; sem conta/publicação. O `ort` foi pinado em `=2.0.0-rc.13`
   (o SDK foi escrito contra rc.13; o resolver puxava rc.15).
2. **Artefato ainda não existe** (ISS-059): o ONNX não está no HF e o export
   exige Python ≥3.10 + ~5 GB + torch ≥2.9. Não foi executado nesta máquina
   (Python 3.9, ~7,8 GB livres). O binding e o eval real estão prontos para
   rodar quando o artefato existir.
3. **`examples/laya_eval.rs` → teste `#[ignore]`** dentro do crate: os módulos
   de `lib.rs` são privados, então um example não enxerga domínio/adaptadores.
   As fixtures e as métricas são as mesmas; o eval real usa
   `eval_laya_on_fixtures`.
4. **Download sem cancelamento no MVP**: o `.part` é sempre limpo em erro; não
   há botão “cancelar” (o enum `Cancelled` já existe para a evolução).

## Notas

- **MSRV:** `rust-version` subiu para **1.88** (edition 2024 do SDK). O bump
  ativou lints de MSRV no Clippy; os 6 avisos pré-existentes foram corrigidos
  (`repeat_n`, `is_none_or`, `as_chunks`, `is_multiple_of`).
- **ORT:** o `ort` pode baixar o binário do ONNX Runtime no build. Avaliar
  vendorizar/`load-dynamic` para o release; documentar o build offline.
- **Modelo:** o `laya-multilingual` é **não calibrado** (ECE ~0,31) e mais fraco
  em English que o checkpoint English. Calibrar o limiar nas fixtures; o
  English não é exposto no produto.
- **`noul`:** não usar; o model card documenta viés de rótulo. A decisão vem da
  `choice` + confiança.
- Se o gate falhar, **não** trocar o padrão; manter `heuristic` e registrar a
  hipótese.
- O model manager é o caminho do usuário final; o override `model_path` é dev.
