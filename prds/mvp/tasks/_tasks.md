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
| 13 | Streaming em tempo real: decisão desacoplada + ASR robusto | CONCLUDED |
| 14 | Sensibilidade do microfone (ganho) e noise gate de entrada | CONCLUDED |
| 15 | Monitor atrasado do microfone ("ouvir minha voz") | CONCLUDED |
| 16 | Sessão contínua (pausar/retomar/continuar) e linha temporal | CONCLUDED |
| 17 | Histórico de sessões e sugestões da IA na linha temporal | CONCLUDED |
| 18 | Exportar a sessão em Markdown (com sugestão da IA e tempo) | CONCLUDED |
| 19 | Revisão do modo stealth (limite real no Linux) | CONCLUDED |
| 20 | Trocar os canais ouvidos a qualquer momento | CONCLUDED |

## Dependências entre tarefas

```
00 ─┬─▶ 01 ─┐
    ├─▶ 02 ─┼─▶ 04 ─▶ 05 ─▶ 06 ─▶ 07 ─▶ 08
    └─▶ 03 ─┘                   └─▶ 09
                                   └─▶ 10
04,11 ─▶ 13 ─▶ 14 ─▶ 15
05 ─▶ 16 ─▶ 17 ─▶ 18
09,10 ─▶ 19
16 ─▶ 20
```

- 00 é pré-requisito de tudo.
- 04 depende de 02 e 03 (fontes de áudio).
- 07 depende de 05 e 06 (dados e providers expostos).
- 08 depende de 07; 09 e 10 dependem de 01/07.
- 13 depende de 04/11 (ASR e streaming); 14 (BR-018) depende de 13; 15 (monitor
  atrasado) depende da 14.
- 16 (sessão contínua, BR-019) depende de 05/13/14; 17 (histórico/sugestões) e
  18 (export Markdown) dependem da 16.
- 19 (revisão do stealth) depende de 09/10 e é independente das demais.
- 20 (troca de canais na sessão) depende da 16 (conceito de sessão).

## Definição de pronto (geral)
- Código compila (`pnpm tauri build` sem erros).
- Critérios de aceite da tarefa atendidos.
- Arquivo da tarefa atualizado para `CONCLUDED`.
- `../memory.md` atualizado com o que foi feito na data.
