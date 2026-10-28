#!/usr/bin/env bash
# Remove AppImages de versões antigas.
#
# Mantém apenas a versão atual dos manifestos (`tauri.conf.json`) nos diretórios
# de bundle. Evita abrir/testar por engano um AppImage de uma release anterior.
#
# Uso: ./scripts/prune-appimages.sh
set -euo pipefail

cd "$(dirname "$0")/.."

version="$(node -p "require('./src-tauri/tauri.conf.json').version")"
keep="GhostWhisper_${version}_amd64.AppImage"

for dir in \
  "src-tauri/target/release/bundle/appimage" \
  "src-tauri/target-vulkan/release/bundle/appimage"; do
  [ -d "$dir" ] || continue
  find "$dir" -maxdepth 1 -type f -name 'GhostWhisper_*_amd64.AppImage' \
    ! -name "$keep" -print -delete
done
