#!/usr/bin/env bash
# Build **final** do GhostWhisper: Vulkan + engine Laya + modelo ONNX empacotado.
#
# É o artefato "tudo incluso" para testar/distribuir: um único AppImage que usa
# GPU quando existe, cai para CPU sozinho e já traz o classificador Laya.
# O build padrão (`pnpm tauri:build`) continua leve e sem o modelo.
#
# Uso:
#   ./scripts/build-final.sh
#   pnpm build:final
set -euo pipefail

cd "$(dirname "$0")/.."

artifact="src-tauri/resources/laya/multilingual"
if [ ! -f "$artifact/model.onnx" ] || [ ! -f "$artifact/model.onnx.data" ]; then
  echo "[final] artefato Laya ausente em $artifact" >&2
  echo "[final] exporte uma vez (Python >=3.10):" >&2
  echo "  python3 vendor/laya/laya-rust/tools/export_onnx.py --model multilingual \\" >&2
  echo "    --out src-tauri/resources/laya" >&2
  exit 1
fi

# O snapshot do HF traz tokenizer.json 0444 e model.onnx.data 0600; o build
# seguinte não consegue sobrescrever o destino read-only e o AppImage monta os
# arquivos como root. Garante leitura para todos antes de empacotar.
chmod -R u+w,a+rX "$artifact"

if [ ! -f vendor/laya/laya-rust/Cargo.toml ]; then
  echo "[final] submódulo vendor/laya ausente; inicializando…" >&2
  git submodule update --init
fi

# Remove AppImages de versões antigas antes de compilar: durante o build não
# pode sobrar um artefato velho para abrir/testar por engano.
bash scripts/prune-appimages.sh

# O tauri-build copia o modelo para <target>/release/laya; uma cópia antiga
# read-only faz o build morrer com "Permission denied (os error 13)".
for dir in \
  "src-tauri/target/laya" \
  "src-tauri/target/release/laya" \
  "src-tauri/target-vulkan/laya" \
  "src-tauri/target-vulkan/release/laya"; do
  chmod -R u+w,a+rX "$dir" 2>/dev/null || true
done

bash scripts/tauri-build-laya.sh "$@"

appimage="$(ls -1t src-tauri/target/release/bundle/appimage/*.AppImage 2>/dev/null | head -n1 || true)"
if [ -n "$appimage" ] && [ -f "$appimage" ]; then
  echo
  echo "[final] AppImage pronto (Vulkan + Laya + modelo):"
  ls -lh "$appimage"
  sha256sum "$appimage"
else
  echo "[final] build terminou, mas nenhum AppImage foi encontrado em src-tauri/target/release/bundle/appimage" >&2
  exit 1
fi
