# 03 — Áudio limpo e engine rápida · Entregável E3

- **Status:** PENDING
- **Depende de:** 01 (catálogo de modelos), 02 (ciclo), 13/14 do MVP (streaming/áudio)
- **Issues:** ISS-026, ISS-027, ISS-028 · **BR nova:** BR-029
- **Refs:** BR-015, BR-018,
  [`../architecture/precision-and-performance.md`](../architecture/precision-and-performance.md) §4/§5

## Objetivo

Dar à transcrição a base que o Perssua tem e que hoje nos falta: **GPU/flash
attention**, modelo grande viável, **AEC** no microfone e menos trabalho por turno.

## Entregáveis

- `src-tauri/Cargo.toml` — features de GPU do `whisper-rs` (`vulkan`, com `cuda`
  alternativa) e documentação de build; sem quebrar o build CPU por padrão.
- `src-tauri/src/adapters/speech/whisper_speech_to_text.rs` — `WhisperContextParameters`
  com `use_gpu`/`flash_attn` quando compilado e disponível; detecção de
  dispositivo com **fallback CPU**; telemetria de backend/uso de GPU no status.
- `src-tauri/src/domain/settings.rs` + `SettingsPage.tsx` — política de modelo por
  hardware: `large-v3-turbo` com GPU, menor em CPU; recomendação visível e override
  manual.
- `src-tauri/src/ports/audio_echo_canceller.rs` (novo) — porta do AEC.
- `src-tauri/src/adapters/audio/audio_echo_canceller.rs` (novo) — AEC referenciado
  pelo áudio do sistema (atraso configurável, default 60 ms), integrado ao
  `MicrophoneFrontEnd` como decorador.
- `src-tauri/src/application/transcription_pipeline.rs` — promover o resultado da
  prévia a final quando o VAD nativo já fechou o turno (`speech_end_ms`), evitando
  o segundo beam search; manter o passe autoritativo quando a prévia não cobrir.
- `src-tauri/src/application/turn_scheduler.rs` — avaliar commit por silêncio
  (650–800 ms) e commit periódico, com guarda que impede picar frase no meio.
- `prds/business-rules.md` — **BR-029** (GPU/flash attention, modelo por hardware,
  AEC e política de commit).

## Critérios de aceite

- [ ] Com GPU disponível, o app carrega o modelo em GPU com flash attention e
      registra isso no status; sem GPU, cai em CPU sem erro.
- [ ] Default por hardware aplicado (`large-v3-turbo` com GPU; menor em CPU), com
      override manual persistido.
- [ ] RTF < 1 com `large-v3-turbo` na GPU (medido); **sem regressão** em CPU.
- [ ] AEC reduz o eco no fixture sintético (mic = voz + eco atrasado) sem cortar o
      começo da fala; atraso configurável e toggle.
- [ ] Fim de turno não paga passe final duplicado quando a prévia já cobriu o turno.
- [ ] Fechamento de turno não fica preso atrás de prévia em voo.
- [ ] Commits menores não picam frase no meio (testes de turno com e sem silêncio).
- [ ] Suíte completa verde.

## Como testar

Automatizado:

```bash
# curva de RTF por modelo (CPU e, se houver, GPU)
cargo test --release --lib -- --ignored --nocapture benchmark_
cargo test --lib && cargo clippy --all-targets -- -D warnings && cargo fmt --check
```

Manual:

1. Benchmark: rodar o mesmo áudio de ~30 s com `tiny` (CPU) e `large-v3-turbo`
   (GPU); comparar RTF e qualidade do texto.
2. AEC: em viva-voz, falar com o outro lado; conferir que a fala não é transcrita
   duas vezes (ME duplicado) e que o começo das palavras não some.
3. Sessão real de ~5 min: conferir bolhas curtas, sem "faltou o final" e sem o
   texto ficar preso no fim do turno.
4. Reverter para CPU (override manual) e confirmar que ainda funciona.

Ponto de parada: usuário aprova qualidade de áudio e desempenho; PRD encerrada ou
abre-se follow-up.

## Riscos / rollback

- Drivers Vulkan/CUDA instáveis → detecção + fallback CPU; nunca tornar GPU
  obrigatória; medir antes de virar default.
- Flash attention com resultado divergente → comparar texto com/sem; manter
  toggle.
- AEC comer fala → atraso configurável, teste sintético e toggle; manter o
  caminho sem AEC.
- Commit agressivo picar frase → limiar conservador no default e testes de turno.

## Verificação (preencher ao concluir)

- RTF por modelo/backend: …
- Redução de eco no fixture: …
- Passes finais evitados (contagem em log): …
- Observações do usuário: …
