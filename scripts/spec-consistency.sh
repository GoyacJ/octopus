#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

fail() {
  echo "spec consistency violation: $*" >&2
  exit 1
}

mapfile_fallback() {
  while IFS= read -r line; do
    printf '%s\0' "$line"
  done
}

crate_count="$(find crates -maxdepth 1 -type d -name 'octopus-harness-*' | wc -l | tr -d ' ')"
[[ "$crate_count" == "19" ]] || fail "expected 19 octopus-harness-* crates, found $crate_count"

while IFS= read -r lib; do
  grep -q 'SPEC: docs/architecture/harness/crates/harness-' "$lib" \
    || fail "$lib is missing SPEC path"
done < <(find crates -path '*/octopus-harness-*/src/lib.rs' -type f | sort)

if grep -RInE 'std::sync::(Mutex|RwLock)' crates/octopus-harness-* --include='*.rs'; then
  fail "blocking std sync primitives are forbidden in harness crates"
fi

if grep -RInE '\bunsafe\s*(fn|impl|\{|trait)' crates/octopus-harness-* --include='*.rs'; then
  fail "unsafe code is forbidden in harness crates"
fi

if grep -RInE '\b(React\.|egui::|tauri::)' crates/octopus-harness-* --include='*.rs'; then
  fail "SDK contracts must not expose UI-specific types"
fi

if find crates -path 'crates/octopus-harness-*' -type f -name '*.rs' \
  ! -path 'crates/octopus-harness-contracts/*' \
  -print0 | xargs -0 grep -InE 'enum (My|Custom)?(Tool|Sandbox|Model|Harness)Error'; then
  fail "custom error families must come from SPEC-defined contracts"
fi

line_required_cards='M1-T07|M2-T01|M2-T06|M2-T11|M2-T16|M2-T21|M2-S01|M3-S01|M3-S02'
while IFS= read -r milestone; do
  awk -v cards="$line_required_cards" '
    /^### / {
      in_card = ($0 ~ cards)
      in_spec = 0
    }
    in_card && /^\*\*SPEC 锚点\*\*/ {
      in_spec = 1
      next
    }
    in_spec && (/^---/ || /^### / || /^\*\*/) {
      in_spec = 0
    }
    in_card && in_spec && /^- `/ {
      if ($0 !~ /^- `docs\/architecture\/harness\//) {
        print FILENAME ":" FNR ": SPEC anchor must use canonical docs/architecture/harness path: " $0
        failed = 1
      } else if ($0 !~ /L[0-9]+/) {
        print FILENAME ":" FNR ": missing line range: " $0
        failed = 1
      }
    }
    END { exit failed }
  ' "$milestone" || fail "line-numbered SPEC anchor check failed"
done < <(find docs/plans/harness-sdk/milestones -type f -name '*.md' | sort)

echo "spec consistency ok"
