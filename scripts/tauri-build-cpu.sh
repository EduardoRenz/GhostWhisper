#!/usr/bin/env bash
# Empacota o app para Linux **CPU-only** (escape hatch).
#
# O AppImage normal já é Vulkan-enabled e cai para CPU sozinho; este alvo existe
# para máquinas muito antigas/sem loader Vulkan. O loader continua sendo embutido
# (o binário não o usa, mas o bundler exige o arquivo mapeado no `tauri.conf.json`).
#
# Uso: pnpm tauri:build:cpu [--bundles deb,appimage]
set -euo pipefail

cd "$(dirname "$0")/.."

# Só a versão atual fica disponível para teste.
bash scripts/prune-appimages.sh

bash scripts/ensure-libvulkan.sh

pnpm tauri build "$@"
bash scripts/publish-bundles.sh
