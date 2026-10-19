#!/usr/bin/env bash
# Prepara o SDK Vulkan para buildar o Whisper com aceleração por GPU (BR-029).
#
# O `whisper-rs-sys` (feature `vulkan`) precisa de:
#   - headers do Vulkan (`vulkan/vulkan.h`);
#   - `glslc` (shaderc) para compilar os shaders do backend Vulkan do ggml;
#   - `VULKAN_SDK` apontando para um diretório com `include/` e `lib/libvulkan.so`.
#
# Em Ubuntu/Debian, `sudo apt install libvulkan-dev glslc` resolve. **Sem sudo**,
# este script baixa esses `.deb` e monta o layout localmente (não toca no sistema).
#
# Uso:
#   source scripts/vulkan-env.sh            # exporta as variáveis na shell atual
#   scripts/vulkan-env.sh <cmd> [args…]     # roda o comando com o ambiente montado
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
sdk="${GHOSTWHISPER_VULKAN_SDK:-$repo_root/.vulkan-sdk}"
multiarch="$(dpkg-architecture -qDEB_HOST_MULTIARCH 2>/dev/null ||
  gcc -print-multiarch 2>/dev/null || echo x86_64-linux-gnu)"

# Baixa os .deb (sem root) e extrai dentro do próprio SDK.
stage_debs() {
  local cache="$sdk/.debs"
  mkdir -p "$cache"
  if ! ls "$cache"/*.deb >/dev/null 2>&1; then
    echo "[vulkan-env] baixando libvulkan-dev/glslc/libshaderc1 (sem sudo)…" >&2
    ( cd "$cache" && apt-get download libvulkan-dev glslc libshaderc1 )
  fi
  for deb in "$cache"/*.deb; do dpkg -x "$deb" "$sdk"; done
}

prepare() {
  mkdir -p "$sdk/include" "$sdk/lib" "$sdk/bin"

  # 1) Preferência: o que já existe no sistema. `vk_video/` é obrigatório: o
  #    `vulkan_core.h` inclui esses headers (Vulkan-Headers dividiu em subdir).
  if [ -f /usr/include/vulkan/vulkan.h ]; then
    cp -rn /usr/include/vulkan "$sdk/include/" 2>/dev/null || true
    cp -rn /usr/include/vk_video "$sdk/include/" 2>/dev/null || true
  fi
  if command -v glslc >/dev/null 2>&1; then
    ln -sf "$(command -v glslc)" "$sdk/bin/glslc"
  fi
  if [ -e "/usr/lib/$multiarch/libvulkan.so" ]; then
    ln -sf "/usr/lib/$multiarch/libvulkan.so" "$sdk/lib/libvulkan.so"
  fi

  # 2) Sem sistema: extrai os .deb.
  if [ ! -f "$sdk/include/vulkan/vulkan.h" ] ||
    [ ! -x "$sdk/bin/glslc" ] ||
    [ ! -e "$sdk/lib/libvulkan.so" ]; then
    stage_debs
    ln -sf "$sdk/usr/bin/glslc" "$sdk/bin/glslc"
    ln -sf "$sdk/usr/lib/$multiarch/libshaderc.so.1" "$sdk/lib/libshaderc.so.1" 2>/dev/null || true
  fi

  # 2b) Headers sempre completos (`vulkan/` + `vk_video/`), de onde vierem.
  if [ ! -f "$sdk/include/vulkan/vulkan.h" ] && [ -d "$sdk/usr/include/vulkan" ]; then
    cp -rn "$sdk/usr/include/vulkan" "$sdk/include/" 2>/dev/null || true
  fi
  if [ ! -d "$sdk/include/vk_video" ] && [ -d "$sdk/usr/include/vk_video" ]; then
    cp -rn "$sdk/usr/include/vk_video" "$sdk/include/" 2>/dev/null || true
  fi

  # 3) Aponta `lib/libvulkan.so` para um arquivo REAL (o pacote `-dev` traz um
  #    symlink para o objeto versionado, que vem do pacote de runtime do sistema).
  local candidate resolved=""
  for candidate in \
    "/usr/lib/$multiarch/libvulkan.so.1" \
    "/usr/lib/$multiarch/libvulkan.so" \
    "$sdk/usr/lib/$multiarch/libvulkan.so.1" \
    "$sdk/usr/lib/$multiarch/libvulkan.so"; do
    if [ -e "$candidate" ]; then
      resolved="$(readlink -f "$candidate")"
      break
    fi
  done
  if [ -n "$resolved" ]; then
    ln -sf "$resolved" "$sdk/lib/libvulkan.so"
  fi
}

prepare

export VULKAN_SDK="$sdk"
export PATH="$sdk/bin:$PATH"
export LD_LIBRARY_PATH="$sdk/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

# O `whisper-rs-sys` emite `rustc-link-lib=vulkan` mas não o `-L` do SDK (0.15);
# damos o caminho explícito para o linker achar `libvulkan.so`.
export RUSTFLAGS="-L $sdk/lib ${RUSTFLAGS:-}"

# Build de GPU em um diretório `target` próprio: `RUSTFLAGS` diferente invalidaria
# o cache do build de CPU a cada alternância entre `tauri dev` e `tauri dev:vulkan`.
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$repo_root/src-tauri/target-vulkan}"

# O `bindgen` (via libclang) precisa achar os headers internos do compilador
# (`stdbool.h` etc.); sem isso ele só emite um aviso, mas é melhor evitar.
gcc_include="$(gcc -print-file-name=include 2>/dev/null || true)"
if [ -n "$gcc_include" ] && [ -d "$gcc_include" ]; then
  export BINDGEN_EXTRA_CLANG_ARGS="-I$gcc_include ${BINDGEN_EXTRA_CLANG_ARGS:-}"
fi

# Executado com comando e argumentos ⇒ roda com o ambiente montado.
# `source`ado ⇒ só exporta (os `$@` são do chamador, não nossos).
if [ "${BASH_SOURCE[0]}" = "$0" ] && [ "$#" -gt 0 ]; then
  exec "$@"
fi

echo "VULKAN_SDK=$VULKAN_SDK"
echo "glslc=$(command -v glslc || echo '?')"
echo "libvulkan=$([ -e "$sdk/lib/libvulkan.so" ] && echo ok || echo '?')"
