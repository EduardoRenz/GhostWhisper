# 06 — Documentação, verificação e pacotes

- **Status:** CONCLUDED
- **Depende de:** 05
- **Habilita:** —
- **Issues:** — · **BR:** BR-032, BR-033, BR-034

## Objetivo

Fechar a PRD: registrar as regras e o histórico, consolidar a verificação completa
e reconstruir os pacotes.

## Entregáveis

- `prds/business-rules.md` — confirmar/aplicar **BR-032** (notch em dois estados,
  por ícones, verde + atalho), **BR-033** (painel em abas; `summary` aposentado;
  resumo auto só na aba ativa) e **BR-034** (idioma em tempo real), e as **revisões**
  de **BR-021/BR-023/BR-026**.
- `prds/notch/memory.md` — entradas `## YYYY-MM-DD — …` com `IDs:`/`Refs:` das
  tasks 01–06 (BR-017).
- `prds/notch/issues.md` — `ISS-033`/`ISS-034` → `FIXED` (ou justificar).
- Atualizar `prds/chat/architecture/chat-ui.md` §7 e `prds/assistant/*`
  (referência cruzada, sem duplicar — BR-016): a janela `chat` virou painel em abas
  e a `summary` foi aposentada.
- `README.md` — estados do overlay e idioma em tempo real, se citados.
- Reconstruir pacotes: `scripts/package-linux.sh deb,appimage`.

## Critérios de aceite

- [ ] `business-rules.md` com BR-032…BR-034 e as revisões de BR-021/023/026
      (escrita única — R14).
- [ ] `memory.md` com as entradas e IDs; `issues.md` atualizado.
- [ ] Referências cruzadas em `chat`/`assistant` apontando para esta PRD.
- [ ] Verificação completa verde: `cargo test` · `cargo clippy` · `cargo fmt --check`
      · `pnpm test` · `tsc --noEmit` · `pnpm build`.
- [ ] `.deb` e AppImage reconstruídos e instaláveis (BR-006/BR-007 documentados).

## Como testar

1. Rodar a bateria completa (Rust + frontend) e colar os totais no `memory.md`.
2. `scripts/package-linux.sh deb,appimage` e conferir os tamanhos.
3. Smoke run do pacote: notch em repouso, iniciar, abas, idioma em tempo real.

Ponto de parada: PRD `CONCLUDED` e pacotes gerados.

## Riscos / rollback

- Novos artefatos grandes no disco → limpar `target/` antes de empacotar se
  necessário (histórico: disco apertado).
- Divergência de doc → `memory.md` é a fonte do "o que foi feito"; BRs a regra.

## Verificação

- Automática: bateria completa (Rust + frontend) + build dos pacotes.
- Manual: smoke run do pacote.
- Observações do usuário: (aguardando)

## Resultado (2026-10-18)

- **Regras:** `business-rules.md` com **BR-032** (notch em dois estados por ícones,
  verde + atalho), **BR-033** (painel em abas; `summary` aposentado; resumo auto só
  na aba ativa) e **BR-034** (idioma em tempo real), mais as **revisões** de
  BR-021/BR-023/BR-026 (referenciando esta PRD).
- **Histórico:** [`memory.md`](memory.md) com a entrada de implementação (T01–T06,
  notas de conflito com o PRD chat).
- **Issues:** `ISS-033`/`ISS-034` → `FIXED`.
- **Referências cruzadas:** `chat/architecture/chat-ui.md` §7.1 e
  `assistant/architecture/assistant-windows.md` §9 (janela `chat` = painel em abas;
  `summary` aposentada); `README.md` reescrito.
- **Verificação completa:**
  - Rust: `cargo test` **334 ✓** (8 `ignored`) · `cargo clippy --all-targets` e
    `cargo fmt --check` limpos.
  - Frontend: `pnpm test` **155 ✓** · `tsc --noEmit` ✓ · `pnpm build` ✓.
- **Pacotes (Vulkan):** `pnpm tauri:build` (features `vulkan`) gerou
  `GhostWhisper_0.1.0_amd64.deb` (**16,32 MiB**) e
  `GhostWhisper_0.1.0_amd64.AppImage` (**91,91 MiB**, reconstruído em 2026-10-19
  após os ajustes de UX), com o loader `usr/lib/libvulkan.so.1` **embutido** no
  AppImage (verificado por extração).
- **Pendente:** teste manual do usuário (notch repouso/ativo, abas, idioma em tempo
  real com áudio).
