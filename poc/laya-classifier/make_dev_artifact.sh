#!/usr/bin/env bash
# Artefato FALSO para testar o model manager sem o ONNX real (1,2 GB).
#
# O motor Laya ainda não está no build (ISS-059); este diretório serve só para
# exercitar catálogo → download → SHA-256 → `.part` → ativar → remover e a
# barra de progresso da UI. O `model.onnx` NÃO é um modelo.
#
# Uso:
#   ./poc/laya-classifier/make_dev_artifact.sh [diretório]
#   # depois: Configurações → Fontes de IA & Modelos → Avançado → URL do
#   # catálogo = <diretório>/catalog.json
set -euo pipefail

out="${1:-${TMPDIR:-/tmp}/ghostwhisper-laya-dev}"
version="0.1.0"
model_id="laya-multilingual"
root="$out/${model_id}-${version}"

rm -rf "$root"
mkdir -p "$root/tokenizer"

printf 'DEV ONLY: not a real ONNX model.\n' > "$root/model.onnx"
printf 'DEV ONLY: no weights.\n' > "$root/model.onnx.data"
printf '{"dev":true}\n' > "$root/rl_agent_config.json"
printf '{}\n' > "$root/tokenizer/tokenizer.json"
printf '{"dev":true}\n' > "$root/tokenizer/tokenizer_config.json"

sha() { sha256sum "$1" | cut -d' ' -f1; }
size() { stat -c%s "$1"; }

total=0
for file in model.onnx model.onnx.data rl_agent_config.json \
  tokenizer/tokenizer.json tokenizer/tokenizer_config.json; do
  total=$((total + $(size "$root/$file")))
done

cat > "$out/catalog.json" <<JSON
{
  "schema": 1,
  "models": [
    {
      "schema": 1,
      "model_id": "${model_id}",
      "version": "${version}",
      "checkpoint": "multilingual",
      "layout": "onnx",
      "label": "Laya Multilingual (DEV)",
      "size_bytes": ${total},
      "license": "Apache-2.0",
      "files": [
        {
          "path": "model.onnx",
          "url": "${root}/model.onnx",
          "size_bytes": $(size "$root/model.onnx"),
          "sha256": "$(sha "$root/model.onnx")"
        },
        {
          "path": "model.onnx.data",
          "url": "${root}/model.onnx.data",
          "size_bytes": $(size "$root/model.onnx.data"),
          "sha256": "$(sha "$root/model.onnx.data")"
        },
        {
          "path": "rl_agent_config.json",
          "url": "${root}/rl_agent_config.json",
          "size_bytes": $(size "$root/rl_agent_config.json"),
          "sha256": "$(sha "$root/rl_agent_config.json")"
        },
        {
          "path": "tokenizer/tokenizer.json",
          "url": "${root}/tokenizer/tokenizer.json",
          "size_bytes": $(size "$root/tokenizer/tokenizer.json"),
          "sha256": "$(sha "$root/tokenizer/tokenizer.json")"
        },
        {
          "path": "tokenizer/tokenizer_config.json",
          "url": "${root}/tokenizer/tokenizer_config.json",
          "size_bytes": $(size "$root/tokenizer/tokenizer_config.json"),
          "sha256": "$(sha "$root/tokenizer/tokenizer_config.json")"
        }
      ],
      "calibration": { "threshold": 0.7, "ece": 0.31, "notes": "DEV" },
      "metrics": { "accuracy": 0.82, "false_positive_rate": 0.11, "p50_ms": 120, "p95_ms": 380 },
      "created_at": "2026-10-26"
    }
  ]
}
JSON

echo "catalog: $out/catalog.json"
echo
echo "No app: Configurações → Fontes de IA & Modelos → Avançado →"
echo "        URL do catálogo = $out/catalog.json"
echo "Depois: card do modelo → Baixar e ativar."
echo
echo "AVISO: artefato falso; o engine Laya ainda não está no build (ISS-059)."
