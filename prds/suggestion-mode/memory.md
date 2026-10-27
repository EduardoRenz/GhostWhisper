# Memory — PRD Modo de Sugestão

Histórico cronológico deste PRD. Formato em
[`../../.agents/rules.md`](../../.agents/rules.md) §6.2 (R15, BR-017). Entradas
mais recentes no fim.

## 2026-10-26 — PRD criada: modo de sugestão com classificador

- **IDs:** BR-092, BR-093, BR-094, BR-095, BR-096, BR-097, BR-098, BR-099,
  BR-100
- **Refs:** [`prd.md`](prd.md),
  [`architecture/suggestion-gating.md`](architecture/suggestion-gating.md),
  [`tasks/_tasks.md`](tasks/_tasks.md)

### Pedido
Criar um modo de sugestão em que o usuário escolhe entre o automático atual e um
**agente classificador** que decide, por blocos de conversa, se deve sugerir algo
(pergunta, hesitação, perda do fio, convite, silêncio, nada a sugerir). A
configuração deve viver no topo da aba “Fontes de IA & Modelos”, o cabeçalho da
janela de Sugestões deve mudar e a última classificação deve ficar sempre
visível. POC com o Laya, com modelo distribuído para usuário final.

### Decisões
- `suggestion_auto_refresh` continua como interruptor mestre; novo
  `suggestion_mode` (`auto` | `classifier`), padrão `auto`.
- O classificador **decide**; o gerador existente **escreve** (cue + foco).
- Bloco **“Modo de ativação de sugestões”** no topo de
  **“Fontes de IA & Modelos”** (dois cards: classificador vs. tempo); o
  avançado fica em drawer/modal. Sem aba “Sugestões” no MVP; o card de
  sugestões sai de “Geral & HUD”.
- Cabeçalho adaptativo: timer no automático, status do classificador no
  classificador.
- Faixa “Última classificação” sempre visível (por padrão), inclusive em decisão
  negativa.
- POC usa o **SDK Rust `laya-onnx`** dentro do backend (ONNX local, sem Python
  em runtime), com o checkpoint **`convaiinnovations/laya-multilingual`**;
  o router de eventos do app aay.sh não é usado. O ONNX é exportado uma vez do
  checkpoint PyTorch.
- Distribuição para usuário final: artefato ONNX versionado com `manifest.json`
  + SHA-256, baixado pelo model manager sob confirmação, instalação atômica e
  rollback (BR-099).
- Fine-tuning futuro: mesmo layout ONNX, mesma pergunta `cue`, mesmo manifesto e
  eval; dados de treino só opt-in/local (BR-100).
- Gate da POC: acurácia ≥ 80%, FP ≤ 15%, p95 ≤ 1 s CPU / 300 ms GPU.

### Próximo passo
Aprovar a PRD e executar as tasks 01→12 na ordem de
[`tasks/_tasks.md`](tasks/_tasks.md).

---

## 2026-10-26 — Task 01: domínio de blocos, cues e decisão pura

- **IDs:** BR-092, BR-093, BR-094 · **Refs:** [`tasks/01-task.md`](tasks/01-task.md)

### Implementação
- Novo `src-tauri/src/domain/suggestion_gate.rs`:
  - `SuggestionMode`, `SuggestionCue` e strings estáveis;
  - `BlockTurn`, `ConversationBlock`, `BlockConfig` e `build_blocks`;
  - `HeuristicConfig` + `heuristic_cue` (pergunta, hesitação, silêncio,
    nada/insuficiente);
  - `GateTick`, `GateDecision`, `GateSkipReason` e `decide`.
- `src-tauri/src/domain/mod.rs` registra o módulo.
- `#![allow(dead_code)]` temporário: o módulo será consumido pelas tasks 02/03;
  remover quando o gate estiver ligado ao scheduler.

### Testes
- 20 testes no módulo: fechamento por gap/duração/turnos, id determinístico,
  classificação ME+THEY, heurística pt/en, silêncio, trailing, decisão.
- `cargo test --lib` **721 passed**, 10 ignored; clippy/fmt limpos.

---

## 2026-10-26 — Task 02: porta do classificador, evento e fake

- **IDs:** BR-094, BR-095 · **Refs:** [`tasks/02-task.md`](tasks/02-task.md)

### Implementação
- `src-tauri/src/ports/suggestion_cue_classifier.rs`:
  - `ClassificationRequest` (bloco + contexto + locale + mode_id + sonda);
  - `ClassificationOutcome` com `normalized()` (confiança/evidência/foco/cues
    negativas);
  - `ClassifierError` e trait `SuggestionCueClassifier`.
- `src-tauri/src/ports/event_publisher.rs`:
  - `SuggestionClassifierState`, `SuggestionClassificationEvent` e
    `emit_suggestion_classification` (default no-op).
- `TauriEventPublisher` emite `suggestion_classification`.
- `FakeSuggestionCueClassifier` (cfg(test)) com fila de resultados/erros e
  requests registrados.

### Testes
- 7 testes da porta/fake + 1 do evento; `cargo test --lib` segue verde.
- `allow(dead_code)` temporário nos tipos do evento/porta até o gate da task 03.

---

## 2026-10-26 — Task 03: serviço de gate e cache da última classificação

- **IDs:** BR-093, BR-094, BR-095 · **Refs:**
  [`tasks/03-task.md`](tasks/03-task.md)

### Implementação
- `src-tauri/src/ports/suggestion_gate.rs`:
  - `SuggestionGateConfig` (bloco, heurístico, limiar, cooldown, allowlist,
    janela, sonda, locale/mode);
  - trait `SuggestionGate` (`classify_latest`, `last`, `pending`,
    `take_pending`, `should_generate`, `mark_generated`, `reset`).
- `src-tauri/src/application/suggestion_gate_service.rs`:
  - monta blocos com `build_blocks`, escolhe o último bloco fechado (ou aberto
    com `force_open`), chama o classificador, normaliza e publica
    `Analyzing`/`Ready`/`Error`;
  - cache da última classificação e da decisão pendente por sessão;
  - cooldown/dedupe/allowlist/confiança via `decide`.
- `TranscriptionsRepository::list_recent_by_session` + implementação SQLite
  (ordem cronológica, só finais, `limit` pega as últimas) e memória.
- Wiring no `AppState` **adiado para a task 10** (ainda não há classificador
  concreto no composition root).

### Testes
- 12 testes do serviço + 2 dos repositórios.
- `cargo test --lib` **743 passed**, 10 ignored; clippy/fmt limpos.
- `allow(dead_code)` temporário na porta/serviço até o scheduler da task 10.

---

## 2026-10-26 — Task 05: backend heurístico local

- **IDs:** BR-094 · **Refs:** [`tasks/05-task.md`](tasks/05-task.md)

### Implementação
- `src-tauri/src/adapters/ai/heuristic_suggestion_cue_classifier.rs`:
  - implementa `SuggestionCueClassifier` sobre `heuristic_cue`;
  - confiança determinística (0,90/0,80/0,75/0,60/0,40);
  - evidência da última fala relevante (THEY para pergunta; ME para
    hesitação/silêncio) e foco curto por cue;
  - `latency_ms = 0`, sem I/O nem relógio.
- `ClassificationRequest` ganhou `now_ms` para a heurística de silêncio; o
  serviço passa o instante do `Clock`.
- Demo end-to-end no teste do serviço:
  `cargo test --lib demo_prints_heuristic_classifications -- --nocapture`.

### Testes
- 6 testes do adaptador + demo; `cargo test --lib` **750 passed**, 10 ignored.
- Saída da demo: `question`/`Generate`, `hesitation`/`Generate`,
  `silence`/`Generate`, `no_suggestion`/`Skip(NoSuggestion)`.
- `allow(dead_code)` temporário até o backend ser escolhido no composition root
  (task 10).

---

## 2026-10-26 — Task 04: settings, migração, tipos e IPC

- **IDs:** BR-092, BR-095, BR-097 · **Refs:**
  [`tasks/04-task.md`](tasks/04-task.md)

### Implementação
- `domain/settings.rs`: campos do modo classificador (modo, sensibilidade,
  backend, modelo/versão/path/catálogo, checkpoint, cooldown, timeout,
  fallback, blocos, allowlist, sonda), defaults, `sanitized()` e validação.
  `suggestion_interval_secs` agora aceita 8–120 e o padrão é 15 s.
- `ports/suggestion_generator.rs`: `SuggestionTrigger::Classifier`,
  `Suggestion.cue`, `SuggestionRequest.cue/focus` + builder.
- `application/suggestion_gate_service.rs`: `config_from_settings` traduz as
  settings para `SuggestionGateConfig`.
- `state.rs`/`lib.rs`: `Arc<dyn SuggestionGate>` no `AppState`; composition root
  injeta o heurístico + gate.
- IPC: `suggestion_classification_last` e `suggestion_classify_now` (sem gerar
  sugestão).
- Frontend: campos/tipos em `core/domain/settings.ts` e `assistant.ts`;
  adaptador IPC `suggestionClassificationLast`/`suggestionClassifyNow`.

### Testes
- `cargo test --lib` **755 passed**, 10 ignored.
- `pnpm test` **485 passed**; `tsc --noEmit` limpo.
- Clippy/fmt limpos. Laya/model manager ainda não escolhidos (task 06/10).

---

## 2026-10-26 — Tasks 07–10: UI do classificador e agendador

- **IDs:** BR-095, BR-096, BR-097 · **Refs:**
  [`tasks/07-task.md`](tasks/07-task.md),
  [`tasks/08-task.md`](tasks/08-task.md),
  [`tasks/09-task.md`](tasks/09-task.md),
  [`tasks/10-task.md`](tasks/10-task.md)

### UI
- `useSuggestionClassification` assina `suggestion_classification`, hidrata
  pelo IPC e expõe `classifyNow`.
- `ClassifierSuggestControl` substitui o cronômetro no modo classificador.
- `LastClassificationStrip` sempre visível: vazio, auto, erro, cue/decisão,
  confiança, evidência, bloco, backend/latência; “Classificar agora”/retry.
- `SuggestionsActivationBlock` no topo de “Fontes de IA & Modelos”: dois cards
  (classificador vs. tempo), modelo/sensibilidade/cadência, avançado inline
  (cooldown, timeout, fallback, blocos, categorias, sonda, teste).
- `CadenceSettings` perdeu o card de sugestões; a aba `sources` virou
  “Fontes de IA & Modelos”.

### Agendador
- Branch `classifier` no `spawn_suggestion_scheduler`: gate por bloco, cooldown,
  pendência, geração com `SuggestionTrigger::Classifier` e cue/foco no request.
- `SuggestionService` respeita `request.trigger` e injeta cue/foco no prompt.
- Evento de classificação publica `generating` antes/depois da geração.

### Pendências da task 10
- Sonda de bloco aberto (hesitação) no scheduler.
- Fallback `auto` em erro do classificador.
- Teste de scheduler com fake/thread + manual no app.

### Testes
- Rust **755 passed**, 10 ignored; frontend **504 passed**; tsc/clippy/fmt
  limpos.
- Novos testes: `ClassifierSuggestControl` (6), `LastClassificationStrip` (6),
  `SuggestionsActivationBlock` (6), `SuggestionsWindow` header (1), e
  `CadenceSettings` sem o card de sugestões.

---

## 2026-10-26 — Task 06: model manager + adaptador Laya (engine bloqueado)

- **IDs:** BR-098, BR-099 · **Issues:** ISS-059 · **Refs:**
  [`tasks/06-task.md`](tasks/06-task.md),
  [`poc/laya-classifier/README.md`](../../poc/laya-classifier/README.md)

### O que entrou
- **Model manager** (`domain/classifier_model.rs`,
  `ports/classifier_model_manager.rs`,
  `adapters/models/local_classifier_model_manager.rs`):
  catálogo `catalog.json`, manifesto tipado com SHA-256 por arquivo, download
  para `<versão>.part/`, verificação de tamanho/hash, rename atômico, ponteiro
  `current`, ativar/remover (a ativa é protegida) e erros tipados
  (`Unavailable`, `ChecksumMismatch`, `NoSpace`, `Cancelled`, `ActiveModel`).
- **Evento** `suggestion_model_download` (fases `downloading`, `verifying`,
  `installing`, `ready`, `failed` + bytes).
- **IPC**: `classifier_model_catalog/installed/active/install/activate/remove`.
  `install` compõe instalar+ativar numa ação; nenhum download fora dela.
- **UI**: `ClassifierModelManager` no card do classificador — recomendado,
  tamanho/licença, confirmação explícita antes do download, progresso, versões
  instaladas com ativar/remover, erro/retry. 7 testes.
- **Adaptador Laya** (`laya_suggestion_cue_classifier.rs`): uma pergunta
  `choice` (`cue`, 7 opções), worker em thread própria com `try_send` +
  `recv_timeout` (timeout → `Timeout`), mapeamento `choice` → cue, abstenção/
  baixa confiança → `insufficient_context`, `should_suggest` = cue positiva +
  confiança ≥ limiar, evidência/foco e `render_state` com teto de 6k chars.
  O engine é a trait `LayaPredictor`.
- **Seleção em runtime**: `SettingsSuggestionCueClassifier` lê
  `suggestion_classifier_backend` a cada bloco; `laya` sem engine cai no
  heurístico com um aviso. `SuggestionGate::reconfigure` aplica
  sensibilidade/cues/cooldown salvos sem reiniciar (o scheduler chama a cada
  tick).
- **POC**: 28 fixtures rotuladas (4 por cue), métricas puras
  (`domain/classifier_eval.rs`: acurácia, FP/FN, P/R/F1, matriz, p50/p95),
  baseline heurístico e README de export/publicação.

### Bloqueio (ISS-059)
- `laya-onnx` não está no crates.io e o repo `NandhaKishorM/laya` não tem
  `Cargo.toml` na raiz (não serve como dep `git`). O ONNX do
  `convaiinnovations/laya-multilingual` não está publicado.
- A feature `laya` **não foi declarada** para não quebrar a resolução do build
  padrão. Falta só o binding `OnnxLayaPredictor`; contrato e testes estão
  prontos.
- `examples/laya_eval.rs` virou teste `#[ignore]` (módulos do `lib.rs` são
  privados).

### Métricas (baseline)
- 28 blocos, 4 por cue: acurácia `should_suggest` **92,9%**, FP **0%**,
  FN **10%**, acurácia de cue **53,6%**; p50/p95 0 ms (heurístico).
- O heurístico confunde `lost_thread`/`invitation` (não implementadas por
  regra) — o espaço do Laya.

### Testes
- Rust **798 passed**, 11 ignored (inclui o baseline ignorado); clippy/fmt
  limpos.
- Frontend **514 passed**; `tsc --noEmit` e `pnpm build` limpos.
- `make_dev_artifact.sh` gera um artefato falso; o source aceita caminho
  absoluto/`file://`, então catálogo → download → SHA → `.part` → ativar roda
  local sem o ONNX real.

### Próximo
1. Publicar `laya-onnx` + artefato ONNX (ou vendorizar oficialmente).
2. Binding `OnnxLayaPredictor` + feature `laya`; eval real nas fixtures.
3. Fechar a task 10 (sonda de bloco aberto + teste de scheduler) e a task 11.

---

## 2026-10-26 — Submódulo `laya-onnx` + binding ONNX

- **IDs:** BR-098 · **Issues:** ISS-059 · **Refs:**
  [`poc/laya-classifier/README.md`](../../poc/laya-classifier/README.md)

### Decisão
- O crate não está no crates.io e o repo upstream não tem `Cargo.toml` na raiz.
  Solução mais simples, sem conta: **submódulo git** `vendor/laya`
  (Apache-2.0) + `path = "../vendor/laya/laya-rust"`.
- `rust-version` subiu de 1.77 para **1.88** (edition 2024 do SDK). O bump
  ativou lints de MSRV no Clippy; 6 avisos pré-existentes foram corrigidos
  (`repeat_n`, `is_none_or`, `as_chunks`, `is_multiple_of`).
- `ort` pinado em `=2.0.0-rc.13`: o SDK usa `Session::builder()` sem
  `Environment`; o resolver puxava rc.15 (API incompatível).

### Integração
- `src-tauri/Cargo.toml`: feature `laya = ["dep:laya-onnx", "dep:ort"]`; o build
  padrão não compila o engine.
- `adapters/ai/laya_onnx_predictor.rs`: `OnnxLayaPredictor` com carga **lazy**
  na primeira classificação (não atrasa o boot), `allow_download=false`, token
  do HF suprimido, falha memorizada para não reler 1,2 GB a cada bloco.
- `lib.rs`: resolve o diretório no boot (override de dev ou `active()` do
  manager) e injeta o `LayaSuggestionCueClassifier`; sem modelo, `laya=None` e
  o seletor cai em `Regras locais`.
- `LayaClassifierConfig::from_settings`: limiar do preset, timeout e checkpoint.
- `eval_laya_on_fixtures` (ignorado, feature `laya`) usa as 28 fixtures e as
  métricas de `domain/classifier_eval.rs`.

### Testes
- `cargo build/check --lib --features laya` compila (submódulo + ORT rc.13).
- `cargo test --lib` **798 passed**; com `--features laya` **801 passed**
  (11/12 ignored); clippy/fmt limpos nos dois.
- Frontend **514 passed**.

### Pendência (ISS-059)
- O ONNX não está no HF; o export exige Python ≥3.10, `torch>=2.9` e ~5 GB
  livres. Não foi executado aqui (Python 3.9, ~7,8 GB). Sem o artefato, o
  engine não carrega um ONNX real; o eval do Laya e o gate de 80% seguem
  pendentes.

### Fluxo para o usuário/dev
1. `git submodule update --init`
2. `pnpm tauri dev --features laya`
3. Exportar o ONNX uma vez e colocar em `app_data/models/laya` (manager) ou no
   override `suggestion_classifier_model_path`.
4. Reiniciar; o engine carrega na primeira classificação.

---

## 2026-10-26 — Task 10: sonda aberta + decisão de tick

- **IDs:** BR-092, BR-093, BR-094 · **Refs:**
  [`tasks/10-task.md`](tasks/10-task.md)

### Código
- `domain/suggestion_gate.rs`: `open_probe_eligible` + `OpenProbeConfig` (ME
  no fim do bloco aberto, fala contínua ≥ limiar, ≥ 3 palavras, throttle 2 s,
  `hesitation` na allowlist).
- `ports/suggestion_gate.rs`: `probe_open` (default no-op) e
  `probe_min_words`/`probe_throttle_ms` na config.
- `application/suggestion_gate_service.rs`: `probe_open` com cache por bloco
  (não reclassifica sem fala nova), throttle por sessão e reset.
- `application/classifier_tick.rs`: decisão do tick (sonda → bloco fechado →
  `Skip`/`FallbackAuto`), testável com fake.
- `lib.rs`: scheduler usa `classify_once` e mantém a geração/prompt.
- `SuggestionGate::reconfigure` já aplicava sensitivity/cues/cooldown a cada
  tick; a sonda usa a config viva.

### Comportamento
- Hesitação detectada com ME ainda falando gera sugestão (se o gate permitir).
- O mesmo bloco não é sondado duas vezes sem segmento novo; duas sondas da
  mesma sessão ficam a ≥ 2 s.
- Erro do classificador na sonda segue a mesma política de fallback (`skip` ou
  `auto`).
- Bloco fechado continua sendo o caminho normal quando a sonda não se aplica.

### Testes
- Rust **806 passed** sem feature; **809 passed** com `--features laya`
  (11/12 ignored); clippy/fmt limpos.
- Novos: `open_probe_requires_me_hesitating_in_an_open_block`,
  `open_probe_triggers_hesitation_and_is_throttled`,
  `open_probe_respects_settings_and_closed_blocks`, 5 de
  `application/classifier_tick`.

### Próximo
- Task 11: hardening/docs/pacote + roteiro manual no app.

---

## 2026-10-26 — Task 11 (automatizado): hardening e docs

- **IDs:** BR-092…BR-100 · **Refs:**
  [`tasks/11-task.md`](tasks/11-task.md)

### Verificação
- `build_blocks` com 40 segmentos: **145 µs** (alvo < 5 ms); teste
  `build_blocks_is_fast_for_forty_segments`.
- Privacidade: `grep` de `tracing` não encontrou texto de transcrição; os logs
  do classificador têm só cue/confiança/bloco/latência; `evidence` é truncada
  em 160 chars; manifesto/checksum não têm dados de sessão.
- Clippy `--all-targets` limpo; o bump de MSRV já tinha corrigido os lints
  antigos; mais um `chunks_exact` em teste do WAV foi corrigido.
- Docs: PRD 0.3 (implementado), task 10 CONCLUDED, task 11 IN PROGRESS,
  `_tasks.md`, `README`/`AGENTS` e este histórico.

### Estado
- Rust **807 passed**, 11 ignored; **810 passed**, 12 ignored com
  `--features laya`; frontend **514 passed**.
- Falta só o **roteiro manual** no app (task 11) e o **artefato ONNX** do Laya
  (ISS-059). O modo automático segue padrão.

### Próximo
1. Rodar o roteiro manual e registrar no `memory.md`.
2. (Opcional) gerar `.deb`/AppImage.
3. Task 12 (fine-tuning) quando o modelo base estiver estável.

---

## 2026-10-26 — Teste manual: gate usava época Unix (ISS-060)

- **IDs:** BR-093, BR-094 · **Issues:** ISS-060 · **Refs:**
  [`issues.md`](issues.md), [`tasks/10-task.md`](tasks/10-task.md)

### Sintoma
No app real, `Classificar agora`/`Testar classificador` devolvia
`Classificador indisponível: configuração inválida: nenhum bloco classificável`
mesmo com falas dos dois lados (o DB tinha ME e THEY na mesma sessão).

### Causa
`SuggestionGateService` usava `clock.now_ms()` (época Unix, ~1,79e12) como
`now`/`since`. As transcrições guardam `end_ms` **relativo ao início da
sessão** (0..N ms). O `WHERE end_ms >= since` descartava tudo; `build_blocks`
também via todos os blocos como fechados/antigos.

### Correção
- `relative_now(session_id)` = `clock.now_ms() - session.started_at_ms`, via
  `SessionRepository`.
- Janela, `build_blocks`, silêncio e sonda usam o tempo relativo.
- `classified_at_ms` continua em época Unix (a UI mostra “Há 8s”).
- `NoBlock` no IPC virou `GhostWhisperError::NoClassifiableBlock` com a
  mensagem “fale dos dois lados e aguarde o silêncio fechar o bloco”.
- Regressão: `uses_session_relative_time_for_transcriptions` (falhava antes).

### Settings conferidas no teste
`mode=classifier`, `auto_refresh=true`, `backend=heuristic`,
`model_path=""`, `gap=3000`, `max=45000`, `turns=12`, `allowed_cues=[]`
(todas), `probe_open_block=true`, `hesitation_probe_ms=4000`,
`sensitivity=balanced` (0,70), `cooldown=15s`, `timeout=3000`,
`show_classification=true`, `ui_language=pt`. Todas corretas para o
heurístico; o erro era o bug de tempo, não configuração.

### Testes
- Rust **808 passed** sem feature; **811 passed** com `--features laya`.
- Um teste novo de erro (`no_classifiable_block_is_not_a_config_error`) → 809.
- Clippy/fmt limpos.

---

## 2026-10-26 — Teste manual: hesitação em bloco só de ME (ISS-061)

- **IDs:** BR-093, BR-094 · **Issues:** ISS-061 · **Refs:**
  [`issues.md`](issues.md)

### O que o teste mostrou
Sessão `975f7073…`:
- ME `0:00–0:03` + THEY `0:00–0:23` → bloco com os dois falantes;
- ME `0:28–0:34` (“Eu não tenho certeza” / “deixa eu pensar”) → **bloco só de
  ME**, com hesitação;
- ME `0:59–1:02` → outro bloco só de ME.

O heurístico classificou o primeiro bloco como `no_suggestion` (THEY sem
pergunta, ME sem filler) e a sonda do bloco de ME devolveu
`insufficient_context` porque `heuristic_cue` exigia os dois falantes antes de
avaliar hesitação.

### Correção
- Hesitação avaliada **antes** da exigência de ME+THEY; `has_uncertainty`
  cobre “não tenho certeza”, “deixa eu pensar”, “não sei”, “let me think” etc.
- Bloco só de ME sem marcador continua `insufficient_context` (não vira
  silêncio).
- `classify_once` consome `take_pending` antes de reclassificar: uma decisão
  positiva que caiu no cooldown não é mais perdida quando o próximo tick
  classifica outro bloco.

### Testes
- Rust **812 passed**, 11 ignored; **815 passed**, 12 ignored com
  `--features laya`; clippy/fmt limpos.
- Novos: `heuristic_detects_uncertainty_in_me_only_block`,
  `heuristic_me_only_without_markers_is_insufficient`,
  `reuses_a_pending_positive_decision_before_classifying`.

### Observação de UI
A faixa mostra a última classificação; depois que o bloco de ME fecha, o gate
pode voltar a classificar o bloco anterior (com os dois falantes) e a faixa
mostra `no_suggestion` mesmo tendo gerado para o bloco de ME. O card da
sugestão continua visível. Histórico/motivo do gate seguem como melhoria de
debug.


---

## 2026-10-26 — English removido + status do engine Laya

- **IDs:** BR-098 · **Refs:** [`tasks/06-task.md`](tasks/06-task.md)

### Decisão
- O checkpoint **English não é mais exposto**: o select do card tem só
  `Laya (Padrão)` (multilingual) e `Regras locais`. O multilingual cobre pt/en
  e ocupa menos espaço.
- `suggestion_classifier_checkpoint` só aceita `multilingual`; settings antigas
  com `english` são sanitizadas para `multilingual` no load/save.
- O tipo TS foi estreitado para `"multilingual"`; a chave
  `suggestion_model_english` saiu do catálogo pt/en.

### Diagnóstico do “falha ao selecionar Laya”
Três causas independentes:
1. **Build sem a feature** — `pnpm tauri dev` compila sem `laya-onnx`; o
   seletor cai em `Regras locais`. Agora o card mostra
   “Este build foi compilado sem o engine Laya…”.
2. **Sem modelo ONNX** — o artefato não está publicado; sem `model.onnx` o
   composition root deixa `laya=None` e cai no heurístico.
3. **Catálogo placeholder** — `https://downloads.ghostwhisper.app/laya/catalog.json`
   não resolve (DNS); “Baixar e ativar/Atualizar catálogo” falha.

Novo IPC `classifier_backend_status` (`laya_feature`, `model_active`,
`model_configured`) alimenta o card com o motivo real, em vez de uma nota
genérica. O fake de dev instala, mas o engine rejeita o ONNX falso — use o
export real.

### Testes
- Rust **813 passed**, 11 ignored; **816 passed**, 12 ignored com
  `--features laya`; clippy/fmt limpos.
- Frontend **515 passed**; `tsc`/build limpos.
- Novo teste de settings (`english_checkpoint_is_sanitized_to_multilingual`) e
  de UI (“explica por que o Laya não roda neste build”).

---

## 2026-10-26 — Export ONNX + AppImage Vulkan/Laya

- **IDs:** BR-098/BR-099 · **Issues:** ISS-062 · **Refs:**
  [`poc/laya-classifier/README.md`](../../poc/laya-classifier/README.md)

### Export
- Python 3.12 via `uv` + `torch 2.14.1+cpu` + onnx/onnxruntime/transformers.
- `python3 vendor/laya/laya-rust/tools/export_onnx.py --model multilingual
  --out src-tauri/resources/laya` → `model.onnx` 2,1 MB + `model.onnx.data`
  1,29 GB + tokenizer/rl_agent_config/fixtures. O diretório está no
  `.gitignore` (1,3 GB, não versionar).

### Empacotamento
- `bundle.resources` mapeia `resources/laya/multilingual` →
  `<resource_dir>/laya/multilingual`.
- `scripts/tauri-build-laya.sh` + `pnpm tauri:build:laya`: exige o artefato
  real (recusa o fake), inicializa o submódulo, compila `vulkan,laya` e gera
  só AppImage.
- AppImage final: **821 MiB**, em
  `src-tauri/target/release/bundle/appimage/GhostWhisper_0.1.0_amd64.AppImage`.
- Runtime: `resolve_model_dir` prefere override > instalado > bundled; o
  engine aquece em background; `classifier_backend_status.model_bundled`
  alimenta a UI (“Incluído no app”).
- O `classifier_backend_status` e o manager não mostram erro de catálogo
  quando o modelo é bundled.

### Eval real (28 fixtures)
- `should_suggest` **60,7%**, FP **12,5%**, FN **50%**, cue **21,4%**,
  p50 **377 ms**, p95 **521 ms**. Gate ≥80%/≤15% **não atingido** (ISS-062).
- O modelo confunde invitation/lost_thread/silence/no_suggestion e é enviesado
  a positivos; `noul`/binário também 8/28. Instruções/descrições em inglês
  melhoraram pouco.
- Decisão: heurístico continua padrão; Laya bundled fica como POC
  experimental e a UI avisa. Fine-tuning (task 12) é o próximo passo.

### Testes
- Rust **814 passed**, 11 ignored (sem feature); **817 passed**, 15 ignored
  (`--features laya`); frontend **518 passed**; clippy/fmt/tsc/build limpos.
- AppImage validado por inspeção do AppDir
  (`usr/lib/GhostWhisper/laya/multilingual`), binário com `laya`/ORT estático.
- O `tauri-build` copia resources com o modo da origem; o `tokenizer.json` do
  snapshot vem 0444 e quebrava o rebuild. O script `tauri:build:laya` faz
  `chmod -R u+w` na origem e no destino.


---

## 2026-10-26 — AppImage: permissão do model.onnx.data (ISS-063)

- **IDs:** BR-099 · **Issues:** ISS-063 · **Refs:**
  [`tasks/06-task.md`](tasks/06-task.md)

### Sintoma
Com Laya selecionado no AppImage, a janela ficava em **Falha** e nenhuma
classificação era gerada.

### Causa
O snapshot do HF entrega `model.onnx.data` **0600**. O AppImage monta os
resources como **root**; o processo do usuário não lia o sidecar de 1,29 GB.
`LayaEngine::create` falhava e o gate publicava `Unavailable`.

### Correção
- `chmod -R u+w,a+rX` no artefato e no target no
  `scripts/tauri-build-laya.sh`.
- AppDir/AppImage com `model.onnx.data` **0644**; AppImage recompilado
  (821 MiB, 19:03).
- O build script também corrige o `tokenizer.json` 0444 que quebrava rebuilds
  (Tauri copia com o modo da origem).

### Observação
Se a primeira classificação ocorrer enquanto o warm-up ainda lê o modelo,
pode haver um `Timeout` (3 s); os blocos seguintes usam o engine já carregado.
Aumentar o timeout para 10 s no Avançado evita esse primeiro aviso.


---

## 2026-10-26 — Integração por situação + agente/adaptive/threshold

- **IDs:** BR-093/BR-094 · **Issues:** ISS-062

### Testes de integração
- `adapters/ai/laya_integration.rs` (`--features laya`, `#[ignore]`) roda as 28
  fixtures uma a uma, imprime cue/confiança/decisão e o relatório de métricas.
  `LAYA_ENFORCE_GATE=1` aplica ≥80% / ≤15%.
- Resultado (agent context Entrevista Técnica, adaptive, limiar 0,50):
  **64,3%** `should_suggest`, FP **25%**, cue **25%**, p50 411 ms, p95 747 ms.
  Pergunta 3/4; hesitação 2/4; lost_thread/invitation/silence/no_suggestion 0/4.
- O relatório por situação está no teste; o gate continua reprovado (ISS-062).

### App
- O classificador recebe o **contexto do agente selecionado** (nome, categoria,
  contexto fixo e recorte do foco) via `ClassificationRequest.mode_context`;
  o Laya injeta no `question`, o heurístico ignora.
- Novo `suggestion_scope`: `blocks` (ME+THEY) | `adaptive` (qualquer bloco,
  **padrão**). O gate usa `is_classifiable_with`; a UI tem “Quando sugerir”.
- Laya usa limiares calibrados (baixa 0,75 / balanceada 0,50 / alta 0,35).
- Se o Laya abstém (`insufficient_context`), o seletor tenta o heurístico e usa
  se ele tiver um sinal positivo; a faixa mostra `heuristic`.
- AppImage precisa ser recompilado para levar essas mudanças.

---

## 2026-10-26 — Taxonomia por intenção, agente e histórico (task 13)

- **IDs:** BR-093/BR-094/BR-101 · **Issues:** ISS-062/ISS-064 · **Task:** 13

### Decisão
- A taxonomia antiga (hesitação/silêncio/perda do fio/convite) foi substituída
  por 5 cues: `question`, `comment`, `already_answered`, `no_action`,
  `insufficient_context`. Só pergunta e comentário geram sugestão.
- O classificador recebe o **agente selecionado** (nome/categoria/contexto/foco)
  e as **últimas 5 perguntas respondidas** (0–20 configurável), com título
  curto. O histórico só registra pergunta que realmente gerou sugestão.
- Contexto ampliado: piso de 8 blocos (até 20), janela de 5 min/120 segmentos.
- A sonda de bloco aberto foi removida de domínio, porta, serviço, tick,
  settings, UI e l10n.

### Heurístico
- Ordem: curto → small talk → já respondida (resposta no bloco ou Jaccard ≥
  0,60) → pergunta → convite de comentário → sem ação.
- `no_action` cobre conversa casual, explicação e fala sem oportunidade.

### Laya + guarda
- Prompt com agente + histórico; opções `question`, `comment`,
  `already_answered`, `no_action`, `insufficient_context`.
- O seletor sempre calcula as regras locais. `already_answered` das regras
  vence; `already_answered` inventado pelo Laya sem evidência cai nas regras;
  abstenção/`no_action` cede às regras quando há sinal positivo.
- Eval real: bruto **60%** de decisão / 30% de cue / FP 25% / p95 767 ms;
  efetivo (Laya+guarda) **100%** de decisão / 100% de cue / FP 0% / p95 599 ms
  em 20 fixtures. O gate bruto continua reprovado (ISS-062).

### Fixtures e build
- `poc/laya-classifier/fixtures/blocks.jsonl`: 20 blocos (4 por cue), com
  `prior_questions` e casos de já respondida.
- `laya_integration` testa o caminho efetivo e imprime backend por situação;
  `LAYA_ENFORCE_GATE=1` aplica ≥80% / ≤15%.
- AppImage recompilado com Vulkan + Laya + modelo; verificação de permissão do
  `model.onnx.data` mantida.


## 2026-10-26 — Correção do “já respondida” em sessão real (ISS-065)

- **IDs:** BR-094/BR-101 · **Issues:** ISS-065 · **Task:** 13

- A sessão real do mutex/fila mostrou a mesma pergunta gerando sugestões em
  blocos consecutivos. O histórico só era gravado para `question`; quando o
  Laya classificava o bloco como `comment`, a sugestão era gerada mas o título
  não entrava no histórico.
- Correção: **toda cue acionável** (`question`/`comment`) entra no histórico.
  O título agora é a fala completa de THEY (até 180 chars), não só a última
  pergunta; a repetição compara a pergunta isolada e o bloco inteiro de THEY.
- Regressão com o texto real: primeiro bloco `question`, segundo
  `already_answered` com backend `heuristic`; a suíte de integração continua
  100% de decisão/cue nas 20 fixtures.
- Testes: `a_generated_comment_also_enters_the_answered_history`,
  `repeated_comment_invitation_is_already_answered`,
  `laya_integration_marks_a_repeated_mutex_question_as_already_answered`.


## 2026-10-26 — Sem sugestão na resposta do ME (ISS-066)

- **IDs:** BR-094/BR-101 · **Issues:** ISS-066 · **Task:** 13

- A sessão real mostrou sugestão sobre a resposta do próprio ME. O Laya
  devolvia `question`/`comment` para um bloco terminando em ME e o seletor
  preservava o positivo.
- Guarda nova: **último turno ME ⇒ regras locais decidem**. Se havia pergunta
  de THEY antes da resposta, `already_answered`; senão `no_action`. Nunca
  positivo.
- Um bloco `already_answered` também entra no histórico sem gerar sugestão,
  então a repetição futura continua bloqueada mesmo quando o ME responde antes
  do copiloto.
- Fixtures: 22 no total (5 `already_answered`, 5 `no_action`), incluindo `a05`
  (pergunta + resposta no mesmo bloco) e `n05` (ME-only). Integração efetiva
  segue 100% de decisão/cue.


## 2026-10-26 — Fechamento das tasks 06/11 e ajuste de UI

- **IDs:** BR-097 · **Issues:** — · **Tasks:** 06, 11, 13
- Tasks 06 e 11 marcadas como `CONCLUDED`: engine/model manager/AppImage e
  hardening/teste manual aprovados. O fine-tuning continua como evolução
  (task 12) e a qualidade bruta do Laya segue em ISS-062.
- Settings de classificadores compactados: descrições longas removidas,
  modelo/escopo/sensibilidade agrupados, model manager movido para o
  “Avançado” e nota “POC” removida. O modelo aparece apenas como “Laya”.


## 2026-10-26 — Timeout do primeiro uso, logs locais e UI compacta

- **IDs:** BR-097 · **Issues:** ISS-067 · **Tasks:** 06, 11, 13
- “Falha” repetida era o timeout de 3 s durante a carga inicial do ONNX
  (~1,2 GB). O padrão passou a 12 s e o piso para 10 s (faixa 10–30 s);
  settings antigas migram no load.
- Logs locais em `<app_data>/logs/ghostwhisper.log` (rotação em 5 MB) com
  timeout/carga/erros, sem texto de transcrição ou segredo. A aba
  **Geral & HUD → Logs** mostra as últimas 300 linhas e abre a pasta.
- A seção de classificadores foi compactada: sem parágrafos descritivos, sem
  “POC”, model manager no “Avançado”, escopo com rótulos claros:
  “Qualquer bloco” vs “Só com ME e THEY”.


## 2026-10-26 — Escopo “Só THEY” e logs de diagnóstico

- **IDs:** BR-093/BR-097 · **Issues:** ISS-067 · **Task:** 13
- Novo `suggestion_scope = they_only`: só classifica quando THEY é o último a
  falar; o bloco enviado ao modelo é projetado apenas com turnos de THEY
  (ME não influencia a decisão). Contexto anterior continua completo.
- UI: opções “Qualquer bloco”, “Só THEY (perguntas do outro)” e “Só com ME e
  THEY”, com legenda curta.
- Timeout do classificador: 12 s padrão, faixa 10–30 s, para cobrir a carga
  inicial de ~1,2 GB do ONNX e evitar “Falha” repetida.
- Logs locais em `<app_data>/logs/ghostwhisper.log`, com consulta em
  Configurações → Geral & HUD → Logs.


## 2026-10-27 — Contexto e alvo separados no classificador

- **IDs:** BR-093/BR-094 · **Issues:** ISS-066/ISS-068 · **Task:** 13
- `ClassificationRequest` agora tem `full_block` (ME+THEY) e `block` (alvo:
  turnos de THEY desde a última fala de ME).
- O serviço usa `they_target` em todos os escopos; bloco terminando em ME não
  gera classificação.
- O prompt do Laya separa `[CONTEXTO — ME e THEY]` e
  `[ALVO — última fala de THEY ainda não respondida]`, com instrução explícita
  para nunca responder falas do ME.
- O heurístico compara o alvo e a fala completa de THEY com o histórico, para
  não perder pergunta fragmentada.
- Regressão do mutex/fila e as 22 fixtures continuam 100% no caminho efetivo;
  Laya bruto subiu de ~64% para ~73% de decisão com o novo estado.


## 2026-10-27 — `comment` só com convite explícito

- **IDs:** BR-094 · **Issues:** ISS-069 · **Task:** 13
- `comment` deixou de significar “THEY expressou uma opinião” e passou a exigir
  convite explícito para ME falar. Afirmação/explicação/opinião solta vira
  `no_action`.
- Heurístico: marcadores fortes valem sozinhos; fracos (`faz sentido`,
  `concorda`) exigem pergunta. O Laya só mantém `comment` se as regras também
  detectarem convite.
- Fixtures: 24 no total (5 `comment`, 6 `no_action`); heurístico e caminho
  efetivo em 100% de decisão/cue. Laya bruto segue como POC.
