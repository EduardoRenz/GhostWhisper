# Arquitetura — Precisão e Desempenho

Base de comportamento no **Perssua 0.27.5** (BR-015), extraído do `app.asar`.
Evidência no §5. Desvios conscientes no §4.

---

## 1. Componentes afetados

```
src-tauri/src/domain/transcription.rs                # prompt de decodificação (puro, E1)
src-tauri/src/application/prompts/assistant.rs       # fast-first (E2)
src-tauri/src/application/suggestion_service.rs      # fast-first + contexto com prévias (E2)
src-tauri/src/application/context.rs                 # prévias no contexto (E2)
src-tauri/src/application/transcription_pipeline.rs  # reuso do final / commits (E3)
src-tauri/src/adapters/speech/whisper_speech_to_text.rs  # initial_prompt, flash attn, GPU (E1/E3)
src-tauri/src/adapters/audio/pcm_resampler.rs        # anti-aliasing (E1)
src-tauri/src/adapters/audio/audio_echo_canceller.rs # NOVO: AEC referenciado (E3)
src-tauri/src/ports/gpu_probe.rs                     # NOVO: detecção de GPU (E1)
src-tauri/src/adapters/system/vulkan_gpu_probe.rs    # NOVO: Vulkan/ICD (E1)
src-tauri/src/ports/llm_controller.rs                # streaming (E2)
src-tauri/src/ports/audio_echo_canceller.rs          # NOVO (E3)
src-tauri/src/ipc/llm_commands.rs                    # eventos de delta (E2)
src/features/chat/ChatBubble.tsx                     # cue colapsável (E2)
src/features/suggestions/SuggestionsWindow.tsx       # fast-first + streaming (E2)
src/core/application/useSuggestions.ts               # deltas + auto contínuo (E2)
```

## 2. E1 — Sinal e decodificação

- **Prompt de decodificação (puro).** Em `domain::transcription`:
  `decode_prompt(base, language)` monta o `initial_prompt` combinando o texto
  configurável + regra de idioma, e `map_whisper_language` mapeia o código
  (`pt-br` → `pt`). Espelha o `pt()` do Perssua (§5). Fica no **domínio** porque
  o adaptador de ASR o consome (adapters não dependem de `application`).
- **Idioma.** `pt-br` → `pt` (mapa), e o nome legível entra no prompt.
- **Reamostragem.** O resampler de mic passa a filtrar antes de decimar
  (passa-baixa) para eliminar espelhamento de frequência; o áudio do sistema
  continua sendo convertido pelo PulseAudio (16 kHz mono).
- **Gate/VAD.** Limiares **permanecem os do BR-018** (gate no limiar de fala do
  app, não nos 0,025 do Perssua — decisão deliberada já registrada); cobertos por
  teste e o `initial_prompt` é medido separado.
- **Catálogo.** `large-v3` e `large-v3-turbo` entram como opção (download sob
  demanda), com aviso; default inalterado nesta fase.
- **Detecção de GPU (sugestão).** Uma porta `GpuProbe` detecta GPU (no Linux,
  Vulkan via ICDs em `/usr/share/vulkan/icd.d` + `/dev/dri`, ignorando rasterizers
  de software como lavapipe/llvmpipe). A UI **sugere** o modelo adequado; a
  aceleração real chega no E3 (`acceleration_enabled`). Nada é trocado sozinho.

## 3. E2 — Percepção (bolha e sugestão)

- **Fast-first.** Primeira sugestão curta (`max_tokens` 300–420, temperatura 0,
  prompt compacto "correct over complete"), publicada por evento; depois a
  sugestão completa. Espelha `fastFirstInsight*` (§5).
- **Streaming.** A porta de LLM ganha um caminho de streaming (SSE para
  OpenAI-compatível; equivalente no Gemini), com deltas por evento e fallback
  não-streaming.
- **Contexto com prévias.** O contexto da sugestão deixa de exigir `is_final`;
  falas em andamento entram marcadas (o modelo continua tratando como dado).
- **Auto contínuo.** Um agendador no core dispara sugestão a cada fala nova
  (debounce + supersede), sem depender da janela aberta; toggle nas configurações.
- **UI compacta.** O cue falado é colapsado (1–3 linhas) e seções longas são
  colapsáveis; o painel pode iniciar menor.

## 4. E3 — Áudio e engine

- **GPU + flash attention.** Features `vulkan`/`cuda` do `whisper-rs`, com
  detecção de dispositivo e fallback CPU; `flash_attn = true` quando suportado
  (Perssua: só `--no-flash-attn` em macOS). No Linux sem ROCm/CUDA, o caminho é
  **Vulkan** (`whisper-rs/vulkan`), viabilizado por `scripts/vulkan-env.sh`
  (headers + `glslc`, sem sudo).

### Medição (RX 480 / RADV POLARIS10, Vulkan, dev `RelWithDebInfo`, amostra JFK)

| modelo | 1 s | 8 s | 20 s | 30 s |
|---|---|---|---|---|
| `tiny` (Vulkan) | 99 ms | 241 ms | 580 ms | 791 ms |
| `small` (Vulkan) | 541 ms | 725 ms | 1 156 ms | 2 107 ms |
| `large-v3-turbo` (Vulkan) | 2 432 ms | 2 573 ms | 2 973 ms | 4 511 ms |
| `tiny` (CPU, referência antiga) | — | ~1,5–2 s | ~1,5–2 s | — |

Leitura: o **`small` na GPU** entrega a latência que o `tiny` tinha na CPU, com
muito mais precisão — é o default recomendado com GPU. O `large-v3-turbo` tem um
**piso de ~2,4 s por chamada** nesta placa (sem fp16/matrix cores), o que atrasa
prévias curtas; fica como opção para turnos longos ou GPUs mais fortes.
- **Modelo por hardware.** Default `large-v3-turbo` com GPU; CPU mantém o menor.
- **AEC referenciado.** O áudio do sistema vira referência para cancelar eco no
  microfone, como `explicit_aec_*` (§5): `SystemAudioReferenceTap` guarda uma
  janela de ~2 s (`SharedAudioReference`), o `MicrophoneFrontEnd` puxa a janela
  atrasada (padrão 60 ms) e um **NLMS** de 512 taps subtrai a estimativa **antes**
  do ganho, com guarda de *double-talk*. Referência muda ⇒ saída intacta; toggle
  em `Settings.aec_enabled`.
- **Reuso do passe final.** Se o VAD nativo do whisper já marcou o fim da fala
  (`speech_end_ms`), o resultado da prévia é promovido a final, sem segundo beam
  search (Perssua: `endedInNoise`).
- **Commits.** Avaliar `silence_ms` menor e commit periódico, com guarda que
  impede picar frase no meio.

### 4.1 Desvios conscientes (R12)

| Desvio | Por quê |
|---|---|
| Manter caminho CPU com modelo menor | Linux sem GPU garantida; `large-v3` em CPU é inviável |
| AEC implementado na porta de áudio (Rust) | Não temos `AudioWorklet`/WebAudio do Electron |
| Commits por turno, não por provider realtime | Nosso ASR é local (whisper.cpp), não `gpt-live-transcribe` |
| Streaming por SSE genérico | Providers BYO key (OpenAI-compatível/Gemini), sem SDK |

## 5. Evidência no bundle do Perssua

Extração: `Perssua-0.27.5-x86_64.AppImage` →
`/tmp/opencode/perssua/extracted`.

| Assunto | Arquivo | Âncora |
|---|---|---|
| Prompt de decodificação + idioma | `build/static/js/310.8def3e52.chunk.js` | `pt(...)` → `"Only transcribe what is clearly spoken. If uncertain, transcribe nothing."`, `The audio is spoken in … Do not translate it to English` |
| Default do prompt | `build/static/js/settingsOnboardingWindow.a901992b.js` | `whisper_prompt`, `local_whisper_user_mic_noise_gate_rms_threshold:"0.025"`, `local_whisper_vad_rms_threshold:"500"` |
| Modelos locais suportados | `electron/local-whisper/WhisperModelManager.js` | `large-v3`, `large-v3-turbo` ("recommended for non-English") |
| Default do modelo | `electron/local-whisper/LocalWhisperService.js` | `config.model || 'large-v3'` |
| GPU / flash attention | `electron/local-whisper/WhisperServerManager.js` | `--no-gpu`, `--no-flash-attn`, `--device` |
| VAD e prévias | `electron/local-whisper/WhisperVad.js`, `WhisperSessionStreaming.js` | `INITIAL_PREVIEW_MS = 1200`, `MAX_CONTEXT_MS = 55000` |
| Reuso do final no VAD | `electron/local-whisper/WhisperSessionStreaming.js` | `endedInNoise` → `finalizeAndEmitComplete(snapshot)` |
| AEC referenciado | `build/static/js/5154.21860ec3.chunk.js` | `explicit_aec_enabled`, `_reference_timeout_ms`, `_capture_delay_ms:60` |
| Fast-first (modelo/prompt/limites) | `electron/flavors.js` | `CODY_FAST_FIRST_*`, `maxOutputTokens: 240/420`, `reasoningEffort: "none"` |
| Config de commits e insights | `build/static/js/8183.892084d8.js` | `manualCommitSilenceMs:650`, `outputManualCommitSilenceMs:300`, `periodicCommitIntervalMs:2000`, `streamAutomaticInsights:true`, `questionsRequireFinalizedTranscript:false` |
| Notch compacto + cue em 1 linha | `electron/notch-bubble.html` | `DEFAULT_EXPANDED_WIDTH=436`, `DEFAULT_EXPANDED_HEIGHT=212`, `formatNotetakerSayNext` |
| Contrato de "replacement snapshot" | `electron/flavors.js` | `NOTETAKER_FLAVOR_ASSISTANT.realtimePrompt` |

Reproduzir: ler os arquivos já extraídos em `/tmp/opencode/perssua/extracted`
(R11–R13).
