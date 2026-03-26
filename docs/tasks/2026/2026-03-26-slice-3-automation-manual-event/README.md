# Slice 3 Automation Manual Event

This task package records the third verified GA runtime slice for the rebuild: add the minimum Automation + TriggerDelivery path for `manual_event` on top of the existing local SQLite-backed governed runtime.

## Package Files

- [design-note.md](design-note.md)
- [contract-change.md](contract-change.md)
- [implementation-summary.md](implementation-summary.md)
- [verification.md](verification.md)
- [delivery-note.md](delivery-note.md)

## Task Definition

- Goal: Implement the first verified non-manual entry path so a `manual_event` Automation can create an idempotent TriggerDelivery, materialize a derived Task, and execute through the existing governed Run lifecycle.
- Scope:
  - Create the Slice 3 task package and keep local design, contract, verification, and delivery notes here.
  - Add an ADR for the Automation -> TriggerDelivery -> derived Task -> Run projection model.
  - Refine shared schemas for `Automation`, `Trigger`, `TriggerDelivery`, and update `Task`, `Run`, and `TraceRecord` for automation references.
  - Extend `crates/runtime` with Automation creation, manual event dispatch, TriggerDelivery persistence, and explicit delivery retry APIs.
  - Extend SQLite migrations and automated tests for allow, dedupe, approval, deny, retry, and reopen flows.
- Out of Scope:
  - `cron`, `webhook`, or `MCP event` trigger implementations.
  - UI, app surfaces, remote transport, MCP gateway behavior, or Shared Knowledge.
  - Background schedulers, automatic retry workers, or complex automation DSL/template expansion.
  - TypeScript packages, schema generation, or frontend consumers.
- Acceptance Criteria:
  - A developer can create an Automation and its default `manual_event` Trigger through the Rust runtime API.
  - Dispatching a manual event creates or reuses a deduped TriggerDelivery and results in a governed `run_type=automation` Run.
  - The derived Task and Run preserve approval, deny, failure, retry, and reopen semantics already proven in Slices 1 and 2.
  - Repeating the same delivery `dedupe_key` does not create duplicate deliveries, tasks, runs, or artifacts.
  - Explicit delivery retry can recover a retryable failed delivery without creating a second delivery record.
  - Refined schemas validate example payloads for the Slice 3 objects.
- Non-functional Constraints:
  - Keep `schemas/` as the only cross-language contract source of truth.
  - Preserve truthful current-state docs; do not imply any UI, scheduler, remote transport, or Shared Knowledge slice exists.
  - Keep crate boundaries unchanged and localize Slice 3 runtime logic to `crates/runtime`.
  - Preserve SQLite durability, idempotency, and reopen behavior across automation records.
- MVP Boundary:
  - The slice ends once the Rust-only `manual_event` automation path is verified through tests.
  - Retry remains explicit API-driven recovery; no autonomous background delivery execution is added.
- Human Approval Points: None.
- Source Of Truth Updates:
  - Update this task package.
  - Update shared schemas under `schemas/`.
  - Add ADR 0003 for the automation delivery projection model.
  - Update repository entry docs whose current-state summary would otherwise become inaccurate.
- Affected Modules:
  - `schemas/runtime`
  - `schemas/observe`
  - `crates/runtime`
  - `docs/decisions`
- Affected Layers:
  - Cross-language contracts
  - Rust runtime/orchestration layer
  - Rust persistence/migration layer
  - Repository architecture decision layer
- Risks:
  - Letting Automation bypass the formal Task/Run/Governance semantics already proven by earlier slices.
  - Breaking Slice 1/2 idempotency or approval behavior while introducing delivery-level dedupe.
  - Expanding Slice 3 into transport, scheduler, or knowledge-system work.
- Validation:
  - Schema parse and instance-validation tests.
  - Cargo tests for allow, approval, deny, dedupe, retry, and reopen automation flows.
