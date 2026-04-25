#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

fail() {
  echo "harness legacy boundary violation: $*" >&2
  exit 1
}

harness_dirs=()
while IFS= read -r dir; do
  harness_dirs+=("$dir")
done < <(find crates -maxdepth 1 -type d -name 'octopus-harness-*' | sort)

if ((${#harness_dirs[@]} > 0)); then
  if grep -RInE 'octopus-sdk|octopus_sdk' "${harness_dirs[@]}" \
    --include='Cargo.toml' \
    --include='*.rs'; then
    fail "new harness crates must not reference legacy octopus-sdk crates"
  fi
fi

if find crates apps -path '*/target' -prune -o -type f \( -name 'Cargo.toml' -o -name '*.rs' \) -print \
  | xargs grep -nE '_octopus[-_]bridge[-_]stub|legacy-sdk'; then
  fail "bridge stubs and legacy-sdk features are forbidden"
fi

echo "harness legacy boundary ok"
