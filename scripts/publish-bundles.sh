#!/usr/bin/env bash
# Publica os artefatos do bundle no caminho canônico do projeto.
#
# O build de release com Vulkan usa `CARGO_TARGET_DIR=src-tauri/target-vulkan`
# (para não invalidar o cache do build de CPU). Sem isso, o AppImage/.deb ficam
# em `target-vulkan/release/bundle/` e quem procura em `target/release/bundle/`
# encontra um artefato antigo. Aqui copiamos para o caminho de sempre.
set -euo pipefail

cd "$(dirname "$0")/.."

source_dir="${CARGO_TARGET_DIR:-src-tauri/target}/release/bundle"
canonical="src-tauri/target/release/bundle"

if [ ! -d "$source_dir" ]; then
  echo "[publish-bundles] nada em $source_dir" >&2
  exit 0
fi

if [ "$(readlink -f "$source_dir")" = "$(readlink -f "$canonical")" ]; then
  echo "[publish-bundles] artefatos já estão em $canonical"
  bash scripts/prune-appimages.sh
  exit 0
fi

mkdir -p "$canonical"
cp -rf "$source_dir/." "$canonical/"
# Deixa só o AppImage da versão atual: abrir/testar o antigo por engano é risco.
bash scripts/prune-appimages.sh
echo "[publish-bundles] artefatos publicados em $canonical:"
find "$canonical" -maxdepth 3 -type f \( -name '*.deb' -o -name '*.AppImage' \) \
  -print -exec ls -lh {} \;
