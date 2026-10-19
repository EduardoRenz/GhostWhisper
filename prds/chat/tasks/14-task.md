# 14 — Documentação, verificação e pacotes (v0.2)

- **Status:** PENDING
- **Depende de:** 10, 11, 12, 13, 15
- **Habilita:** —
- **Issues:** — · **BR:** BR-040, BR-041, BR-042
- **Refs:** [`../prd.md`](../prd.md) §9,
  [`../architecture/transcription-panel.md`](../architecture/transcription-panel.md),
  [`15-task.md`](15-task.md), [`../memory.md`](../memory.md)

## Objetivo

Fechar a v0.2 do PRD Chat: consolidar a documentação (PRD, arquitetura, regras,
memória), rodar a verificação completa e reconstruir os pacotes de release.

## Entregáveis

- `prds/chat/prd.md` §9 revisado com o que foi implementado (sem duplicar a
  arquitetura — BR-016).
- `prds/chat/architecture/transcription-panel.md` e as notas de revisão de
  `chat-ui.md`/`suggestions-window.md` atualizadas.
- `prds/business-rules.md`: BR-040/BR-041/BR-042 conferidas e as revisões de
  BR-019 e BR-021 apontando para elas.
- `prds/chat/memory.md`: entrada de implementação com IDs, decisões, desvios e
  resultados de teste.
- `prds/chat/tasks/_tasks.md`: tasks 10–15 em `CONCLUDED` (ou justificativa de
  cancelamento).
- Pacotes: `pnpm tauri build` (`.deb`/AppImage), se o ambiente permitir.

## Critérios de aceite

- [ ] `cargo test` / `cargo clippy` / `cargo fmt --check` verdes.
- [ ] `pnpm test` / `tsc --noEmit` / `pnpm build` verdes.
- [ ] Nenhuma referência antiga a "Você/Interlocutor" ou `max-width:75%` fica no
      **contrato vigente** do ao vivo (o histórico documental pode citar).
- [ ] Nenhum número de latência/WebRTC inventado no rodapé.
- [ ] A documentação aponta para os IDs e não duplica conteúdo (R14/BR-016).
- [ ] Os pacotes sobem e o painel abre em `?window=chat`.

## Verificação manual

- Sessão real com microfone + sistema, em X11/XWayland: layout, cores, hora,
  streaming e ação de IA.
- Abrir/fechar Sugestões com o contexto manual; gerar, copiar, fixar e limpar.
- Reabrir o app: posição/tamanho do painel preservados (BR-026) e nenhum
  contexto manual persistido indevidamente.
- Conferir o histórico de uma sessão antiga com o mesmo componente de bolha.

## Notas

- A latência/WebRTC do print **não** foi transposta: o rodapé usa o estado real
  do Whisper. Se a latência real for desejada, deve virar uma task de
  instrumentação própria.
