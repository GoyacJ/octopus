# M0 Bootstrap Gate

> Status: Local gate passed on 2026-04-26.
> Scope: Harness SDK M0 bootstrap only. M1 implementation has not started.

## Harness Crates

The workspace contains 19 `octopus-harness-*` crates:

- `octopus-harness-context`
- `octopus-harness-contracts`
- `octopus-harness-engine`
- `octopus-harness-hook`
- `octopus-harness-journal`
- `octopus-harness-mcp`
- `octopus-harness-memory`
- `octopus-harness-model`
- `octopus-harness-observability`
- `octopus-harness-permission`
- `octopus-harness-plugin`
- `octopus-harness-sandbox`
- `octopus-harness-sdk`
- `octopus-harness-session`
- `octopus-harness-skill`
- `octopus-harness-subagent`
- `octopus-harness-team`
- `octopus-harness-tool`
- `octopus-harness-tool-search`

## Legacy SDK Freeze

The 14 legacy `octopus-sdk*` crates remain frozen under:

- `docs/plans/harness-sdk/audit/M0-legacy-sdk-freeze.md`

They are retained through M7 and may be removed only after the M8 business cutover gate passes.

## Local Gate Evidence

The following commands are the local M0 gate. Each must exit 0 before M1 starts:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo check --workspace --all-features
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-features --no-fail-fast
bash scripts/harness-legacy-boundary.sh
bash scripts/spec-consistency.sh
bash scripts/feature-matrix.sh
bash scripts/dep-boundary-check.sh
bash scripts/depgraph-snapshot.sh
bash scripts/deny-feature-matrix.sh
cargo deny check
cargo doc --no-deps --workspace
```

Additional invariant checks:

```bash
find crates -maxdepth 1 -type d -name 'octopus-harness-*' | wc -l
find crates -maxdepth 1 -type d -name 'octopus-sdk*' | wc -l
test ! -d crates/_octopus-bridge-stub
! rg -n 'legacy-sdk|_octopus[-_]bridge[-_]stub' crates apps
```

Expected counts:

- harness crates: 19
- legacy SDK crates: 14

## Boundary Evidence

- `scripts/harness-legacy-boundary.sh` enforces zero `octopus-harness-*` references to `octopus-sdk*`.
- `scripts/dep-boundary-check.sh` enforces the harness layer whitelist.
- `scripts/depgraph-snapshot.sh` generates `target/octopus-harness-depgraph.dot` from `cargo metadata --all-features` and diffs it against `docs/architecture/harness/expected-depgraph.dot`.
- `scripts/deny-feature-matrix.sh` runs cargo-deny against default, typical desktop, all-providers, and all-features profiles.

## CI Evidence

GitHub Actions coverage is defined in:

- `.github/workflows/ci.yml`
- `.github/workflows/deny.yml`
- `.github/workflows/nightly.yml`

The first green branch or PR run must be attached here before M1 starts:

- CI run: pending external GitHub Actions evidence

## Known Unresolved Issues

- None in the repository-local M0 gate.
