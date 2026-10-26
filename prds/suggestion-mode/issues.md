# Issues — PRD Modo de Sugestão

Registry de bugs e limitações deste PRD. Convenções em
[`../../.agents/rules.md`](../../.agents/rules.md) (`ISS-NNN` é canônico por
arquivo de PRD; a numeração **continua** a dos PRDs anteriores para o ID ser
único no repositório). Próximo ID livre: **ISS-064**.

| ID | Resumo | Severidade | Status |
|---|---|---|---|
| ISS-059 | Artefato ONNX do Laya não publicado | Alta | OPEN |
| ISS-060 | Gate usava relógio de parede com timestamps relativos | Alta | FIXED |
| ISS-061 | Heurístico ignorava hesitação em bloco só de ME | Média | FIXED |
| ISS-062 | Laya zero-shot abaixo do gate de qualidade | Alta | OPEN |
| ISS-063 | AppImage montava model.onnx.data 0600 root e o engine falhava | Alta | FIXED |

---

## ISS-059 — Artefato ONNX do Laya não publicado

- **Severidade:** Alta · **Status:** OPEN (bloqueio de dados, não de conta)
- **Sintoma:** com `--features laya` e sem modelo ativo, selecionar
  `Laya (Padrão)` cai em `Regras locais`; o eval do Laya real não roda.
- **Causa raiz:** o checkpoint `convaiinnovations/laya-multilingual` só publica
  safetensors; o ONNX ainda não está no HF. O export exige Python ≥3.10,
  `torch>=2.9` e ~5 GB livres (não roda nesta máquina: Python 3.9, ~7,8 GB).
- **Resolvido:** o crate `laya-onnx` entrou pelo submódulo `vendor/laya`
  (Apache-2.0) + feature `laya`; `rust-version = 1.88`; `ort` pinado em
  `=2.0.0-rc.13`. Binding `OnnxLayaPredictor` implementado (carga lazy,
  `allow_download=false`, token suprimido).
- **Correção pendente:** rodar o export uma vez
  (`python3 vendor/laya/laya-rust/tools/export_onnx.py --model multilingual --out ./onnx`),
  publicar o layout + `catalog.json` e rodar
  `eval_laya_on_fixtures` com `LAYA_ONNX_DIR`.
- **Mitigação:** model manager completo (catálogo, SHA-256, `.part`, `current`),
  fallback heurístico com aviso, UI de erro e artefato falso de dev
  (`make_dev_artifact.sh`).
- **Arquivos:**
  `src-tauri/src/adapters/ai/laya_onnx_predictor.rs`,
  `src-tauri/src/adapters/ai/laya_suggestion_cue_classifier.rs`,
  `src-tauri/src/adapters/models/local_classifier_model_manager.rs`,
  `poc/laya-classifier/`.
- **Task:** [`tasks/06-task.md`](tasks/06-task.md)

## ISS-060 — Gate usava relógio de parede com timestamps relativos

- **Severidade:** Alta · **Status:** FIXED (2026-10-26)
- **Sintoma:** no modo classificador, “Classificar agora”/“Testar classificador”
  retornava `Classificador indisponível: configuração inválida: nenhum bloco
  classificável` mesmo com falas dos dois lados na sessão.
- **Causa raiz:** `SuggestionGateService` usava `clock.now_ms()` (época Unix,
  ~1,79e12) como `now`/`since`, mas `transcriptions.end_ms` é **relativo ao
  início da sessão** (0..N ms). O `WHERE end_ms >= since` descartava todas as
  falas; `build_blocks` também via todos os blocos como fechados.
- **Correção:** `relative_now(session_id)` = `clock.now_ms() -
  session.started_at_ms` (via `SessionRepository`); a janela, `build_blocks`,
  silêncio e sonda usam o tempo relativo. `classified_at_ms` continua em época
  Unix (a UI mostra “Há 8s”). Erro `NoBlock` agora vira
  `NoClassifiableBlock` (“fale dos dois lados e aguarde o silêncio…”), não
  “configuração inválida”.
- **Testes:** `uses_session_relative_time_for_transcriptions` (falharia antes),
  `build_blocks_is_fast_for_forty_segments`.
- **Arquivos:** `src-tauri/src/application/suggestion_gate_service.rs`,
  `src-tauri/src/error.rs`, `src-tauri/src/ipc/suggestion_commands.rs`.
- **Task:** [`tasks/10-task.md`](tasks/10-task.md) / [`tasks/11-task.md`](tasks/11-task.md)

## ISS-061 — Heurístico ignorava hesitação em bloco só de ME

- **Severidade:** Média · **Status:** FIXED (2026-10-26)
- **Sintoma:** depois de THEY falar, ME diz “Eu não tenho certeza” / “deixa eu
  pensar”; a faixa mostrava `insufficient_context`/`no_suggestion` e nenhuma
  sugestão era gerada no modo `Regras locais`.
- **Causa raiz:** `heuristic_cue` retornava `InsufficientContext` logo no início
  quando o bloco não tinha os dois falantes — antes de avaliar hesitação. A
  sonda aberta (BR-093) entrega um bloco só de ME, então a hesitação nunca era
  detectada. Os marcadores também não incluíam incerteza explícita.
- **Correção:** hesitação é avaliada antes da exigência dos dois falantes; novo
  `has_uncertainty` (“não tenho certeza”, “deixa eu pensar”, “não sei”, “let me
  think”…). Bloco só de ME sem marcador continua `insufficient_context`.
- **Extra:** `classify_once` agora reusa a decisão positiva que ficou no
  cooldown (`take_pending`) antes de reclassificar; antes o `pending` do gate
  nunca era consumido pelo scheduler.
- **Testes:** `heuristic_detects_uncertainty_in_me_only_block`,
  `heuristic_me_only_without_markers_is_insufficient`,
  `reuses_a_pending_positive_decision_before_classifying`.
- **Arquivos:** `src-tauri/src/domain/suggestion_gate.rs`,
  `src-tauri/src/application/classifier_tick.rs`.
- **Task:** [`tasks/05-task.md`](tasks/05-task.md) / [`tasks/10-task.md`](tasks/10-task.md)

## ISS-062 — Laya zero-shot abaixo do gate de qualidade

- **Severidade:** Alta · **Status:** OPEN (qualidade; engine/modelo ok)
- **Sintoma:** com o `laya-multilingual` real e a pergunta `cue`, o eval nas 28
  fixtures ficou em **60,7%** de acurácia de `should_suggest`, FP **12,5%**,
  FN **50%** e acurácia de cue **21,4%** (gate: ≥80%/≤15%). O modelo confunde
  `invitation`, `lost_thread`, `silence` e `no_suggestion`, e é enviesado a
  positivos; `noul` e uma pergunta binária também falharam (8/28).
- **Causa raiz:** o checkpoint é um modelo de decisão tipada treinado em
  schemas próprios; a taxonomia de 7 cues do GhostWhisper é zero-shot e não
  foi treinada. Descrições/instruções em inglês melhoraram pouco.
- **Mitigação:** engine + artefato ONNX + AppImage prontos (p50 377 ms, p95
  521 ms); heurístico segue padrão e a UI avisa que Laya é POC. O adaptador e o
  eval ficam disponíveis.
- **Correção pendente:** fine-tuning do head/checkpoint na taxonomia (task 12)
  com dataset rotulado real; reavaliar antes de ativar por padrão.
- **Arquivos:** `src-tauri/src/adapters/ai/laya_suggestion_cue_classifier.rs`,
  `poc/laya-classifier/fixtures/blocks.jsonl`,
  `src-tauri/resources/laya/multilingual/` (não versionado).
- **Task:** [`tasks/06-task.md`](tasks/06-task.md) / [`tasks/12-task.md`](tasks/12-task.md)

## ISS-063 — AppImage montava `model.onnx.data` 0600 root

- **Severidade:** Alta · **Status:** FIXED (2026-10-26)
- **Sintoma:** com Laya selecionado no AppImage, o cabeçalho ficava em “Falha”
  e nenhuma classificação era gerada; no terminal, o log do engine citava
  permissão ao ler o modelo.
- **Causa raiz:** o snapshot do HF entrega `model.onnx.data` com modo `0600`.
  O AppImage monta os resources como **root**, então o processo (usuário) não
  conseguia ler o sidecar de 1,29 GB; `LayaEngine::create` virava
  `Unavailable` e o gate publicava erro.
- **Correção:** `chmod -R u+w,a+rX` no artefato e no target antes do bundle
  (`scripts/tauri-build-laya.sh`); o AppDir agora tem `model.onnx.data` 0644.
  AppImage recompilado.
- **Arquivos:** `scripts/tauri-build-laya.sh`,
  `src-tauri/resources/laya/multilingual/` (não versionado).
- **Task:** [`tasks/06-task.md`](tasks/06-task.md) / [`tasks/11-task.md`](tasks/11-task.md)

<!--
Modelo:

## ISS-064 — Título curto

- **Severidade:** Alta/Média/Baixa · **Status:** OPEN/FIXED
- **Sintoma:** …
- **Causa raiz:** …
- **Correção:** …
- **Arquivos:** …
- **Task:** [`tasks/NN-task.md`](tasks/NN-task.md)
-->
