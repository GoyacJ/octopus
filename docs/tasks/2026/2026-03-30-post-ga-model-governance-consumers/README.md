# Post-GA: Model Governance Consumers

This task package froze the first post-GA model-governance consumer boundary after the token-lifecycle closeout and Model Center Foundation. It defined how the existing model-governance contracts are first consumed by Rust persistence, runtime decision recording, and a later read-only transport follow-on.

## Closeout Status

- Completed as a design-only boundary freeze.
- The persistence/runtime-recording subset was promoted into [Post-GA: Model Governance Persistence](../2026-03-30-post-ga-model-governance-persistence/README.md).
- Read-only transport remains deferred into [Post-GA: Model Governance Read Transport](../2026-03-30-post-ga-model-governance-read-transport/README.md).

## Package Files

- [design-note.md](design-note.md)
- [contract-change.md](contract-change.md)
- [verification.md](verification.md)
- [delivery-note.md](delivery-note.md)

## Task Definition

- Goal:
  - Freeze the next bounded model-governance consumer slice so later implementation knows where persistence, runtime consumption, and read-only transport belong without reopening the terminology or contract baseline.
- Scope:
  - Create and maintain this design-only task package.
  - Define the Rust-side ownership split for model-governance persistence across `crates/governance` and `crates/runtime`.
  - Define the minimum runtime consumption boundary for `ModelSelectionDecision`.
  - Define whether any read-only transport surface is required and which module should own its assembly.
  - Freeze validation and promotion criteria for any later implementation slice.
- Out Of Scope:
  - `ProviderAdapter` SPI, provider connectivity, provider built-in tool modeling, ToolSearch / CapabilityResolver rewiring, or new desktop / web pages.
  - New runtime implementation, new SQLite migrations, new endpoints, or new shared schema objects in this package.
  - RBAC, tenant admin, external IdP, Org Graph, vector retrieval, or any other Beta/later expansion.
- Acceptance Criteria:
  - The design note names clear module ownership for persistence, runtime consumption, and any read-only transport assembly.
  - The contract-change note states whether the existing five model-governance contracts are sufficient for the first consumer slice.
  - Owner docs can register this package as the queued next design-only candidate without implying implementation is approved.
  - Verification criteria for later implementation are explicit.
- Non-functional Constraints:
  - Keep the slice design-only.
  - Preserve `schemas/` as the cross-language contract source of truth.
  - Do not weaken ADR 0007 or the existing capability-runtime boundaries.
- MVP Boundary:
  - One design-only task package.
  - One frozen ownership split for persistence and runtime consumption.
  - One read-only transport decision boundary.
  - One later-implementation verification strategy.
- Human Approval Points:
  - Promoting this package from design-only into an implementation slice.
- Source Of Truth Updates:
  - Update this task package.
  - Update `README.md`, `docs/README.md`, `docs/architecture/ga-implementation-blueprint.md`, and `docs/tasks/README.md` only to register the queue order and design-only status.
- Affected Modules:
  - `docs/tasks`
  - `docs/architecture`
  - `crates/governance`
  - `crates/runtime`
  - `packages/hub-client`
  - `apps/remote-hub`
  - `apps/desktop`
- Affected Layers:
  - Architecture design
  - Governance persistence boundary
  - Runtime decision-consumption boundary
  - Read-only transport boundary
- Risks:
  - Accidentally widening the slice into provider integration or runtime implementation.
  - Introducing transport or schema work before the consumer boundary is stable.
  - Blurring model-governance truth with capability-governance truth.
- Validation:
  - Manual review of task-package completeness, owner-doc alignment, and boundary clarity only.
