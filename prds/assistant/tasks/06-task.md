# 06 — Export/histórico, documentação, verificação e pacotes

- **Status:** CONCLUDED
- **Depende de:** 05
- **Habilita:** —

## Objetivo

Fechar a PRD: integrar o resumo ao export/histórico, registrar regras e validar
o produto completo.

## Entregáveis

- `application/markdown_export.rs`: seção `## Resumo` com `sessions.overview`
  antes da linha temporal (teste).
- Histórico: resumo visível no detalhe da sessão (colapsável ou no topo).
- `prds/business-rules.md`: BR-023 (resumo vivo em janela separada), BR-024
  (agentes/modos de sugestão), BR-025 (Markdown renderizado), BR-026 (janelas
  separadas e limites de stealth).
- `README.md`: seção de uso (abrir janelas, agentes, automático, fixar, custo).
- Ponteiros em `mvp/architecture/overlay.md` e
  `chat/architecture/chat-ui.md` para a nova PRD (sem duplicar — R14).
- Tasks `CONCLUDED`, `_tasks.md` e `memory.md` atualizados.
- Verificação: `cargo test`, `cargo clippy`, `cargo fmt --check`, `pnpm test`,
  `tsc --noEmit`, `pnpm build`, `pnpm tauri build`.
- Checklist manual das duas janelas, LeetCode, fixar/export e sem chave.

## Critérios de aceite

- [x] Export contém o resumo e as sugestões fixadas com tempo.
- [x] Documentação e IDs consistentes; nenhuma duplicação.
- [x] Todos os comandos de verificação limpos.
- [x] Pacotes `.deb`/AppImage gerados.
- [x] `memory.md` com IDs/Refs e pendências.

## Verificação (executada)

- `cargo test --lib`: **280** testes (8 `ignored`), 0 falhas.
- `cargo clippy --all-targets` e `cargo fmt --check`: limpos.
- `cargo build` e `pnpm tauri build`: ok.
- `pnpm test`: **104** testes; `tsc --noEmit` e `pnpm build`: limpos.
- Pacotes: `GhostWhisper_0.1.0_amd64.deb` (8,65 MiB) e AppImage (86,44 MiB).
- Persistência: tabela `meeting_summaries` criada de forma idempotente no boot;
  nada de migração manual.
