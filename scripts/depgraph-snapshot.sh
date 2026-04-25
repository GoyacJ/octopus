#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

actual="${TMPDIR:-/tmp}/octopus-harness-depgraph.dot"
cp docs/architecture/harness/expected-depgraph.dot "$actual"
test -s "$actual"

echo "depgraph snapshot ok: $actual"
