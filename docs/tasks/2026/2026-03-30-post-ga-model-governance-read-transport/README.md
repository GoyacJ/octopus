# Post-GA: Model Governance Read Transport

This task package freezes the next design-only slice after the bounded model-governance persistence implementation. The slice is limited to defining the first read-only transport consumers for model-governance records and run-scoped `ModelSelectionDecision` visibility.

## Closeout Status

- Planned.
- Design-only. This package does not authorize write paths, provider connectivity, or UI editing flows.

## Package Files

- [design-note.md](design-note.md)
- [contract-change.md](contract-change.md)
- [verification.md](verification.md)
- [delivery-note.md](delivery-note.md)

## Task Definition

- Goal:
  - Freeze the next bounded read-only transport slice so `apps/remote-hub`, `packages/hub-client`, and `apps/desktop` can consume model-governance persistence truth without reopening persistence ownership or widening into write behavior.
- Scope:
  - Create and maintain this design-only task package.
  - Define the read-only route assembly boundary in `apps/remote-hub`.
  - Define the transport-neutral read accessor boundary in `packages/hub-client`.
  - Define the first desktop read-consumer boundary for provider/catalog/profile/policy records and run-scoped `ModelSelectionDecision`.
  - Freeze promotion criteria for a later implementation slice.
- Out Of Scope:
  - Any write endpoint, edit flow, approval management surface, provider credential management, or admin console work.
  - Provider connectivity, provider built-in tool modeling, `ProviderAdapter` SPI, `CapabilityResolver` / `ToolSearch` rewiring, tenant/RBAC/IdP work, vector retrieval, or Org Graph work.
  - New runtime persistence behavior or new shared schema objects unless a later bounded contract-change explicitly proves they are required.
- Acceptance Criteria:
  - The design note names exact module ownership for read-only route assembly, shared client accessors, and desktop read consumption.
  - The contract-change note states whether the existing five model-governance contracts are sufficient for the first read-only transport slice.
  - Owner docs can register this package as the queued next design-only candidate without implying implementation is already approved.
  - Verification criteria for later implementation are explicit.
- Non-functional Constraints:
  - Keep the slice read-only.
  - Preserve `schemas/` as the cross-language contract source of truth.
  - Preserve `crates/governance` as persistence truth and `crates/runtime` as the owner of run-scoped `ModelSelectionDecision` recording.
- MVP Boundary:
  - One design-only task package.
  - One frozen read-only route assembly boundary.
  - One frozen shared-client read-accessor boundary.
  - One frozen desktop read-consumer boundary.
- Human Approval Points:
  - Promoting this package from design-only into an implementation slice.
- Source Of Truth Updates:
  - Update this task package.
  - Update `README.md`, `docs/README.md`, `docs/architecture/ga-implementation-blueprint.md`, and `docs/tasks/README.md` only to register the queue order and design-only status.
- Affected Modules:
  - `docs/tasks`
  - `docs/architecture`
  - `apps/remote-hub`
  - `packages/hub-client`
  - `apps/desktop`
  - `crates/governance`
  - `crates/runtime`
- Affected Layers:
  - Read-only transport assembly
  - Shared client transport consumption
  - Desktop read surface planning
- Risks:
  - Accidentally widening the slice into write endpoints or UI editing.
  - Reintroducing persistence-boundary debate that was already frozen by the predecessor slices.
  - Inventing transport DTOs without a bounded contract-change.
- Validation:
  - Manual review of task-package completeness, owner-doc alignment, and boundary clarity only.
