# M1 Contracts Gate Audit

> Scope: Harness SDK M1 L0 contracts only. M2 implementation has not started.

## Surface

- `octopus-harness-contracts` now exports IDs, enums, events, messages, blob refs, capabilities, redaction, errors, and schema export.
- `strum` is recorded in the contracts SPEC dependency table and crate dependencies.
- Generated schemas are written under `schemas/harness-contracts/`.
- Legacy `octopus-sdk*` crates remain untouched.

## Local Gate Evidence

Run from `/Users/goya/Work/weilaizhihuigu/super-agent/octopus/.worktrees/harness-sdk-refactor` on 2026-04-26.

```bash
cargo fmt --all -- --check
cargo check -p octopus-harness-contracts --all-features
cargo clippy -p octopus-harness-contracts --all-targets --all-features -- -D warnings
cargo test -p octopus-harness-contracts --all-features
cargo test --doc -p octopus-harness-contracts
cargo run -p octopus-harness-contracts --example export_schemas
test "$(find schemas/harness-contracts -name '*.json' | wc -l | tr -d ' ')" -ge 60
cargo doc --no-deps -p octopus-harness-contracts
bash scripts/spec-consistency.sh
bash scripts/harness-legacy-boundary.sh
bash scripts/dep-boundary-check.sh
cargo check --workspace --all-targets
```

Observed results:

- contracts check: pass
- contracts clippy: pass
- contracts tests: 5 integration tests pass; 1 doctest pass
- schema export: pass; 105 JSON schema files
- contracts docs: pass
- SPEC consistency: pass
- harness legacy boundary: pass
- dependency boundaries: pass
- workspace all-targets check: pass

## M1 Boundaries

- No M2 crate implementation was started.
- No business crate was cut over to `octopus-harness-sdk`.
- No old SDK crate was removed.
