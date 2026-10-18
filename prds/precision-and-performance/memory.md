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
- Pendente: teste manual pt-BR com/sem prompt e benchmark de RTF (release);
  task 01 segue `IN PROGRESS` até a aprovação.
