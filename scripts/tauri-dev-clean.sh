#!/usr/bin/env bash
# Roda o app em desenvolvimento com um ambiente limpo.
#
# Por que existe: o terminal integrado do VS Code instalado por **snap** exporta
# variáveis apontando para `/snap/code` e `/snap/core20` (GTK_PATH, GIO_MODULE_DIR,
# LOCPATH, XDG_DATA_DIRS, ...). O binário do Tauri carrega os módulos GTK desses
# diretórios e acaba puxando o `libpthread.so.0` do core20, que não casa com a
# glibc do host:
#
#   symbol lookup error: /snap/core20/.../libpthread.so.0:
#   undefined symbol: __libc_pthread_init, version GLIBC_PRIVATE
#
# Limpar essas variáveis antes de subir o `pnpm tauri dev` resolve. O app em si
# não é afetado — rodar de um terminal nativo (GNOME Terminal, etc.) também basta.
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

exec env "${clean[@]}" pnpm tauri dev "$@"
