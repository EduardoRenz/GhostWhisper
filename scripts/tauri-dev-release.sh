#!/usr/bin/env bash
# Compila o binário de release completo (Vulkan + Laya) sem empacotar e roda
# direto, sem FUSE/AppImage.
#
# Motivo: no Ubuntu 26.04 o runtime do AppImage não monta via FUSE (o `fusermount3`
# novo rejeita o fd do runtime antigo). Para **iterar em release "toda hora"** sem
# depender do bundler, este script pula o empacotamento (mksquashfs de ~1,5 GB) e
# executa o binário solto, que resolve o modelo Laya ao lado do próprio binário
# (`resource_dir()` de um binário solto é o dir do exe).
#
# É o mesmo artefato do `pnpm tauri:build` (Vulkan + Laya + modelo), só que roda
# do binário direto. Usa o mesmo `app_data_dir` do AppImage/deb — você testa com
# seus dados, configs e modelos reais.
#
# Args extras vão pro app: pnpm tauri:dev:release -- --help
set -euo pipefail

cd "$(dirname "$0")/.."

# Falha cedo e explicado se as dependências de build do Tauri faltarem (o update
# do Ubuntu pode remover os `-dev`). Sem elas NENHUM build compila (glib-sys etc.).
if ! pkg-config --exists glib-2.0 2>/dev/null; then
  cat >&2 <<'EOF'
[dev-release] faltam dependências de build do Tauri (glib-2.0 não encontrado).
Instale uma vez (Tauri v2 em Ubuntu/Debian):
  sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
    libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
EOF
  exit 1
fi

# Limpa as variáveis do terminal snap do VS Code (ver tauri-dev-clean.sh).
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

# Monta o SDK Vulkan e exporta CARGO_TARGET_DIR=target-vulkan. `source`ado só
# exporta (não executa nada).
# shellcheck disable=SC1091
source scripts/vulkan-env.sh

target_dir="$CARGO_TARGET_DIR"
target_bin="${target_dir}/release/ghostwhisper"

# O artefato Laya (model.onnx + model.onnx.data) é obrigatório pra feature `laya`.
artifact="src-tauri/resources/laya/multilingual"
if [ ! -f "$artifact/model.onnx" ] || [ ! -f "$artifact/model.onnx.data" ]; then
  echo "[dev-release] artefato Laya ausente em $artifact; exporte uma vez:" >&2
  echo "  python3 vendor/laya/laya-rust/tools/export_onnx.py --model multilingual \\" >&2
  echo "    --out src-tauri/resources/laya" >&2
  exit 1
fi
# Snapshot do HF traz tokenizer 0444 e model.onnx.data 0600; o build não consegue
# sobrescrever destino read-only. Garante leitura antes de empacotar.
chmod -R u+w,a+rX "$artifact"

# Uma cópia read-only do modelo no target faz o tauri-build morrer com "Permission
# denied"; destrava antes.
chmod -R u+w,a+rX "${target_dir}/laya" 2>/dev/null || true
chmod -R u+w,a+rX "${target_dir}/release/laya" 2>/dev/null || true

# Compila o binário completo (Vulkan + Laya). O `--no-bundle` pula deb/appimage
# (onde mora o mksquashfs lento); ainda roda o `beforeBuildCommand` (tsc + vite).
# `./node_modules/.bin/tauri` (e não o script `tauri` do package.json, que
# redireciona builds para o AppImage completo) para gerar só o binário de iteração.
env "${clean[@]}" ./node_modules/.bin/tauri build --features "vulkan,laya" --no-bundle

# Garante o modelo ONNX ao lado do binário (fallback `bundled` de
# `resolve_model_dir`). Cópia só se ausente — o build seguinte é incremental e não
# repassa os 1,3 GB.
dest="${target_dir}/release/laya/multilingual"
if [ ! -f "$dest/model.onnx" ] && [ -f "$artifact/model.onnx" ]; then
  echo "[dev-release] copiando modelo Laya para $dest (só na primeira vez)…"
  mkdir -p "$dest"
  cp -rf "$artifact/." "$dest/"
fi

echo "[dev-release] rodando $target_bin (Vulkan + Laya, sem FUSE)…"
exec "$target_bin" "$@"
