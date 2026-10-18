# Tasks — GhostWhisper MVP

Índice de tarefas do MVP. Cada tarefa tem um arquivo `NN-task.md` com detalhamento.

**Legenda de status:** `PENDING` · `IN PROGRESS` · `CONCLUDED` · `CANCELED`

| # | Resumo | Status |
|---|--------|--------|
| 00 | Bootstrap do projeto Tauri v2 + React/TS e estrutura de pastas | CONCLUDED |
| 01 | Janela overlay X11: transparência, always-on-top e click-through | CONCLUDED |
| 02 | Captura de microfone (`cpal`) + pipeline PCM/resample | CONCLUDED |
| 03 | Captura de áudio do sistema (monitor de sink) + sink virtual helper | CONCLUDED |
| 04 | VAD Silero + ASR whisper.cpp (transcrição local streaming) | CONCLUDED |
| 05 | Persistência SQLite (schema + FTS5) e migrações | CONCLUDED |
| 06 | Settings + providers de LLM (BYO key via keyring) | CONCLUDED |
| 07 | UI do overlay React (transcrição ao vivo + ActionBar) | CONCLUDED |
| 08 | Tradução ao vivo + insights/sugestões | CONCLUDED |
| 09 | Atalhos globais + stealth básico | CONCLUDED |
| 10 | Empacotamento (.deb/AppImage) + spike Wayland/invisibilidade | CONCLUDED |
| 11 | Streaming de transcrição estilo Perssua (whisper fluindo) | CONCLUDED |
| 12 | Configurações em abas, chave redactada e compat com providers | CONCLUDED |

## Dependências entre tarefas

```
00 ─┬─▶ 01 ─┐
    ├─▶ 02 ─┼─▶ 04 ─▶ 05 ─▶ 06 ─▶ 07 ─▶ 08
    └─▶ 03 ─┘                   └─▶ 09
                                   └─▶ 10
```

- 00 é pré-requisito de tudo.
- 04 depende de 02 e 03 (fontes de áudio).
- 07 depende de 05 e 06 (dados e providers expostos).
- 08 depende de 07; 09 e 10 dependem de 01/07.

## Definição de pronto (geral)
- Código compila (`pnpm tauri build` sem erros).
- Critérios de aceite da tarefa atendidos.
- Arquivo da tarefa atualizado para `CONCLUDED`.
- `../memory.md` atualizado com o que foi feito na data.
