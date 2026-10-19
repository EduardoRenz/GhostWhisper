#!/usr/bin/env bash
# Empacotamento Linux do GhostWhisper (.deb e AppImage).
#
# Uso:
#   scripts/package-linux.sh                 # deb + appimage
#   scripts/package-linux.sh deb             # apenas .deb (não baixa AppImage tools)
#
# Requisitos de build: pnpm, Rust, cmake, libwebkit2gtk-4.1-dev, libgtk-3-dev,
# libasound2-dev, libpulse-dev. O empacotamento AppImage baixa `linuxdeploy` e o
# runtime do AppImage na primeira execução (precisa de rede).
set -euo pipefail

cd "$(dirname "$0")/.."

targets="${1:-deb,appimage}"

echo "[package-linux] alvo(s): ${targets}"

if [ ! -d node_modules ]; then
  echo "[package-linux] instalando dependências do frontend..."
  pnpm install
fi

echo "[package-linux] build (release) + bundle..."
bash scripts/tauri-build-linux.sh --bundles "${targets}"

echo "[package-linux] artefatos:"
find src-tauri/target/release/bundle -maxdepth 3 -type f \
  \( -name '*.deb' -o -name '*.AppImage' \) -print -exec ls -lh {} \;
