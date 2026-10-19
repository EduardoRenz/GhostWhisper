#!/usr/bin/env bash
# Roda o app em desenvolvimento **com aceleração Vulkan** (BR-029).
#
# Junta os dois ajustes de ambiente que o projeto precisa:
#  1. limpa as variáveis do terminal snap do VS Code (ver tauri-dev-clean.sh);
#  2. monta o SDK Vulkan (ver vulkan-env.sh) e compila com `--features vulkan`.
#
# Uso: pnpm tauri:dev:vulkan  (ou `bash scripts/tauri-dev-vulkan.sh -- <args>`)
set -euo pipefail

cd "$(dirname "$0")/.."

snap_vars=(
  GTK_PATH GTK_EXE_PREFIX GTK_MODULES GTK_IM_MODULE_FILE
  GIO_MODULE_DIR GIO_EXTRA_MODULES
  LOCPATH GSETTINGS_SCHEMA_DIR
  XDG_DATA_DIRS LD_LIBRARY_PATH LD_PRELOAD
)
clean=()
for var in "${snap_vars[@]}"; do
  clean+=("-u" "$var")
done

exec env "${clean[@]}" bash scripts/vulkan-env.sh pnpm tauri dev --features vulkan "$@"
