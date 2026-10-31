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
| ISS-016 | Pausar/continuar a gravação apagava as mensagens (sem conceito de sessão) | Alta | FIXED |
| ISS-017 | Seletor de canais travado durante a captura (`disabled` enquanto gravava) | Média | FIXED |
| ISS-087 | Canal "Sistema" indisponível sem `pactl` e troca para "Ambos" silenciosa | Alta | FIXED |
| ISS-088 | Wayland (Ubuntu 26.04): notch atrás, atalhos e captura quebrados | Alta | FIXED |
| ISS-089 | Gravação de tela indisponível no Wayland (ScreenCast/GStreamer) | Alta | FIXED |
| ISS-019 | Whisper repete a mesma frase sem parar e a repetição entra na conversa | Média | FIXED |
| ISS-020 | Interface travava/engasgava: IPC demorado na main thread e IPC em excesso pela barra de nível | Alta | FIXED |
| ISS-021 | Bolha "em andamento" ficava pendurada com o texto cru da prévia | Média | FIXED |

> `ISS-018` (falas de canais diferentes colidiam na linha temporal) vive em
> [`../chat/issues.md`](../chat/issues.md) — a numeração é única no repositório e
> continua entre PRDs.

---

## ISS-021 — Bolha "em andamento" pendurada com o texto cru da prévia

- **Severidade:** Média · **Status:** FIXED
- **Sintoma (relato do usuário):** *"o balão da última frase processada vem bem
  certinho, porém ele cria um balão mais escuro (como se fosse loading ou
  processando) com uma frase que não faz sentido, acho que é a frase antes dele
  refinar. Por que fica esse balão estranho sempre como primeiro balão?"*
- **Causa:** a bolha mais escura é a **prévia "em andamento"** (paridade com o
  `.unified-message-current` do Perssua: texto cru do parcial, opacidade reduzida e
  indicador `…`) — por desenho ela aparece **antes** do texto refinado e é
  substituída no lugar quando o turno fecha. O bug é que, **quando o passe final não
  produz texto**, nada era publicado para o id do turno e a prévia ficava
  **pendurada para sempre** com o texto cru:
  - final filtrado como alucinação/loop/silêncio (`clean` devolve `None`);
  - turno descartado pelo gate de fala sustentada (`begin_final`);
  - `flush` sem buffer suficiente (parar/pausar com sobra pequena).
  Como a prévia pendurada é a única marca daquela fala, ela fica com o **texto cru**
  (o "antes de refinar") e — se for a primeira fala da sessão — **é literalmente a
  primeira bolha** da conversa, enquanto o resto cresce abaixo. O filtro de loop da
  ISS-019 aumentou a frequência do caso (mais finais descartados).
- **Correção (task 23):**
  - `TranscriptionPipeline::retract_turn`: quando o turno termina sem texto **e uma
    prévia foi mostrada**, publica um final de **texto vazio** com o mesmo id — a UI
    substitui a prévia e a bolha sai. O vazio não vira bolha nem é persistido
    (`publish_turn` continua sendo o único que grava).
  - Chamado nos três caminhos: gate de fala, final filtrado e `flush` sem buffer.
  - `splitChatTimeline` ignora prévias em branco; `useTranslation` não traduz final
    vazio (não é fala).
  - A bolha em andamento ganhou `title` "transcrevendo — o texto ainda pode mudar",
    para o estado não ser confundido com erro/carregamento.
- **Arquivos:** `application/transcription_pipeline.rs`, `core/domain/chat.ts`,
  `core/application/useTranslation.ts`, `features/chat/ChatBubble.tsx`.
- **Evidência:** Rust — `discarded_final_retracts_the_pending_preview` (prévia →
  final filtrado ⇒ prévia + retratação de mesmo id, **nada persistido**) e
  `discarded_final_without_preview_does_not_publish_anything`; frontend —
  retratação some da conversa (`chat.test.ts`), prévia tem `title` (`ChatView.test.tsx`),
  tradução ignora vazio (`useTranslation.test.ts`).
- **Nota de UX:** a prévia **continua** aparecendo durante a fala (é o texto ao vivo
  que o usuário pediu): ela é dimmed, com `…` e agora com o `title` explicando.
- **Task:** [`23-task.md`](tasks/23-task.md)

---

## ISS-020 — Interface travava e os botões não respondiam

- **Severidade:** Alta · **Status:** FIXED
- **Sintoma (relato do usuário):** *"a main-thread sendo segurada, não conseguia
  clicar nos botões, algumas vezes o app até congelava"*.
- **Causas (três, somadas):**
  1. **IPC síncrono na main thread.** O IPC do Tauri chega na main thread (loop
     GTK) e `#[tauri::command]` **sem `async` é chamado inline ali**
     (`tauri-macros`: `body_blocking`). Vários comandos demoram segundos:
     `session_start/stop/pause/resume/set_channels/continue` (o `pipeline.stop()`
     espera o passe final do turno — `FLUSH_WAIT = 20 s`), `llm_insight` /
     `llm_translate` / `settings_validate_key` (`ureq` **bloqueante**, timeout de
     **60 s**), `audio_*` e `system_audio_*` (abrir stream, `pactl`), keyring
     (DBus), `db_*` (leem o banco que o ASR escreve). Durante esse tempo a janela
     não repinta, não processa clique nem arraste: o app *parece* congelado.
  2. **Tempestade de IPC.** `useInteractiveRegions` observava `class`/`style` no
     `document.body` inteiro e **a barra de nível (`.levels__fill`) muda de largura
     ~10×/s por canal** — cada mudança republicava as regiões para o X11 (comando
     síncrono, main thread) ~20×/s, sem que nada clicável tivesse mudado.
  3. **Re-render a 20 Hz.** `useAudioLevel` vivia no `Overlay`, então **cada**
     evento de nível re-renderizava a conversa inteira (até 300 bolhas) e o resto do
     painel, 20×/s na thread de render do webview.
  4. **Atalhos e bandeja.** O menu da bandeja roda na **main thread** (sinal do GTK)
     e o listener de atalhos do X11 na thread dele (`global-hotkey` — verificado no
     fonte); os dois chamavam o mesmo `dispatch_shortcut`, que faz
     `session.stop()` (passe final, até 20 s) e `insight.generate()` (**HTTP
     bloqueante**, até 60 s). Pela bandeja, congelava a interface; pelo atalho,
     engolia os próximos atalhos.
- **Correção (task 22):**
  - **37 comandos** passaram a `#[tauri::command(async)]` (executam no runtime, fora
    da main thread): sessão, áudio/sistema, LLM, settings/chave, export, banco e
    `overlay_set_interactive_regions`. Ficaram síncronos só os instantâneos
    (`ping`, getters de estado) e os que **exigem** a main thread (criação da janela
    de settings, operações de janela do overlay). Regra documentada em
    [`architecture/engineering-guidelines.md`](architecture/engineering-guidelines.md) §8.
  - `useInteractiveRegions` **ignora mutações dentro de `.levels-group`** — o
    input-shape só é republicado quando uma região clicável realmente muda.
  - `LevelMeters` assina `audio_level` **internamente** (memoizado) e a `ChatView`
    é memoizada: o evento de nível deixou de re-renderizar a conversa.
  - `dispatch_shortcut` (atalho global e menu da bandeja) manda `ToggleSession` e
    `RequestInsight` para uma **thread própria**; só as ações instantâneas
    (`ToggleOverlay`, `ToggleStealth`) seguem na thread do evento.
  - `HistoryPanel` memoiza a conversão da linha temporal em bolhas (sessão longa não
    reconstrói a lista a cada render).
- **Arquivos:** `src-tauri/src/ipc/*.rs`, `src-tauri/src/lib.rs`
  (`dispatch_shortcut`), `src/core/application/useInteractiveRegions.ts`,
  `src/features/overlay/LevelMeters.tsx`, `src/features/overlay/Overlay.tsx`,
  `src/features/chat/ChatView.tsx`.
- **Evidência:** 6 testes frontend novos (`useInteractiveRegions`: publica ao montar,
  **não** republica com a barra mudando, republica quando uma região muda;
  `LevelMeters`: nível por canal, zerado fora de captura, estado sem canais). Suíte:
  80 frontend; `cargo check --all-targets` limpo.
- **Task:** [`22-task.md`](tasks/22-task.md)

---

## ISS-019 — Loop de frase do Whisper entra na conversa

- **Severidade:** Média · **Status:** FIXED
- **Sintoma (relato do usuário):** *"às vezes um chat fica com algumas frases
  repetidas, como 'O que eu quero fazer? O que eu quero fazer? …'"* — a mesma frase
  dezenas de vezes numa única fala.
- **Causa:** loop de decodificação do Whisper na **própria janela de 30 s**. A
  guarda do `whisper.cpp` só descarta a sequência repetitiva quando ela tem
  **mais de 32 tokens E** entropia média abaixo de `entropy_thold`
  (`whisper.cpp`: `if (decoder.sequence.result_len > 32 && decoder.sequence.entropy
  < params.entropy_thold) decoder.failed = true;`). Com `entropy_thold = 2.2` e
  `temperature_inc = 0` — configuração herdada do Perssua
  (`WhisperServerManager.js`) e já espelhada por nós — a repetição **confiante**
  morre, mas a repetição de **frase** com entropia acima do limiar passa inteira.
  O nosso filtro (`domain::transcription::is_likely_hallucination`) não cobria esse
  caso: só detectava **palavra** repetida ≥4× seguidas (`a a a a`).
- **Correção (task 21):**
  - `has_repeated_phrase_loop` — transposição fiel do detector `h` do Perssua
    (`build/static/js/22.75b118cd.chunk.js`): frase de até 5 palavras repetida
    ≥3× a partir do início cobrindo ≥70% do texto, mínimo 6 palavras. O Perssua só
    aplica isso ao provider Gemini; aplicamos a todos (desvio documentado — nosso
    whisper local repete assim).
  - `strip_phrase_loop` — encolhe o loop que **termina** o texto para **uma**
    ocorrência, mantendo a fala que veio antes ("bom dia pessoal" + "O que eu quero
    fazer?" ×15 → "bom dia pessoal O que eu quero fazer?") e o loop puro para a
    frase uma vez ("…×15" → "O que eu quero fazer?"). **Desvio consciente:** o
    Perssua descarta o texto inteiro (`null`); aqui o turno já passou pelo portão de
    fala sustentada, então houve fala real e devolver vazio perderia a fala.
    Exceção: loop de **uma palavra só** ("a a a a a") é descartado inteiro — palavra
    repetida é alucinação, não vira "a" na conversa.
  - Guardas contra falso positivo: ≥3 repetições de frase (≤12 palavras no corte) e
    ≥6 palavras de repetição — "não não não" e "muito muito muito bom" ficam
    intactos.
- **Arquivos:** `domain/transcription.rs`, `application/transcription_pipeline.rs`
  (`clean`), `prds/mvp/tasks/21-task.md`.
- **Evidência:** +8 testes Rust (6 no domínio — incluindo a frase exata do relato,
  o loop puro encolhido, o loop de uma palavra descartado e as ênfases naturais
  intactas — e 2 no pipeline: loop encolhido para uma ocorrência; fala + loop vira
  fala + uma ocorrência, na prévia **e** no final). Suíte: 225 testes, clippy/fmt
  limpos.
- **Limite conhecido:** uma lista legitimamente repetida 3× seguidas
  (ex.: "primeiro segundo terceiro" ×3) é **encolhida** para uma ocorrência — o
  conteúdo fica, a repetição sai.
- **Limite conhecido 2:** uma alucinação **não** repetida (uma frase só, plausível,
  dita no silêncio) continua possível na **prévia** — o portão de fala sustentada
  (`has_sustained_speech`) só age no fechamento do turno, então ela não é
  persistida, mas pode piscar na bolha "em andamento". O filtro desta task cobre
  exatamente o caso relatado (a repetição).
- **Task:** [`21-task.md`](tasks/21-task.md)

## ISS-017 — Não dava para trocar os canais com a sessão gravando

- **Severidade:** Média · **Status:** FIXED
- **Sintoma (relato do usuário):** *"por que não é permitido mais trocar os canais a
  ouvir? isso deveria ser sempre possível, inclusive sem precisar pausar ou parar"*.
- **Causa:** o `<select>` de canais ficava `disabled={listening}` — herdado de
  antes do conceito de sessão, quando trocar de canal exigia encerrar e começar
  outra sessão. Somado à sessão contínua (BR-019), virou um contrassenso.
- **Correção (task 20):** `SessionController::set_channels` troca a captura
  **mantendo a sessão** (mesmo id, mesma origem de tempo): para os canais que
  saem, liga os que entram, atualiza a escolha quando pausada e preserva os
  canais que seguem funcionando. O seletor fica sempre habilitado e reflete o que
  está sendo captado.
- **Arquivos:** `ports/session_controller.rs`, `application/session_service.rs`,
  `ipc/session_commands.rs`, `lib.rs`, `core/application/useSessionState.ts`,
  `adapters/ipc/session.ts`, `features/session/SessionControls.tsx` (+ testes).
- **Evidência:** +9 testes Rust e +4 frontend (ver [`20-task.md`](tasks/20-task.md)).
- **Task:** [`20-task.md`](tasks/20-task.md)

---

## ISS-016 — Pausar/continuar apagava as mensagens

- **Severidade:** Alta · **Status:** FIXED
- **Sintoma (relato do usuário):** *"toda vez que eu pauso e continuo a
  gravação, a aplicação está apagando as mensagens e reiniciando"*.
- **Causa:** faltava o **conceito de sessão** no ciclo de captura. Cada
  `session_start` encerrava a anterior e criava **um id novo**
  (`SessionService::start`), e a UI (`useLiveTranscription`) **zerava a lista**
  quando o `session_id` mudava. A linha do tempo antiga continuava no banco, sem
  nenhum caminho na UI para vê-la. Os timestamps eram relativos ao **início da
  captura**, então uma retomada também reiniciaria o relógio em 0.
- **Correção (BR-019):** pausar suspende a captura e **mantém a sessão aberta**
  (mesmo id); retomar volta a capturar nela; continuar reativa uma sessão
  encerrada; só "Nova sessão" cria outro id. A UI recarrega o histórico
  persistido da sessão em vez de começar vazia, e cada linha mostra `mm:ss` desde
  o início da sessão (`with_time_origin`). O id do segmento passou a carregar a
  origem de tempo (sem colisão ao continuar) e sessões órfãs são encerradas no
  boot.
- **Referência (BR-015):** `transcriptionPreview.pause/resume` ("Pausado — o áudio
  não está sendo capturado") e `DatabaseManager.{resumeSession,endAllActiveSessions}`.
- **Arquivos:** `ports/session_controller.rs`, `ports/repositories.rs`,
  `ports/event_publisher.rs`, `application/session_service.rs`,
  `application/transcription_pipeline.rs`, `adapters/persistence/{sqlite,memory}/*`,
  `ipc/session_commands.rs`, `lib.rs`, `core/{domain/transcript.ts,application/useLiveTranscription.ts,application/useSessionState.ts}`,
  `features/session/SessionControls.tsx`, `features/overlay/*`, `features/transcript/TranscriptView.tsx`.
- **Evidência:** 175 testes Rust (+13) e 40 frontend (+17) — ver
  [`16-task.md`](tasks/16-task.md).
- **Task:** [`16-task.md`](tasks/16-task.md) · **Continua em:** histórico/export nas
  tasks 17/18.

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

---

## ISS-087 — Canal "Sistema" indisponível sem `pactl` e troca para "Ambos" silenciosa

- **Severidade:** Alta · **Status:** FIXED (task 25)
- **Sintoma (relato do usuário):** *"iniciei o programa e comecou só capturando
  meu mic. ao tentar mudar para 'ambos' nao mudava, e nem dava alguma mensagem
  de feedback em caso de erro"*.
- **Causa raiz:** a descoberta do monitor do sistema usava **`pactl`**
  (`get-default-sink`/`list short sources`) sem alternativa. Em PipeWire com
  `pipewire-pulse` mas **sem o pacote `pulseaudio-utils`** (o caso do ambiente),
  `pactl` não existe: `default_monitor_name()` falhava, `open_channel` devolvia
  `None` e `set_channels` mantinha o microfone (política da task 20) e retornava
  `Ok`. A UI continuava em "Microfone" e nada explicava a falha. A conexão
  libpulse também era aberta dentro da thread, então erros de fonte só apareciam
  no log.
- **Correção:**
  - `default_sink_name` tenta `pactl` → `wpctl inspect @DEFAULT_AUDIO_SINK@` →
    `pw-metadata -n default`; sem `pactl`, `list_devices` cai para os sinks do
    `pw-cli ls Node` (ou para o monitor padrão) — ISS-087.
  - `Simple::new` é aberto **antes** de `start` devolver `Ok`: falha de conexão
    chega ao chamador.
  - `SessionWarningEvent` (`session_warning`) é emitido por `SessionService`
    quando um canal não abre; `useSessionState` traduz e o overlay mostra
    "Não foi possível capturar o áudio do sistema…" em vez de silêncio.
- **Testes anti-regressão:** parsers `wpctl`/`pw-metadata`/`pw-cli`; teste real
  (ignorado) `lists_monitors_on_real_server` e
  `opens_the_default_monitor_on_real_server` passam no host sem `pactl`;
  `start_warns_about_the_channel_that_could_not_capture` e
  `set_channels_keeps_the_working_one_when_the_other_fails` verificam o aviso;
  `useSessionState` cobre o evento `session_warning`.
- **Arquivos:** `src-tauri/src/adapters/audio/pulse_system_audio_source.rs`,
  `src-tauri/src/ports/event_publisher.rs`,
  `src-tauri/src/adapters/events/tauri_event_publisher.rs`,
  `src-tauri/src/application/session_service.rs`,
  `src/core/application/useSessionState.ts`, locale `pt`/`en`.
- **Task:** [`25-task.md`](tasks/25-task.md)
- **Dano em dados existentes:** nenhum.

---

## ISS-088 — Wayland (Ubuntu 26.04): notch atrás, atalhos e captura quebrados

- **Severidade:** Alta · **Status:** FIXED (task 26)
- **Sintoma (relato do usuário):** depois da atualização para Ubuntu 26.04, o
  notch deixou de ficar sempre no topo; atalhos como `Ctrl+Shift+C` e a captura
  de tela (`Ctrl+Shift+K`) pararam; a captura falhava com
  `X11Error { … GetImage }`.
- **Causa raiz:** o Ubuntu 26.04/GNOME 50 passou a usar **Wayland puro** (não há
  mais sessão Xorg instalada). O app era X11/XWayland: `keep_above` e
  `set_position` são ignorados em janelas Wayland nativas; `XGrabKey` não recebe
  teclas de apps Wayland (Chrome); `GetImage` na root do XWayland falha com
  `BadMatch` e não vê janelas nativas.
- **Correção:**
  - `domain/session_backend.rs` decide X11 × Wayland; no Wayland o app força
    `GDK_BACKEND=x11` antes do GTK para o overlay continuar em XWayland
    (`_NET_WM_STATE_ABOVE`, posição e input-shape funcionam).
  - Atalhos globais via portal **GlobalShortcuts** (`PortalGlobalShortcutRegistry`)
    com o mesmo contrato `GlobalShortcutRegistry`; X11 mantém o plugin do Tauri.
  - O host portal exige app id (xdg-desktop-portal ≥1.20): o app registra
    `com.ghostwhisper.app` na **mesma conexão D-Bus** e mantém um `.desktop`
    oculto em `$XDG_DATA_HOME/applications` (ISS-088).
  - Captura no Wayland via portal **Screenshot** em modo interativo (UI do
    GNOME; o compositor exige confirmação). O arquivo é lido, recomprimido e
    apagado; X11 continua `GetImage` não-interativo.
  - O build único baixa e embute o **runtime atual do AppImage** (o antigo não
    monta no Ubuntu 26.04 por causa do `fusermount3` 3.18).
- **Testes anti-regressão:** decisão de backend; mapeamento de acelerador →
  trigger do portal e id → ação; app id/`.desktop`; URI do portal; a suíte X11
  existente continua igual. Manual no GNOME 50: atalho funciona com o Chrome
  ativo, notch fica acima dele e a captura retorna a imagem pela UI do GNOME.
- **Arquivos:** `src-tauri/src/domain/session_backend.rs`,
  `src-tauri/src/adapters/system/portal.rs`,
  `src-tauri/src/adapters/shortcuts/portal_global_shortcut_registry.rs`,
  `src-tauri/src/adapters/screen/portal_screen_capture.rs`,
  `src-tauri/src/ports/*`, `src-tauri/src/lib.rs`,
  `src-tauri/src/ipc/screen_commands.rs`, `scripts/tauri-build-linux.sh`.
- **Task:** [`26-task.md`](tasks/26-task.md)
- **Dano em dados existentes:** nenhum.

## ISS-089 — Gravação de tela indisponível no Wayland (ScreenCast/GStreamer)

- **Severidade:** Alta · **Status:** FIXED (task 27)
- **Sintoma (relato do usuário):** no Ubuntu 26.04/GNOME 50 a gravação da sessão
  aparecia indisponível; o caminho antigo (`ffmpeg`/`x11grab`) não via as janelas
  Wayland nativas, o `ffmpeg` do host não tinha entrada `pulse` e não havia
  `parec` para o áudio.
- **Causa raiz:** o `DISPLAY` do XWayland não contém as janelas nativas; gravar a
  root produz desktop vazio/incompleto. O compositor só entrega a captura por
  `org.freedesktop.portal.ScreenCast` (PipeWire). Sem Pulse no `ffmpeg`, o áudio
  precisa ser capturado pelo GStreamer no mesmo pipeline.
- **Correção:**
  - `PortalScreenRecorder` abre uma sessão ScreenCast (`Monitor | Window`, fonte
    única, cursor embutido), mantém a sessão entre pausas e grava cada segmento
    com `gst-launch` (`pipewiresrc` → `x264enc` → `matroskamux` + `pulsesrc` →
    `audiomixer` → `avenc_aac`). O fd remoto é duplicado no filho via `pre_exec`
    e cada `resume` chama `OpenPipeWireRemote` de novo.
  - Áudio-only continua no GStreamer (`pulsesrc` → `flacenc`), sem portal.
  - `pause` encerra o `gst-launch` com SIGINT + `-e`; `stop` fecha a sessão do
    portal. A finalização segue no `FfmpegRecordingFinalizer`.
  - `gstreamer_args` (builder puro/testado) e `PortalScreenTargets` (alvo único
    do seletor do GNOME). Capacidades novas: `GStreamerMissing`,
    `target_mode = "portal"`, `follow_supported = false`.
  - A UI esconde o alvo interno e o "seguir a janela ativa" no modo portal e
    explica que a fonte é escolhida no GNOME; l10n pt/en.
  - X11 mantém `FfmpegScreenRecorder` + `X11ScreenTargets` sem mudança.
- **Testes anti-regressão:** builder do pipeline (muxer/mixer antes das
  ramificações, caps, FLAC), decisão de capacidades, alvo do portal, seleção do
  recorder por sessão e testes reais ignorados:
  `records_and_finalizes_a_real_portal_stream` (2 segmentos, pausa/retomada,
  MP4 9,0 s com H.264 1920×1080 + AAC 48 kHz estéreo) e
  `records_audio_only_with_gstreamer`.
- **Arquivos:** `src-tauri/src/adapters/recording/portal_screen_recorder.rs`,
  `src-tauri/src/adapters/recording/gstreamer_args.rs`,
  `src-tauri/src/adapters/recording/portal_targets.rs`,
  `src-tauri/src/ports/screen_recorder.rs`,
  `src-tauri/src/application/recording_service.rs`,
  `src-tauri/src/lib.rs`, `src/features/recording/RecordingPickerWindow.tsx`,
  `src/features/settings/RecordingSettings.tsx`,
  `src/core/domain/recording.ts`, `src/core/l10n/*`.
- **Task:** [`27-task.md`](tasks/27-task.md)
- **Dano em dados existentes:** nenhum.
- **Limite:** não há follow automático da janela ativa no Wayland; a fonte fica
  fixa até o `stop`. Tela inteira inclui o overlay (BR-006).
