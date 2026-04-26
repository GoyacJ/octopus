# M3 Spike Hook Replay

Status: submitted for review.

## Scope

This spike validates the M3 hook transport risk called out by the architecture review: in-process, Exec, and HTTP hook failure behavior must remain compatible with replay idempotency before context/session work continues.

Covered SPEC anchors:

- `docs/architecture/harness/crates/harness-hook.md` §2.6.2
- `docs/architecture/harness/crates/harness-hook.md` §3.1-§3.3
- `docs/architecture/harness/crates/harness-hook.md` §11
- `docs/architecture/harness/audit/2026-04-25-architecture-review.md` §4.4 item 3

## Coverage

`crates/octopus-harness-hook/tests/spike_replay_idempotent.rs` validates:

- in-process panic with `FailOpen` records failure and continues
- in-process panic with `FailClosed` records failure and blocks
- Exec non-zero exit records a transport failure
- Exec timeout records a timeout failure
- HTTP 5xx records a transport failure
- HTTP SSRF guard records a transport failure before request dispatch
- invalid HTTP mTLS config is rejected before registration
- `ReplayMode::Audit` does not re-trigger hook side effects

## Result

The spike passed without production code changes. Current M3-T10 hook behavior satisfies the pre-context risk check.

Target verification:

```bash
cargo test -p octopus-harness-hook --test spike_replay_idempotent --all-features
```

## Boundaries

- mTLS is validated as registration-time rejection for invalid identity material. Runtime TLS handshake coverage remains outside M3-S01.
- Replay idempotency is validated at dispatcher level: Audit mode returns the recorded/default dispatch view and does not invoke handlers or transports.
- Journal-backed replay reconstruction remains owned by later session/journal integration work.
