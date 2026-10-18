# 01 — Precisão do ASR (sinal + decodificação) · Entregável E1

- **Status:** IN PROGRESS
- **Depende de:** 04/13/14 do MVP (ASR, streaming, front-end de áudio)
- **Habilita:** 03 (default de modelo com GPU)
- **Issues:** ISS-024, ISS-025 · **BR nova:** BR-027
- **Refs:** BR-015, BR-018,
  [`../architecture/precision-and-performance.md`](../architecture/precision-and-performance.md) §2/§5

## Objetivo

Reduzir erro e alucinação da transcrição **sem GPU e sem regressão de latência**,
transpondo o que o Perssua já faz na decodificação e no sinal.

## Entregáveis

- `src-tauri/src/domain/transcription.rs` — funções **puras**
  `decode_prompt(base, language)` e `map_whisper_language(code)` espelhando o
  `pt()` do Perssua: base + `The audio is spoken in <idioma>. Transcribe it in
  <idioma>. Do not translate it to English.` (no domínio porque o adaptador de ASR
  as consome).
- `src-tauri/src/domain/settings.rs` — campo `whisper_prompt: String` com default
  `"Only transcribe what is clearly spoken. If uncertain, transcribe nothing."`;
  validação/sanitização e serialização retrocompatível (`#[serde(default)]`).
- `src-tauri/src/adapters/speech/whisper_speech_to_text.rs` — aceitar o prompt
  (config → adapter) e chamar `params.set_initial_prompt(...)`; mapear o idioma
  (`pt-br` → `pt`) antes de `set_language`.
- `src-tauri/src/adapters/audio/pcm_resampler.rs` — filtro passa-baixa antes da
  decimação (anti-aliasing), mantendo a assinatura pública e o ganho unitário.
- `src-tauri/src/adapters/speech/whisper_model_store.rs` — `large-v3` e
  `large-v3-turbo` no catálogo (download sob demanda; `sha256: None`).
- `src-tauri/src/ports/gpu_probe.rs` (novo) — porta `GpuProbe` + `GpuInfo`
  (`available`, `backend`, `detail`, `acceleration_enabled`).
- `src-tauri/src/adapters/system/vulkan_gpu_probe.rs` (novo) — detecção Linux-first
  por ICDs Vulkan (`/usr/share/vulkan/icd.d`, `/etc/vulkan/icd.d`, `VK_ICD_FILENAMES`)
  + `/dev/dri/renderD*`, ignorando rasterizers de software (lavapipe/llvmpipe);
  parsing **puro** e testável.
- `src-tauri/src/ipc/settings_commands.rs` — comando `settings_gpu_info`.
- `src-tauri/src/{state.rs,lib.rs}` — `AppState.gpu` e wiring do probe no
  composition root.
- `src/core/domain/settings.ts` + `src/features/settings/SettingsPage.tsx` —
  `whisper_prompt` editável, os dois modelos novos na lista (com aviso "exige GPU
  para tempo real; em CPU use tiny/small") e o **aviso de GPU detectada**
  sugerindo o modelo adequado (sem trocar sozinho).
- `src/adapters/ipc/settings.ts` — `gpuInfo()`.
- `prds/business-rules.md` — **BR-027** (prompt de decodificação + política de
  modelo desta fase + detecção de GPU como sugestão).

## Critérios de aceite

- [x] Toda decodificação (prévia e final) envia o `initial_prompt` configurado.
- [x] Idioma mapeado (`pt-br` → `pt`) e nome legível no prompt; `auto` não
      acrescenta regra.
- [x] `whisper_prompt` vazio ⇒ nenhum prompt (comportamento antigo preservado).
- [x] Resampler atenua tom acima de 8 kHz em vez de espelhá-lo; ganho unitário
      preservado; testes de borda (vazio, mesma taxa, estéreo 48k).
- [x] Gate/VAD continuam nos limiares do BR-018 (sem adotar 0,025), cobertos por
      teste, com override por env preservado.
- [x] `large-v3` e `large-v3-turbo` aparecem na UI e baixam ao selecionar.
- [x] A configurações mostram **GPU detectada** e **sugerem** o modelo adequado,
      deixando claro que a aceleração chega no E3; nada é trocado automaticamente.
- [x] Detecção de GPU não confunde rasterizer de software (lavapipe/llvmpipe) com
      hardware.
- [x] **Default do modelo permanece `tiny`** (muda só no E3, com GPU).
- [ ] RTF sem regressão > 10 % no benchmark `tiny`/CPU (rodar no release).
- [x] `cargo test` (300 ✓), `clippy -D warnings` ✓, `fmt --check` ✓, `pnpm test`
      (119 ✓), `tsc --noEmit` ✓.
- [ ] Teste manual do usuário: sessão pt-BR comparando com/sem `initial_prompt`.

## Como testar

Automatizado:

```bash
cargo test --lib
cargo test --release --lib -- --ignored --nocapture benchmark_rtf
pnpm test && pnpm exec tsc --noEmit && pnpm build
```

Manual (sessão real, pt-BR):

1. Configurações → Transcrição: conferir o campo de prompt, a lista de modelos
   (default ainda `tiny`) e o aviso de GPU (se a máquina tiver Vulkan).
2. Rodar um vídeo/reunião pt-BR de ~2 min com fala contínua e algum ruído.
3. Comparar com a execução anterior: contagem de palavras erradas visíveis e de
   trechos alucinados; observar no log `passe de transcrição concluído` o RTF.
4. Repetir com o prompt de decodificação **vazio** para confirmar que o efeito é
   do prompt.
5. Conferir em `settings_get`/UI que o idioma `pt` produz o prompt com a regra de
   idioma ("Transcribe it in Portuguese… Do not translate it to English").

Ponto de parada: usuário aprova (ou pede ajuste no prompt/limiares) antes de E2.

## Riscos / rollback

- Prompt pode induzir resposta em vez de transcrição → manter curto, testar com
  áudio real e permitir desligar (campo vazio).
- Filtro passa-baixa pode atrasar a prévia → medir RTF; filtro curto e linear.

## Verificação

- **Automática:** `cargo test --lib` 300 ✓ (8 `ignored`: benchmarks/E2E) ·
  `cargo clippy --all-targets -- -D warnings` ✓ · `cargo fmt --check` ✓ ·
  `pnpm test` 119 ✓ · `tsc --noEmit` ✓. Novos testes: prompt de decodificação +
  idioma (domínio), `whisper_prompt` (default/limite/NUL), catálogo de modelos
  grandes, anti-aliasing do resampler (tom > Nyquist, nível da fala, blocos
  contínuos), classificação de ICDs Vulkan (software ignorado) e o sugerido no
  painel de configurações.
- **Pendente (manual):** sessão real pt-BR com/sem o prompt de decodificação;
  benchmark de RTF no release. Status segue `IN PROGRESS` até a aprovação.
- **Observações do usuário:** (aguardando)
