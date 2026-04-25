#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

python3 scripts/check_layer_boundaries.py
bash scripts/harness-legacy-boundary.sh

echo "dependency boundaries ok"
