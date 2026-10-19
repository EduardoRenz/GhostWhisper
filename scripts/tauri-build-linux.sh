#!/usr/bin/env bash
# Empacota o app para Linux **com Vulkan** — é o build de release padrão (BR-029).
#
#  1. monta o SDK Vulkan e copia o loader para embutir no AppImage
#     (`scripts/ensure-libvulkan.sh`, sem sudo);
#  2. compila com `--features vulkan`: um binário só, que usa GPU quando existe e
#     cai para CPU quando não existe.
#
# Uso: pnpm tauri:build [--bundles deb,appimage]
set -euo pipefail

cd "$(dirname "$0")/.."

bash scripts/ensure-libvulkan.sh

# `source` exporta VULKAN_SDK/PATH/LD_LIBRARY_PATH/RUSTFLAGS/CARGO_TARGET_DIR.
# shellcheck disable=SC1091
source scripts/vulkan-env.sh

pnpm tauri build --features vulkan "$@"

# O build de GPU compila em `target-vulkan/`; publica os artefatos no caminho
# canônico (`target/release/bundle/`) para não confundir com artefato antigo.
bash scripts/publish-bundles.sh
