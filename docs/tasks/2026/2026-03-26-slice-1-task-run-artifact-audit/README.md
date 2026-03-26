# Slice 1 Task Run Artifact Audit

This task package records the first real implementation slice for the GA rebuild: prove the Task -> Run -> Artifact -> Audit closed loop with SQLite-backed Rust runtime members, while keeping UI, approval, automation, MCP, and shared-knowledge depth out of scope.

## Package Files

- [design-note.md](design-note.md)
- [contract-change.md](contract-change.md)
- [implementation-summary.md](implementation-summary.md)
- [verification.md](verification.md)
- [delivery-note.md](delivery-note.md)

## Task Definition

- Goal: Implement the first verified Octopus runtime slice that can create a Task in a Workspace/Project context, create and execute a Run, persist Artifact/Audit/Trace records, and preserve retry or terminate semantics in SQLite.
- Scope:
  - Create the Slice 1 task package and keep all local design and delivery notes here.
  - Refine the shared schemas for `Workspace`, `Project`, `Task`, `Run`, `Artifact`, `AuditRecord`, and `TraceRecord`.
  - Start real Rust workspace members under `crates/domain-context`, `crates/execution`, `crates/observe-artifact`, and `crates/runtime`.
  - Implement a deterministic local execution action path only.
  - Persist context, task, run, artifact, audit, and trace state to SQLite.
  - Cover success, failure, retry, terminate, idempotency, and reopen/reload verification in automated tests.
- Out of Scope:
  - HTTP API, Tauri invoke, Desktop or Remote Hub UI surfaces.
  - Approval, Policy, Budget, Inbox, Notification, Automation, TriggerDelivery, Shared Knowledge, MCP, or A2A implementation.
  - TypeScript packages, schema code generation, or frontend contract consumers.
  - Beta-only objects and flows.
- Acceptance Criteria:
  - A developer can create a Task for a Workspace/Project context and execute it through a verified Rust API.
  - Successful execution persists a completed Run, an Artifact, and related Audit/Trace records.
  - Failed execution persists failure records and supports either retry-to-success or explicit terminate behavior.
  - Re-opening the same SQLite database preserves the formal Run and observation state.
  - Refined schemas validate example payloads for the Slice 1 objects without breaking parseability of untouched placeholder schemas.
- Non-functional Constraints:
  - Keep `schemas/` as the only cross-language contract source of truth.
  - Keep implementation strictly within the approved crate ownership groups.
  - Use SQLite as the local authority store for Slice 1 instead of an in-memory substitute.
  - Preserve truthful current-state documentation and avoid implying any UI or remote runtime exists.
- MVP Boundary:
  - The slice ends once the local Rust-only closed loop is verified through tests.
  - No additional governance or UI plane depth is allowed in this task.
- Human Approval Points: None.
- Source Of Truth Updates:
  - Update this task package.
  - Update the refined shared schemas under `schemas/`.
  - Update repository entry docs whose current-state notes would otherwise become inaccurate.
- Affected Modules:
  - `schemas/context`
  - `schemas/runtime`
  - `schemas/observe`
  - `crates/domain-context`
  - `crates/execution`
  - `crates/observe-artifact`
  - `crates/runtime`
- Affected Layers:
  - Cross-language contracts
  - Rust domain/context layer
  - Rust runtime/orchestration layer
  - Rust execution adapter layer
  - Rust observation/persistence layer
- Risks:
  - Over-expanding Slice 1 contracts into later-slice governance or knowledge semantics.
  - Letting runtime implementation drift away from schema naming.
  - Using SQLite in a way that hides reopen/recovery regressions.
- Validation:
  - Schema parse and instance-validation tests.
  - Cargo test coverage for success, failure, retry, terminate, idempotency, and reopen scenarios.
