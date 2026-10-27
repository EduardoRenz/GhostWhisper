# POC — classificador Laya no GhostWhisper

Classificador de cue do modo de sugestão (BR-094/BR-098). O app roda **um**
modelo ONNX local sobre o bloco de conversa e decide **se** deve sugerir; o
gerador existente continua escrevendo o texto.

## Status (2026-10-26)

| Parte | Estado |
| --- | --- |
| Contrato/mapeamento (`choice` → cue, abstenção, limiar) | ✅ implementado e testado |
| Worker com timeout (o scheduler nunca fica preso) | ✅ implementado e testado |
| Model manager (catálogo, SHA-256, `.part`, `current`, ativar/remover) | ✅ implementado e testado |
| UI de download/ativação (confirmação explícita) | ✅ implementada |
| Engine `laya-onnx` (submódulo `vendor/laya`) | ✅ feature `laya` + binding `OnnxLayaPredictor` |
| Artefato ONNX `laya-multilingual` | ✅ exportado em `src-tauri/resources/laya/multilingual` (não versionado) |
| Eval bruto do Laya real (5 cues) | ❌ **60% de decisão** (ISS-062); fine-tuning pendente |
| Caminho efetivo do app (Laya + guarda) | ✅ **100% de decisão** / 100% de cue nas 20 fixtures |
| AppImage Vulkan + Laya + modelo | ✅ `pnpm tauri:build:laya` |

### Como o engine entra no build

O crate **não está no crates.io**, então ele vive num submódulo git:

```bash
git submodule update --init
```

```toml
[features]
laya = ["dep:laya-onnx", "dep:ort"]

[dependencies]
laya-onnx = { path = "../vendor/laya/laya-rust", optional = true, default-features = false }
ort = { version = "=2.0.0-rc.13", optional = true }  # pin: o SDK exige rc.13
```

- `rust-version` do projeto: **1.88** (edition 2024 do SDK).
- `default-features = false` desliga o download HTTP do SDK; o modelo vem do
  model manager.
- O `ort` ainda baixa o binário do ONNX Runtime no build; no release,
  vendorizar/`load-dynamic`.
- Sem a feature, o app compila e roda só com `Regras locais`.

Build:

```bash
cargo build --lib --features laya
pnpm tauri dev --features laya
```

O `OnnxLayaPredictor` resolve o diretório no boot (override > modelo instalado
> **modelo empacotado** em `<resource_dir>/laya/multilingual`) e aquece o engine
em background. Sem modelo, selecionar `Laya (Padrão)` cai em `Regras locais`
com aviso.

## Export do artefato (Python só aqui, nunca no runtime)

O ONNX ainda não está no Hugging Face; o export roda **uma vez** (Python ≥3.10,
~5 GB livres, ~2 min de CPU nesta máquina):

```bash
# o submódulo já tem o script
python3 vendor/laya/laya-rust/tools/export_onnx.py --model multilingual \
  --out src-tauri/resources/laya
```

> Executado em 2026-10-26 com Python 3.12 + `uv` + torch CPU:
> `src-tauri/resources/laya/multilingual` (`model.onnx.data` 1,29 GB). O
> diretório está no `.gitignore` (não versionar 1,3 GB).

Layout publicado (uma pasta por versão):

```text
laya-multilingual-0.1.0/
  model.onnx
  model.onnx.data
  rl_agent_config.json
  tokenizer/tokenizer.json
  tokenizer/tokenizer_config.json
  manifest.json   # gravado pelo model manager, não pelo exporter
```

`manifest.json` segue `ClassifierModelManifest` (domínio). Ele **não** entra em
`files[]`: o model manager grava esse arquivo a partir do catálogo depois de
verificar os demais arquivos. Campos:
`schema`, `model_id`, `version`, `checkpoint`, `layout`, `size_bytes`, `license`,
`files[]` (`path` relativo, `url`, `size_bytes`, `sha256`), `calibration`,
`metrics`, `created_at`.

O `catalog.json` aponta para os manifestos e é lido pelo app em
`suggestion_classifier_model_catalog_url` (padrão:
`https://downloads.ghostwhisper.app/laya/catalog.json` — **placeholder**, trocar
antes do release). O primeiro modelo do catálogo é o recomendado.

Exemplos de forma (hashes/URLs fictícios — os `sha256` precisam ser substituídos
ou a validação falha): [`manifest.example.json`](manifest.example.json) e
[`catalog.example.json`](catalog.example.json).

## Model manager

- Diretório: `app_data/models/laya`.
- Download por arquivo para `laya-multilingual-<versão>.part/`; cada arquivo é
  conferido por tamanho + SHA-256.
- Só depois do último checksum o diretório é renomeado (atômico).
- `current` (ponteiro) é gravado por `current.part` + rename.
- Um download interrompido/corrompido apaga o `.part` e **não** toca na versão
  ativa.
- `install` **não** ativa; o comando IPC `classifier_model_install` instala e
  ativa (uma ação do usuário).
- Sem download silencioso: a UI mostra o tamanho e exige “Baixar”.
- A versão ativa não pode ser removida.

## Testar a UI sem o ONNX real (dev)

O model manager aceita um caminho absoluto/`file://` além de HTTP. Para
exercitar catálogo → download → SHA-256 → `.part` → ativar → remover sem baixar
1,2 GB:

```bash
./poc/laya-classifier/make_dev_artifact.sh
# imprime o caminho de catalog.json
```

No app: Configurações → **Fontes de IA & Modelos** → **Avançado** → *URL do
catálogo* = `<diretório>/catalog.json` → salvar → card do modelo → **Baixar e
ativar**.

O `model.onnx` gerado é **falso** (texto); serve só para a UI/progresso. O
engine existe no build com `--features laya`, mas não consegue carregar um ONNX
falso — use `Regras locais` até ter o artefato real.

## Eval

### Taxonomia atual (5 cues)

`question`, `comment`, `already_answered`, `no_action`,
`insufficient_context`. Só as duas primeiras geram sugestão. Silêncio e
hesitação foram removidos (BR-094).

### Baseline atual (heurístico)

```bash
cd src-tauri
cargo test --lib eval_heuristic_baseline_on_fixtures -- --ignored --nocapture
```

Resultado de 2026-10-27 (24 blocos; 5 `comment`, 6 `no_action`, 5 `already_answered`, 4 `question`/`insufficient_context`):

```text
Decisão: 100.0% · FP: 0.0% · FN: 0.0% · cue: 100.0% · p50/p95: 0 ms
```

As fixtures foram desenhadas para cobrir as regras (pergunta, convite de
comentário, repetição, small talk/explicação, contexto curto).

### Laya real (bruto, executado)

```bash
LAYA_ONNX_DIR=src-tauri/resources/laya/multilingual \
  cargo test --lib eval_laya_on_fixtures --features laya -- --ignored --nocapture
```

Resultado de 2026-10-27 (24 blocos, prompt com contexto/alvo, limiar 0,50):

```text
Decisão: 60.0% · FP: 25.0% · FN: 62.5%
Cue: 30.0% · p50: 626 ms · p95: 767 ms
```

O modelo zero-shot ainda confunde `comment` com `question`, inventa
`already_answered` sem histórico e abstém em casos claros. **Gate bruto não
atingido** (≥80% / ≤15%); o próximo passo é fine-tuning (task 12), não mais
prompt.

### Caminho efetivo do app (Laya + guarda)

O seletor `SettingsSuggestionCueClassifier` sempre calcula as regras locais e
usa o Laya como classificador contextual, com duas guardas determinísticas:

- pergunta repetida (`already_answered` nas regras) nunca é sugerida de novo;
- se o Laya abstém, diz `no_action` ou inventa `already_answered` sem evidência,
  as regras locais decidem; caso contrário, a decisão positiva do Laya é
  preservada.

Resultado no teste de integração (24 fixtures):

```text
Decisão: 100.0% · FP: 0.0% · FN: 0.0% · cue: 100.0% · p95: 599 ms
```

Todas as 20 fixtures ficaram corretas no caminho efetivo; a guarda local
também corrigiu o convite de comentário que o Laya chamava de `question`.

### Testes de integração por situação

Além do eval agregado, `laya_integration` executa as 20 fixtures uma a uma e
imprime a cue/confiança/decisão/backend de cada situação. O teste usa o
**classificador efetivo do app** (Laya + guarda local):

```bash
cd src-tauri
LAYA_ONNX_DIR=$PWD/resources/laya/multilingual \
  cargo test --lib laya_integration --features laya -- --ignored --nocapture
```

O teste padrão é **relatório**; com `LAYA_ENFORCE_GATE=1` ele aplica o gate
(≥80% de decisão / ≤15% de FP). O resultado efetivo de 2026-10-26 (agent context
de Entrevista Técnica, adaptive, threshold balanceado 0,50):

```text
Decisão 100,0% · FP 0,0% · FN 0,0% · cue 100,0% · p95 599 ms
question 4/4 · comment 4/4 · already_answered 4/4 · no_action 4/4
insufficient_context 4/4
```

A latência p95 mistura Laya (~600 ms) com as decisões das regras locais (0 ms).
O gate bruto do modelo continua reprovado; o gate do **caminho efetivo** passa
por causa da guarda determinística.

### AppImage com Laya

```bash
git submodule update --init
python3 vendor/laya/laya-rust/tools/export_onnx.py --model multilingual \
  --out src-tauri/resources/laya
pnpm tauri:build:laya
```

Gera `src-tauri/target/release/bundle/appimage/GhostWhisper_0.1.0_amd64.AppImage`
(~821 MiB) com Vulkan + `laya-onnx` + o modelo em
`<resource_dir>/laya/multilingual`. O build default continua sem Laya; o
`pnpm tauri:build:laya` exige o artefato real (o fake de dev é recusado).

> O AppImage monta os resources como **root**; `model.onnx.data` precisa ser
> **0644** (o snapshot do HF vem 0600). O script faz `chmod a+rX` na origem e
> no target; sem isso o engine falha com permissão negada (ISS-063).

## Fixtures

[`fixtures/blocks.jsonl`](fixtures/blocks.jsonl): 22 blocos rotulados (pt/en);
4 por cue e um extra em `already_answered`/`no_action`. Campos:

```json
{"id":"q01","locale":"pt","state":"ME: ...\nTHEY: ...","context":"",
 "gap_ms":0,"prior_questions":[],"label_cue":"question","label_should_suggest":true}
```

`prior_questions` alimenta o histórico de já respondidas. `gap_ms` é mantido
por compatibilidade, mas a taxonomia nova não usa silêncio. O teste
`laya_eval_fixture_shape` valida o shape e o balanceamento.

## Privacidade

- O estado enviado ao modelo é o bloco + contexto (a própria conversa); não sai
  da máquina.
- Logs só têm metadados (cue, confiança, bloco, latência). Nenhum texto de
  transcrição é logado.
- O artefato é baixado de um servidor de downloads; nenhum áudio/texto é
  enviado no download.

## Referências

- PRD: [`../../prds/suggestion-mode/prd.md`](../../prds/suggestion-mode/prd.md)
- Arquitetura: [`../../prds/suggestion-mode/architecture/suggestion-gating.md`](../../prds/suggestion-mode/architecture/suggestion-gating.md)
- Fine-tuning: [`../../prds/suggestion-mode/architecture/fine-tuning.md`](../../prds/suggestion-mode/architecture/fine-tuning.md)
