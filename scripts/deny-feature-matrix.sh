#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

run_deny() {
  local label="$1"
  shift

  echo "cargo deny: ${label}"
  cargo deny --workspace "$@" check
}

run_deny "default features"
run_deny "typical desktop profile" --features "sqlite-store,local-sandbox,interactive-permission,provider-anthropic"
run_deny "all providers" --features all-providers
run_deny "all features" --all-features

echo "deny feature matrix ok"
