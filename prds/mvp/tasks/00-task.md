# 00 — Bootstrap do projeto

- **Status:** CONCLUDED
- **Depende de:** —
- **Habilita:** 01, 02, 03

## Objetivo
Criar o esqueleto do projeto **Tauri v2 + React/Vite/TS** com a estrutura de pastas
do [architecture.md](../architecture/architecture.md), buildando e abrindo uma
janela vazia.

## Entregáveis (arquivos novos)
- `package.json`, `pnpm-lock.yaml`, `vite.config.ts`, `tsconfig.json`, `index.html`
- `src/main.tsx`, `src/App.tsx`, `src/App.test.tsx`, `src/styles/globals.css`
- `src/core/{domain,ports,application}/` e `src/adapters/ipc/` (esqueleto)
- `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `src-tauri/build.rs`
- `src-tauri/capabilities/default.json`
- `src-tauri/src/{main.rs,lib.rs,error.rs,state.rs}` (`lib.rs` = composition root)
- `src-tauri/src/{domain,application,ports,adapters,ipc}/mod.rs` (esqueleto)
- `src-tauri/src/ipc/ping.rs` (comando smoke + teste)
- `assets/icons/icon-source.png` e `src-tauri/icons/` (ícones gerados)
- `scripts/package-linux.sh`, `.gitignore`

## Critérios de aceite
- [x] `pnpm install` sem erros.
- [x] `pnpm tauri dev`/binário abre a janela.
- [x] `cargo test` passa.
- [x] Estrutura de pastas do MVP criada.

## Evidências
- `pnpm install` → ok; `pnpm test` → 1 teste (vitest) passou;
  `pnpm build` (tsc + vite) → ok.
- `cargo test` → **3 testes** passaram (`error`, `state`, `ipc::ping`).
- `cargo clippy --all-targets` → sem warnings.
- `cargo fmt --all --check` → limpo.
- `cargo build` → binário `target/debug/ghostwhisper` gerado.
- Smoke run: `timeout 8 ./target/debug/ghostwhisper` → `exit=124` (inicializou e
  permaneceu ativo até o timeout, sem erros). Display: X11 (`DISPLAY=:0`).

## Correções durante a implementação
1. `#[tauri::command]` gera o helper `__cmd__ping` no módulo da função; o
   `generate_handler!` passou a usar `ipc::ping::ping` (módulo `public`).
2. `tauri::generate_context!` exige `src-tauri/icons/icon.png`; gerado a partir de
   `assets/icons/icon-source.png` com `pnpm tauri icon`.

## Ambiente (pré-requisitos)
Ubuntu 24.04 x64. Instalado para buildar/rodar:
`libwebkit2gtk-4.1-dev libxdo-dev libssl-dev libayatana-appindicator3-dev
librsvg2-dev libasound2-dev libpulse-dev build-essential`. Toolchain Rust via
rustup (cargo/rustc 1.99.1, rustfmt, clippy). Verificado por `pnpm tauri info`
(`webkit2gtk-4.1` e `rsvg2` OK).

## Riscos
- Versões de `webkit2gtk`/deps por distro → documentado no README.
