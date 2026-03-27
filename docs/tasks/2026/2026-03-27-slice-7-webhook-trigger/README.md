# Slice 7 Webhook Trigger

This task package records the second concrete GA trigger expansion slice: add the minimum governed webhook ingress path on top of the shared automation delivery projection model.

## Package Files

- [design-note.md](design-note.md)
- [contract-change.md](contract-change.md)
- [implementation-summary.md](implementation-summary.md)
- [verification.md](verification.md)
- [delivery-note.md](delivery-note.md)

## Task Definition

- Goal:
  - Implement the minimum webhook trigger so authenticated external event posts can create deduped trigger deliveries and governed automation runs.
- Scope:
  - Create this task package and keep local design, contract, verification, and delivery notes here.
  - Add runtime support for webhook-trigger metadata and validated ingress dispatch.
  - Add one remote-hub webhook route with secret validation and idempotency enforcement.
  - Add tests for happy path, invalid secret, missing idempotency key, duplicate ingress, reopen recovery, and approval/deny behavior.
- Out of Scope:
  - Desktop automation UI.
  - Public automation-management APIs.
  - Real external connector ecosystems beyond one governed webhook ingress.
- Acceptance Criteria:
  - A webhook trigger can be persisted with its ingress metadata.
  - Remote-hub accepts authenticated webhook posts and projects them into the shared delivery path.
  - Duplicate webhook events reuse the same delivery/run projection.
  - Invalid secret or missing idempotency key is rejected explicitly.
- Non-functional Constraints:
  - Secrets must not be persisted in plaintext.
  - Webhook ingress must remain a thin validation + dispatch layer.
  - Delivery and knowledge-gate semantics must remain unchanged.
- MVP Boundary:
  - One POST ingress route only.
  - No webhook-management surface and no partner-specific adapters.
- Human Approval Points:
  - None.
- Source Of Truth Updates:
  - Update this task package.
  - Update shared trigger contracts if webhook metadata is refined.
  - Update current-state docs if the tracked state summary changes materially.
- Affected Modules:
  - `schemas/runtime`
  - `crates/runtime`
  - `apps/remote-hub`
  - `docs/tasks`
- Affected Layers:
  - Cross-language contracts
  - Rust runtime/orchestration layer
  - Remote-hub ingress layer
- Risks:
  - Secret leakage through persistence or logs.
  - Silent duplicate processing.
  - Letting webhook ingress accumulate business logic.
- Validation:
  - Runtime and remote-hub integration tests for happy, invalid-secret, missing-key, duplicate, reopen, approval, and deny paths.
