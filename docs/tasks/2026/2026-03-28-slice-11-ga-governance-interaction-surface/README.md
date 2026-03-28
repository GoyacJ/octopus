# Slice 11 GA Governance Interaction Surface

This task package records the next GA surface-facing slice after the minimum automation surface: formalize the approval / inbox / knowledge-promotion governance interaction path on top of the verified runtime and surface foundation.

## Package Files

- [design-note.md](design-note.md)
- [contract-change.md](contract-change.md)
- [implementation-summary.md](implementation-summary.md)
- [verification.md](verification.md)
- [delivery-note.md](delivery-note.md)

## Task Definition

- Goal:
  - Deliver the minimum GA governance interaction surface so approval requests can be inspected and resolved through shared clients, and `KnowledgeCandidate` promotion enters the formal approval path instead of bypassing governance.
- Scope:
  - Create this task package and keep local design, contract, verification, and delivery notes here.
  - Extend shared governance / observe schemas so `ApprovalRequest`, `InboxItem`, and `Notification` carry stable `target_ref` truth and approvals can distinguish `execution` from `knowledge_promotion`.
  - Add `RequestKnowledgePromotionCommand` and extend `crates/runtime` with an approval-driven knowledge-promotion path that preserves the existing execution approval semantics.
  - Extend `packages/schema-ts`, `packages/hub-client`, `apps/remote-hub`, and `apps/desktop` so both local and remote modes expose the same approval-detail, approval-resolution, and request-promotion behavior.
  - Update owner docs so the tracked truth reflects the already completed minimum automation surface and this Slice 11 delivery.
- Out of Scope:
  - New Inbox / Board routes or a standalone notification center.
  - Tenant / RBAC / external IdP administration.
  - Vector retrieval, Org Graph promotion, or deeper knowledge-governance workflows.
  - Automation editor / dashboard expansion or a tracked `window.__OCTOPUS_LOCAL_HUB__` bridge implementation.
- Acceptance Criteria:
  - `ApprovalRequest` supports `execution | knowledge_promotion` and all approval-centric surface records carry a required `target_ref`.
  - One `KnowledgeCandidate` can have at most one open promotion approval at a time; approval creates `KnowledgeAsset` and lineage without mutating the completed Run state.
  - `HubClient` exposes approval detail and request-promotion parity across local and remote transports.
  - `remote-hub` exposes approval detail and request-promotion routes with existing auth and workspace-membership enforcement.
  - `apps/desktop` lets users review open inbox approvals, resolve approvals inline, and request knowledge promotion from the existing workspace / run surfaces while preserving read-only behavior.
- Non-functional Constraints:
  - Keep `schemas/` as the only cross-language contract truth.
  - Keep approval and promotion side effects in `crates/runtime`; do not introduce a second governance or promotion state machine in app code.
  - Reuse the existing workspace / run surfaces instead of creating a parallel governance shell.
  - Keep notification handling read-only in this slice.
- MVP Boundary:
  - One shared approval detail surface, one request-promotion command, and inline governance actions in the existing desktop views.
  - Direct `promoteKnowledge(...)` remains an internal primitive path, but shared client and desktop GA flows use approval-driven promotion.
  - `knowledge_promotion` approvals do not mutate the Run state machine; candidate status changes only after approval.
- Human Approval Points:
  - Execution approvals remain in place.
  - Knowledge promotion becomes a formal approval point in this slice.
- Source Of Truth Updates:
  - Update this task package.
  - Update `docs/tasks/README.md`.
  - Update `README.md`, `docs/README.md`, `docs/architecture/SAD.md`, and `docs/architecture/ga-implementation-blueprint.md` after implementation and verification complete.
  - Add an ADR only if approval ownership or knowledge-promotion ownership changes beyond this slice.
- Affected Modules:
  - `schemas/governance`
  - `schemas/observe`
  - `crates/governance`
  - `crates/observe-artifact`
  - `crates/runtime`
  - `packages/schema-ts`
  - `packages/hub-client`
  - `apps/remote-hub`
  - `apps/desktop`
  - `docs/tasks`
- Affected Layers:
  - Shared contracts
  - Governed runtime
  - Shared client transport boundary
  - Remote surface assembly
  - Desktop surface assembly
- Risks:
  - Letting knowledge-promotion approvals drift from existing execution approval semantics.
  - Allowing duplicate open promotion approvals for the same candidate.
  - Accidentally mutating Run state when approving or rejecting knowledge promotion.
  - Bypassing read-only / token-expired protections in desktop governance actions.
- Validation:
  - Add failing runtime, remote-hub, schema/client, and desktop tests first.
  - Re-run the Slice 11 runtime, remote-hub, schema-ts, hub-client, desktop, and desktop typecheck suites before claiming completion.
