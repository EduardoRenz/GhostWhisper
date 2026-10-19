# 12 — Validação visual, documentação e pacote (drill-hover)

- **Status:** CONCLUDED
- **Depende de:** 11
- **Habilita:** —
- **Issues:** ISS-035 · **BR:** BR-036

## Objetivo

Fechar o drill-hover com validação manual, documentação sincronizada e pacote
(AppImage Vulkan).

## Escopo

- Checklist visual:
  - fill da esquerda para a direita nos dois estágios;
  - confirmação **acima** com espaço e **abaixo** perto do topo; sem clipping;
  - notch **não** redimensiona/move;
  - grace de 0,5 s (sair e voltar; depois expirar);
  - clique imediato no gatilho e no "Sim";
  - desligado nos Ajustes e duração 0,3/2,0 s;
  - `prefers-reduced-motion`;
  - click-through/stealth (X11 e Wayland) e foco (a confirmação não rouba foco).
- Atualizar:
  - `prd.md` (status/CA), `business-rules.md` (BR-036, se revisar);
  - `architecture/drill-hover.md`, `notch-ui.md`, `design-tokens.md`;
  - `issues.md` (ISS-035, se confirmado/ajustado);
  - `memory.md`, `_tasks.md`.
- Rodar `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check`,
  `pnpm test`, `tsc --noEmit`, `pnpm build`.
- Gerar **AppImage com Vulkan** e conferir `libvulkan.so.1` embutido.

## Critérios de aceite

- [ ] Checklist executado e registrado no `memory.md`.
- [ ] Todos os testes/lints/build verdes.
- [ ] AppImage e `.deb` gerados; Vulkan embutido.
- [ ] Tasks 09–12 e `_tasks.md` em `CONCLUDED`.

## Como testar

Seguir o checklist acima no app empacotado (ou `pnpm tauri dev`), com sessão real e
depois com o drill-hover desligado.
