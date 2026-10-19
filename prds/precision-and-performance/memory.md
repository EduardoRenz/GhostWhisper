# Memory — PRD Precision and Performance

Nota histórica **deste PRD**: decisões tomadas, medições e o que foi feito, em
ordem cronológica (R15, BR-017). Entradas `## YYYY-MM-DD — Título curto`, com
`IDs:`/`Refs:` logo abaixo; a mais recente no fim. Busca é via `rg` (R14).

| Objetivo | Comando |
|----------|---------|
| Palavra-chave | `rg -n -i 'aec' prds/precision-and-performance/memory.md` |
| Entrada do dia | `rg -n '^## 2026-10-18' prds/precision-and-performance/memory.md` |
| Período | `rg -n '^## 2026-10-(1[0-9]\|2[0-9])' prds/precision-and-performance/memory.md` |
| Entrada inteira | `awk '/^## 2026-10-18 —/,/^## /' prds/precision-and-performance/memory.md` |
| Por ID | `rg -n 'ISS-023\|BR-027' prds/precision-and-performance/memory.md` |

---

## 2026-10-18 — PRD de precisão e desempenho aberta

- **IDs:** ISS-023, ISS-024, ISS-025, ISS-026, ISS-027, ISS-028
- **Refs:** BR-015, BR-018, prds/mvp/memory.md (2026-10-18 — Streaming em tempo
  real), prds/assistant/prd.md

### Contexto
Usuário reportou que o Perssua gera bolhas menores (~3 linhas) e é "um pouco mais
rápido e mais acertivo na transcrição". REA no bundle do Perssua 0.27.5
(`/tmp/opencode/perssua/extracted`) apontou causas concretas; teoria e evidências
em [`architecture/precision-and-performance.md`](architecture/precision-and-performance.md).

### Decisões
- Quebrar em **3 entregáveis** testáveis: E1 sinal/decodificação, E2 percepção
  (bolha + fast-first), E3 áudio/engine (GPU, AEC, reuso do final).
- Cada entregável é aprovado em teste manual **antes** do próximo.
- `ISS-022` fica reservada ao PRD `assistant`; numeração desta PRD começa em
  `ISS-023`; regras novas serão `BR-027`…`BR-029`.

### Pendências
- Aprovar a PRD e iniciar a task 01 (E1).

---

## 2026-10-18 — E1: prompt de decodificação, anti-alias e detecção de GPU

- **IDs:** ISS-024, ISS-025, BR-027
- **Refs:** BR-015, BR-018, [`tasks/01-task.md`](tasks/01-task.md),
  [`architecture/precision-and-performance.md`](architecture/precision-and-performance.md)

### Contexto
Entregável E1 do PRD: reduzir erro/alucinação **sem GPU**, transpondo o que o
Perssua faz na decodificação e no sinal.

### Implementado
- `domain::transcription`: `decode_prompt(base, language)` (Perssua `pt()`) e
  `map_whisper_language` (`pt-br` → `pt`); NUL removido (o `whisper-rs` entra em
  pânico em `set_initial_prompt`).
- `Settings.whisper_prompt` (default do Perssua, teto 1 000, NUL rejeitado no
  `validate`/removido no `sanitized`); adapter chama `set_initial_prompt` quando
  não-vazio; modelo/idioma **não** recarregam quando só o prompt muda
  (`update_prompt`).
- Catálogo: `large-v3` e `large-v3-turbo` (default segue `tiny`); UI lista os dois
  com tamanho e aviso.
- `pcm_resampler`: FIR passa-baixa (windowed-sinc, 63 taps) + reamostrador **com
  estado** (histórico e fase entre blocos) na captura do microfone; aliasing
  acima da Nyquist de 16 kHz.
- `ports::gpu_probe` + `adapters::system::vulkan_gpu_probe`: detecta GPU por ICDs
  Vulkan/`/dev/dri`, ignora rasterizers de software (lavapipe/llvmpipe/zink),
  sugere o modelo; `settings_gpu_info` expõe à UI. Features `cuda`/`hipblas`/
  `metal` declaradas (aceleração real é o E3).
- Decisão mantida: gate/VAD **não** adotam os 0,025 do Perssua (BR-018).

### Verificação
- `cargo test --lib` 300 ✓ (8 `ignored`) · `clippy -D warnings` ✓ · `fmt --check`
  ✓ · `pnpm test` 119 ✓ · `tsc --noEmit` ✓.
- Aprovado pelo usuário em sessão real pt-BR (aviso de GPU `vulkan` presente,
  prompt editável, modelos grandes listados, sem regressão de latência). Task 01
  `CONCLUDED`; E2 iniciado.

### Descoberta de ambiente
- Terminal do VS Code via **snap** exporta `GTK_PATH`/`GIO_MODULE_DIR`/`LOCPATH`/
  `XDG_DATA_DIRS` do sandbox e o binário quebra ao carregar o `libpthread` do
  `core20` (`__libc_pthread_init, version GLIBC_PRIVATE`). Solução:
  `pnpm tauri:dev:clean` (`scripts/tauri-dev-clean.sh`) ou terminal nativo;
  documentado no `README.md`.

---

## 2026-10-18 — E2: fast-first, streaming e automático por fala nova

- **IDs:** ISS-023, BR-028
- **Refs:** BR-023, BR-026 (revogada em parte), BR-015,
  [`tasks/02-task.md`](tasks/02-task.md),
  [`architecture/precision-and-performance.md`](architecture/precision-and-performance.md)

### Contexto
Entregável E2: fazer a sugestão aparecer **rápida e curta** (~3 linhas) e
funcionar sem a janela aberta — o que o usuário percebe como velocidade.

### Implementado
- **Contrato fast-first** (`prompts::assistant::fast_first_system_prompt`): só a
  próxima fala, ≤3 linhas, sem seções, temperatura 0, teto de 420 tokens
  (Perssua `fastFirstInsight*`).
- **Streaming**: `LlmController::complete_streaming` / `ChatBotProvider::complete_streaming`
  (padrão sem streaming) e SSE real no `openai_compatible`
  (`chat_completion_streaming`, acumulado, com fallback para o passe único quando
  o provider não devolve `text/event-stream`). Gemini/custom caem no fallback.
- **Evento `suggestion`** (`EventPublisher::emit_suggestion`) leva o rascunho e
  cada delta acumulado; a janela substitui a bolha.
- **Contexto ao vivo**: `context::recent_context_live` inclui a fala em andamento
  (o final sempre vence a prévia do mesmo turno).
- **Automático no core**: `application::suggestion_scheduler` (decisão pura) +
  `spawn_suggestion_scheduler` no composition root; `TranscriptionsRepository::
  latest_final_ms` dá o cursor barato. Debounce 700 ms, mínimo 3 s entre chamadas,
  respeita `Settings.suggestion_auto_refresh` — **sem depender da janela aberta**.
- **UI**: a janela assina o evento (rascunho + deltas), o toggle Auto persiste em
  `suggestion_auto_refresh` e a bolha da IA no chat começa colapsada em ~3 linhas
  com "mostrar mais".

### Verificação
- `cargo test --lib` 315 ✓ (8 `ignored`) · `clippy -D warnings` ✓ · `fmt --check`
  ✓ · `pnpm test` 121 ✓ · `tsc --noEmit` ✓ · `pnpm build` ✓.
- Manual: o usuário validou o comportamento geral e seguiu para o E3. Ajustes de
  UX de **tradução** que apareceram no teste foram para outra PRD
  ([`../translation/prd.md`](../translation/prd.md), BR-030). Task 02 `CONCLUDED`.

---

## 2026-10-18 — E3 (parcial): reuso do passe final e plumbing de GPU

- **IDs:** ISS-027, ISS-028, BR-029
- **Refs:** BR-015,
  [`tasks/03-task.md`](tasks/03-task.md),
  [`architecture/precision-and-performance.md`](architecture/precision-and-performance.md) §4

### Contexto
Entregável E3 ("engine rápida"): GPU/flash attention, modelo por hardware, AEC e
menos trabalho por turno.

### Implementado
- **Reuso do passe final** (`transcription_pipeline`): quando o VAD nativo fecha o
  turno dentro da prévia (`speech_end_ms` + silêncio), o texto já decodificado é
  promovido a final (`begin_final_reusing`), sem o segundo beam search
  (Perssua: `endedInNoise`). Teste novo prova **1** decodificação no caso.
- **GPU/flash attention** (`whisper_speech_to_text`): `flash_attn(true)` só em
  builds com `cuda`/`hipblas` (nunca macOS/CPU); log do backend no carregamento;
  `GpuInfo.acceleration_enabled` reflete o build.
- **Modelo por hardware (sugestão)**: botão "Usar large-v3-turbo" na tela de
  transcrição quando há GPU + aceleração. Desvio consciente: é **sugestão com um
  clique**, não troca automática (evita surpreender e não temos GPU para validar).

### Verificação
- `cargo test --lib` 316 ✓ · `clippy -D warnings` ✓ · `fmt --check` ✓ ·
  `pnpm test` 122 ✓ · `tsc` ✓ · `pnpm build` ✓. Boot CPU segue idêntico.
- **Pendente na task 03:** AEC referenciado e ajuste de commits.

### GPU por Vulkan — build e medição (mesma data)

Hardware: **AMD Radeon RX 480 (RADV POLARIS10)**, i5-6400, sem ROCm/CUDA.

- `whisper-rs 0.16` tem feature **`vulkan`** (→ `whisper-rs-sys/vulkan`); não é
  preciso trocar de engine nem rodar processo externo.
- O build exige headers do Vulkan + `glslc`. Sem sudo, `scripts/vulkan-env.sh`
  baixa `libvulkan-dev`/`glslc`/`libshaderc1` e monta `.vulkan-sdk/`. Ajustes que
  o build precisou: copiar também `include/vk_video/`, apontar `libvulkan.so` para
  a lib real do sistema e `RUSTFLAGS=-L .vulkan-sdk/lib` (o `whisper-rs-sys` 0.15
  emite `-lvulkan` sem o `-L`).
- Comandos: `pnpm tauri:dev:vulkan` / `pnpm tauri:build:vulkan` (1º build ~8 min).
- **Verificado em execução:** `ggml_vulkan: 0 = AMD Radeon RX 480 ... (radv)` e
  `whisper_backend_init_gpu: using Vulkan0 backend`.

Benchmark (RTF, amostra JFK, dev `RelWithDebInfo`):

| modelo | 1 s | 8 s | 20 s | 30 s |
|---|---|---|---|---|
| `tiny` | 99 ms | 241 ms | 580 ms | 791 ms |
| `small` | 541 ms | 725 ms | 1 156 ms | 2 107 ms |
| `large-v3-turbo` | 2 432 ms | 2 573 ms | 2 973 ms | 4 511 ms |

Conclusão: **`small` na GPU** tem a latência que o `tiny` tinha na CPU (~1,5–2 s
por chamada) com precisão muito maior → passa a ser o modelo recomendado quando
há GPU. O `large-v3-turbo` tem piso de ~2,4 s por chamada nesta placa (sem
fp16/matrix cores) e fica como opção, não default.

### Um artefato só (decisão do usuário: embutir o loader)

- O backend é **de compilação**: um binário sem a feature nunca usa GPU. Por isso
  o **release padrão passou a ser Vulkan** (`pnpm tauri:build` → `--features
  vulkan`), com fallback automático para CPU (confirmado no `whisper.cpp`: sem
  dispositivo ele loga `no GPU found` e segue).
- O binário Vulkan **linka** `libvulkan.so.1` (o ggml não faz `dlopen`), então o
  AppImage **embute o loader** (`scripts/ensure-libvulkan.sh` → `resources/` →
  `usr/lib/libvulkan.so.1`); os drivers/ICDs continuam do sistema. `.deb` declara
  `libvulkan1`.
- Novo seletor **Aceleração** (`auto`/`cpu`/`gpu`) em `Settings`; `auto` = GPU se
  o build tiver backend. `cpu` para driver instável.
- **Marca de crash** (`whisper-gpu-active.json`, padrão Perssua): gravada no boot
  quando a GPU pode rodar, apagada no `RunEvent::Exit`; se sobrar, o próximo boot
  desliga a GPU. Também há queda para CPU se a **carga** da GPU falhar.
- Builds isolados: `CARGO_TARGET_DIR=src-tauri/target-vulkan` no ambiente Vulkan,
  para alternar CPU/GPU sem invalidar cache.

### Empacotamento verificado

`pnpm tauri:build` (release + Vulkan): AppImage **92 MB** com
`usr/lib/libvulkan.so.1` embutido, `.deb` **16 MB** com `libvulkan1` em Depends,
binário **55 MB** com `NEEDED libvulkan.so.1`. O `appimage.files` do Tauri usa
**destino → origem** (a primeira tentativa invertida falhou no bundler: "does not
exist"). O AppImage rodou com `VK_ICD_FILENAMES=/nonexistent` sem crash e logou
`acceleration_enabled=true recommended_model=small`. Fallback para CPU sem
dispositivo é garantido pelo `whisper_backend_init` (loga `no GPU found` e segue).

---

## 2026-10-18 — E3 (fim): AEC referenciado e commits avaliados

- **IDs:** ISS-026, BR-029
- **Refs:** BR-018, [`tasks/03-task.md`](tasks/03-task.md),
  [`architecture/precision-and-performance.md`](architecture/precision-and-performance.md) §4

### Implementado — AEC
- `domain::audio::EchoCanceller`: **NLMS** (passo 0,3) com piso de energia e
  **guarda de double-talk** (congela a adaptação quando a fala local domina a
  referência); referência muda ⇒ saída idêntica à entrada.
- `ports::audio_reference::AudioReference` + `SharedAudioReference` (anel de ~2 s)
  e `SystemAudioReferenceTap` (decorador que alimenta a referência e repassa o
  áudio do sistema ao pipeline de sempre).
- `MicrophoneFrontEnd`: AEC **antes** do ganho e do gate; `Settings.aec_enabled`
  (padrão ligado), atraso/taps por env (`GHOSTWHISPER_AEC_DELAY_MS` = 60,
  `GHOSTWHISPER_AEC_TAPS` = 512, ambos com clamp).
- Testes: eco atrasado cai **≥10 dB**; fala local preservada no double-talk;
  remoção comprovada através do estágio de entrada; anel/atraso com padding.

### Decisão — commits
- Mantido **1 s de silêncio**: é o `local_whisper_vad_silence_duration_ms` do
  Perssua. Os 650 ms valem para o provider realtime em nuvem; reduzir picaria
  pausa natural de frase. As bolhas grandes de **sugestão** já foram tratadas no
  E2. `GHOSTWHISPER_VAD_SILENCE_MS` continua como override.

### Verificação
- `cargo test --lib` **330 ✓** · `clippy -D warnings` ✓ · `fmt --check` ✓ ·
  `pnpm test` 123 ✓ · `tsc` ✓. E3 encerrado (task 03 `CONCLUDED`).
