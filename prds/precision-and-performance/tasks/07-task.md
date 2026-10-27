# 07 — Ordem entre canais e GPU sem falso negativo

- **Status:** IN PROGRESS (código e testes prontos; falta o teste manual no app)
- **Depende de:** 03 (engine/GPU) e 04 (métrica de commit)
- **Issues:** ISS-060, ISS-061 · **BR:** BR-029, BR-040
- **Refs:** BR-015, BR-019, R16, R17,
  [`../architecture/precision-and-performance.md`](../architecture/precision-and-performance.md) §4,
  [`../../mvp/architecture/transcription-streaming.md`](../../mvp/architecture/transcription-streaming.md) §3

## Objetivo

Corrigir a ordem das falas entre **microfone (ME)** e **sistema (THEY)** e
impedir que um encerramento sem GPU condene a próxima execução a rodar em CPU.

O relato de 27/10 tinha duas causas independentes:

1. **Relógio por canal:** a origem era medida antes de criar o motor e cada turno
   avançava só pela soma de amostras. Uma fonte que demorava a abrir (ou um
   backlog descartado) deixava um canal segundos fora de fase; o balão do ME
   aparecia antes/depois do turno do THEY.
2. **Prévia no fim:** enquanto o turno longo do THEY estava em processamento, o
   final curto do ME era desenhado acima dele; ao fechar, o turno do THEY era
   inserido antes e a bolha “pulava”.

Além disso, o log mostrou a sessão inteira em **CPU** (`backend="cpu"`) porque a
marca de crash de GPU era gravada no boot, antes de qualquer sessão: abrir e
fechar o app algumas vezes bastava para o próximo boot desligar a GPU.

## Entregáveis

- `application/session_service.rs` — a origem de tempo é calculada **depois** de
  criar o motor, por canal (a criação do modelo pode levar segundos).
- `application/transcription_pipeline.rs` — quando o buffer esvazia, a base do
  próximo turno é ancorada no relógio de parede (`next_turn_base`), nunca para
  trás; o descarte do backlog de 30 s avança a base e os índices de fala/prévia.
- `src/core/domain/chat.ts` / `src/features/chat/ChatView.tsx` — `buildChatRows`
  intercala a fala em andamento pelo `start_ms`; a posição não muda ao finalizar.
- `adapters/speech/whisper_speech_to_text.rs` — a marca de GPU é armada no
  carregamento do contexto GPU e desarmada no fallback para CPU; o boot continua
  lendo a marca para o fallback de segurança.
- `lib.rs` / `whisper_speech_engine_factory.rs` — o caminho da marca passa pela
  fábrica; some a gravação no boot.

## Fora de escopo

- Segmentação (commit por frase) continua desligada; a etapa 3 foi rejeitada no
  teste manual de 21/10. Se o monólogo ainda atrasar as Sugestões **com GPU**,
  avaliar um corte opt-in só para turnos longos (`> 10 s`) em task futura.
- Fila/preempção de prévias em execução: o `SpeechGate` já prioriza o final;
  não há como abortar um passe do whisper no meio.

## Verificação

- `cargo test --lib` **832 ✓** (11 ignorados) · clippy `-D warnings` ✓.
- `pnpm test` **522 ✓** (82 arquivos) · `tsc --noEmit` ✓.
- Build final (`pnpm build:final`): AppImage Vulkan+Laya+modelo **821,45 MiB**,
  sha256 `2cfc515dc9afd27e…`.
- Testes novos:
  - `next_turn_base_anchors_late_chunks_without_going_before_capture`;
  - `dropped_backlog_advances_the_timeline_and_the_speech_index`;
  - `an_empty_buffer_reanchors_only_forward`;
  - `gpu_marker_is_armed_and_disarmed`;
  - `a fala em andamento fica no lugar do tempo, não no fim da lista`;
  - `ao finalizar, a fala não muda de posição (mesmo start_ms)`.

## Teste manual (pendente)

1. Abrir o app **sem** sessão e fechar; abrir de novo e iniciar uma sessão.
2. Conferir no log (`Configurações → Geral & HUD → Logs`) a linha
   `carregando modelo whisper backend="gpu"`; o `commit_ms` deve ficar perto de
   1–2 s (não 6–14 s).
3. Conversar alternando ME/THEY com pausas: a bolha azul do turno em andamento
   deve ficar **no lugar** (antes da fala do ME que veio depois) e não pular ao
   fechar.
4. Forçar um turno longo (> 10 s) e conferir que a fala seguinte não aparece
   acima dele enquanto ele processa.
