# GA Trigger Expansion Foundation

This task package records the trigger-substrate foundation work for GA trigger expansion: generalize the single-trigger automation model from `manual_event` only to the full GA trigger set without changing the one-automation-one-trigger boundary.

## Package Files

- [design-note.md](design-note.md)
- [contract-change.md](contract-change.md)
- [implementation-summary.md](implementation-summary.md)
- [verification.md](verification.md)
- [delivery-note.md](delivery-note.md)

## Task Definition

- Goal:
  - Generalize the current automation trigger substrate so `manual_event`, `cron`, `webhook`, and `mcp_event` can all project into the same governed `TriggerDelivery -> Task -> Run` execution shell.
- Scope:
  - Create this task package and keep local design, contract, verification, and delivery notes here.
  - Update trigger schemas from a single `manual_event` constant to a discriminated union that still fits the current one-trigger-per-automation model.
  - Extend runtime models, persistence, and internal dispatch flow so all GA trigger types can reuse one delivery projection path.
  - Preserve the existing manual-event API as a compatibility wrapper.
- Out of Scope:
  - `cron` ticking behavior, webhook ingress, and MCP-event ingress implementation.
  - Desktop or shared TypeScript automation UI/client APIs.
  - Multi-trigger automations, schedulers, or real MCP credential transport.
- Acceptance Criteria:
  - Shared trigger contracts support `manual_event`, `cron`, `webhook`, and `mcp_event`.
  - Runtime persistence can store trigger-type-specific metadata without changing `Automation.trigger_id`.
  - Existing `manual_event` behavior remains green through the same public entrypoint.
  - New trigger types can enter one common delivery projection path without parallel execution shells.
- Non-functional Constraints:
  - Keep `schemas/` as the only cross-language contract source of truth.
  - Preserve current delivery state semantics, dedupe, retry, reopen, approval waiting, and knowledge gating.
  - Avoid desktop/package surface expansion in this slice.
- MVP Boundary:
  - This slice ends once the shared substrate is generalized and the existing manual-event path still passes regression coverage.
  - No external ingress behavior is proven yet.
- Human Approval Points:
  - None.
- Source Of Truth Updates:
  - Update this task package.
  - Update shared schemas under `schemas/runtime`.
  - Update repository entry docs and blueprint sequencing if tracked state or follow-up order changes.
- Affected Modules:
  - `schemas/runtime`
  - `crates/runtime`
  - `docs/architecture`
  - `docs/tasks`
- Affected Layers:
  - Cross-language contracts
  - Rust runtime/orchestration layer
  - Rust persistence/migration layer
  - Architecture/task-package documentation
- Risks:
  - Breaking the verified `manual_event` path while generalizing trigger typing.
  - Accidentally introducing multi-trigger semantics or a second execution shell.
  - Leaking trigger-specific ingress concerns into the substrate layer.
- Validation:
  - Schema contract tests.
  - Existing Slice 3 automation regression tests.
  - New foundation regression tests for trigger-type persistence and common dispatch compatibility.
