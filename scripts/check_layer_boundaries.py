#!/usr/bin/env python3
from __future__ import annotations

import pathlib
import sys

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover
    print("Python 3.11+ is required for tomllib", file=sys.stderr)
    sys.exit(2)


ROOT = pathlib.Path(__file__).resolve().parents[1]

ALLOWED = {
    "octopus-harness-contracts": set(),
    "octopus-harness-model": {"octopus-harness-contracts"},
    "octopus-harness-journal": {"octopus-harness-contracts"},
    "octopus-harness-sandbox": {"octopus-harness-contracts"},
    "octopus-harness-permission": {"octopus-harness-contracts", "octopus-harness-model"},
    "octopus-harness-memory": {"octopus-harness-contracts"},
    "octopus-harness-tool": {
        "octopus-harness-contracts",
        "octopus-harness-permission",
        "octopus-harness-sandbox",
    },
    "octopus-harness-tool-search": {
        "octopus-harness-contracts",
        "octopus-harness-model",
        "octopus-harness-tool",
    },
    "octopus-harness-skill": {"octopus-harness-contracts", "octopus-harness-memory"},
    "octopus-harness-mcp": {"octopus-harness-contracts", "octopus-harness-tool"},
    "octopus-harness-hook": {"octopus-harness-contracts"},
    "octopus-harness-context": {
        "octopus-harness-contracts",
        "octopus-harness-journal",
        "octopus-harness-memory",
        "octopus-harness-model",
    },
    "octopus-harness-session": {
        "octopus-harness-contracts",
        "octopus-harness-context",
        "octopus-harness-hook",
        "octopus-harness-journal",
        "octopus-harness-mcp",
        "octopus-harness-memory",
        "octopus-harness-model",
        "octopus-harness-permission",
        "octopus-harness-sandbox",
        "octopus-harness-skill",
        "octopus-harness-tool",
    },
    "octopus-harness-engine": {
        "octopus-harness-contracts",
        "octopus-harness-context",
        "octopus-harness-hook",
        "octopus-harness-journal",
        "octopus-harness-mcp",
        "octopus-harness-memory",
        "octopus-harness-model",
        "octopus-harness-permission",
        "octopus-harness-sandbox",
        "octopus-harness-session",
        "octopus-harness-skill",
        "octopus-harness-subagent",
        "octopus-harness-tool",
        "octopus-harness-tool-search",
    },
    "octopus-harness-subagent": {
        "octopus-harness-contracts",
        "octopus-harness-permission",
        "octopus-harness-session",
        "octopus-harness-tool",
    },
    "octopus-harness-team": {
        "octopus-harness-contracts",
        "octopus-harness-journal",
        "octopus-harness-session",
    },
    "octopus-harness-plugin": {
        "octopus-harness-contracts",
        "octopus-harness-hook",
        "octopus-harness-memory",
        "octopus-harness-mcp",
        "octopus-harness-skill",
        "octopus-harness-tool",
    },
    "octopus-harness-observability": {
        "octopus-harness-contracts",
        "octopus-harness-journal",
    },
}


def package_name(cargo_toml: pathlib.Path) -> str:
    data = tomllib.loads(cargo_toml.read_text())
    return data["package"]["name"]


def harness_deps(cargo_toml: pathlib.Path) -> set[str]:
    data = tomllib.loads(cargo_toml.read_text())
    deps = set()
    for section_name in ("dependencies", "dev-dependencies", "build-dependencies"):
        section = data.get(section_name, {})
        for dep_name in section:
            if dep_name.startswith("octopus-harness-"):
                deps.add(dep_name)
    return deps


def main() -> int:
    failures: list[str] = []
    crates = sorted((ROOT / "crates").glob("octopus-harness-*/Cargo.toml"))
    if len(crates) != 19:
        failures.append(f"expected 19 harness crates, found {len(crates)}")

    for cargo_toml in crates:
        name = package_name(cargo_toml)
        if name == "octopus-harness-sdk":
            continue
        allowed = ALLOWED.get(name)
        if allowed is None:
            failures.append(f"{name}: unknown harness crate")
            continue
        actual = harness_deps(cargo_toml)
        extra = actual - allowed
        if extra:
            failures.append(f"{name}: disallowed deps: {', '.join(sorted(extra))}")

    if failures:
        for failure in failures:
            print(failure, file=sys.stderr)
        return 1

    print("layer boundaries ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
