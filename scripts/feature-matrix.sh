#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

cargo check --workspace --all-targets
cargo check -p octopus-harness-sdk --no-default-features
cargo check -p octopus-harness-sdk --features all-providers
cargo check -p octopus-harness-sdk --features "sqlite-store,local-sandbox,interactive-permission,provider-anthropic"
cargo check -p octopus-harness-sdk --features testing
cargo check -p octopus-harness-sdk --all-features

echo "feature matrix ok"
