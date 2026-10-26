#!/usr/bin/env bash
# AppImage com Vulkan + Laya (engine ONNX compilado e modelo empacotado).
#
# Requisitos:
#   - submódulo `vendor/laya` inicializado;
#   - artefato real em `src-tauri/resources/laya/multilingual/`
#     (model.onnx + model.onnx.data ~1,3 GB + tokenizer + rl_agent_config.json).
#
# O artefato NÃO vai para o git (ver .gitignore). Exporte uma vez:
#   python3 vendor/laya/laya-rust/tools/export_onnx.py --model multilingual \
#     --out src-tauri/resources/laya
#
# Uso: pnpm tauri:build:laya [--bundles appimage]
set -euo pipefail

cd "$(dirname "$0")/.."

artifact="src-tauri/resources/laya/multilingual"
if [ ! -f "$artifact/model.onnx" ] || [ ! -f "$artifact/model.onnx.data" ]; then
  echo "[laya] artefato ausente em $artifact" >&2
  echo "[laya] exporte uma vez (Python >=3.10):" >&2
  echo "  python3 vendor/laya/laya-rust/tools/export_onnx.py --model multilingual --out src-tauri/resources/laya" >&2
  exit 1
fi

data_size="$(stat -c%s "$artifact/model.onnx.data")"
if [ "$data_size" -lt 100000000 ]; then
  echo "[laya] model.onnx.data tem só $data_size bytes; parece o artefato falso de dev." >&2
  echo "[laya] use o export real antes de empacotar." >&2
  exit 1
fi

# O snapshot do HF vem com tokenizer.json 0444 e model.onnx.data 0600. O
# `tauri-build` copia os resources para o target e, no build seguinte, não
# consegue sobrescrever um destino read-only; além disso, o AppImage monta os
# arquivos como root e um `0600` fica ilegível para o usuário. Garanta leitura
# para todos e escrita na origem/destino.
chmod -R u+w,a+rX "$artifact"

if [ ! -f vendor/laya/laya-rust/Cargo.toml ]; then
  echo "[laya] submódulo ausente; rodando git submodule update --init" >&2
  git submodule update --init
fi

bash scripts/ensure-libvulkan.sh

# shellcheck disable=SC1091
source scripts/vulkan-env.sh

chmod -R u+w,a+rX "${CARGO_TARGET_DIR:-src-tauri/target}/laya" 2>/dev/null || true

pnpm tauri build --features "vulkan,laya" --bundles appimage "$@"

bash scripts/publish-bundles.sh
