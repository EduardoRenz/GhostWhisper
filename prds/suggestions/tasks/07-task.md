# 07 — Acabamento, usabilidade, validação visual, docs e pacote

- **Status:** CONCLUDED
- **Depende de:** 03, 04, 05, 06
- **Habilita:** —
- **Issues:** — · **BR:** BR-037, BR-038, BR-039

## Objetivo

Fechar o redesenho: acabamento de usabilidade, estados vazios/erro no novo
layout, validação visual, documentação sincronizada e pacote.

## Escopo

- **Usabilidade:**
  - “Contexto extra” recolhido por padrão, com ícone no cabeçalho;
  - estados vazio/erro/gerando no layout novo (sem “pulo” de layout);
  - janela mantém **largura mínima 520 px** e é redimensionável (código cabe);
  - estado vazio orienta o próximo passo (“inicie uma escuta” / “fale para
    sugerir”).
- **Validação visual:** screenshot do header, seletor, cartão (rápida + completa
  + código), histórico expandido e o caso “aguardando fala”.
- **Docs:** `prd.md` (status), `architecture/suggestions-window.md` (§7
  evidência do Perssua, uma vez), `architecture/agent-identity.md`,
  `business-rules.md` (BR-037…BR-039), `memory.md`, `_tasks.md`.
- **Verificação:** `cargo test/clippy/fmt`, `pnpm test`, `tsc --noEmit`,
  `pnpm build` e AppImage (Vulkan) com `libvulkan.so.1` embutido.

## Critérios de aceite

- [x] Estados vazio/erro/gerando no layout novo.
- [x] Checklist visual executado e registrado no `memory.md`.
- [x] BRs e docs atualizados.
- [x] Todos os testes/lints verdes.
- [x] AppImage e `.deb` gerados; Vulkan embutido.

## Como testar

Seguir o checklist visual no app empacotado, com sessão real e com o automático
pausado/ligado.

## Realizado (2026-10-19)

- **Contexto extra** virou um **ícone no cabeçalho** (recolhido por padrão); o
  `<details>` no corpo saiu.
- **Estados vazio/erro/gerando** no layout novo (bloco `.suggestion-empty` com
  ícone + orientação), sem “pulo” de layout.
- **Janela de Sugestões** com mínimo **520×420** (`tauri.conf.json`), para o
  código caber; o piso salvo em Rust continua 320×240.
- **Validação visual** por screenshot (Chrome headless, HTML renderizado com o
  CSS real): header + LIVE + contagem, seletor, cartão com os dois blocos e o
  caption do código, histórico com accordions, popover do agente aberto.
- **Evidência Perssua** extraída com o REA e registrada em
  `architecture/suggestions-window.md` §7 (uma vez, R14/BR-016).
- **Verificação:** `cargo test` 364 ✓ · `cargo clippy` ✓ · `cargo fmt --check` ✓
  · `pnpm test` 233 ✓ · `npx tsc --noEmit` ✓ · `pnpm build` ✓.
- **Pacote:** AppImage (93 MiB) e `.deb` (17 MiB) em
  `src-tauri/target/release/bundle/` (Vulkan, `libvulkan.so.1` embutido).
