# Tasks — PRD Gravação da sessão

Índice de tarefas deste PRD. Cada tarefa tem um arquivo `NN-task.md`.

**Legenda de status:** `PENDING` · `IN PROGRESS` · `CONCLUDED` · `CANCELED`

| # | Resumo | Status |
|---|--------|--------|
| 01 | Domínio `Recording`, schema `recordings` e porta `ScreenRecorder` (+ fake) | CONCLUDED |
| 02 | `FfmpegScreenRecorder`: binário, args puros, segmentos MKV, alvo janela/tela | CONCLUDED |
| 03 | Seleção de janela (`_NET_ACTIVE_WINDOW` + seletor) e eventos de resize/close | IN PROGRESS |
| 04 | Finalização: concat `-c copy`, remux MP4, progresso, re-encode e órfãos | CONCLUDED |
| 05 | `RecordingService`: ciclo de vida amarrado à sessão e falha isolada | CONCLUDED |
| 06 | IPC + eventos `recording_*` + `SessionEndDto` no `session_stop` | CONCLUDED |
| 07 | Botão de câmera no notch (4 estados, atalho, acessibilidade) | IN PROGRESS |
| 08 | Modal "Sessão Finalizada" (janela, progresso, ações) | CONCLUDED |
| 09 | Configurações: aba "Gravação" (campos, padrões, normalização, estimativa) | CONCLUDED |
| 10 | Histórico/export, documentação, verificação e pacotes | IN PROGRESS |

## Ordem e dependências

```
01 ─▶ 02 ─▶ 04 ─▶ 05 ─▶ 06 ─▶ 07 ─┐
      03 ─┘                       ├─▶ 08 ─▶ 10
                       09 ────────┘
```

- A **01** cria o modelo, a porta e a tabela; é pré-requisito de tudo que
  persiste ou grava.
- A **02** é o coração (spawn do ffmpeg + segmentos); a **03** acrescenta o alvo
  por janela, que depende do recorte X11.
- A **04** fecha o arquivo (concat/remux), inclusive o caso de dimensões
  diferentes (ISS-053).
- A **05** amarra o ciclo de vida à sessão (BR-065) e isola falhas (RNF-07).
- A **06** expõe o estado e o resumo do fim de sessão para o frontend.
- A **07** e a **08** são a superfície (botão e modal); dependem de 05/06.
- A **09** é independente do núcleo (settings) e pode andar em paralelo após a 01.
- A **10** fecha: Histórico, export, BRs, ISS, `memory.md`, verificação e
  empacotamento (bloqueado por ISS-049).

## Entregáveis por camada

| Task | `domain` | `application` | `ports` | `adapters` | `ipc` | frontend |
|---|---|---|---|---|---|---|
| 01 | `recording.rs` | — | `screen_recorder.rs`, `recordings_repository.rs` | `schema.sql`, `in_memory_…` | — | — |
| 02 | — | — | — | `ffmpeg_args.rs`, `ffmpeg_path.rs`, `ffmpeg_screen_recorder.rs` | — | — |
| 03 | `recording.rs` (alvo) | — | `screen_recorder.rs` | `x11_window_picker.rs` | `recording_commands.rs` (listar janelas) | `WindowPicker` |
| 04 | — | — | — | `ffmpeg_finalizer.rs` | — | — |
| 05 | — | `recording_service.rs` | — | — | — | — |
| 06 | `recording.rs` (DTO) | — | — | `tauri_event_publisher.rs` | `recording_commands.rs`, `session_commands.rs` | `adapters/ipc/recording.ts` |
| 07 | — | — | — | — | — | `NotchTabs`, `RecordingButton` |
| 08 | — | — | — | — | `session_end_commands.rs` | `features/session-end/*` |
| 09 | `settings.rs` | — | — | `file_settings_store` (já existe) | `settings_commands` (já existe) | `RecordingSettings` |
| 10 | — | `markdown_export.rs` | — | — | `export_commands.rs` | `HistoryPanel` |

> Nenhum adaptador é instanciado fora de `src-tauri/src/lib.rs` (R2).
