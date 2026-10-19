# 03 — Áudio limpo e engine rápida · Entregável E3

- **Status:** CONCLUDED
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

- [x] Build declara features de GPU e liga `flash_attn` só com GPU (nunca
      macOS/CPU); o backend é registrado no log de carga.
      **Verificado:** `AMD Radeon RX 480 (RADV POLARIS10)` → `using Vulkan0 backend`.
- [x] Modelo por hardware: `small` é **sugerido** com um clique quando há GPU +
      aceleração (`large-v3-turbo` fica como opção). **Desvio consciente:** não
      trocamos automaticamente (override manual).
- [x] RTF medido na GPU (Vulkan): `small` 8 s → 725 ms; `tiny` 8 s → 241 ms;
      `large-v3-turbo` tem piso de ~2,4 s (não é o default). CPU sem regressão.
- [x] **Um artefato só**: release padrão (`pnpm tauri:build`) com Vulkan + loader
      `libvulkan.so.1` embutido no AppImage (`libvulkan1` no `.deb`); escolhe GPU
      em runtime e cai para CPU sem dispositivo.
- [x] Seletor **Aceleração** (`auto`/`cpu`/`gpu`) nas configurações; fallback para
      CPU quando a carga da GPU falha.
- [x] **Marca de crash** (padrão Perssua): morrer com GPU ativa desliga a GPU no
      próximo boot; a marca é apagada na saída limpa.
- [x] AEC referenciado no áudio do sistema: NLMS (512 taps) com guarda de
      *double-talk*, rodando **antes** do ganho; referência muda ⇒ saída intacta.
      Testes: eco atrasado cai ≥10 dB, fala local preservada no double-talk e
      remoção comprovada **através do estágio de entrada**. Toggle em
      `Settings.aec_enabled` (padrão ligado); atraso/taps por env.
- [x] Fim de turno **não** paga passe final duplicado quando o VAD já fechou o
      turno na prévia (`endedInNoise`); teste prova 1 decodificação no caso.
- [x] Fechamento de turno não fica preso atrás de prévia em voo (T13/ISS-013).
- [x] **Commits: avaliado e mantido em 1 s de silêncio.** É o
      `local_whisper_vad_silence_duration_ms` do Perssua; os 650 ms que motivaram o
      item são do **provider realtime em nuvem**, não do whisper local. Reduzir
      cortaria pausa natural de frase (piora a transcrição) e as bolhas grandes de
      **sugestão** já foram resolvidas no E2 (fast-first + clamp). Override por
      `GHOSTWHISPER_VAD_SILENCE_MS` para quem quiser arriscar.
- [x] Suíte completa verde: `cargo test` 330 ✓ · `clippy -D warnings` ✓ ·
      `fmt --check` ✓ · `pnpm test` 123 ✓ · `tsc` ✓ · `pnpm build` ✓.

## Progresso

Feito: **reuso do passe final**, **GPU por Vulkan** (build + verificação real na
RX 480) e **modelo por hardware** (`small` recomendado). Restam: **AEC
referenciado** e **ajuste de commits**.

### Empacotamento verificado

`pnpm tauri:build` gerou:

| artefato | tamanho |
|---|---|
| `GhostWhisper_0.1.0_amd64.AppImage` (Vulkan + loader embutido) | 92 MB |
| `GhostWhisper_0.1.0_amd64.deb` (Depends: `libvulkan1`) | 16 MB |
| binário `ghostwhisper` (release, NEEDED `libvulkan.so.1`) | 55 MB |

O AppImage foi extraído e contém `usr/lib/libvulkan.so.1`; rodou com
`VK_ICD_FILENAMES=/nonexistent` **sem crash** e logou
`acceleration_enabled=true recommended_model=small` (é a build Vulkan).

### GPU: como buildar/rodar

```bash
pnpm tauri:build           # release com Vulkan + loader embutido (padrão)
pnpm tauri:build:cpu       # release CPU-only (escape hatch)
pnpm tauri:dev:vulkan      # dev com GPU (1º build ~8 min: compila shaders)
```

`scripts/vulkan-env.sh` monta o SDK (usa `libvulkan-dev`/`glslc` do sistema ou
baixa os `.deb` sem sudo em `.vulkan-sdk/`); `scripts/ensure-libvulkan.sh` copia o
loader para embutir no AppImage. O build de GPU usa `src-tauri/target-vulkan/`,
isolado do build de CPU.

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
