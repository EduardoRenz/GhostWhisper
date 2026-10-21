# 08 — Modal "Sessão Finalizada"

- **Status:** PENDING
- **Depende de:** 06, 07
- **Habilita:** 10
- **Issues:** — · **BR:** BR-068
- **Refs:** [`../prd.md`](../prd.md) US-03 (CA-03.1…CA-03.10), mockup de
  2026-10-21; [`../architecture/recording.md`](../architecture/recording.md) §7

## Objetivo

Entregar, no fim da sessão, o resumo do que foi produzido e os caminhos para o
material — com progresso honesto quando houver finalização.

## Entregáveis

- `src-tauri/tauri.conf.json`: janela `session-end` (560×400, sem moldura,
  transparente, sempre no topo, centralizada, `visible:false`).
- `src-tauri/capabilities/default.json`: incluir `session-end`.
- `src-tauri/src/ipc/session_end_commands.rs`: `session_end_open(payload)`,
  `session_end_context`, `session_end_clear`, `session_end_close`.
- `src/main.tsx` / roteamento de janelas: `SessionEndWindow`.
- `src/features/session-end/SessionEndWindow.tsx` + `SessionEndCard.tsx`:
  - header com `WindowHeader` (BR-064): selo, "Sessão Finalizada", X;
  - cartões **Duração total** e **Mídia** (MP4 com specs / FLAC / "Sem
    gravação");
  - linha **Itens processados** (`N falas · M sugestões IA`);
  - **Abrir local** (só com arquivo) e **Baixar Sessão** (primário);
  - rodapé **Descartar gravação** (confirmação em dois toques) e **Nova Sessão**;
  - barra `Finalizando gravação… 62%` no cartão de mídia quando `finalizing`.
- `src/core/application/useSessionEnd.ts`: carrega o payload, assina
  `recording_progress`, executa as ações.
- `src/adapters/ipc/sessionEnd.ts`.

## Critérios de aceite

- [ ] Abre **somente** após `session_stop` do usuário (CA-03.8).
- [ ] Layout fiel ao mockup: dois cartões em linha, faixa de itens, dois botões
      grandes, rodapé com descartar/nova sessão.
- [ ] Sem gravação: cartão "Sem gravação", **sem** "Abrir local", **sem**
      progresso.
- [ ] Com gravação pronta: tamanho + `MP4 · 1080p · 8 fps`; "Abrir local" revela
      o arquivo (`tauri-plugin-opener`).
- [ ] Durante `finalizing`: barra com percentual, ações desabilitadas; ao
      `ready`, reabilita.
- [ ] "Baixar Sessão" usa `pickMarkdownPath` + `saveSessionMarkdown` (mesmo
      fluxo do Histórico) e mostra o caminho salvo.
- [ ] "Descartar gravação" pede confirmação, apaga arquivo + segmentos e muda o
      cartão para "Sem gravação"; a sessão e a transcrição permanecem.
- [ ] "Nova Sessão" inicia a sessão com os mesmos canais e fecha o modal.
- [ ] `Esc` e X fecham sem apagar nada.
- [ ] `pnpm test` cobre os cenários (com/sem gravação, progresso, descartar).

## Verificação

- `pnpm test` · `pnpm build` · `cargo clippy` · teste manual de fim de sessão

## Notas

- A janela usa o padrão `capture_pending`/`capture_clear` (task 06).
- O modal **não** é modal no sentido de bloquear o app: é sempre-no-topo, mas o
  usuário pode continuar usando o GhostWhisper.
