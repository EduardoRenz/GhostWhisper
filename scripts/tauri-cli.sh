#!/usr/bin/env bash
# Encaminha o CLI do Tauri, mas `build` é SEMPRE o build completo (Vulkan + Laya
# + modelo ONNX). É o único caminho de release; não existe build parcial.
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

# `--help` continua sendo ajuda; qualquer outro `build` é o bundle completo.
if [ "${1:-}" = "build" ] && [ "${2:-}" != "--help" ] && [ "${2:-}" != "-h" ]; then
  exec bash scripts/tauri-build-linux.sh
fi

exec "$repo_root/node_modules/.bin/tauri" "$@"
