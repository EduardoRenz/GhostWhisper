# Issues — PRD Modo de Sugestão

Registry de bugs e limitações deste PRD. Convenções em
[`../../.agents/rules.md`](../../.agents/rules.md) (`ISS-NNN` é canônico por
arquivo de PRD; a numeração **continua** a dos PRDs anteriores para o ID ser
único no repositório). Próximo ID livre: **ISS-083**.

| ID | Resumo | Severidade | Status |
|---|---|---|---|
| ISS-059 | Artefato ONNX do Laya não publicado | Alta | OPEN |
| ISS-060 | Gate usava relógio de parede com timestamps relativos | Alta | FIXED |
| ISS-061 | Heurístico ignorava hesitação em bloco só de ME | Média | FIXED |
| ISS-062 | Laya zero-shot abaixo do gate de qualidade | Alta | OPEN |
| ISS-063 | AppImage montava model.onnx.data 0600 root e o engine falhava | Alta | FIXED |
| ISS-076 | `already_answered` não pegava pergunta longa repetida (janela de 180) | Média | FIXED |
| ISS-077 | Pergunta longa sem `?` caía em no_action (contexto fragmentado) | Alta | FIXED |
| ISS-078 | Backend “Regras locais” ainda depende de palavras-chave (viola R23) | Média | OPEN |
| ISS-080 | Última pergunta não classificada: modo_context de geração + alvo fragmentado | Alta | FIXED |
| ISS-081 | `already_answered` não reconhece repetição deslocada/fragmento (janela fixa) | Alta | FIXED |

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
- **Mitigação:** engine + artefato ONNX + AppImage prontos; heurístico segue
  recomendado e a UI avisa que Laya é POC. Na taxonomia nova, a guarda
  determinística (`already_answered`/abstenção) eleva o caminho efetivo a 100%
  de decisão / 100% de cue nas 20 fixtures, com p95 ~599 ms. O eval bruto
  continua 60% de decisão.
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


## ISS-064 — Laya inventava `already_answered` sem histórico

- **Severidade:** Alta · **Status:** FIXED (2026-10-26)
- **Sintoma:** com a taxonomia nova, o Laya escolhia `already_answered` para
  uma pergunta inédita (q03), suprimindo uma sugestão válida; em outras
  perguntas repetidas ele escolhia `question` e gerava de novo.
- **Causa raiz:** o modelo zero-shot não separa “repetida” de “pergunta
  parecida” e não recebia (antes da task 13) um histórico explícito de
  perguntas respondidas.
- **Correção:** `ClassificationRequest.prior_questions` + prompt do Laya com os
  títulos; no seletor, a guarda determinística vence: se as regras locais dizem
  `already_answered`, não sugere; se o Laya diz `already_answered` sem
  evidência nas regras, usa a decisão das regras. A mesma guarda cobre
  abstenção/`no_action` quando as regras têm sinal positivo.
- **Testes:** `repeated_question_overrides_a_positive_laya_decision`,
  `generated_question_enters_the_answered_history`,
  `laya_integration_reports_each_situation`.
- **Arquivos:** `src-tauri/src/adapters/ai/settings_suggestion_cue_classifier.rs`,
  `src-tauri/src/application/suggestion_gate_service.rs`,
  `src-tauri/src/adapters/ai/laya_suggestion_cue_classifier.rs`.
- **Task:** [`tasks/13-task.md`](tasks/13-task.md)


## ISS-065 — “Já respondida” não pegava pergunta repetida classificada como comentário

- **Severidade:** Alta · **Status:** FIXED (2026-10-26)
- **Sintoma:** na sessão real de 2026-10-26, a mesma pergunta do mutex/fila
  gerou sugestões repetidas em blocos consecutivos.
- **Causa raiz:** o histórico só era gravado quando a cue era `question`
  (`if outcome.cue == Question`). Se o Laya classificasse o bloco como
  `comment`, a sugestão era gerada, mas nenhum título entrava no histórico;
  o bloco seguinte era tratado como novo. Além disso, o título usava só a
  última pergunta de THEY, perdendo a pergunta quando ela vinha fragmentada
  em vários turnos (“... uma fila. Quais são as diferenças...”).
- **Correção:** qualquer cue acionável (`question` ou `comment`) entra no
  histórico; o título passa a ser a fala completa de THEY (até 180 chars); a
  checagem de repetição compara a pergunta isolada **e** o bloco inteiro de
  THEY. Teste de regressão com o texto real da sessão e Laya.
- **Testes:** `a_generated_comment_also_enters_the_answered_history`,
  `repeated_comment_invitation_is_already_answered`,
  `laya_integration_marks_a_repeated_mutex_question_as_already_answered`.
- **Arquivos:** `src-tauri/src/application/suggestion_gate_service.rs`,
  `src-tauri/src/domain/suggestion_gate.rs`,
  `src-tauri/src/adapters/ai/laya_integration.rs`.
- **Task:** [`tasks/13-task.md`](tasks/13-task.md)


## ISS-066 — Sugestão gerada na resposta do ME

- **Severidade:** Alta · **Status:** FIXED (2026-10-26)
- **Sintoma:** depois de THEY perguntar e ME responder, o classificador ainda
  gerava sugestão sobre a fala do ME (“minha resposta achando que tem que
  sugerir”).
- **Causa raiz:** quando o Laya devolvia `question`/`comment` para um bloco que
  terminava em ME, o seletor preservava a decisão positiva. O heurístico via
  `no_action`, mas só era usado em abstenção/`no_action` do Laya ou em
  `already_answered`.
- **Correção:** se o último turno do bloco é ME, as regras locais decidem:
  `already_answered` quando havia pergunta de THEY antes da resposta; caso
  contrário `no_action`. Nunca há sugestão positiva nesse caso. Além disso,
  um bloco `already_answered` entra no histórico mesmo sem gerar sugestão, para
  que uma repetição futura continue bloqueada.
- **Testes:** `me_last_turn_is_not_a_suggestion_opportunity`,
  `a_block_ending_in_me_never_uses_a_positive_laya_decision`,
  `an_already_answered_block_enters_history_without_generation`, fixtures
  `a05`/`n05`.
- **Arquivos:** `src-tauri/src/domain/suggestion_gate.rs`,
  `src-tauri/src/adapters/ai/settings_suggestion_cue_classifier.rs`,
  `src-tauri/src/application/suggestion_gate_service.rs`.
- **Task:** [`tasks/13-task.md`](tasks/13-task.md)


## ISS-067 — “Falha” repetida no topo ao carregar o Laya

- **Severidade:** Alta · **Status:** FIXED (2026-10-26)
- **Sintoma:** o cabeçalho do classificador exibia “Falha” várias vezes nos
  primeiros blocos, mesmo com Laya selecionado e modelo empacotado.
- **Causa raiz:** o timeout padrão era 3 s. A primeira inferência carrega o
  ONNX de ~1,2 GB dentro do worker e pode levar mais que isso; cada tick
  estourava o timeout e publicava `Error`.
- **Correção:** timeout padrão 12 s e piso de 10 s (faixa 10–30 s); migração
  de settings antigas para o piso. O log registra timeout, carga do modelo e
  falha de classificação. Logs locais em `<app_data>/logs/ghostwhisper.log`,
  consultáveis em Configurações → Geral & HUD → Logs.
- **Testes:** `tail_keeps_only_the_requested_lines`,
  `LogsSettings` (frontend), suíte de settings.
- **Arquivos:** `src-tauri/src/domain/settings.rs`,
  `src-tauri/src/adapters/ai/laya_suggestion_cue_classifier.rs`,
  `src-tauri/src/adapters/ai/laya_onnx_predictor.rs`,
  `src-tauri/src/lib.rs`, `src-tauri/src/ipc/log_commands.rs`,
  `src/features/settings/LogsSettings.tsx`.
- **Task:** [`tasks/13-task.md`](tasks/13-task.md)


## ISS-068 — Laya confundia resposta do ME com pergunta pendente

- **Severidade:** Alta · **Status:** FIXED (2026-10-27)
- **Sintoma:** o classificador às vezes sugeria uma resposta para a própria
  resposta do ME, ou reabria uma pergunta já respondida.
- **Causa raiz:** o bloco enviado ao modelo misturava ME+THEY e não separava o
  alvo; o Laya via a resposta do ME e a pergunta anterior no mesmo texto.
- **Correção:** `full_block` (ME+THEY) para contexto e `block` (THEY desde a
  última fala de ME) para o alvo; prompt com `[CONTEXTO]`/`[ALVO]`; serviço
  retorna `NoBlock` quando o último turno é ME; heurístico compara também a
  fala completa de THEY contra o histórico.
- **Testes:** `they_target_keeps_only_they_turns_after_the_last_me`,
  `they_only_does_not_reopen_a_question_answered_before_the_last_they_turn`,
  `render_prompt_state_separates_context_and_target`,
  `laya_integration_marks_a_repeated_mutex_question_as_already_answered`.
- **Arquivos:** `src-tauri/src/domain/suggestion_gate.rs`,
  `src-tauri/src/application/suggestion_gate_service.rs`,
  `src-tauri/src/adapters/ai/laya_suggestion_cue_classifier.rs`,
  `src-tauri/src/adapters/ai/heuristic_suggestion_cue_classifier.rs`.
- **Task:** [`tasks/13-task.md`](tasks/13-task.md)


## ISS-069 — `comment` disparava em opinião que não pedia comentário

- **Severidade:** Média · **Status:** FIXED (2026-10-27)
- **Sintoma:** THEY fazia uma afirmação/opinião e o copiloto sugeria um
  comentário mesmo sem convite explícito.
- **Causa raiz:** o heurístico aceitava marcadores fracos (`faz sentido`,
  `concorda`) sem pergunta, e o Laya podia escolher `comment` sozinho. A
  definição de `comment` não deixava claro que é um **convite**, não uma
  opinião.
- **Correção:** `comment` agora exige convite explícito. Marcadores fortes
  (“o que você acha”, “qual sua leitura”, “faz sentido para você”, “do you
  agree”) valem sozinhos; fracos (“faz sentido”, “concorda”) exigem `?`. O
  Laya só mantém `comment` se as regras também virem convite. Prompt do Laya
  atualizado com a distinção “opinião = no_action”.
- **Testes:** `a_mere_statement_is_not_a_comment_invitation`,
  `a_laya_comment_without_an_explicit_invitation_is_ignored`, fixtures `n06`
  (afirmação) e `c05` (convite explícito).
- **Arquivos:** `src-tauri/src/domain/suggestion_gate.rs`,
  `src-tauri/src/adapters/ai/settings_suggestion_cue_classifier.rs`,
  `src-tauri/src/adapters/ai/laya_suggestion_cue_classifier.rs`.
- **Task:** [`tasks/13-task.md`](tasks/13-task.md)


## ISS-070 — Ruído curto do ME escondia a pergunta de THEY

- **Severidade:** Alta · **Status:** FIXED (2026-10-27)
- **Sintoma:** em uma sessão real, o último segmento era ME (“tchau tchau”)
  captado pelo microfone; o gate não analisava e não sugeria, mesmo com uma
  pergunta completa de THEY no mesmo bloco.
- **Causa raiz:** `they_target` considerava qualquer turno de ME como resposta;
  um ruído curto depois de THEY fazia o alvo ficar vazio. Além disso, o alvo
  era cortado a partir do último ME, podendo remover a pergunta.
- **Correção:** só uma fala de ME com **≥ 3 palavras** conta como resposta
  anterior. Ruído/backchannel curto é ignorado; a pergunta de THEY continua no
  alvo. O modo `they_only` também não exige mais que o último turno bruto seja
  THEY; a checagem é feita pelo alvo.
- **Testes:** `short_me_noise_does_not_hide_the_they_question`,
  `short_me_noise_does_not_block_the_they_question`.
- **Arquivos:** `src-tauri/src/domain/suggestion_gate.rs`,
  `src-tauri/src/application/suggestion_gate_service.rs`.
- **Task:** [`tasks/13-task.md`](tasks/13-task.md)


## ISS-071 — “Deixa eu pensar” interrompia a análise da pergunta

- **Severidade:** Alta · **Status:** FIXED (2026-10-27)
- **Sintoma:** THEY fazia a pergunta, ME dizia “Hmmm, deixa eu pensar” e o
  classificador não analisava; a pergunta ficava sem sugestão.
- **Causa raiz:** a fala de espera do ME tem 3+ palavras e era tratada como
  resposta, encerrando o alvo.
- **Correção:** `is_meaningful_me_turn` agora detecta expressões de hesitação
  (“deixa eu pensar”, “deixa eu ver”, “vou pensar”, “espera”, “let me think”)
  e as ignora quando não há conteúdo depois delas. “deixa eu pensar, a resposta
  é B” continua contando como resposta; “não sei” também.
- **Testes:** `thinking_filler_does_not_block_the_they_question`,
  `short_me_noise_does_not_hide_the_they_question`.
- **Arquivos:** `src-tauri/src/domain/suggestion_gate.rs`,
  `src-tauri/src/application/suggestion_gate_service.rs`.
- **Task:** [`tasks/13-task.md`](tasks/13-task.md)


## ISS-072 — Conversa contínua nunca fechava bloco e não sugeria

- **Severidade:** Alta · **Status:** FIXED (2026-10-27)
- **Sintoma:** em conversa sem pausa de 3 s, o bloco ficava aberto e o
  classificador não rodava; a sugestão nunca aparecia.
- **Causa raiz:** `classify_latest` só aceitava blocos fechados
  (`is_open = false`), e o fechamento dependia do gap de silêncio.
- **Correção:** blocos abertos passam a ser classificados 300 ms depois da
  última fala finalizada (`OPEN_BLOCK_SETTLE_MS`). O gap de 3 s continua
  fechando blocos e montando contexto, mas não bloqueia mais a análise. O
  cache por `block_id`/alvo evita reclassificar a mesma ponta.
- **Testes:** `an_open_block_is_classified_after_the_bubble_settles`.
- **Arquivos:** `src-tauri/src/application/suggestion_gate_service.rs`.
- **Task:** [`tasks/13-task.md`](tasks/13-task.md)

---

## ISS-076 — `already_answered` não pegava pergunta longa repetida

- **Severidade:** Média · **Status:** FIXED (task 14)
- **Sintoma:** na sessão real de 2026-10-27 18:19, a mesma pergunta de *load
  balancer* foi feita três vezes, com a resposta do ME no meio, e o classificador
  nunca devolveu `already_answered` — caía em `comment`/`question`.
- **Causa raiz:** `PriorQuestion::normalized` guarda o título da pergunta com
  **≤ 180 caracteres**, mas `is_repeated_question` comparava esse título com a
  pergunta atual **inteira** (400+ caracteres). As palavras extras diluíam o
  Jaccard para ~0,51, abaixo do limiar de 0,60. Em perguntas curtas o bug não
  aparecia; o questionário da entrevista expôs. A guarda efetiva de
  `already_answered` é o heurístico (o Laya zero-shot é descartado para não
  suprimir pergunta válida), então a função ficou desligada na prática.
- **Correção (task 14):** `is_repeated_question` trunca a pergunta atual na
  **mesma janela de 180 caracteres** (`QUESTION_TITLE_MAX_CHARS`, compartilhada
  com `question_title`) antes do Jaccard e da comparação exata.
- **Testes anti-regressão (R16/R17):** teste de domínio com pergunta longa
  repetida (falharia antes) + teste ponta a ponta no gate (pergunta → resposta do
  ME em bloco separado → pergunta repetida = `already_answered`).
- **Arquivos:** `src-tauri/src/domain/suggestion_gate.rs`,
  `src-tauri/src/application/suggestion_gate_service.rs`.
- **Task:** [`tasks/14-task.md`](tasks/14-task.md)
- **Dano em dados existentes:** nenhum (histórico é memória de sessão).
- **Revisão (ISS-081/task 18):** a janela fixa de 180 caracteres nos dois lados
  foi substituída pelo texto completo + janelas deslizantes + contenção. A
  janela funcionava para o caso de 2026-10-27, mas falhava quando a repetição
  começava em outro ponto ou vinha só como cauda.

---

## ISS-077 — Pergunta longa sem `?` caía em `no_action` (contexto fragmentado)

- **Severidade:** Alta · **Status:** FIXED (task 15)
- **Sintoma:** nas sessões `606de489…` (load balancer) e `0e882d0a…`
  (arquitetura), THEY fez uma pergunta clara e a classificação caiu em
  `no_action`. O Whisper não pontuou com `?`, a pergunta veio em vários turnos e
  o `build_blocks` a dividiu no limite de tempo; o alvo passado ao classificador
  era só o último fragmento.
- **Causa raiz (dupla):**
  1. o caminho efetivo do Laya era **sobrescrito pela guarda heurística** de
     palavras-chave, que não reconhecia a pergunta sem `?`/com interrogativo no
     meio do turno; e
  2. o alvo era o último bloco/fragmento, não a pergunta inteira que atravessava
     blocos contíguos. O estado do prompt separava `[CONTEXTO]`/`[ALVO]` e o
     modelo via só o fragmento.
- **Correção (task 15, R23/R24):**
  - extraí as conversas reais das duas sessões e criei **testes de
    integração/regressão com o Laya real** (`LAYA_ONNX_DIR`);
  - `target_across_blocks` junta blocos **contíguos** (divisão por tempo, não
    por pausa) para o alvo conter a pergunta inteira;
  - o estado do Laya passa a ser a **conversa inteira** (contexto + bloco atual,
    sem marcadores `[CONTEXTO]/[ALVO]`), com prompt curto e genérico (sem
    exemplos/palavras de idioma);
  - **removi a guarda heurística do caminho Laya**: a decisão de
    `question`/`comment`/`no_action` é do modelo; resta apenas a guarda lexical
    de `already_answered` (Jaccard de tokens, independente de idioma);
  - Laya indisponível → **não sugerir** (não cai em palavras-chave).
  As inclusões de palavras-chave foram **revertidas** (violavam R23).
- **Testes anti-regressão (R16/R17/R24):** três testes reais com Laya:
  `0e882d0a…` (arquitetura) → `question`; `606de489…` (load balancer) →
  `question`; pergunta repetida com resposta do ME → `already_answered`
  (Jaccard). Testes de gate garantem que o alvo leva os 4 turnos de THEY.
- **Arquivos:** `src-tauri/src/adapters/ai/laya_suggestion_cue_classifier.rs`,
  `src-tauri/src/adapters/ai/laya_integration.rs`,
  `src-tauri/src/adapters/ai/settings_suggestion_cue_classifier.rs`,
  `src-tauri/src/adapters/ai/heuristic_suggestion_cue_classifier.rs`,
  `src-tauri/src/application/suggestion_gate_service.rs`,
  `src-tauri/src/domain/suggestion_gate.rs`.
- **Task:** [`tasks/15-task.md`](tasks/15-task.md)
- **Dano em dados existentes:** nenhum.
- **Débito separado:** o backend “Regras locais” ainda é keyword-based; ver
  **ISS-078** (task 16).


---

## ISS-078 — Backend “Regras locais” ainda depende de palavras-chave

- **Severidade:** Média · **Status:** OPEN (task 16)
- **Sintoma:** o caminho efetivo do Laya já não usa palavras-chave (R23), mas o
  backend **“Regras locais”** (`HeuristicSuggestionCueClassifier`) ainda decide
  `question`/`comment`/`no_action` com listas de gatilhos em pt/en.
- **Causa raiz:** dívida anterior ao R23; o backend é oferecido nas
  configurações e no fallback de build sem Laya.
- **Correção proposta (task 16):** remover o backend do seletor e do fallback;
  migrar `suggestion_classifier_backend=heuristic` para `laya`; sem modelo, não
  sugerir. Remover `heuristic_cue`/listas de `suggestion_gate.rs` (manter só a
  guarda Jaccard de `already_answered`, que é independente de idioma).
- **Task:** [`tasks/16-task.md`](tasks/16-task.md)

---

## ISS-080 — Última pergunta não classificada: modo_context de geração + alvo fragmentado

- **Severidade:** Alta · **Status:** FIXED (task 17)
- **Sintoma (sessão `5984de1d…`, 22:02):** a janela de Sugestões estava aberta,
  mas só a primeira pergunta (carteira) foi classificada; a do amigo/dinheiro e
  a de arquitetura não geraram sugestão. A de arquitetura veio em 3 finais e o
  ME só falou “Está bom, deixa eu pensar…” no fim.
- **Causa raiz (dupla):**
  1. `selected_mode_context` mandava ao Laya o **`suggestion_prompt`** (500
     caracteres de instrução de **geração**: “write the exact words ME should
     say…”). Isso enviesava o classificador e podia transformar pergunta em
     `no_action`;
  2. `target_across_blocks` retornava `None` quando o bloco atual tinha ME,
     então o alvo era só o último turno (fragmento), não a pergunta inteira.
- **Correção (task 17):**
  - o contexto do agente no classificador passa a ser **nome/categoria/
    contexto**; o `suggestion_prompt` de geração **não** entra;
  - `target_across_blocks` junta o bloco anterior quando o **alvo começa no
    primeiro turno do bloco**, mesmo que exista um ME não significativo depois
    (“deixa eu pensar”); se houver ME antes do alvo, não junta;
  - testes reais com Laya da sessão `5984de1d…`: arquitetura e amigo/dinheiro →
    `question`; a suíte das sessões anteriores continua verde.
  - O gate de janela **não mudou**: você escolheu classificar só com a janela
    aberta.
- **Testes anti-regressão (R16/R17/R24):**
  `regression_5984de1d_architecture_question_is_question`,
  `regression_5984de1d_friend_money_question_is_question`,
  `target_across_blocks_merges_a_contiguous_they_block_with_a_trailing_filler`.
- **Arquivos:** `src-tauri/src/application/suggestion_gate_service.rs`,
  `src-tauri/src/domain/suggestion_gate.rs`,
  `src-tauri/src/adapters/ai/laya_integration.rs`.
- **Task:** [`tasks/17-task.md`](tasks/17-task.md)
- **Dano em dados existentes:** nenhum (classificação é memória de sessão).

---

## ISS-081 — `already_answered` não reconhece repetição deslocada/fragmento

- **Severidade:** Alta · **Status:** FIXED (task 18)
- **Sintoma (sessão `5f8595a7…`, 2026-10-28):** as 7 sugestões geradas saíram
  como `question`; nenhuma repetição virou `already_answered` mesmo com a janela
  de Sugestões aberta. A pergunta do load balancer e a do amigo/dinheiro foram
  repetidas e sugeridas de novo.
- **Causa raiz (três):**
  1. `is_repeated_question` truncava os **dois lados** nos primeiros 180
     caracteres. A repetição do load balancer começava no meio (`web que
     receberá...`) e o Jaccard caía para ~0,22; a do amigo/dinheiro vinha só
     como cauda (`... C. Ignora e muda de assunto.`) e o Jaccard caía para
     ~0,12.
  2. o histórico guardava `question_title(&target)`. Um ME curto no meio
     ("e pode perguntar então.") cortava a pergunta e o título ficava só com a
     segunda metade.
  3. a comparação era só Jaccard simétrico, sem contenção: fragmento/cauda de
     uma pergunta longa nunca alcançava 0,60.
- **Correção (task 18):**
  - histórico guarda a pergunta do **bloco inteiro** (`question_title(&full_block)`)
    até 600 caracteres; o prompt do Laya mostra até 180 por item;
  - `is_repeated_question` compara o texto completo, o **maior Jaccard entre
    janelas deslizantes** (24 tokens, passo 6) e a **contenção**
    (`|A∩B|/min(|A|,|B|)` ≥ 0,75, com ≥ 6 tokens em comum e ≥ 8 no lado menor);
  - a guarda avalia o **alvo** e o **bloco inteiro**, para pegar cauda e
    repetição deslocada.
  - Nenhuma lista de palavras/idioma (R23).
- **Testes anti-regressão (R16/R17/R24):**
  - domínio: load balancer com começo diferente (título antigo de 180 e novo
    completo), cauda do amigo/dinheiro, fragmento da carteira, negativos
    (pergunta diferente e "Beleza.");
  - adaptador: Laya devolve `question` e a guarda sobrescreve para
    `already_answered` (as duas repetições reais) e pergunta diferente chega ao
    Laya;
  - gate ponta a ponta com os segmentos reais de `5f8595a7…`: o histórico guarda
    as duas metades e a repetição vira `already_answered`.
- **Arquivos:** `src-tauri/src/domain/suggestion_gate.rs`,
  `src-tauri/src/adapters/ai/heuristic_suggestion_cue_classifier.rs`,
  `src-tauri/src/adapters/ai/settings_suggestion_cue_classifier.rs`,
  `src-tauri/src/application/suggestion_gate_service.rs`,
  `src-tauri/src/adapters/ai/laya_suggestion_cue_classifier.rs`.
- **Task:** [`tasks/18-task.md`](tasks/18-task.md)
- **Dano em dados existentes:** nenhum (histórico é memória de sessão).
- **Fora de escopo:** a 1ª ocorrência da carteira não gerou sugestão (o bloco
  terminou com ME), então não entrou no histórico — comportamento da BR-101;
  registrá-la exigiria outra decisão de produto.
