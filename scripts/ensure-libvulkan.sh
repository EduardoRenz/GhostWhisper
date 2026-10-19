#!/usr/bin/env bash
# Garante `src-tauri/resources/libvulkan.so.1` para o bundler embutir no AppImage.
#
# O `tauri.conf.json` mapeia esse arquivo para `usr/lib/libvulkan.so.1` dentro do
# AppImage (BR-029): assim o app abre mesmo numa máquina sem o loader do sistema,
# e continua usando os drivers/ICDs que existirem lá.
#
# Monta o SDK se necessário (`scripts/vulkan-env.sh`, sem sudo) e copia o loader.
set -euo pipefail

cd "$(dirname "$0")/.."

# shellcheck disable=SC1091
source scripts/vulkan-env.sh

loader="$(readlink -f "$VULKAN_SDK/lib/libvulkan.so")"
if [ ! -e "$loader" ]; then
  echo "[ensure-libvulkan] loader Vulkan não encontrado em $VULKAN_SDK/lib" >&2
  exit 1
fi

mkdir -p src-tauri/resources
cp -f "$loader" src-tauri/resources/libvulkan.so.1
echo "[ensure-libvulkan] embutindo $(basename "$loader")"
