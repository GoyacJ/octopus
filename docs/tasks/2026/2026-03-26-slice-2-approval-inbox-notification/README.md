# Slice 2 Approval Inbox Notification

This task package records the second verified GA runtime slice for the rebuild: add the governed execution path for approval, inbox, notification, and policy decision logging on top of the existing local SQLite-backed Task -> Run -> Artifact -> Audit / Trace slice.

## Package Files

- [design-note.md](design-note.md)
- [contract-change.md](contract-change.md)
- [implementation-summary.md](implementation-summary.md)
- [verification.md](verification.md)
- [delivery-note.md](delivery-note.md)

## Task Definition

- Goal: Implement the first verified governance slice that can evaluate a Task before execution, require approval when needed, persist ApprovalRequest / InboxItem / Notification / PolicyDecisionLog records, and resume or block the Run based on the approval result.
- Scope:
  - Create the Slice 2 task package and keep local design, contract, verification, and delivery notes here.
  - Refine shared schemas for `Task`, `Run`, `ApprovalRequest`, `CapabilityDescriptor`, `CapabilityBinding`, `CapabilityGrant`, `BudgetPolicy`, `InboxItem`, `Notification`, and add `PolicyDecisionLog`.
  - Add a real Rust workspace member under `crates/governance`.
  - Extend `crates/observe-artifact` with inbox, notification, and policy-decision persistence.
  - Extend `crates/runtime` with governance evaluation, approval lifecycle handling, and query APIs for governance outputs.
  - Extend SQLite migrations and automated tests for approval-required, approval-approved, approval-rejected, deny, dedupe, and reopen flows.
- Out of Scope:
  - HTTP, Tauri invoke, Desktop, Remote Hub, Web, Mobile, or any other app surface.
  - Automation, TriggerDelivery, MCP, Shared Knowledge, A2A, or Beta-only collaboration flows.
  - User-, team-, tenant-, or org-level identity/governance graphs beyond the current Workspace / Project-scoped subject assumption.
  - TypeScript packages, schema generation, or frontend contract consumers.
- Acceptance Criteria:
  - A low-risk task with an allowed capability can still execute directly and produce the same Slice 1 outputs.
  - A high-risk or soft-limit-exceeding task creates an `ApprovalRequest`, `InboxItem`, `Notification`, and `PolicyDecisionLog`, and leaves the Run in `waiting_approval`.
  - Approving a pending request resumes the existing Run and allows it to complete.
  - Rejecting a pending request blocks the Run without producing execution artifacts.
  - A denied task does not execute and still records governance output.
  - Reopening the same SQLite database preserves pending approvals and does not silently continue execution.
  - Repeated start or approval operations do not create duplicate pending governance records.
- Non-functional Constraints:
  - Keep `schemas/` as the only cross-language contract source of truth.
  - Preserve truthful current-state documentation and do not imply any runnable UI or transport exists.
  - Keep runtime truth in Rust crates, not in `apps/` or `packages/`.
  - Preserve Slice 1 behavior and tests while extending the runtime.
- MVP Boundary:
  - This slice ends once the local Rust-only governance closed loop is verified through tests.
  - Approval is limited to `execution` requests and Workspace / Project-scoped subject evaluation.
  - Inbox and notification remain Rust-queryable records, not surfaced UI workflows.
- Human Approval Points: None.
- Source Of Truth Updates:
  - Update this task package.
  - Update refined shared schemas under `schemas/`.
  - Update entry docs whose current-state summary would otherwise be inaccurate.
- Affected Modules:
  - `schemas/runtime`
  - `schemas/governance`
  - `schemas/observe`
  - `crates/governance`
  - `crates/observe-artifact`
  - `crates/runtime`
- Affected Layers:
  - Cross-language contracts
  - Rust runtime/orchestration layer
  - Rust governance layer
  - Rust observation/persistence layer
- Risks:
  - Over-expanding Slice 2 into Automation, Shared Knowledge, or transport work.
  - Letting governance records drift away from schema naming or run-state semantics.
  - Creating approval paths that are not durable across SQLite reopen or are not idempotent.
- Validation:
  - Schema parse and instance-validation tests.
  - Cargo tests for allow, approval wait, approval approve, approval reject, deny, dedupe, and reopen scenarios.
