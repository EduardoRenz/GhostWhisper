# Issues — PRD Precision and Performance

Registry de bugs e problemas deste PRD. Convenções em
[`../../.agents/rules.md`](../../.agents/rules.md) (`ISS-NNN` é canônico por
arquivo de PRD; a numeração **continua** a dos PRDs anteriores para o ID ser
único no repositório). Próximo ID livre: **ISS-060**.

> `ISS-022` permanece reservada pelo PRD [`assistant`](../assistant/issues.md);
> esta PRD começa em `ISS-023` para não colidir.

| ID | Resumo | Severidade | Status |
|---|---|---|---|
| ISS-023 | Bolhas de sugestão longas e sugestão tardia/ausente (paridade Perssua) | Alta | FIXED |
| ISS-024 | Transcrição menos precisa: sem `initial_prompt` de decodificação | Alta | FIXED |
| ISS-025 | Reamostragem linear do microfone (aliasing) e gate de ruído permissivo | Média | FIXED |
| ISS-026 | Microfone sem AEC referenciado (eco/duplicata em viva-voz) | Média | FIXED |
| ISS-027 | Engine CPU-only sem flash attention: modelo grande fica inviável | Média | FIXED |
| ISS-028 | Passe final beam search duplicado no fechamento do turno | Baixa | FIXED |
| ISS-058 | Commit de fala preso atrás de prévia e silêncio fixo de 1 s | Alta | FIXED |
| ISS-059 | Prompt instrucional vazava para a transcrição (frases sem sentido) | Alta | FIXED |

---

## ISS-023 — Bolhas de sugestão longas e sugestão tardia/ausente

- **Severidade:** Alta · **Status:** FIXED
- **Sintoma:** no Perssua a bolha tem ~3 linhas e aparece rápido; aqui a sugestão
  sai longa, demora e, fora da janela de sugestões, não aparece.
- **Causa raiz:** 1 chamada não-streaming de 900–1400 tokens a cada 20 s, só com
  a janela aberta; contexto só de falas finalizadas; sem *fast-first*; cue não
  colapsado; painel `540×680` contra notch `436×212`.
- **Correção:** E2 — fast-first + streaming + prévias no contexto + agendador no
  core (sem depender da janela) + bolha colapsada (BR-028).
- **Arquivos:** `suggestion_service.rs`, `context.rs`, `suggestion_scheduler.rs`,
  `ports/{llm_controller,chat_bot_provider,event_publisher}.rs`,
  `openai_compatible.rs`, `lib.rs`, `ChatBubble.tsx`, `SuggestionsWindow.tsx`,
  `useSuggestions.ts`
- **Task:** [`tasks/02-task.md`](tasks/02-task.md)

## ISS-024 — Transcrição menos precisa: sem `initial_prompt` de decodificação

- **Severidade:** Alta · **Status:** FIXED
- **Sintoma:** mais erro e alucinação que o Perssua no mesmo áudio.
- **Causa raiz:** nenhuma chamada define `initial_prompt` (`decode_with`); o
  Perssua envia `whisper_prompt` + regra de idioma em toda request.
- **Correção:** E1 — prompt de decodificação puro + setting + `set_initial_prompt`.
- **Arquivos:** `whisper_speech_to_text.rs`, `domain/transcription.rs`,
  `domain/settings.rs`
- **Task:** [`tasks/01-task.md`](tasks/01-task.md)

## ISS-025 — Reamostragem linear do microfone e gate permissivo

- **Severidade:** Média · **Status:** FIXED
- **Sintoma:** fala do mic com qualidade inferior à do áudio do sistema; mais
  ruído passando ao ASR.
- **Causa raiz:** `resample_linear` (48k→16k) sem anti-aliasing; gate em
  `0.008` (f32) contra `0.025` do Perssua.
- **Correção:** E1 — filtro passa-baixa na decimação + alinhamento de limiares.
- **Arquivos:** `adapters/audio/pcm_resampler.rs`, `domain/audio.rs`
- **Task:** [`tasks/01-task.md`](tasks/01-task.md)

## ISS-026 — Microfone sem AEC referenciado

- **Severidade:** Média · **Status:** FIXED
- **Sintoma:** em viva-voz, o microfone reouve a fala e a transcreve de novo.
- **Causa raiz:** não há cancelamento de eco; só ganho + noise gate.
- **Correção:** E3 — AEC com o áudio do sistema como referência.
- **Arquivos:** `adapters/audio/audio_echo_canceller.rs`, `ports/audio_echo_canceller.rs`
- **Task:** [`tasks/03-task.md`](tasks/03-task.md)

## ISS-027 — Engine CPU-only sem flash attention

- **Severidade:** Média · **Status:** FIXED
- **Sintoma:** impossível subir de modelo sem travar a transcrição.
- **Causa raiz:** `whisper-rs` sem features `cuda`/`vulkan`; `flash_attn=false`.
- **Correção:** E3 — GPU + flash attention com detecção/fallback; modelo por hardware.
- **Arquivos:** `Cargo.toml`, `whisper_speech_to_text.rs`
- **Task:** [`tasks/03-task.md`](tasks/03-task.md)

## ISS-028 — Passe final beam search duplicado

- **Severidade:** Baixa · **Status:** FIXED
- **Sintoma:** o fechamento do turno faz um segundo passe mesmo quando a prévia já
  decodificou o turno inteiro.
- **Causa raiz:** `begin_final` sempre enfileira `SpeechQuality::Final`; o Perssua
  promove o resultado da prévia quando o VAD nativo já fechou (`endedInNoise`).
- **Correção:** E3 — reusar o resultado da prévia quando ela cobre o turno.
- **Arquivos:** `application/transcription_pipeline.rs`
- **Task:** [`tasks/03-task.md`](tasks/03-task.md)

## ISS-058 — Commit de fala preso atrás de prévia e silêncio fixo de 1 s

- **Severidade:** Alta · **Status:** FIXED
- **Sintoma:** o texto final da frase demora ~1,7–2,2 s (e até 25 s em fala
  contínua) para chegar ao histórico/sugestor; o usuário pediu "pedaços de frases
  concluídas o mais rápido possível".
- **Causa raiz:** três somas no mesmo caminho:
  1. o agendador continuava pedindo **prévia durante o silêncio** (passo de
     0,75 s), então o passe final entrava na fila atrás de uma prévia descartável;
  2. a fila global do ASR era **FIFO entre canais** — o final do microfone
     esperava a prévia do sistema, mesmo sendo autoritativo;
  3. o fechamento só acontecia com **1 s cheio** de silêncio, mesmo quando a
     prévia já mostrava uma frase concluída.
- **Correção:** etapa 1 do plano de latência — corte de prévias após **250 ms**
  de pausa (`preview_cutoff_ms`); prioridade `Final` > `Preview` na fila global
  (`SpeechGate`, sem preempção do passe em execução); fechamento em **700 ms**
  quando a prévia cobre o fim da fala e termina em `.?!`
  (`silence_sentence_ms`), mantendo 1 s para vírgula/reticências; métrica
  `commit_ms = espera + decodificação` no log (`"commit de fala"`).
- **Arquivos:** `application/turn_scheduler.rs`,
  `application/transcription_pipeline.rs`, `domain/transcription.rs`,
  `adapters/speech/speech_gate.rs`,
  `adapters/speech/serialising_speech_to_text.rs`,
  `adapters/speech/whisper_speech_engine_factory.rs`
- **Task:** [`tasks/04-task.md`](tasks/04-task.md)

## ISS-059 — Prompt instrucional vazava para a transcrição (frases sem sentido)

- **Severidade:** Alta · **Status:** FIXED
- **Sintoma:** “frases sem sentido” no **microfone**, repetidas na prévia e às
  vezes mantidas no final; pior quando o áudio estava incerto.
- **Causa raiz:** o `whisper_prompt` estava salvo como **instrução**
  (“Only transcribe what is clearly spoken…”) e o app ainda anexava a regra de
  idioma. O `initial_prompt` do whisper é **contexto** (texto anterior), não
  comando: em áudio incerto o modelo o **continua** — a frase “Se você não sabe,
  você pode falar em português” no log é paráfrase da regra de idioma.
- **Correção:** default do prompt **vazio**; migração de settings limpa o texto
  antigo; `decode_prompt` não anexa mais a regra de idioma (`set_language` +
  `set_translate(false)` já bastam); a UI virou “Contexto da transcrição” com
  aviso explícito de que instruções atrapalham.
- **Verificação:** limpar o campo corrigiu a transcrição na hora (teste do
  usuário, 21/10); prévias caíram de ~700–1800 ms para ~250–350 ms.
- **Arquivos:** `domain/settings.rs`, `domain/transcription.rs`,
  `adapters/speech/whisper_speech_engine_factory.rs`,
  `src/core/domain/settings.ts`, `src/features/settings/SettingsPage.tsx`
- **Task:** [`tasks/05-task.md`](tasks/05-task.md)

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
