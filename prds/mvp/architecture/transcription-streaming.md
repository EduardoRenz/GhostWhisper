# Tech Spec — Streaming de transcrição em tempo real

- **Versão:** 0.1
- **Data:** 2026-10-18
- **Status:** Ativo
- **Relacionados:** [`../prd.md`](../prd.md),
  [`../../business-rules.md`](../../business-rules.md) (BR-015),
  [`../../.agents/rules.md`](../../../.agents/rules.md) (§6, R11–R13),
  [`../issues.md`](../issues.md) (ISS-013, ISS-014),
  [`../tasks/13-task.md`](../tasks/13-task.md)

Especifica **como** o GhostWhisper mantém a transcrição acompanhando a fala.
O comportamento de produto vem do Perssua (BR-015); a medição abaixo é deste
projeto (o hardware não é o mesmo: lá é macOS/Metal, aqui Linux/CPU).

---

## 1. Referência de produto (Perssua 0.27.5)

Extração estática do `resources/app.asar` (AppImage x86_64). Arquivos citados,
não copiados (BR-008):

| Arquivo no asar | O que define |
|---|---|
| `electron/local-whisper/WhisperSessionStreaming.js` | `processStreamingAudio` (decodifica o turno que cresce), `applyPreview` (o texto da prévia **substitui** o anterior), `applyRecognition` (`endedInNoise`), `finishProcessing` (re-agenda com o buffer mais novo) |
| `electron/local-whisper/WhisperSession.js` | `appendAudio` (buffers + VAD de amplitude), `refineTranscriptWithFinalUtterance` (passe final), `runWhisperViaServer` (`prompt`, `best_of`/`beam_size`) |
| `electron/local-whisper/WhisperSessionCompletion.js` | `finalizeAndEmitComplete`, `prepareNextUtterance` |
| `electron/local-whisper/WhisperSessionSupport.js` | Constantes: janela `0.75 s`, mínimo `0.35 s`, idle force `1000 ms`, `VAD_SILENCE_DURATION_MS = 1000`, `VAD_MIN_SPEECH_DURATION_MS = 160`, `FINAL_UTTERANCE_MIN_SEC = 0.8`, `FINISHED_PRE_ROLL 0.25 s`, `INITIAL_PREVIEW_MS 1200`, `MAX_CONTEXT_MS 55000` |
| `electron/local-whisper/WhisperServerManager.js` | `whisper-server` persistente (`-nf -sns -nt`), `temperature 0`/`temperature_inc 0`, `entropy_thold 2.2`, `suppress_nst` |
| `electron/local-whisper/WhisperVad.js` | VAD Silero nativo (`threshold 0.5`, `min_speech 250`, `min_silence 300`, `speech_pad 200`) |
| `electron/local-whisper/runtimeConfig.js` | Limiares de VAD com override por ambiente |
| `build/static/js/*` (renderer do local-whisper) | Front-end de captura: AEC → ganho de mic (`local_whisper_user_mic_gain = 8`) → noise gate (`local_whisper_user_mic_noise_gate_rms_threshold = 0.025`, hangover `600 ms`) → PCM16; prompt padrão `whisper_prompt` |

Conclusão da comparação: **as constantes do GhostWhisper já estavam certas**; o
que faltava era o que as acompanha (decodificação fora do caminho de decisão,
front-end de áudio e um ASR que não se enrosca).

---

## 2. Medição (deste projeto)

`cargo test --release --lib -- --ignored --nocapture benchmark_rtf_by_audio_duration`
(modelo `tiny`, CPU, 4 threads, mínimo de 3 repetições):

| áudio | prévia | RTF | final | RTF |
|---|---|---|---|---|
| 1 s | 1 665 ms | 0,60 | 1 749 ms | 0,57 |
| 2 s | 1 862 ms | 1,07 | 1 748 ms | 1,14 |
| 4 s | 1 475 ms | 2,71 | 1 556 ms | 2,57 |
| 8 s | 1 759 ms | 4,55 | 1 972 ms | 4,06 |
| 11 s | 1 596 ms | 6,89 | 2 436 ms | 4,52 |
| 20 s | 1 970 ms | 10,15 | 2 400 ms | 8,33 |
| 30 s | **20 510 ms** | 1,46 | 18 206 ms | 1,65 |

`benchmark_vad_overhead` mostra que o VAD Silero **não** é o custo (≈ igual com e
sem VAD) e que a diferença 2 → 4 threads vale ~1,7× no custo por chamada.

**Leituras que viraram decisão:**

1. **O custo por chamada é ~constante (1,5–2 s) até ~20 s de áudio.** Portanto
   prévia em janela deslizante **não** ajudaria — decodificar o turno inteiro,
   como o Perssua, continua sendo o certo. *(Decisão da fase de planejamento,
   resolvida por medição.)*
2. **Há um penhasco de ~10× a partir de 30 s** (o encoder do whisper só enxerga
   30 s). O teto do turno (`GHOSTWHISPER_MAX_TURN_MS`) fica em **25 s**, abaixo do
   penhasco — desvio consciente do `MAX_CONTEXT_MS = 55 s` do Perssua.
3. **A prévia não pode rodar a cada 750 ms** num custo de ~1,7 s. O passo continua
   750 ms como *gatilho*, mas o agendamento nunca acumula prévias: uma por vez,
   a mais recente (`coalesce`), e o passe final tem prioridade.
4. **`cargo test`/`tauri dev` ficam em perfil `dev`**: com `whisper-rs-sys` sem
   otimização o custo por chamada chega a ~4,5 s. O build otimiza só esse pacote
   (`[profile.dev.package.whisper-rs-sys]`), então medir **sempre** em `--release`.

A medição é da máquina do desenvolvedor (máquina compartilhada, `core = 4`).
Reexecute o benchmark na máquina alvo antes de tirar conclusões.

---

## 3. Arquitetura

```
captura ──consume──► buffer de turno
                         │
      (a cada 50 ms)  tick() ──► TurnScheduler (decisão PURA)
                         │            │ Wait | Preview | Finalize
                         │            ▼
                         └──► dispatch ──► thread de decodificação ──► ASR
                                                  │
                                        handle_outcome ──► publica/persiste
```

- **[`application/turn_scheduler.rs`](../../../src-tauri/src/application/turn_scheduler.rs)** —
  decisão pura (`decide`), sem relógio, sem threads, sem áudio. É o único lugar
  com as regras de "quando pré-visualizar" e "quando fechar".
- **[`application/transcription_pipeline.rs`](../../../src-tauri/src/application/transcription_pipeline.rs)** —
  mantém o estado (buffer, instantes, contadores) e executa a decisão. A
  decodificação roda numa **thread de trabalho**: `tick()` apenas enfileira
  (`[tick_never_blocks_on_a_slow_decode]`).
- **[`adapters/speech/whisper_speech_to_text.rs`](../../../src-tauri/src/adapters/speech/whisper_speech_to_text.rs)** —
  uma thread dedicada possui o `WhisperState` e a fila; prévias são **coalescidas**
  (só a última sobrevive) e um `Final` **nunca** é descartado. Elimina a
  patologia do `try_lock` (lock preso após timeout → `AsrBusy` em cascata).

Regras transpostas (equivalência com o Perssua):

| Regra | Perssua | Aqui |
|---|---|---|
| Primeira prévia | `INITIAL_PREVIEW_MS 1200` | `initial_preview_ms` |
| Passo entre prévias | `SERVER_WINDOW_DURATION_SEC 0.75` | `preview_step_ms` |
| Força por ociosidade | `SERVER_IDLE_FORCE_PROCESS_MS 1000` | `idle_force_ms` |
| Fim do turno por silêncio | `VAD_SILENCE_DURATION_MS 1000` | `silence_ms` |
| Fim do turno por **frase concluída** | (não existe) | `silence_sentence_ms` (experimento, default = `silence_ms` — desligado) |
| Prévia durante a pausa | continua até o commit | `preview_cutoff_ms` (experimento, default = `silence_ms` — desligado) |
| Fim do turno pelo VAD do whisper | `endedInNoise` | `speech_end_ms` da prévia |
| Mínimo para virar segmento | `FINAL_UTTERANCE_MIN_SEC 0.8` | `min_segment_ms` |
| Turno em fala contínua | `MAX_CONTEXT_MS 55000` | `max_turn_ms` **25 000** (§2.2) |
| Fila de decodificação | `whisper-server` único | `SpeechGate` global: `Final` > `Preview` |
| Fechamento pendente | `pendingUtteranceComplete` | `final_requested` + resultado aplicado por turno |

### Invariantes

- **I1.** O resultado de um passe só é aplicado se pertence ao turno certo
  (prévia de turno fechado é descartada; `Final` vale enquanto for o pendente).
- **I2.** A cadência só avança com decodificação **bem-sucedida** (uma falha
  transitória não "queima" 750 ms).
- **I3.** Após uma falha, a próxima prévia espera o relógio de ociosidade (nunca
  re-tenta a cada tick).
- **I4.** `Final` é prioritário: no adaptador nunca é coalescido/descartado e, na
  fila global entre canais (`SpeechGate`), passa na frente de prévias que ainda
  **esperam** — sem preempção de um passe em execução (o whisper não interrompe).
- **I5.** `stop()`/`flush()` esperam (com teto) o passe final do turno aberto.
- **I6.** A decisão de fechar cedo por frase só vale quando a **mesma prévia** que
  ancorou a cadência cobre o fim da fala e termina em pontuação forte; a métrica
  `commit_ms` (espera + decodificação) é logada em `info`.

---

## 4. Ajuste sem recompilar

Env lidos em [`TurnTimings::from_env`](../../../src-tauri/src/application/turn_scheduler.rs)
e no ASR (`GHOSTWHISPER_ASR_THREADS`), espelhando o `runtimeConfig.js` do Perssua:

`GHOSTWHISPER_INITIAL_PREVIEW_MS`, `GHOSTWHISPER_PREVIEW_STEP_MS`,
`GHOSTWHISPER_IDLE_FORCE_MS`, `GHOSTWHISPER_VAD_SILENCE_MS`,
`GHOSTWHISPER_SILENCE_SENTENCE_MS`, `GHOSTWHISPER_PREVIEW_CUTOFF_MS`,
`GHOSTWHISPER_MIN_SEGMENT_MS`, `GHOSTWHISPER_MIN_WINDOW_MS`,
`GHOSTWHISPER_MAX_TURN_MS`, `GHOSTWHISPER_ASR_THREADS`,
`GHOSTWHISPER_WHISPER_FLASH_ATTN` (`0` desliga o flash attention mantendo a GPU;
A/B de qualidade no Vulkan) e `GHOSTWHISPER_WHISPER_FALLBACK` (`1` religa o
fallback por temperatura **no passe final**; default desligado, paridade `-nf`).

Telemetria: `RUST_LOG=info` (padrão) já traz RTF, áudio e parede por passe
(`log_rtf` em `transcription_pipeline.rs`), o **commit de fala**
(`commit_ms = wait_ms + queue_ms + asr_ms`: fim da fala → segmento final, com a
espera de parede, a fila do canal e a fila global + decode separadas) e a linha
`fila do ASR` (`gate_wait_ms`/`decode_ms`) por final. Os logs do whisper.cpp/ggml
são roteados para o `tracing` (`whisper-rs/tracing_backend` +
`install_logging_hooks`) e ficam em `whisper_rs=warn` por padrão — para depurar o
ASR, `RUST_LOG=info,whisper_rs=debug`.

---

## 5. Pendências (decididas, fora deste ciclo)

- **Fase 3 — qualidade:** `initial_prompt` (`whisper_prompt`), feature opcional
  `vulkan` do `whisper-rs` (o único caminho para prévias realmente rápidas em
  CPU lenta) e/ou `openblas`.
- **REA pendente:** AEC (`se.eo`) e o que é `whisper_speed = 2` nos defaults do
  Perssua.

---

## 6. Front-end de áudio do microfone (BR-018, task 14)

O gate de fala do pipeline é um limiar **absoluto** no sinal cru. Um microfone
que entrega RMS abaixo dele nunca dispara prévias e tem o turno rejeitado — o
app parece "surdo". O Perssua resolve com um estágio de entrada antes do ASR.

### Extração (REA, `build/static/js/main.f180d025.js`, módulo 7125)

| Peça | Regra do Perssua | Aqui |
|---|---|---|
| Ganho (`dy`) | `clamp(cfg local_whisper_user_mic_gain, padrão 8, mín 1, máx 12)`, só no mic do usuário | `clamp_mic_gain` (1×–12×, padrão 8×), só no mic |
| Gate (`Kv`) | `rms >= threshold` ⇒ passa; senão passa em *hangover* (`Δ ≤ hangover_ms`); senão **mudo** (`muteInsteadOfDrop`) | `domain::audio::NoiseGate` — idêntico, exceto o descarte |
| Defaults do local-whisper | `threshold 0.025`, `hangover 600 ms`, `muteSilenceDurationMs 1200` | `threshold = SPEECH_RMS_THRESHOLD`, `hangover 600 ms` |
| Ordem | ganho → gate → ASR (o limiar do gate é pós-ganho) | igual (`MicrophoneFrontEnd`) |
| Escopo | só `sourceType === "device"` | só o canal de microfone |

### Desvios deliberados (documentados)

1. **Nunca descartar, só silenciar.** O Perssua, depois de
   `hangover + muteSilenceDurationMs`, para de transmitir os blocos. O pipeline
   mede o silêncio pelo **comprimento do buffer** (`buffered_ms -
   last_voiced_end_ms`); descartar amostras impediria o turno de fechar por
   silêncio. Zerar preserva a linha do tempo e tem o mesmo efeito no ASR.
2. **Limiar do gate = limiar de fala do app**, não 0,025. O valor do Perssua é
   mais restritivo que `SPEECH_RMS_THRESHOLD` (0,008) e silenciaria fala que hoje
   é transcrita — exatamente a sensibilidade que a task 14 foi corrigir. O valor
   do Perssua continua acessível por ambiente
   (`GHOSTWHISPER_MIC_NOISE_GATE_RMS=0.025`); `0` desliga.

### Ajuste sem recompilar

`GHOSTWHISPER_MIC_NOISE_GATE_RMS` (0 desliga), `GHOSTWHISPER_MIC_NOISE_GATE_HANGOVER_MS`;
a sensibilidade fica na UI (persistida em `settings.mic_gain`).

### Medição sugerida no alvo

Abrir **Configurações → Transcrição & Áudio → Teste de áudio**, falar e observar
o RMS exibido (já pós-ganho e pós-gate): o valor deve ficar acima de
`SPEECH_RMS_THRESHOLD` (0,008) durante a fala e em 0 no silêncio.

