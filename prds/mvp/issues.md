# Issues / Radar — GhostWhisper MVP

Registro de bugs e pendências observadas em uso. Legenda de status:
`OPEN` · `INVESTIGATING` · `FIXED` · `WONTFIX`.

| ID | Título | Severidade | Status |
|----|--------|-----------|--------|
| ISS-001 | Clique no botão do overlay não captura de forma confiável (X11) | Média | FIXED |
| ISS-002 | Janela de configurações abre só uma vez | Alta | FIXED |
| ISS-003 | Provider rejeita `max_tokens` (modelos novos) | Alta | FIXED |
| ISS-004 | Idioma de origem/destino em texto livre; Whisper ignorava idioma e modelo | Média | FIXED |
| ISS-005 | Transcrição lenta e "trava" (espiral de buffer no pipeline) | Alta | FIXED |
| ISS-006 | Sem teste de áudio; captura podia parar em silêncio | Média | FIXED |
| ISS-007 | Transcrição pouco confiável (texto que "gruda"/some, alucinações, lenta) | Alta | FIXED |
| ISS-008 | Só aparecia **uma** linha de transcrição e depois parava | Crítica | FIXED |
| ISS-009 | Trocar para um modelo maior não dava nenhum retorno (silêncio durante o download) | Alta | FIXED |
| ISS-010 | Transcrição não "flui" (turno longo, prévia de 2 s e VAD caro no buffer inteiro) | Alta | FIXED |
| ISS-011 | ChatGPT/provider OpenAI recusa `temperature` (gpt-5/o1/o3 aceitam só o padrão) | Alta | FIXED |
| ISS-013 | Fim de turno preso atrás da prévia (decodificação bloqueava o agendamento) | Alta | FIXED |
| ISS-014 | ASR enroscava após timeout (`AsrBusy` em cascata) e threads subdimensionadas | Alta | FIXED |
| ISS-015 | Sensibilidade do microfone muito baixa (nenhum ganho de entrada) | Alta | FIXED |

---

## ISS-015 — Sensibilidade do microfone muito baixa

- **Severidade:** Alta · **Status:** FIXED
- **Sintoma (relato do usuário):** "o meu mic acusa sensibilidade muito baixa";
  a fala quase não aparecia e o nível exibido era baixo.
- **Causa:** o GhostWhisper **não aplicava ganho** algum, e o gate de fala é um
  limiar **absoluto** no sinal cru (`SPEECH_RMS_THRESHOLD`): microfones quietos
  nunca cruzavam o limiar, então não havia prévia, o passe final era rejeitado e
  o app parecia surdo. O Perssua aplica `local_whisper_user_mic_gain = 8` antes
  do gate e do ASR (BR-018).
- **Correção:** estágio de entrada do microfone
  ([`adapters/audio/microphone_front_end.rs`](../../../src-tauri/src/adapters/audio/microphone_front_end.rs)):
  **ganho** (1×–12×, padrão 8×, saturando em ±1) e **noise gate** (mudo + 600 ms
  de hangover), ambos só no microfone; controle **“Sensibilidade do microfone”**
  na aba *Transcrição & Áudio*; o painel de teste passa a mostrar o ganho
  aplicado e explica o silêncio do gate.
- **Referência (BR-015):** módulo 7125 de `build/static/js/main.f180d025.js`
  (`dy` = ganho, `Kv` = gate) — ver §6 da tech spec, incluindo os dois desvios
  deliberados (nunca descartar áudio; limiar = o do app).
- **Arquivos:** `domain/audio.rs`, `domain/settings.rs`,
  `adapters/audio/microphone_front_end.rs`, `lib.rs`,
  `application/transcription_pipeline.rs` (limiar unificado),
  `features/settings/{SettingsPage,AudioTestPanel}.tsx`,
  `core/domain/settings.ts`, `styles/globals.css`.
- **Evidência:** 162 testes Rust (+19) — ganho (saturação/identidade/inválidos),
  ganho levanta fala fraca acima do limiar, gate (fala, ruído, hangover, mudo
  inicial, desligável), front-end (ordem ganho→gate, delegação, leitura das
  configurações a cada `start`), settings (padrão 8×, faixa, round-trip, config
  antiga) — e 23 testes frontend (slider).
- **Task:** [`14-task.md`](tasks/14-task.md) · **Pendente:** monitor atrasado
  ([`15-task.md`](tasks/15-task.md)).

---

## ISS-013 — Fim de turno preso atrás da prévia em voo

- **Severidade:** Alta · **Status:** FIXED
- **Sintoma:** o texto demora a estabilizar no fim da fala; parece que "faltou o
  final" e a última frase aparece com atraso. Em fala contínua o texto fica
  vários segundos atrás.
- **Causa:** `TranscriptionPipeline::tick()` fazia a decodificação **na própria
  thread de agendamento**. Enquanto uma prévia decodificava (~1,7 s por chamada,
  medido em [`transcription-streaming.md`](architecture/transcription-streaming.md)),
  o loop ficava parado: o silêncio não era avaliado, o passe final esperava a
  prévia terminar e uma falha transitória **ainda avançava a cadência**
  (`last_decode_ms`), "queimando" 750 ms de áudio novo.
- **Referência (BR-015):** o Perssua resolve o mesmo com `isProcessing` +
  `pendingUtteranceComplete` + `finishProcessing`
  (`WhisperSessionStreaming.js`, `WhisperSessionCompletion.js`): o áudio continua
  sendo avaliado durante a decodificação e o turno fecha assim que ela volta.
- **Correção:**
  - decisão extraída para **`TurnScheduler` puro** (`application/turn_scheduler.rs`):
    `Wait | Preview{cut_ms} | Finalize{cut_ms}`, testável sem threads/relógio;
  - decodificação movida para **thread de trabalho própria** (`dispatch`);
    `tick()` apenas enfileira e segue avaliando o silêncio;
  - **I1/I2/I3** da tech spec: resultado aplicado por turno, cadência só avança em
    sucesso, e após falha só o relógio de ociosidade re-arma;
  - `Final` tem prioridade e nunca é coalescido; `stop()`/`flush()` aguardam-no
    com teto (`FLUSH_WAIT`);
  - se o passe final falhar, persiste a última prévia do turno (não perde o texto).
- **Arquivos:** `application/transcription_pipeline.rs`,
  `application/turn_scheduler.rs`, `application/session_service.rs`,
  `Cargo.toml` (perfil `dev` do `whisper-rs-sys`).
- **Evidência:** `tick_never_blocks_on_a_slow_decode` (tick < 150 ms com ASR de
  400 ms), `failed_preview_does_not_consume_the_preview_step`,
  `final_job_is_never_superseded`, 10 testes do scheduler; E2E
  `end_to_end_streams_preview_before_final` (prévia durante a fala + final).
- **Docs:** [`architecture/transcription-streaming.md`](architecture/transcription-streaming.md).

---

## ISS-014 — ASR enroscava após timeout e threads subdimensionadas

- **Severidade:** Alta · **Status:** FIXED
- **Sintoma:** depois de uma decodificação longa, o ASR passava um tempo sem
  produzir nada (só `AsrBusy`), e em máquinas pequenas tudo era lento.
- **Causa:**
  1. cada `transcribe` criava uma **thread nova (8 MB de pilha)** e tentava
     `try_lock` no estado; se o `full()` anterior estourou o timeout, o lock
     seguia preso e os pedidos seguintes falhavam em cascata, desperdiçando
     thread e log a cada 50 ms;
  2. `n_threads = núcleos/2` (em 2 núcleos, **1** thread) — o whisper.cpp usa
     `min(4, núcleos)`.
- **Correção:**
  - **thread de trabalho única** por contexto, dona do `WhisperState`, com fila:
    prévias **coalescidas** (só a mais recente), trabalho com `deadline` vencido
    é respondido sem decodificar, e o adaptador é descartado com segurança (a
    thread morre quando o último `Sender` cai);
  - `timeout_for(quality, audio_ms)`: prévia 15–30 s, final 30–120 s (a prévia é
    descartável; não segura o turno por 2 minutos);
  - `resolve_threads`: `min(4, núcleos)` com override `GHOSTWHISPER_ASR_THREADS`.
- **Arquivos:** `adapters/speech/whisper_speech_to_text.rs`.
- **Evidência:** `preview_timeout_is_shorter_than_final`,
  `timeouts_scale_with_audio_and_respect_caps`, `threads_are_bounded_and_overridable`,
  `transcribe_before_preload_is_not_ready`, `superseded_job_notifies_the_caller`,
  `final_job_is_never_superseded`; `benchmark_vad_overhead` (VAD não é o custo;
  4 threads ≈ 1,7× mais rápido que 2).---

## ISS-007 — Transcrição pouco confiável (streaming preview/final)

- **Severidade:** Alta · **Status:** FIXED
- **Sintoma:** transcrição lenta, texto que às vezes sumia/duplicava, e
  surgiam frases falsas em silêncio ("thanks for watching", "[BLANK_AUDIO]").
- **Referência:** análise estática (REA) do `Perssua 0.27.5` (AppImage Electron),
  módulo `electron/local-whisper` — mesmo `ggml-silero-v5.1.2.bin` de VAD.
- **Correção (equivalente em Rust):**
  - **`SpeechQuality`** (`Preview`/`Final`) na porta `SpeechToText`: preview usa
    busca **gulosa** (barata, revisável); o passe final usa **beam search**
    (`beam_size=5`) sobre o turno inteiro — texto autoritativo;
  - **buffer de turno** único com teto de 60 s (antes: janela fixa de 2 s);
  - **gate de fala sustentada** (`has_sustained_speech`: RMS > 0.008 por ≥160 ms)
    antes de chamar o Whisper — rejeita clique/ruído sem custo de decodificação;
  - **filtro de alucinação** (`domain::transcription::is_likely_hallucination`):
    frases-modelo do whisper.cpp, marcadores `[...]`/`(...)`, só-símbolos e
    repetição adjacente (≥4×);
  - **timeout por pedido** no `WhisperSpeechToText` (30 s base, ~15 s por segundo
    de áudio, teto 120 s) com a decodificação numa **thread isolada** — um chunk
    travado não congela mais o app;
  - **serialização do ASR** (`SerialisingSpeechToText`): uma decodificação por
    vez entre os canais, evitando contenção de CPU;
  - **telemetria de RTF** por passe (razão áudio/tempo real) nos logs.
- **UI:** preview em *itálico discreto* (`transcript__line--interim`) e o final
  substitui pelo **mesmo id de turno** (`{session}-t{n}`) — sem duplicar; só o
  final é persistido.
- **Evidência:** 112 testes Rust (incl. gate, corte, preview/final, filtro,
  serialização) + integração real JFK com beam search; 14 testes frontend.

---

## ISS-005 — Transcrição lenta e travamento (espiral de buffer)

- **Severidade:** Alta · **Status:** FIXED
- **Sintoma:** transcrição lenta; depois de um tempo **para de transcrever** e
  os logs não mudam.
- **Causa:** `TranscriptionPipeline::drain_and_process` fazia `std::mem::take`
  do **buffer inteiro** assim que passava de 3 s. Se o ASR demorasse mais que o
  tempo real (dois canais disputando CPU, modelo maior), o bloco seguinte vinha
  maior → cada `whisper_full` mais lento → **espiral**: um único `whisper_full`
  de dezenas de segundos, sem log algum (parecia travado). Buffer sem teto
  (memória crescia) e nenhum aviso quando a fonte de áudio morria.
- **Correção:**
  - janela limitada (2 s) com **corte em ponto de silêncio** (fim da última fala
    seguida de silêncio) → latência baixa e sem cortar palavras;
  - corte forçado em `MAX_WINDOW_SECONDS` (10 s) para fala contínua;
  - **teto rígido** do buffer (30 s) com descarte do mais antigo (backpressure) e
    contador de amostras descartadas;
  - **watchdog**: avisa nos logs quando não chega áudio por > 3 s (fonte parou);
  - logs `debug` por janela (buffered/cut/spans/vad_ms/asr_ms/dropped);
  - Whisper com `n_threads = núcleos/2` (evita contenção com 2 canais).
- **Arquivos:** `application/transcription_pipeline.rs`,
  `adapters/speech/whisper_speech_to_text.rs`.
- **Evidência:** testes de `decide_cut_ms`, drain, teto de buffer; integração
  real `end_to_end_transcribes_jfk_speech` (transcreve "country").

---

## ISS-006 — Teste de áudio + fonte que para em silêncio

- **Severidade:** Média · **Status:** FIXED
- **Sintoma:** sem forma de conferir o microfone; quando a captura falhava, o
  app continuava "escutando" sem produzir nada.
- **Correção:**
  - **aba "Teste de áudio"** nas configurações: escolhe canal/dispositivo,
    inicia a captura e mostra o nível (RMS/pico + barra) em tempo real;
    desabilitado durante sessão ativa;
  - watchdog de ausência de áudio (ISS-005) sinaliza a fonte parada nos logs.
- **Arquivos:** `src/features/settings/AudioTestPanel.tsx`,
  `src/adapters/ipc/audio.ts`, `SettingsPage.tsx`, `styles/globals.css`.

---

## ISS-002 — Janela de configurações abre só uma vez

- **Severidade:** Alta · **Status:** FIXED
- **Sintoma:** "Ajustes" funcionou na primeira vez; depois não abria mais.
- **Causa:** a janela `settings` (`visible: false`) era **destruída** ao fechar
  (sem handler `CloseRequested`); depois `get_webview_window("settings")` → `None`
  e `settings_open` falhava **em silêncio** (o front usava `invokeSafe` sem catch).
- **Correção:** handler `CloseRequested` → `prevent_close()` + `hide()` (a janela
  nunca é destruída); `settings_open`/tray fazem `unminimize()+show()+set_focus()`
  e logam falhas; janela `alwaysOnTop` para não ficar atrás do overlay; erro de
  abertura agora aparece no overlay.
- **Arquivos:** `src-tauri/src/lib.rs`, `ipc/settings_commands.rs`,
  `tauri.conf.json`, `src/features/overlay/Overlay.tsx`.

---

## ISS-003 — Provider rejeita `max_tokens`

- **Severidade:** Alta · **Status:** FIXED
- **Sintoma:** `400 Unsupported parameter: 'max_tokens' is not supported with this
  model. Use 'max_completion_tokens' instead.` (ex.: `gpt-6-luna`).
- **Causa:** o adaptador OpenAI-compatível sempre enviava `max_tokens`.
- **Correção:** em `400 unsupported_parameter`, o parâmetro recusado é
  renomeado/removido e a chamada é reenviada (até 3×): `max_tokens` →
  `max_completion_tokens`; outros (`temperature`, …) são removidos. Vale para
  OpenAI, OpenRouter e Custom.
- **Arquivos:** `adapters/ai/openai_compatible.rs` (+ mock de sequência em
  `application/test_support.rs`).

---

## ISS-004 — Idiomas e modelo do Whisper ignorados

- **Severidade:** Média · **Status:** FIXED
- **Sintoma:** detecção de idioma errava com frequência; seleção de modelo
  whisper não tinha efeito.
- **Causa:** `source_language`/`target_language` eram texto livre e o Whisper
  usava `set_language("auto")` fixo; `whisper_model` era fixado em `"tiny"` no
  composition root.
- **Correção:** seletores **Auto/Português/English** para origem e destino;
  `WhisperSpeechToText` recebe o idioma; `WhisperSpeechEngineFactory` lê
  `whisper_model` + `source_language` das settings a cada sessão; `validate()`
  restringe os idiomas e `Settings::sanitized()` corrige configs antigas;
  destino `auto` (ou igual à origem) **não traduz**; prompts usam nomes legíveis.
- **Arquivos:** `domain/settings.rs`, `adapters/speech/*`,
  `application/{translation_service,prompts}.rs`, `lib.rs`,
  `src/core/domain/settings.ts`, `src/features/settings/SettingsPage.tsx`,
  `src/features/overlay/Overlay.tsx`.

---

## ISS-001 — Clique no botão do overlay não captura de forma confiável

- **Severidade:** Média
- **Status:** FIXED (definitivo)
- **Reportado em:** 2026-10-17 (Tarefa 01) · **Corrigido em:** Tarefa 07 e
  reescrito em 2026-10-17 (follow-up)
- **Relaciona-se a:** Tarefa 01/07, `overlay.md` §3

### Sintoma
O clique nos botões do overlay não era capturado de forma confiável, com atraso
perceptível para o mouse "reconhecer" os botões.

### Causa
A correção da Tarefa 07 (polling do cursor + histerese) reduziu cliques perdidos,
mas a latência vinha de **polling + round-trips à thread principal do GTK** e da
alternância global do input shape (janela inteira ↔ 1×1).

### Correção definitiva
Click-through por **input shape do X11** (`ShapeRectangles`, `kind=Input`) com a
união das regiões interativas — sem polling e sem corrida. Ver
`adapters/overlay/x11_input_shape_click_through.rs`. O controlador por polling
permanece apenas como **fallback** (Wayland/sem SHAPE).

### Evidências
- Boot registra `click-through via X11 input-shape (sem polling)`.
- Testes do domínio (margem/histerese) e do controlador de fallback.
- Pendente: validação manual de hover/clique em GNOME/KDE (X11).



---

## ISS-008 — Uma linha e depois silêncio (custo de decodificação descontrolado)

- **Severidade:** Crítica · **Status:** FIXED
- **Sintoma:** uma linha de transcrição aparecia e nenhuma outra (logs paravam).
- **Causas (três, todas no caminho de streaming):**
  1. **Preview decodificava o turno inteiro** a cada ~900 ms: o custo crescia a
     cada passo e, com beam search no passe final segurando a fila global, o
     ASR saturava. O preview passou a decodificar só a **janela recente de 2 s**
     (custo constante); o turno inteiro continua no passe final.
  2. **Corte no teto devolvia o buffer inteiro** (`Some(buffered_ms)`), não o
     teto: com o VAD throttled o buffer podia ter 30–60 s e ser decodificado com
     beam search de uma vez. Passou a cortar **no teto** (`Some(max_ms)`).
  3. **Throttle do VAD por milissegundos absolutos**: ao consumir um turno o
     buffer encolhia, mas `last_vad_ms` guardava o pico antigo — o VAD ficava
     bloqueado até o buffer *ultrapassar* esse pico. Passou a contar **amostras
     de crescimento** (auto-corrige quando o buffer encolhe).
- **Bônus:** `stop()` agora faz **flush** do turno em aberto (a última frase
  não era publicada) e `process()` deixou de duplicar o caminho antigo.
- **Regressão coberta:** `preview_decodes_only_the_recent_window`,
  `vad_resumes_right_after_a_turn_is_consumed`,
  `turn_emits_preview_then_final_with_the_same_id`, `cut_forced_at_max`.
- **E2E real (JFK) verde** com o fluxo preview→final e beam search.

---

## ISS-009 — Trocar de modelo não dava retorno ao usuário

- **Severidade:** Alta · **Status:** FIXED
- **Causa:** o modelo era baixado/carregado **dentro da primeira transcrição**,
  em silêncio — e um modelo grande podia estourar o timeout da requisição.
- **Correção:**
  - `WhisperSpeechToText::preload()` baixa/carrega em **background** no início
    da sessão;
  - novo evento `asr_status` (`preparing`/`ready`/`failed`) com o modelo e o
    tamanho aproximado (~75 MB … ~1,5 GB);
  - banner no overlay com spinner e erro destacado
    (`AsrStatusBanner` + hook `useAsrStatus`);
  - enquanto não está pronto o pipeline recebe `AsrNotReady` e **aguarda**
    (sem warn), em vez de travar;
  - **`try_lock`** no worker do Whisper: se uma decodificação estourou o timeout
    e ainda roda, a próxima retorna `AsrBusy` em vez de **deadlock permanente**
    do ASR (o `lock()` ficava preso enquanto segurava a fila global).

---

## ISS-010 — Transcrição não "flui" (turno longo, prévia de 2 s e VAD caro)

- **Severidade:** Alta · **Status:** FIXED
- **Sintoma:** a transcrição demora a aparecer, sai em pedaços esparsos e o texto
  do turno parece "piscar"/sumir durante uma fala contínua.
- **Referência:** análise (REA) do `Perssua 0.27.5` (AppImage), módulo
  `electron/local-whisper` (`WhisperSession.js`, `WhisperSessionStreaming.js`,
  `WhisperSessionSupport.js`, `WhisperVad.js`, `WhisperServerManager.js`).
  Regra do projeto: **BR-015 / R11–R13** (Perssua é a fonte de decisão).
- **Causas (no GhostWhisper):**
  1. **VAD Silero separado sobre o buffer inteiro** a cada ~250 ms — custo O(n)
     que cresce com a fala e trava o worker.
  2. **Prévia decodificava só os últimos 2 s** e reaproveitava o id do turno: o
     texto exibido era substituído por um trecho curto, em vez de crescer.
  3. **Cadência/turno longos:** prévia a cada 900 ms e teto de 10 s; faltava o
     comportamento "fluido" do Perssua.
  4. **Parâmetros de decode sem os controles do Perssua** (`temperature_inc = 0.2`
     → fallback por temperatura, até ~5 decodificações extras; sem
     `suppress_nst`/`entropy_thold`).
  5. **Dois contextos whisper** (um por canal) carregavam o modelo duas vezes.
- **Correção (transposição do Perssua para Rust):**
  - `WhisperSpeechToText`: **VAD nativo do whisper.cpp** (`set_vad_model_path` +
    `enable_vad` + `WhisperVadParams` com `threshold 0.5`,
    `min_speech 250`, `min_silence 300`, `speech_pad 200`); modelo Silero
    resolvido no `preload()`;
  - decode com `temperature 0`, `temperature_inc 0`, `entropy_thold 2.2`,
    `suppress_nst true`;
  - `TranscriptionPipeline` reescrito: buffer de turno (teto 60 s), **prévias
    frequentes sobre o turno que cresce** (áudio mínimo 0,35 s, passo 0,75 s,
    primeira prévia em 1,2 s, ociosidade 1 s) publicando **texto rolante**;
    **fechamento por 1 s de silêncio** (ou teto de 55 s) com passe final (beam),
    além do encerramento pelo **fim de fala do VAD do whisper**
    (`endedInNoise`, cobre ruído de fundo);
  - removido o VAD Silero separado (porta `VoiceActivityDetector` + adapter) e o
    gate passou a ser RMS barato + VAD do whisper;
  - `WhisperSpeechEngineFactory` passa a **compartilhar um único contexto** entre
    os canais (uma carga de modelo), mantendo a fila global de decodificação.
- **Arquivos:** `adapters/speech/whisper_speech_to_text.rs`,
  `application/transcription_pipeline.rs`, `adapters/speech/whisper_speech_engine_factory.rs`,
  `ports/speech_engine_factory.rs`, `application/session_service.rs`,
  `domain/transcription.rs`, `ports/voice_activity.rs` (removido),
  `adapters/speech/silero_voice_activity.rs` (removido).

---

## ISS-011 — Provider OpenAI recusa `temperature` nos modelos novos (400)

- **Severidade:** Alta · **Status:** FIXED
- **Erro reportado:** `400 unsupported_value` — "Unsupported value: 'temperature'
  does not support 0.3 with this model. Only the default (1) value is
  supported." (modelos gpt-5/o1/o3/o4).
- **Referência (BR-015):** Perssua `electron/utils/providerParameterErrors.js` +
  `providerParameterContracts.js` — classifica `unsupported_parameter` **e**
  `unsupported_value`, resolve o parâmetro contra um contrato de campos
  opcionais e **omiti + reenvia**; `builtInOmissions` não envia `temperature`
  para modelos que não suportam.
- **Correção (transposta para Rust):**
  - `adapters/ai/openai_compatible.rs`: `rejected_param` reconhece os códigos
    `unsupported_value`/`invalid_value` e as mensagens "Unsupported value"
    /"Only the default"/"does not support";
  - `apply_rejected_param_fallback` só omiti parâmetros **opcionais** da lista
    de amostragem (`temperature`, `top_p`, `seed`, `penalties`,
    `max_tokens`/`max_completion_tokens`) — `model`/`messages` nunca;
  - `temperature_supported(model)`: `gpt-5*`/`o1*`/`o3*`/`o4*` não recebem
    `temperature` (primeiro envio já correto, sem desperdiçar uma chamada).
- **Evidência:** testes com o JSON exato do erro do usuário; mock de sequência
  400→200 sem `temperature`; 122 testes Rust.

---

## ISS-012 — Configurações em abas + chave de API redactada

- **Severidade:** Média · **Status:** FIXED
- **Pedido do usuário:** janela de configurações em estilo **abas** (Perssua:
  `settings.tabs` → "API tab", "Audio & Screen", "Shortcuts"…) e a chave de API
  exibida **redactada** para indicar que está configurada.
- **Referência (BR-015):** Perssua mostra status "Connected"/"Add API Key" nos
  cards de provider e organiza as configurações em abas.
- **Correção:**
  - `SettingsPage.tsx`: tablist com 4 abas — **Fontes de IA** (provider, modelo,
    base URL, chave, ações), **Transcrição & Áudio** (whisper, idiomas,
    tradução, teste de captura), **Ajustes** (stealth), **Atalhos**; status
    global no rodapé;
  - chave configurada é exibida mascarada (`•••••••·abcd`, somente leitura,
    estilo "dashed"); Hidden: form de nova chave só aparece quando não há chave
    ou em "Trocar chave";
  - `domain::settings::redact_secret` (regra pura, testada) + novo comando
    `settings_key_preview` que devolve **apenas** a máscara — o segredo
    inteiro nunca sai do backend.
- **Arquivos:** `features/settings/SettingsPage.tsx` (+ teste),
  `styles/globals.css`, `adapters/ipc/settings.ts`, `ipc/settings_commands.rs`,
  `domain/settings.rs`, `lib.rs`.
- **Evidência:** 4 testes do componente (abas, chave mascarada, nova chave,
    troca de aba); 22 testes frontend.
- **Evidência:** 114 testes Rust (10 novos/reescritos de streaming) + 18 frontend;
  clippy/fmt/tsc limpos; E2E JFK (`--ignored`) mantido.
