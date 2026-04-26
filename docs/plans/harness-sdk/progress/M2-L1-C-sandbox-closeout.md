# M2 L1-C Sandbox Closeout

> Status: ready for review
> Branch: `goya/m2-l1-c-t11-sandbox-backend`

## Delivered

- T11: `octopus-harness-sandbox` now exposes `SandboxBackend`, `ActivityHandle`, `ProcessHandle`, `ExecSpec`, `ExecContext`, snapshot types, output policy types, CWD marker protocol, shared policy re-exports, and `CodeSandbox` API behind `code-runtime`.
- T12: `LocalSandbox` supports local process spawn, piped stdout/stderr/stdin, env allowlist filtering, relative cwd normalization, wall-clock timeout, inactivity timeout, heartbeat events, and process-scope kill.
- T13: `NoopSandbox` records and rejects exec requests; Docker and SSH backends are object-safe stubs behind their features.
- T14: `DangerousPatternLibrary` provides Unix, Windows, and combined defaults with 30+ rules and representative safe/unsafe tests.
- T15: `tests/contract.rs` runs the shared Local/Noop sandbox contract with 4 contract scenarios.

## Boundaries

- No OS-level isolation backend was implemented.
- Docker and SSH remain explicit M2 stubs.
- Snapshot and restore remain explicit stubs for Local/Noop.
- Output spill, bounded backpressure, process-group/session-leader kill, and CWD side-FD execution are not implemented in this lane.
- `octopus-harness-sandbox` depends only on `octopus-harness-contracts` plus base utility crates; it does not reference `octopus-sdk-*`.

## Verification

Observed on this branch:

```bash
cargo fmt --all -- --check
cargo check -p octopus-harness-sandbox
cargo check -p octopus-harness-sandbox --all-features
cargo check -p octopus-harness-sandbox --features local
cargo check -p octopus-harness-sandbox --features noop
cargo check -p octopus-harness-sandbox --features docker
cargo check -p octopus-harness-sandbox --features ssh
cargo check -p octopus-harness-sandbox --features code-runtime
cargo clippy -p octopus-harness-sandbox --all-targets --all-features -- -D warnings
cargo test -p octopus-harness-sandbox
cargo test -p octopus-harness-sandbox --all-features
cargo test -p octopus-harness-sandbox --all-features --test contract
bash scripts/spec-consistency.sh
bash scripts/dep-boundary-check.sh
bash scripts/harness-legacy-boundary.sh
git diff --check
```

All commands above exited 0.

## Notes

- `docs/architecture/harness/api-contracts.md` sandbox section was updated to match the D3 `harness-sandbox.md` shape: lifecycle hooks, restore/shutdown, `ProcessHandle` as struct, and `ActivityHandle` as the wait/kill/heartbeat surface.
- `programmatic-tool-calling` is the facade/product feature; `octopus-harness-sandbox` exposes the lower-level `code-runtime` crate feature used by that facade feature.
- M2 global gate still depends on the other L1 lanes and M2-S01; this document closes only L1-C.
