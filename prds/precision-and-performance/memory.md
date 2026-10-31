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
  UX de **tradução** que apareceram no teste foram para outra PRD (removida em
  2026-10-24; BR-030). Task 02 `CONCLUDED`.

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

---

## 2026-10-20 — PRD fechada: status do cabeçalho e das issues reconciliados

- **IDs:** ISS-023, ISS-024, ISS-025, ISS-026, ISS-027, ISS-028
- **Refs:** [`prd.md`](prd.md), [`tasks/_tasks.md`](tasks/_tasks.md),
  [`issues.md`](issues.md), BR-027, BR-028, BR-029

### Contexto
As tasks 01, 02 e 03 (E1, E2, E3) já estavam `CONCLUDED` e o `memory.md` registrava
o fechamento do E3, mas dois documentos seguiam com estado antigo:
- `prd.md` ainda dizia **"Aprovada — E1 implementado (task 01 `IN PROGRESS`)"**;
- `issues.md` mantinha ISS-024…ISS-028 como `OPEN` (e ISS-023 como `FIXED`, mas
  ainda "a validar no teste manual da task 02").

### Decisão
- `prd.md` → **Status: Implementado (tasks 01–03 `CONCLUDED` — E1, E2 e E3
  entregues)**; versão `0.1` deixa de ser rascunho.
- `issues.md` → ISS-023…ISS-028 todos `FIXED` (as correções estão descritas em
  "Correção:" de cada issue e aplicadas nas tasks 01 e 03).
- Sem mudança de código: reconciliação de status (R14).

### Verificação
- `cargo test --lib` **330 ✓** · `clippy -D warnings` ✓ · `fmt --check` ✓ ·
  `pnpm test` 123 ✓ · `tsc` ✓ (última rodada registrada no E3).

## 2026-10-21 — L1: commit de fala mais rápido (fila + endpointing) (ISS-058, task 04)

- **IDs:** ISS-058, T04, BR-029, R16, R17
- **Refs:** [`issues.md`](issues.md), [`tasks/04-task.md`](tasks/04-task.md),
  [`../../mvp/architecture/transcription-streaming.md`](../../mvp/architecture/transcription-streaming.md) §3–§4,
  [`../../business-rules.md`](../../business-rules.md) BR-029

### Relato
*“Preciso de pedaços de frases concluídas o mais rápido possível para que o
sugestor e a sessão possam utilizar o mais rápido possível.”*

### Diagnóstico
- O sugestor só consome **finais** (`latest_final_end_ms`), então toda a latência
  do turno entra nele: silêncio + passe final.
- Três somas no caminho crítico: (1) o agendador continuava pedindo **prévia
  durante o silêncio** (passo de 0,75 s), e o final entrava atrás dela na fila;
  (2) a fila global do ASR era **FIFO entre canais** — o final do microfone
  esperava a prévia do sistema; (3) o fechamento exigia **1 s cheio**, mesmo com
  a prévia já mostrando frase concluída.
- Em fala contínua o teto de `max_turn_ms` (25 s) continua sendo o pior caso;
  isso é a **etapa 3** (commit por frase), fora desta task.
- Revisão de segunda opinião: manter 1→2→3; medir antes de chamar de "risco
  baixo"; **descartar prévia enfileirada ≠ cancelar a que está executando**; a
  disputa entre canais na fila global não estava no plano original.

### Decisão
- **Etapa 1** (esta): decisão pura nova em `turn_scheduler` — `preview_cutoff_ms`
  (250 ms) para a rolagem na pausa e `silence_sentence_ms` (700 ms) quando a
  prévia **cobre** o fim da fala e termina em `.?!`; vírgula/reticências mantêm
  1 s. Desvio consciente do Perssua (1 s fixo), documentado em BR-029.
- **`SpeechGate`** substitui o `Mutex<()>`: `Final` passa na frente de prévias que
  esperam, sem preempção do passe em execução (o whisper não interrompe).
- **Métrica `commit_ms`** = espera de silêncio + decodificação, logada em `info`;
  o acessor fica em `#[cfg(test)]` para os testes de latência.
- A prévia guarda `(turno, corte, texto)` e o fechamento curto exige o **mesmo
  corte** que ancorou a cadência — pontuação de texto antigo não decide.

### Verificação
- `cargo test --lib` **614 ✓** (10 ignorados) · clippy `-D warnings` ✓ ·
  rustfmt nos arquivos alterados ✓.
- Teste pesado com modelo real (`tiny`, JFK streaming):
  `end_to_end_streams_preview_before_final` ✓.
- **Pendente:** teste manual no app (task 04, seção "Como testar").
- Nota: `cargo fmt --check` global continua com diffs **pré-existentes** em 18
  arquivos do PRD `recording` (não tocados aqui).

### Próximos passos
- **Etapa 2:** commit otimista (publicar a prévia estável como final e corrigir
  com o beam em background) — exige `update_text` na porta de transcrições
  (`insert` hoje é `INSERT` puro) e merge por id na UI.
- **Etapa 3:** commit por frase (corta a unidade; única que ataca o teto de 25 s
  em fala contínua).

## 2026-10-21 — Métrica de commit confiável e ruído do whisper silenciado (ISS-058, task 04)

- **IDs:** ISS-058, T04, BR-029, R16, R17
- **Refs:** [`tasks/04-task.md`](tasks/04-task.md),
  [`../../mvp/architecture/transcription-streaming.md`](../../mvp/architecture/transcription-streaming.md) §4

### Relato
Log real de sessão (21/10): `commit de fala channel=System turn=0 wait_ms=1792
asr_ms=1616 commit_ms=3408` com `audio_ms=21888`, e uma prévia do microfone de
23,5 s (1,8 s de decode). Além disso, o stderr estava inundado de
`whisper_full_with_state: beam search...`.

### Diagnóstico
- **Turnos longos** (21–23 s) dominam: o passe final custa 1,6 s e a prévia
  custa 1,8 s — é o caso que só a etapa 3 (commit por frase) resolve.
- `wait_ms=1792` com `silence_ms=1000`: a espera era calculada por **offset de
  áudio**; um tick atrasado (ou bloco grande) fazia o offset "pular" e inflar o
  número. Não dava para separar espera real de atraso do agendador.
- `asr_ms` misturava **fila global** (o final pode esperar uma prévia em
  execução — sem preempção) com decode; não dava para provar a prioridade do
  `SpeechGate`.
- O flood do whisper.cpp vinha do logger padrão (stderr), fora do `tracing`.

### Decisão
- `wait_ms` por **relógio de parede** desde a última janela falada, com piso no
  silêncio contido no áudio (determinístico nos testes).
- `commit_ms = wait_ms + queue_ms + asr_ms`; `queue_ms` é a fila do **canal** e
  `asr_ms` a chamada ao ASR (fila global + decode). Nova linha `fila do ASR`
  (`gate_wait_ms`/`decode_ms`) por final separa os dois.
- `whisper-rs/tracing_backend` + `install_logging_hooks()`; filtro padrão
  `RUST_LOG` + `whisper_rs=warn` (mesmo com `RUST_LOG=info`), com escape em
  `whisper_rs=debug`.

### Verificação
- `cargo test --lib` **614 ✓** · clippy `-D warnings` ✓ · rustfmt nos arquivos
  alterados ✓.
- Hook do whisper verificado em execução: com `RUST_LOG=info` os eventos chegam
  pelo alvo `whisper_rs::whisper_logging_hook`; com o filtro padrão, **zero**
  linhas no buffer do subscriber.
- **Pendente:** nova rodada de log no app (esperado: `wait_ms` ≈ 700/1000,
  `queue_ms` ≈ 0, `asr_ms` = fila + decode, sem ruído do whisper).

## 2026-10-21 — Regressão: fechamento por pontuação piorou a transcrição (ISS-058, task 04)

- **IDs:** ISS-058, T04, BR-029, R16, R17
- **Refs:** [`tasks/04-task.md`](tasks/04-task.md),
  [`../../business-rules.md`](../../business-rules.md) BR-029

### Relato
*“Senti uma piora, transcrição erradas, mais demoras.”* — depois de testar a L1
com o fechamento antecipado em 0,7 s ligado por padrão.

### Diagnóstico
- O whisper marca **ponto em pausa interna** com frequência; a regra "última
  prévia cobre o fim + termina em `.?!`" disparava em pausas que não eram fim de
  frase.
- Cada fechamento antecipado cria **mais um turno**: o whisper decodifica cada
  turno isolado (sem contexto do anterior) → mais erros de fronteira e mais
  decodificações (com dois canais, mais fila e mais demora percebida).
- O efeito composto com a prioridade do final era o oposto do desejado: mais
  finais ⇒ prévias esperando ⇒ texto rolante mais lento.

### Decisão
- `silence_sentence_ms` volta a **1000** por padrão (= `silence_ms`), ou seja, o
  fechamento antecipado fica **desligado**. Continua disponível por env
  (`GHOSTWHISPER_SILENCE_SENTENCE_MS=700`) para experimentos com medição.
- Mantidos: corte de prévias na pausa (`preview_cutoff_ms=250` — não muda texto),
  prioridade `Final` > `Preview` (`SpeechGate`) e a métrica decomposta.
- Testes anti-regressão (R16) que **falhariam** no código anterior:
  `early_close_is_off_by_default` (decisão pura) e
  `default_does_not_close_on_punctuation_alone` (pipeline).

### Verificação
- `cargo test --lib` **616 ✓** · clippy `-D warnings` ✓ · rustfmt nos arquivos
  alterados ✓.
- **Pendente:** novo teste manual do usuário (default 1 s) — se a piora persistir,
  o próximo suspeito é o `preview_cutoff_ms`, desligável com
  `GHOSTWHISPER_PREVIEW_CUTOFF_MS=60000`.

## 2026-10-21 — Rollback da L1 para o baseline (ISS-058, task 04)

- **IDs:** ISS-058, T04, BR-029, R16, R17
- **Refs:** [`tasks/04-task.md`](tasks/04-task.md), BR-029

### Relato
*“Aliás, gerando bem mais frases sem sentido agora também.”* — depois do rollback
do fechamento em 0,7 s, a percepção de piora continuou.

### Decisão
- **Voltar o comportamento ao baseline** e manter só o que é observável:
  - `silence_sentence_ms = 1000` (fechamento por pontuação **desligado**);
  - `preview_cutoff_ms = 1000` (prévia continua rolando na pausa, como antes);
  - mantidos: `SpeechGate` com prioridade `Final` > `Preview`, métrica
    decomposta e o silenciamento do whisper.cpp.
- Assim, a única diferença de texto/segmentação em relação ao commit anterior é
  **nenhuma**; o que resta é ordenação de fila (neutra para o texto) e
  telemetria.
- Experimentos continuam disponíveis por env:
  `GHOSTWHISPER_SILENCE_SENTENCE_MS=700` e `GHOSTWHISPER_PREVIEW_CUTOFF_MS=250`.

### Verificação
- `cargo test --lib` **617 ✓** · clippy `-D warnings` ✓ · rustfmt ✓.
- Testes de baseline (R16): `early_close_is_off_by_default`,
  `previews_continue_during_pause_by_default`,
  `default_does_not_close_on_punctuation_alone`.
- **Pendência aberta:** investigar a origem das “frases sem sentido” no canal
  System. O log de 19:49 mostra **turno de 25 s** (teto) com áudio contínuo e
  `flash_attn=true` num build Vulkan — contrário à BR-029, que prevê flash
  attention só em CUDA/hipBLAS. Hipóteses a testar: (a) Aceleração = CPU nas
  configurações; (b) `GHOSTWHISPER_MAX_TURN_MS=10000`; (c) desligar flash
  attention no Vulkan.

## 2026-10-21 — Diagnóstico das "frases sem sentido" (ISS-058, task 04)

- **IDs:** ISS-058, T04, BR-029, R16, R17
- **Refs:** [`tasks/04-task.md`](tasks/04-task.md), BR-029

### Relato
*“Acho que ambos, mas os sem sentido, os últimos foram minhas”* — as frases sem
sentido aparecem no **microfone** (fala do próprio usuário), não só no System.
*“O AEC eu testando microfone uma vez estava mais atrapalhando… sem isso ela fica
limpa”* — AEC fica **desligado por escolha** do usuário (o NLMS abafa a voz).

### Achados nas configurações reais
- `whisper_model: small`, `acceleration: auto` (GPU), `aec_enabled: false`.
- `whisper_prompt` customizado e **instructional**:
  *"Only transcribe what is clearly spoken… Probabilly you are in an techincal
  context, some tech words will appear, like Python, Javascript, Code etc"*.
  O `initial_prompt` do whisper é **contexto**, não instrução; com áudio
  incerto o modelo tende a continuar o prompt — candidato forte às frases
  plausíveis e sem sentido.
- O `decode_prompt` ainda anexa *"The audio is spoken in Portuguese…"*; com
  `source_language=pt` o `set_language` já força o idioma, então a regra é
  redundante e aumenta a superfície de vazamento.

### Decisão
- **Não** mexer no AEC (preferência do usuário; voz limpa sem ele).
- Diagnóstico em duas frentes, sem mudar defaults:
  1. teste limpando o `whisper_prompt` (só configuração);
  2. novo switch `GHOSTWHISPER_WHISPER_FLASH_ATTN=0` para A/B do flash attention
     **mantendo a GPU** (o log mostra `flash_attn=true` num build Vulkan; a
     BR-029 prevê flash attention só em CUDA/hipBLAS).

### Verificação
- `cargo test --lib` **618 ✓** · clippy `-D warnings` ✓ · `parse_switch` testado.
- Switch extra: `GHOSTWHISPER_WHISPER_FALLBACK=1` (fallback de temperatura só no
  final; default desligado, paridade `-nf`).
- **Pendente:** resultado do A/B (prompt vazio e/ou flash attention off).

## 2026-10-21 — Prompt instrucional vazava para a transcrição (ISS-059, task 05)

- **IDs:** ISS-059, T05, BR-027, BR-034, R16, R17
- **Refs:** [`issues.md`](issues.md), [`tasks/05-task.md`](tasks/05-task.md),
  [`../../business-rules.md`](../../business-rules.md) BR-027/BR-034

### Relato
*“Frases sem sentido… inclusive repetida… na prévia e, às vezes, no final”* no
**microfone**. Depois: *“removi o prompt do whisper… melhorou bastante, prévia
sem erro praticamente, bolhas sensatas.”*

### Diagnóstico
- O `whisper_prompt` salvo era **instrucional** (“Only transcribe what is clearly
  spoken…”) e o app ainda anexava a regra de idioma em inglês.
- O `initial_prompt` do whisper é **contexto** (fala anterior), não comando: em
  áudio incerto o modelo **continua o prompt**. A frase “Se você não sabe, você
  pode falar em português” vista no log é paráfrase da regra de idioma.
- Prova prática: limpar o campo corrigiu na hora; as prévias caíram de
  ~700–1800 ms para ~250–350 ms (menos tokens de prompt no decoder).

### Correção
- `DEFAULT_WHISPER_PROMPT` **vazio** (backend e frontend) + migração em
  `Settings::sanitized` que limpa exatamente o default antigo (contexto real é
  preservado).
- `decode_prompt(base)` virou só sanitização (trim + NUL); a regra de idioma
  saiu — `set_language` + `set_translate(false)` já fixam idioma e não-tradução.
- `update_language` não recomputa mais prompt (ele não depende do idioma).
- UI: “Contexto da transcrição (opcional)”, placeholder de nomes/jargões e aviso
  de que **instruções atrapalham** e podem ser repetidas; recomendação de deixar
  vazio.
- BR-027 reescrita; CA-01.1/CA-01.2 do PRD atualizados; BR-034 ajustada.

### Verificação
- `cargo test --lib` **619 ✓** · clippy `-D warnings` ✓ · `pnpm test` **388 ✓** ·
  `tsc --noEmit` ✓.
- Testes anti-regressão (R16): `sanitized_clears_the_legacy_instruction_prompt`,
  `whisper_prompt_defaults_to_empty`,
  `decode_prompt_keeps_only_context_and_drops_instructions`,
  `engine_params_reads_settings`.
- **Pendente:** abrir Configurações no app e conferir o novo texto (task 05).

## 2026-10-21 — Etapa 3: commit por frase (ISS-058, task 06)

- **IDs:** ISS-058, T06, BR-029, R16, R17
- **Refs:** [`tasks/06-task.md`](tasks/06-task.md),
  [`../../mvp/architecture/transcription-streaming.md`](../../mvp/architecture/transcription-streaming.md) §3–§4

### Objetivo
Quebrar o monólogo (fala contínua sem 1 s de silêncio) em blocos de ~2–5 s, sem
picar frase: o turno fecha só o prefixo que termina numa **frase completa
estável entre duas prévias**, e o resto do áudio continua no buffer.

### Decisão
- **Opt-in** (`GHOSTWHISPER_SENTENCE_COMMIT=1`), default desligado: a etapa 1
  mostrou que mudar segmentação sem A/B piora a percepção de qualidade.
- `complete_sentences(spans)` no domínio (spans com pontuação forte, fim no
  áudio + texto normalizado); o pipeline escolhe a **primeira** que passa do
  mínimo (blocos curtos) e o `note_sentence` exige a **mesma frase em duas
  prévias seguidas**.
- No scheduler, o commit devolve `Finalize { cut_ms = fim + pad }` **somente** se
  `cut_ms ≥ sentence_min_ms` (1,2 s) e `cut_ms < buffered_ms` (tem resto). O
  prefixo é decodificado com beam 5; o restante fica no buffer (o
  `finalize_turn` já drena só o `cut_ms`).
- Sem sobreposição nem prompt de contexto na v1 (fronteira exata no timestamp do
  modelo). Pad default 0 para não cortar o início da palavra seguinte.
- Estado (frase anterior/estável) é limpo ao fechar o turno; loop detectado no
  `clean` desliga a âncora daquela prévia.

### Verificação
- `cargo test --lib` **627 ✓** (10 ignorados) · clippy `-D warnings` ✓ · rustfmt ✓.
- Testes novos: `complete_sentences_lists_the_strong_punctuation_anchors`,
  `sentence_normalization_ignores_case_and_punctuation`,
  `sentence_commit_closes_only_the_stable_prefix`,
  `sentence_commit_is_off_by_default`,
  `sentence_commit_requires_a_leftover_and_a_minimum`,
  `sentence_commit_uses_the_pad_after_the_boundary`,
  `sentence_commit_publishes_the_stable_prefix_and_keeps_the_rest`,
  `sentence_commit_is_off_by_default_in_the_pipeline`.
- **Pendente:** teste manual com a flag, comparando qualidade e latência com o
  default. Só depois cogitar virar default.

## 2026-10-21 — Etapa 3 avaliada e rejeitada no teste manual (ISS-058, task 06)

- **IDs:** ISS-058, T06, BR-029, R16, R17
- **Refs:** [`tasks/06-task.md`](tasks/06-task.md), BR-029

### Relato
*“Testei aqui, me parece um pouco melhor sem o SENTENCE COMMIT, erra menos e
parece um pouco mais rápido.”*

### Diagnóstico
- Cada commit re-decoda o prefixo e **reinicia a cadência de prévias** do resto
  do áudio: o texto rolante fica mais tempo parado, o que é percebido como
  lentidão mesmo com commits mais frequentes.
- Mais finais competem com prévias na fila global do ASR (dois canais).
- A pontuação do whisper não é uma fronteira semântica 100% confiável; cortar ali
  pode custar contexto entre frases e gerar mais erros.

### Decisão
- **Default permanece desligado** (`sentence_commit = false`); a task 06 passa a
  **CANCELED** e o código fica atrás da flag (`GHOSTWHISPER_SENTENCE_COMMIT=1`)
  para experimentos futuros.
- A meta de latência do monólogo será atacada pela **etapa 2** (commit otimista:
  publicar a prévia no fechamento e corrigir com o beam em background), que **não
  muda a segmentação** — exige `update_text` na porta de transcrições e merge por
  id na UI.

### Verificação
- `cargo test --lib` **627 ✓** · clippy `-D warnings` ✓.
- Nenhuma mudança de comportamento no default.

## 2026-10-21 — Etapa 3 removida do código (ISS-058, task 06)

- **IDs:** ISS-058, T06, BR-029, R16
- **Refs:** [`tasks/06-task.md`](tasks/06-task.md)

### Decisão
Após o teste manual (sem a flag erra menos e parece mais rápida), o usuário
escolheu **remover** o experimento em vez de mantê-lo atrás da flag. O default
não muda em nada.

### Removido
- `turn_scheduler.rs`: `sentence_commit`, `sentence_min_ms`, `sentence_pad_ms`,
  o ramo de decisão, `stable_sentence_end_ms` e os 4 testes.
- `domain/transcription.rs`: `complete_sentences`/`normalize_sentence` e testes
  (o `ends_with_strong_punctuation` fica — é usado pelo experimento de 0,7 s).
- `transcription_pipeline.rs`: estado `previous_sentence`/`stable_sentence`,
  `note_sentence`, `stable_sentence_end_ms`, `sentence_anchors` no
  `DecodedTurn`, o fake `SentenceAsr` e os 2 testes.
- Docs: BR-029, doc de streaming (tabela/I7/envs), `_tasks.md`; a task 06 fica
  como registro `CANCELED`.

### Verificação
- `cargo test --lib` **619 ✓** · clippy `-D warnings` ✓ · rustfmt ✓.
- Nenhum símbolo da etapa 3 restante no código (`grep` limpo).

## 2026-10-27 — Ordem entre canais e GPU sem falso positivo (ISS-060/ISS-061, task 07)

- **IDs:** ISS-060, ISS-061, T07, BR-029, BR-040, R16, R17
- **Refs:** [`issues.md`](issues.md), [`tasks/07-task.md`](tasks/07-task.md),
  [`../../mvp/architecture/transcription-streaming.md`](../../mvp/architecture/transcription-streaming.md) §3

### Relato
*“Às vezes fica fora de ordem EU e ELES, meu balão vai para frente, mesmo que
haja pausas e uma distância entre as falas. O balão fica muito tempo em azul
processando, mesmo que a pessoa já parou de falar; preciso mandar o bloco para
as Sugestões mais rápido.”*

### Diagnóstico
- O log da sessão de 27/10 mostrava `backend="cpu" gpu_build=true`: a sessão
  rodou em CPU porque a **marca de crash** era gravada no boot. Abrir/fechar o
  app antes de qualquer sessão já condenava o próximo boot. Em GPU (sessão de
  02:25) o mesmo `small` decodificava 7,9 s em 1,26 s; em CPU, 15 s levaram
  10 s. `commit_ms` variava de 6 a 14 s em CPU contra 1,4–2,5 s em GPU.
- A ordem tinha duas causas: a origem era medida antes de criar o motor e cada
  turno avançava pela soma de amostras (fontes abrindo em tempos diferentes e
  backlog de 30 s descartado sem avançar a base); e a prévia ficava sempre no
  fim da lista, então o final curto do ME aparecia acima do turno longo do THEY
  ainda em processamento.

### Correção
- Origem calculada **depois** de criar o motor, por canal; base do turno
  re-ancorada no relógio de parede quando o buffer esvazia (nunca para trás);
  descarte de backlog avança base e índices de fala/prévia.
- `buildChatRows` intercala a fala em andamento pelo `start_ms`; ao finalizar,
  a posição não muda.
- A marca de GPU é armada no carregamento do contexto GPU e desarmada no
  fallback para CPU; abrir/fechar o app sem sessão não afeta mais o boot.

### Verificação
- `cargo test --lib` **832 ✓** (11 ignorados) · clippy `-D warnings` ✓.
- `pnpm test` **522 ✓** (82 arquivos) · `tsc --noEmit` ✓.
- Build final (`pnpm build:final`): AppImage Vulkan+Laya+modelo **821,45 MiB**,
  sha256 `2cfc515dc9afd27e…`, publicado em
  `src-tauri/target/release/bundle/appimage/`.
- **Pendente:** teste manual no app com GPU (conferir `backend="gpu"` no log e a
  ordem das bolhas em conversa alternada).

---

## 2026-10-30 — Build único: AppImage completo (Vulkan + Laya + modelo)

- **IDs:** BR-029 · **Refs:** [`../business-rules.md`](../business-rules.md),
  [`../../README.md`](../../README.md), [`../../AGENTS.md`](../../AGENTS.md)

O usuário definiu que **todo build de release** deve ser o **bundle completo**
(Vulkan + engine Laya + modelo ONNX). Removidos os caminhos parciais:

- `scripts/tauri-build-linux.sh` agora é o build único: confere o artefato ONNX,
  corrige permissões, monta o SDK Vulkan, compila `--features "vulkan,laya"`,
  empacota **só AppImage**, publica e imprime SHA-256.
- Removidos `scripts/build-final.sh`, `scripts/tauri-build-laya.sh`,
  `scripts/tauri-build-cpu.sh` e `scripts/package-linux.sh`, além dos scripts
  `build:final`, `tauri:build:laya` e `tauri:build:cpu`.
- `pnpm tauri build` (CLI) passa por `scripts/tauri-cli.sh`, que redireciona
  `build` para o mesmo caminho completo. `pnpm tauri:build` é o comando canônico.
- O `tauri:dev:release` usa `./node_modules/.bin/tauri` direto para continuar
  gerando só o binário de iteração (sem bundle), com as mesmas features.

### Verificação

- AppImage final (Vulkan + Laya + modelo) **829,48 MiB**, sha256
  `3399bd7da11e4e4cf1f5c2e40135762576f6415cf72aca2ed14614c6e99ea369`,
  em `src-tauri/target/release/bundle/appimage/GhostWhisper_0.3.1_amd64.AppImage`.
- `bash -n` nos scripts e `package.json` válido.
