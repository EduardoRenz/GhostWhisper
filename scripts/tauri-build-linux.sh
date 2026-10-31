#!/usr/bin/env bash
# Build **único** de release do GhostWhisper no Linux.
#
# SEMPRE gera o AppImage completo: Vulkan + engine Laya + modelo ONNX
# empacotado. Não existe build leve/CPU de release; o binário escolhe a GPU em
# runtime e cai para CPU sozinho quando não há Vulkan.
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
# Uso: pnpm tauri:build
#      ./scripts/tauri-build-linux.sh
set -euo pipefail

cd "$(dirname "$0")/.."

artifact="src-tauri/resources/laya/multilingual"
if [ ! -f "$artifact/model.onnx" ] || [ ! -f "$artifact/model.onnx.data" ]; then
  echo "[build] artefato Laya ausente em $artifact" >&2
  echo "[build] exporte uma vez (Python >=3.10):" >&2
  echo "  python3 vendor/laya/laya-rust/tools/export_onnx.py --model multilingual \\" >&2
  echo "    --out src-tauri/resources/laya" >&2
  exit 1
fi

data_size="$(stat -c%s "$artifact/model.onnx.data")"
if [ "$data_size" -lt 100000000 ]; then
  echo "[build] model.onnx.data tem só $data_size bytes; parece o artefato falso de dev." >&2
  echo "[build] use o export real antes de empacotar." >&2
  exit 1
fi

# Snapshot do HF traz tokenizer/model read-only; o build seguinte não consegue
# sobrescrever o destino e o AppImage monta os arquivos como root. Garante
# leitura para todos e escrita na origem.
chmod -R u+w,a+rX "$artifact"

if [ ! -f vendor/laya/laya-rust/Cargo.toml ]; then
  echo "[build] submódulo ausente; rodando git submodule update --init" >&2
  git submodule update --init
fi

# Só a versão atual fica disponível para teste.
bash scripts/prune-appimages.sh

bash scripts/ensure-libvulkan.sh

# `source` exporta VULKAN_SDK/PATH/LD_LIBRARY_PATH/RUSTFLAGS/CARGO_TARGET_DIR.
# shellcheck disable=SC1091
source scripts/vulkan-env.sh

# O tauri-build copia o modelo para <target>/release/laya; uma cópia antiga
# read-only faz o build morrer com "Permission denied".
for dir in \
  "${CARGO_TARGET_DIR}/laya" \
  "${CARGO_TARGET_DIR}/release/laya"; do
  chmod -R u+w,a+rX "$dir" 2>/dev/null || true
done

# Binário direto (e não o script `tauri` do package.json) para não reentrar no wrapper.
./node_modules/.bin/tauri build --features "vulkan,laya" --bundles appimage

bash scripts/publish-bundles.sh

appimage="$(ls -1t src-tauri/target/release/bundle/appimage/*.AppImage 2>/dev/null | head -n1 || true)"
if [ -n "$appimage" ] && [ -f "$appimage" ]; then
  echo
  echo "[build] AppImage pronto (Vulkan + Laya + modelo):"
  ls -lh "$appimage"
  sha256sum "$appimage"
else
  echo "[build] build terminou, mas nenhum AppImage foi encontrado em src-tauri/target/release/bundle/appimage" >&2
  exit 1
fi
