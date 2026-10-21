# 05 — `RecordingService`: ciclo de vida amarrado à sessão

- **Status:** CONCLUDED
- **Depende de:** 01, 04
- **Habilita:** 06, 07, 08
- **Issues:** ISS-050 · **BR:** BR-065, BR-067, BR-071
- **Refs:** [`../prd.md`](../prd.md) US-01, US-02;
  [`../architecture/recording.md`](../architecture/recording.md) §3, §4

## Objetivo

Orquestrar a gravação **como parte da sessão**: congelar a configuração no
início, acompanhar `pause`/`resume`/`stop` do `SessionController`, persistir o
estado e **isolar falhas** (RNF-07).

## Entregáveis

- `src-tauri/src/application/recording_service.rs`:
  - `RecordingController` (porta de aplicação usada pelo IPC):
    `start(session_id, spec, target)`, `pause`, `resume`, `stop`, `state`,
    `capabilities`, `discard`;
  - `start` congela `RecordingSpec` de `Settings` **uma vez** (CA-02.7);
  - assina as transições da sessão: `pause` → `pause`; `resume` → `resume`
    **se** a gravação estava ativa; `stop` → `stop` + finalização;
  - grava `recordings` na transição (`recording`/`paused`/`finalizing`/`ready`);
  - falha do adaptador → `failed`, emite evento, **não** propaga `Err` ao
    `SessionService` (RNF-07);
  - no boot: linhas `recording`/`paused` → `interrupted` e tentativa de
    recuperação (task 04).
- `src-tauri/src/state.rs`: `recording: Arc<dyn RecordingController>`.
- `src-tauri/src/lib.rs`: composição (R2).
- `src-tauri/src/domain/settings.rs`: campos de gravação + normalização pura
  (task 09 fornece a UI; o modelo nasce aqui).

## Critérios de aceite

- [ ] `start` sem sessão devolve erro de configuração (nunca grava solto).
- [ ] Pausar a **gravação** não pausa a sessão; pausar a **sessão** pausa a
      gravação e retomar a sessão retoma a gravação.
- [ ] `stop` da sessão finaliza o arquivo e marca `ready`.
- [ ] `FakeScreenRecorder` que falha no `start` deixa a sessão `active` e o
      estado `failed`, com evento emitido.
- [ ] Mudar `Settings` no meio da sessão **não** altera `RecordingSpec`
      (teste com dois `start` e settings diferentes).
- [ ] Boot com linha `recording` órfã → `interrupted`, sem abrir modal.
- [ ] `discard` apaga arquivo e `parts/` e marca `discarded`, mantendo a sessão.

## Verificação

- `cargo test --lib` com `FakeScreenRecorder` +
  `InMemoryRecordingsRepository` · `cargo clippy`

## Notas

- O serviço **não** conhece `ffmpeg`; só `ScreenRecorder` + repositório.
- A sessão continua sendo a dona do tempo; a gravação tem duração própria
  (soma dos segmentos) e as duas podem divergir por causa das pausas.
