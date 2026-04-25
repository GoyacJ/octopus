#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

expected="docs/architecture/harness/expected-depgraph.dot"
actual="target/octopus-harness-depgraph.dot"
metadata="${TMPDIR:-/tmp}/octopus-harness-metadata.json"

test -f "$expected"
mkdir -p target

cargo metadata --format-version 1 --all-features > "$metadata"

python3 - "$metadata" "$actual" <<'PY'
from __future__ import annotations

import json
import pathlib
import sys

metadata_path = pathlib.Path(sys.argv[1])
actual_path = pathlib.Path(sys.argv[2])
metadata = json.loads(metadata_path.read_text())

package_names = {package["id"]: package["name"] for package in metadata["packages"]}
harness_names = {
    name for name in package_names.values() if name.startswith("octopus-harness-")
}

if len(harness_names) != 19:
    print(f"expected 19 harness crates, found {len(harness_names)}", file=sys.stderr)
    sys.exit(1)


def node_name(package_name: str) -> str:
    return package_name.removeprefix("octopus-harness-").replace("-", "_")


nodes = sorted(node_name(name) for name in harness_names)
edges: set[tuple[str, str]] = set()

for node in metadata["resolve"]["nodes"]:
    package_name = package_names[node["id"]]
    if package_name not in harness_names:
        continue
    for dep in node["deps"]:
        dep_name = package_names[dep["pkg"]]
        if dep_name in harness_names:
            edges.add((node_name(package_name), node_name(dep_name)))

lines = ["digraph octopus_harness {", "  rankdir=BT;", ""]
for node in nodes:
    lines.append(f"  {node};")
lines.append("")
for source, target in sorted(edges):
    lines.append(f"  {source} -> {target};")
lines.append("}")
lines.append("")

actual_path.write_text("\n".join(lines))
PY

diff -u "$expected" "$actual"

echo "depgraph snapshot ok: $actual"
