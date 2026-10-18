# Issues — PRD Precision and Performance

Registry de bugs e problemas deste PRD. Convenções em
[`../../.agents/rules.md`](../../.agents/rules.md) (`ISS-NNN` é canônico por
arquivo de PRD; a numeração **continua** a dos PRDs anteriores para o ID ser
único no repositório). Próximo ID livre: **ISS-023**.

> `ISS-022` permanece reservada pelo PRD [`assistant`](../assistant/issues.md);
> esta PRD começa em `ISS-023` para não colidir.

| ID | Resumo | Severidade | Status |
|---|---|---|---|
| ISS-023 | Bolhas de sugestão longas e sugestão tardia/ausente (paridade Perssua) | Alta | OPEN |
| ISS-024 | Transcrição menos precisa: sem `initial_prompt` de decodificação | Alta | OPEN |
| ISS-025 | Reamostragem linear do microfone (aliasing) e gate de ruído permissivo | Média | OPEN |
| ISS-026 | Microfone sem AEC referenciado (eco/duplicata em viva-voz) | Média | OPEN |
| ISS-027 | Engine CPU-only sem flash attention: modelo grande fica inviável | Média | OPEN |
| ISS-028 | Passe final beam search duplicado no fechamento do turno | Baixa | OPEN |

---

## ISS-023 — Bolhas de sugestão longas e sugestão tardia/ausente

- **Severidade:** Alta · **Status:** OPEN
- **Sintoma:** no Perssua a bolha tem ~3 linhas e aparece rápido; aqui a sugestão
  sai longa, demora e, fora da janela de sugestões, não aparece.
- **Causa raiz:** 1 chamada não-streaming de 900–1400 tokens a cada 20 s, só com
  a janela aberta; contexto só de falas finalizadas; sem *fast-first*; cue não
  colapsado; painel `540×680` contra notch `436×212`.
- **Correção:** E2 — fast-first + streaming + prévias no contexto + auto contínuo
  + UI compacta.
- **Arquivos:** `suggestion_service.rs`, `context.rs`, `ports/llm_controller.rs`,
  `ChatBubble.tsx`, `SuggestionsWindow.tsx`, `useSuggestions.ts`
- **Task:** [`tasks/02-task.md`](tasks/02-task.md)

## ISS-024 — Transcrição menos precisa: sem `initial_prompt` de decodificação

- **Severidade:** Alta · **Status:** OPEN
- **Sintoma:** mais erro e alucinação que o Perssua no mesmo áudio.
- **Causa raiz:** nenhuma chamada define `initial_prompt` (`decode_with`); o
  Perssua envia `whisper_prompt` + regra de idioma em toda request.
- **Correção:** E1 — prompt de decodificação puro + setting + `set_initial_prompt`.
- **Arquivos:** `whisper_speech_to_text.rs`, `domain/transcription.rs`,
  `domain/settings.rs`
- **Task:** [`tasks/01-task.md`](tasks/01-task.md)

## ISS-025 — Reamostragem linear do microfone e gate permissivo

- **Severidade:** Média · **Status:** OPEN
- **Sintoma:** fala do mic com qualidade inferior à do áudio do sistema; mais
  ruído passando ao ASR.
- **Causa raiz:** `resample_linear` (48k→16k) sem anti-aliasing; gate em
  `0.008` (f32) contra `0.025` do Perssua.
- **Correção:** E1 — filtro passa-baixa na decimação + alinhamento de limiares.
- **Arquivos:** `adapters/audio/pcm_resampler.rs`, `domain/audio.rs`
- **Task:** [`tasks/01-task.md`](tasks/01-task.md)

## ISS-026 — Microfone sem AEC referenciado

- **Severidade:** Média · **Status:** OPEN
- **Sintoma:** em viva-voz, o microfone reouve a fala e a transcreve de novo.
- **Causa raiz:** não há cancelamento de eco; só ganho + noise gate.
- **Correção:** E3 — AEC com o áudio do sistema como referência.
- **Arquivos:** `adapters/audio/audio_echo_canceller.rs`, `ports/audio_echo_canceller.rs`
- **Task:** [`tasks/03-task.md`](tasks/03-task.md)

## ISS-027 — Engine CPU-only sem flash attention

- **Severidade:** Média · **Status:** OPEN
- **Sintoma:** impossível subir de modelo sem travar a transcrição.
- **Causa raiz:** `whisper-rs` sem features `cuda`/`vulkan`; `flash_attn=false`.
- **Correção:** E3 — GPU + flash attention com detecção/fallback; modelo por hardware.
- **Arquivos:** `Cargo.toml`, `whisper_speech_to_text.rs`
- **Task:** [`tasks/03-task.md`](tasks/03-task.md)

## ISS-028 — Passe final beam search duplicado

- **Severidade:** Baixa · **Status:** OPEN
- **Sintoma:** o fechamento do turno faz um segundo passe mesmo quando a prévia já
  decodificou o turno inteiro.
- **Causa raiz:** `begin_final` sempre enfileira `SpeechQuality::Final`; o Perssua
  promove o resultado da prévia quando o VAD nativo já fechou (`endedInNoise`).
- **Correção:** E3 — reusar o resultado da prévia quando ela cobre o turno.
- **Arquivos:** `application/transcription_pipeline.rs`
- **Task:** [`tasks/03-task.md`](tasks/03-task.md)

---

<!--
Modelo:

## ISS-0NN — Título curto

- **Severidade:** Alta/Média/Baixa · **Status:** OPEN/FIXED
- **Sintoma:** …
- **Causa raiz:** …
- **Correção:** …
- **Arquivos:** …
- **Task:** [`tasks/NN-task.md`](tasks/NN-task.md)
-->
