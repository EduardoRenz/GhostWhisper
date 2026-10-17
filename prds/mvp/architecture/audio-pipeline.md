# Arquitetura — Pipeline de Áudio

Detalha as portas e adaptadores de áudio/ASR do core Rust
(`ports/{audio_source,speech_to_text,voice_activity}` e `adapters/{audio,speech}`).

---

## 1. Fontes de áudio

| Fonte | Módulo | Crate | Rótulo |
|-------|--------|-------|--------|
| Microfone | `adapters/audio/cpal_microphone_source.rs` | `cpal` | `ME` |
| Áudio do sistema (monitor de sink) | `adapters/audio/pulse_system_audio_source.rs` | `libpulse-binding` | `THEY` |
| Sink virtual (fallback) | `adapters/audio/pactl_sink_helper.rs` | `pactl`/libpulse | `THEY` |

Cada fonte produz um stream contínuo de **PCM f32 mono 16 kHz**, que é o formato
aceito pelo whisper.cpp.

## 2. Fluxo

```
mic (cpal)     ─┐
                ├─▶ resample (rubato) ─▶ ring buffer ─▶ VAD (Silero) ─▶ ASR (whisper-rs) ─▶ evento Tauri
system (pulse) ─┘                                   │
                                                    └─▶ nível de áudio (medidor UI)
```

### Etapas
1. **Captura:** `cpal` abre o device do mic; `libpulse-binding` abre o monitor do
   sink padrão (`@DEFAULT_MONITOR@`).
2. **Resample:** `rubato` converte de 44.1/48 kHz → 16 kHz e downmix para mono.
3. **Buffer:** ring buffer com backpressure; descarte controlado se o ASR atrasar
   (nunca crescer sem limite).
4. **VAD:** Silero decide início/fim de fala; só segmentos com fala vão ao ASR.
5. **ASR:** `whisper-rs` processa janelas com sobreposição; emite
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

## 4. VAD + ASR

- **Silero VAD:** modelo `ggml-silero-v5.1.2.bin` (MIT) embarcado em `assets/`,
  carregado por `whisper-rs` (feature VAD) ou por runtime dedicado.
- **whisper.cpp:** modelos `tiny`/`base`/`small`/`medium` baixados sob demanda e
  cacheados no diretório de dados do app.
- **Streaming:** janela deslizante com sobreposição; VAD delimita turnos; texto
  parcial (`final=false`) é refinado até `final=true`.

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
  `adapters/speech/{silero_voice_activity,whisper_speech_to_text,whisper_model_store}.rs`.
- Novos (ports): `ports/{audio_source,speech_to_text,voice_activity}.rs`.
- Novos (application): `application/transcription_service.rs`.
- Modificados: `state.rs`, `ipc/{audio_commands,session_commands}.rs`, `lib.rs`.
