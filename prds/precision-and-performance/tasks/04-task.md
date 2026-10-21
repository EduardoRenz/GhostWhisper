# 04 — Commit de fala mais rápido · Etapa 1 (fila + endpointing)

- **Status:** IN PROGRESS (código e testes automatizados prontos; falta o teste
  manual no app)
- **Depende de:** 03 (reuso do passe final, VAD nativo) e da fila global do ASR
- **Issues:** ISS-058 · **Desvio de paridade:** BR-029 (Perssua fecha em 1 s)
- **Refs:** BR-015, BR-029, R16, R17,
  [`../architecture/precision-and-performance.md`](../architecture/precision-and-performance.md) §4,
  [`../../mvp/architecture/transcription-streaming.md`](../../mvp/architecture/transcription-streaming.md) §3–§4

## Objetivo

O texto final da frase (o que o **sugestor** e a **sessão** consomem) precisa
aparecer o mais perto possível de 1 s depois da última palavra, sem picar frase
no meio. Esta etapa ataca o que acontece **antes e em volta** da decodificação —
sem mudar a unidade de commit (o turno), que fica para a etapa 3.

## Entregáveis

- `src-tauri/src/application/turn_scheduler.rs` — decisão pura:
  - **corte de prévias** `preview_cutoff_ms` (250 ms de silêncio): a prévia pedida
    no início da pausa entrava na frente do passe final na fila do ASR;
  - **fechamento por frase** `silence_sentence_ms` (700 ms): fecha antes do
    silêncio cheio **somente** quando a última prévia cobre o fim da fala e
    termina em pontuação forte (`.`, `!`, `?`);
  - clamps de coerência (`sentence ≤ silence`, `cutoff ≤ sentence`).
- `src-tauri/src/domain/transcription.rs` — `ends_with_strong_punctuation`
  (ignora fechamentos como `"`/`)`; reticências contam como pausa interna).
- `src-tauri/src/application/transcription_pipeline.rs` — `preview_ends_sentence`
  só aceita a **mesma prévia** que ancorou a cadência (turno + corte + texto);
  métrica `commit_ms = wait_ms + queue_ms + asr_ms`, logada em `info`
  (`"commit de fala"`) e exposta para teste. `wait_ms` é medido por **relógio de
  parede** desde a última janela falada (com piso no silêncio contido no áudio),
  para não inflar com blocos grandes de áudio ou tick atrasado.
- `src-tauri/src/adapters/speech/speech_gate.rs` (novo) — fila global do ASR com
  prioridade `Final` > `Preview`; sem preempção de um passe **em execução**.
- `src-tauri/src/adapters/speech/serialising_speech_to_text.rs` — linha
  `"fila do ASR"` separa a espera na **fila global** (`gate_wait_ms`) do decode
  (`decode_ms`); finais sempre reportam, prévias só quando a espera passa de 1 s.
- `src-tauri/src/lib.rs` + `Cargo.toml` — `whisper-rs/tracing_backend` +
  `install_logging_hooks()`: o whisper.cpp/ggml passa a logar pelo `tracing` e o
  filtro padrão (`RUST_LOG` + `whisper_rs=warn`) silencia o ruído de beam search.
- Envs novos: `GHOSTWHISPER_SILENCE_SENTENCE_MS` (700) e
  `GHOSTWHISPER_PREVIEW_CUTOFF_MS` (250).

## Critérios de aceite

- [x] **Experimental (desligado por padrão):** com
      `GHOSTWHISPER_SILENCE_SENTENCE_MS=700`, uma pausa em frase terminada em
      `.?!` fecha em 0,7 s; com vírgula/reticências continua em 1 s.
      **Testes:** `finished_sentence_closes_at_the_short_silence`,
      `comma_pause_does_not_close_the_turn` (pipeline) e
      `finished_sentence_closes_before_full_silence`,
      `comma_pause_waits_for_full_silence` (decisão pura).
- [x] **Regressão 21/10:** o default **não** fecha por pontuação sozinha
      (**Testes:** `early_close_is_off_by_default`,
      `default_does_not_close_on_punctuation_alone` — falhariam no código que
      fechava em 0,7 s por padrão).
- [x] **Experimental (desligado por padrão):** com
      `GHOSTWHISPER_PREVIEW_CUTOFF_MS=250`, nenhuma prévia é pedida durante a
      pausa (0,8 s de silêncio ⇒ só o final). **Testes:**
      `previews_stop_when_speech_pauses`.
- [x] **Baseline (regressão de 21/10):** por padrão a prévia **continua** rolando
      durante a pausa, como antes da L1 (`previews_continue_during_pause_by_default`).
- [x] O passe final **não espera** uma prévia que ainda não começou: o `Final`
      fura a fila. **Teste:** `final_overtakes_a_waiting_preview`.
- [x] A fila continua serializada (nunca dois passes ao mesmo tempo) e o
      timeout de 20 s limpa o contador de espera.
      **Testes:** `never_decodes_in_parallel`, `queue_timeout_does_not_leave_a_stale_waiter`.
- [x] Métrica de commit disponível e **decomposta**:
  `commit_ms = wait_ms (parede desde a última fala) + queue_ms (fila do canal) +
  asr_ms (fila global + decode)`. **Teste:**
  `commit_latency_counts_the_silence_wait`.
- [x] Log do whisper.cpp/ggml silenciado por padrão: roteado para `tracing`
  (`tracing_backend` + `install_logging_hooks`) e filtrado em `whisper_rs=warn`
  mesmo com `RUST_LOG=info`. **Verificado:** com `info` os eventos chegam pelo
  alvo `whisper_rs::whisper_logging_hook`; com o filtro padrão, zero linhas.
- [x] Teste pesado com modelo real (JFK, streaming) continua passando:
      `end_to_end_streams_preview_before_final`.
- [ ] Teste manual no app (ver "Como testar").

## Desvio consciente vs. Perssua (BR-015/R12)

O Perssua fecha o turno local em **1 s** de silêncio
(`local_whisper_vad_silence_duration_ms`). Aqui o **default também é 1 s**. O
fechamento em 0,7 s por pontuação forte existe como **experimento opt-in**
(`GHOSTWHISPER_SILENCE_SENTENCE_MS=700`): o teste manual de 21/10 indicou piora
de qualidade e latência com ele ligado, porque o whisper marca ponto em pausas
internas e a antecipação pica a frase.

## Como testar

1. `cargo test --lib` (616 ✓) e, opcionalmente, o teste pesado:
   `cargo test --lib end_to_end_streams_preview_before_final -- --ignored --test-threads=1`.
2. Abrir o app, iniciar sessão e falar frases terminadas em ponto, com pausas
   curtas de vírgula no meio.
   - **Default:** fecha em **1 s** de silêncio (frase ou vírgula) — mesma
     segmentação de antes da task.
   - **Experimento:** `GHOSTWHISPER_SILENCE_SENTENCE_MS=700` fecha em ~0,7 s
     frases terminadas em `.`/`?`/`!`; vírgula continua em 1 s.
3. No log (`RUST_LOG=info`), conferir as linhas:
   - `commit de fala` com `wait_ms` ≈ **1000** no default (≈700 com o
     experimento), `queue_ms` ≈ 0 e `asr_ms` = fila global + decode;
     `commit_ms` é a soma.
   - `fila do ASR` (uma por final) com `gate_wait_ms` e `decode_ms` — se o final
     esperou uma prévia em execução, aparece aqui.
   - Nenhuma linha `whisper_full_with_state` (o ruído do whisper.cpp fica fora);
     para depurar o ASR: `RUST_LOG=info,whisper_rs=debug`.
4. Comparar antes/depois com os envs:
   `GHOSTWHISPER_SILENCE_SENTENCE_MS=1000` desliga o fechamento curto;
   `GHOSTWHISPER_VAD_SILENCE_MS=1000` mantém o fechamento conservador.
5. Em sessão de dois canais, falar junto com o interlocutor e verificar que o
   final do microfone não fica atrás da prévia do sistema.

## Verificação automatizada

- `cargo test --lib` **617 ✓** (10 ignorados) · clippy `-D warnings` ✓ ·
  rustfmt nos arquivos alterados ✓.
- Teste pesado (`tiny`, JFK): `end_to_end_streams_preview_before_final` ✓.
