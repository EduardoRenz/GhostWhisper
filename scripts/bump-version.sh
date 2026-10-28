#!/usr/bin/env bash
# Atualiza a versão do app nos manifestos e nos exemplos do README.
#
# Uso:
#   ./scripts/bump-version.sh 0.2.0
#
# Fontes de verdade: package.json, src-tauri/tauri.conf.json e
# src-tauri/Cargo.toml. O Cargo.lock é atualizado pelo `cargo check`.
set -euo pipefail

cd "$(dirname "$0")/.."

version="${1:?uso: $0 <versao> (ex.: 0.2.0)}"
if ! [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "versão inválida: $version (use X.Y.Z)" >&2
  exit 1
fi

node - "$version" <<'NODE'
const fs = require("fs");
const version = process.argv[2];
for (const file of ["package.json", "src-tauri/tauri.conf.json"]) {
  const data = JSON.parse(fs.readFileSync(file, "utf8"));
  data.version = version;
  fs.writeFileSync(file, JSON.stringify(data, null, 2) + "\n");
}
NODE

python3 - "$version" <<'PY'
import re
import sys
from pathlib import Path

version = sys.argv[1]
path = Path("src-tauri/Cargo.toml")
text = path.read_text()
text, count = re.subn(
    r'(?m)^version = "[^"]+"$',
    f'version = "{version}"',
    text,
    count=1,
)
if count != 1:
    raise SystemExit("não encontrei a versão em src-tauri/Cargo.toml")
path.write_text(text)

readme = Path("README.md")
text = readme.read_text()
text = re.sub(
    r"GhostWhisper_[0-9]+\.[0-9]+\.[0-9]+_amd64",
    f"GhostWhisper_{version}_amd64",
    text,
)
readme.write_text(text)
PY

# Atualiza o Cargo.lock (versão do pacote local) e valida o manifesto.
(cd src-tauri && cargo check --lib >/dev/null)

echo "versão atualizada para $version:"
grep -n '"version"' package.json src-tauri/tauri.conf.json
grep -n '^version' src-tauri/Cargo.toml | head -n 1
