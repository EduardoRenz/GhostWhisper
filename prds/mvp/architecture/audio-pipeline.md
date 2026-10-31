# Arquitetura — Pipeline de Áudio

Detalha as portas e adaptadores de áudio/ASR do core Rust
(`ports/{audio_source,speech_to_text,voice_activity}` e `adapters/{audio,speech}`).

---

## 1. Fontes de áudio

| Fonte | Módulo | Crate | Rótulo |
|-------|--------|-------|--------|
| Microfone | `adapters/audio/cpal_microphone_source.rs` | `cpal` | `ME` |
| Áudio do sistema (monitor de sink) | `adapters/audio/pulse_system_audio_source.rs` | `libpulse-binding` (descoberta: `pactl`, `wpctl`/`pw-metadata`/`pw-cli`) | `THEY` |
| Sink virtual (fallback) | `adapters/audio/pactl_sink_helper.rs` | `pactl`/libpulse | `THEY` |

Cada fonte produz um stream contínuo de **PCM f32 mono 16 kHz**, que é o formato
aceito pelo whisper.cpp.

A descoberta do monitor **não depende de `pactl`** (ISS-087): quando o binário
não existe (PipeWire com `pipewire-pulse` sem `pulseaudio-utils`), o sink padrão
vem de `wpctl inspect @DEFAULT_AUDIO_SINK@` ou de `pw-metadata -n default`, e a
lista de monitores de `pw-cli ls Node` (fallback para o monitor padrão). A
conexão libpulse é aberta antes de `start` devolver `Ok`, então erro de fonte
chega ao chamador; o canal que falha mantém os demais e publica
`session_warning` para a UI.

## 2. Fluxo

```
mic (cpal)     ─┐
                ├─▶ resample ─▶ buffer de turno ─▶ ASR (whisper-rs + VAD Silero) ─▶ evento Tauri
system (pulse) ─┘         (gate RMS)             │
                                                 └─▶ nível de áudio (medidor UI)
```

### Etapas
1. **Captura:** `cpal` abre o device do mic; `libpulse-binding` abre o monitor do
   sink padrão (`@DEFAULT_MONITOR@`).
2. **Resample:** `rubato` converte de 44.1/48 kHz → 16 kHz e downmix para mono.
3. **Buffer:** ring buffer com backpressure; descarte controlado se o ASR atrasar
   (nunca crescer sem limite).
4. **Gate/segmentação:** um gate RMS barato marca fala/onsets; o **VAD Silero
   embutido no whisper.cpp** corta o silêncio e devolve os trechos de fala.
5. **ASR:** `whisper-rs` decodifica o **turno que cresce** em prévias frequentes
   (texto rolante) e um **passe final** ao fechar o turno; emite
   `transcript_segment { id, speaker, text, start_ms, end_ms, final }`.
6. **Eventos:** cada segmento é emitido para o frontend e persistido no SQLite.

## 3. Sink virtual (fallback)

Quando não há monitor de sink (ou o usuário prefere),
`adapters/audio/pactl_sink_helper.rs` cria um sink nulo:

```bash
pactl load-module module-null-sink sink_name=ghostwhisper session.sink_name=ghostwhisper
# usuário roteia a saída do app de reunião para "GhostWhisper"
```

- Remoção no encerramento: `pactl unload-module <id>`.
- Não empacotamos nenhum driver de kernel (BR-008).
- O sink virtual **ainda depende de `pactl`**; sem ele, o comando devolve erro
  explícito (o caminho normal de captura usa o monitor do sink padrão).

## 4. VAD + ASR

- **VAD:** o **Silero embutido no whisper.cpp** (`enable_vad` +
  `WhisperVadParams`: `threshold 0.5`, `min-speech 250 ms`, `min-silence 300 ms`,
  `speech-pad 200 ms`), com o modelo `ggml-silero-v5.1.2.bin` (MIT) resolvido pelo
  `ModelStore` no `preload()`. Igual ao Perssua (`WhisperVad.js`). O pipeline não
  roda mais um VAD separado sobre o buffer inteiro — usa um **gate de amplitude
  (RMS)** barato para timing/onsets.
- **whisper.cpp:** modelos `tiny`/`base`/`small`/`medium` baixados sob demanda e
  cacheados no diretório de dados do app; **um único contexto** é compartilhado
  entre os canais (a decodificação é serializada por uma fila global).
- **Decode:** `temperature 0`, `temperature_inc 0`, `entropy_thold 2.2`,
  `suppress_nst`; prévia **greedy** (`best_of 1`) e turno final **beam 5**.
- **Streaming (Perssua):** buffer de turno (teto 60 s) decodificado em **prévias
  frequentes** (áudio mínimo 0,35 s, passo 0,75 s, 1ª em 1,2 s, ociosidade 1 s)
  que publicam **texto rolante**; o turno fecha após **1 s de silêncio** (ou teto
  de 55 s) com o passe final. Ver `application/transcription_pipeline.rs`.

## 5. Atribuição de falante

Como cada fonte é capturada separadamente, `ME` = mic e `THEY` = sistema. Isso é
estrutural (não diarização por ML) — mais simples e confiável para o MVP.

## 6. Parâmetros iniciais

| Parâmetro | Valor |
|-----------|-------|
| Taxa de trabalho | 16 000 Hz |
| Canais | 1 (mono) |
| Formato | f32 |
| Janela ASR | ~5 s (ajustável) |
| Overlap | ~0.5 s |
| Buffer máx. | ~30 s |

## 7. Falhas e degradação

- Sem monitor de sink → oferecer sink virtual ou modo só-microfone.
- Sem dispositivo de entrada → erro claro na UI.
- CPU insuficiente → sugerir modelo menor (`tiny`) ou provider cloud.
- Compositor/áudio indisponível → app continua com o que estiver funcional.

## 8. Arquivos envolvidos

- Novos (adapters): `adapters/audio/{cpal_microphone_source,pulse_system_audio_source,pactl_sink_helper,pcm_resampler}.rs`,
  `adapters/speech/{whisper_speech_to_text,whisper_model_store,whisper_speech_engine_factory,serialising_speech_to_text}.rs`.
- Novos (ports): `ports/{audio_source,speech_to_text,speech_engine_factory}.rs`.
- Novos (application): `application/transcription_service.rs`.
- Modificados: `state.rs`, `ipc/{audio_commands,session_commands}.rs`, `lib.rs`.
