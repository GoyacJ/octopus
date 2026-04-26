# M2-P0 L1 Alignment Audit

> Scope: Harness SDK M2-P0 only. No M2-T01/T06/T11/T16/T21 implementation has started.

## Sources

- `docs/plans/harness-sdk/milestones/M2-l1-primitives.md`
- `docs/architecture/harness/crates/harness-model.md`
- `docs/architecture/harness/crates/harness-journal.md`
- `docs/architecture/harness/crates/harness-sandbox.md`
- `docs/architecture/harness/crates/harness-permission.md`
- `docs/architecture/harness/crates/harness-memory.md`
- `docs/architecture/harness/feature-flags.md`
- `docs/architecture/harness/module-boundaries.md`
- `scripts/check_layer_boundaries.py`
- `crates/octopus-harness-{model,journal,sandbox,permission,memory}/Cargo.toml`

## Decision Legend

- `OK`: current state can enter the named M2 task.
- `DEFER-M2-Txx`: fix inside the named task card before implementing behavior that depends on it.
- `ALIGN-NOW`: fix before any M2 task starts.
- `SPEC-CLARIFY-REQUIRED`: stop before implementing the affected surface; SPECs conflict or the source of truth is unclear.

## L1 Matrix

| Crate | Current | M2 Plan | SPEC / D10 | Boundary Script | Decision |
|---|---|---|---|---|---|
| `octopus-harness-model` | Features include provider features, `all-providers`, `mock`, and internal `http-client` / `openai-compatible`; depends on `contracts`, `async-trait`, `serde`, `thiserror`, `tokio`. | T01 starts trait/types; T02+ add Anthropic; T04.5+ add provider implementations. | `http-client` and `openai-compatible` are registered in D10 §2.2 as internal transport/provider-base features. Other `harness-model.md` internal features (`rate-limit-observer`, `cassette`, `oauth`, `redactor`, `otel`) remain unregistered. | Only `contracts` is allowed by default. `model -> observability` is registered as `redactor` exception in D2 §10, but M2 says not to implement it. | `OK` for T04.5 with `openai/openrouter -> openai-compatible -> http-client`. `SPEC-CLARIFY-REQUIRED` before adding other non-D10 model features. |
| `octopus-harness-journal` | Features: `sqlite`, `jsonl`, `in-memory`; `sqlite` uses optional `rusqlite`; no blob features. | T06 starts EventStore skeleton; T07 jsonl; T08a sqlite; T08b blob stores; T09 in-memory. | D10 lists only `sqlite / jsonl / in-memory`. `harness-journal.md` also lists `blob-file / blob-sqlite / blob-memory` and uses `sqlx` in the sqlite feature example. | Only `contracts` is allowed. Current deps comply. | `OK` for T06. `SPEC-CLARIFY-REQUIRED` before T08a/T08b for `sqlx` vs `rusqlite` and blob feature registration. |
| `octopus-harness-sandbox` | Features match D10: `local`, `docker`, `ssh`, `noop`, `code-runtime`; depends only on `contracts` plus base async/serde/error/tokio deps. | T11 starts trait/types; T12 local; T13 noop plus Docker/SSH stubs; T14 heartbeat/dangerous patterns. | `harness-sandbox.md` lists `local-bubblewrap`, `local-seatbelt`, `local-job-object`; D10 does not. SPEC also names OS isolation deps not present in workspace deps. | Only `contracts` is allowed. Current deps comply. | `OK` for T11. `SPEC-CLARIFY-REQUIRED` before adding OS-isolation feature names or deps. `DEFER-M2-T12/T13` for backend deps. |
| `octopus-harness-permission` | Features match D10/SPEC: `interactive`, `stream`, `rule-engine`, `mock`, `auto-mode`; optional dep on `octopus-harness-model`. | T16 starts broker/types; T17 direct/stream; T18 rule engine; T19 signer/dangerous patterns; T20 mock/contract tests. | D2 default L1 rule forbids L1-to-L1 deps, but D2 §10 registers `permission -> model` as `auto-mode` default-off exception. | Script currently allows `permission -> model` at package level and does not verify feature-gated-only usage. | `OK` for T16. Keep `auto-mode` unimplemented and default-off in M2-P0. Add feature-aware boundary enforcement in a later governance task if needed. |
| `octopus-harness-memory` | Features match D10: `builtin`, `external-slot`, `threat-scanner`; `default = []`; no `consolidation`; no `regex` dep. | T21 starts Store/Lifecycle; T22 builtin Memdir; T23 threat scanner; T24 external slot; T25 contract/recall tests. | `harness-memory.md` lists `default = ["builtin", "threat-scanner"]`, `threat-scanner = ["dep:regex"]`, and `consolidation`; D10 lists no default and no `consolidation`. | Only `contracts` is allowed. Current deps comply. | `OK` for T21. `SPEC-CLARIFY-REQUIRED` before changing default features or adding `consolidation`. `DEFER-M2-T23` for `regex`. |

## Cross-Crate Findings

- No `ALIGN-NOW` item blocks the five M2 entry cards.
- The five entry cards can begin in parallel after this audit: T01, T06, T11, T16, T21.
- Each route must stay in its own crate until the next coordination point.
- L1 crates must not depend on each other except registered default-off exceptions.
- `permission auto-mode` is registered but must remain out of scope until a dedicated task implements and tests it.
- `model redactor` is registered as a default-off reverse dependency exception but is out of scope for M2-P0 and should not be enabled during M2 entry work.

## Required Follow-Up Before Deeper M2 Cards

| Before | Required action |
|---|---|
| M2-T04.6+ | Decide whether remaining `harness-model.md` internal features are canonical or should be folded behind D10 provider features. `http-client` and `openai-compatible` are already registered for M2-T04.5. |
| M2-T08a | Resolve `sqlx` vs `rusqlite` for `SqliteEventStore`; update SPEC, D10, workspace deps, and Cargo feature wiring together. |
| M2-T08b | Register `blob-file / blob-sqlite / blob-memory` in D10 or remove them from `harness-journal.md`; do not add unregistered features. |
| M2-T12/T13 | Decide whether OS-isolation features are public crate features or implementation details under `local`. |
| M2-T23 | Decide whether `memory` default features should follow D10 `default = []` or memory SPEC `default = ["builtin", "threat-scanner"]`. |

## M2 Start Recommendation

Start these five cards in parallel:

- `M2-T01` in `octopus-harness-model`
- `M2-T06` in `octopus-harness-journal`
- `M2-T11` in `octopus-harness-sandbox`
- `M2-T16` in `octopus-harness-permission`
- `M2-T21` in `octopus-harness-memory`

Do not start these surfaces until the listed conflicts are resolved:

- non-D10 `harness-model` features
- journal blob features
- journal sqlite backend crate choice
- sandbox OS-isolation feature names
- memory default feature policy
- memory `consolidation` feature

## Verification

M2-P0 changes only this audit document. Required commands:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets
bash scripts/spec-consistency.sh
bash scripts/harness-legacy-boundary.sh
bash scripts/dep-boundary-check.sh
bash scripts/feature-matrix.sh
git diff --check
```
