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

---

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
