# Slice 12 GA Governance Explainability

This task package records the first post-Slice 11 GA follow-up: make governance execution paths explainable without expanding into governance-management or tenant-admin work.

## Package Files

- [design-note.md](design-note.md)
- [contract-change.md](contract-change.md)
- [implementation-summary.md](implementation-summary.md)
- [verification.md](verification.md)
- [delivery-note.md](delivery-note.md)

## Task Definition

- Goal:
  - Deliver the minimum GA governance explainability surface so users can see why a project-bound capability is executable, approval-required, or denied for the current estimated cost, and why a run entered a given governance path.
- Scope:
  - Create this task package and keep local design, contract, verification, and delivery notes here.
  - Fix the existing `pnpm typecheck:ts` baseline blocker in `packages/hub-client/test/hub-client.contract.test.ts`.
  - Replace the visible-only shared capability explanation contract with a cost-aware `CapabilityResolution`.
  - Extend `crates/governance` and `crates/runtime` with a read-only capability-resolution evaluation path that reuses current binding, grant, budget, and risk checks.
  - Extend `packages/schema-ts`, `packages/hub-client`, `apps/remote-hub`, and `apps/desktop` so local and remote surfaces expose the same capability explainability behavior.
  - Surface existing `policy_decisions` in the desktop Run view without changing the run-detail wire shape.
  - Update owner docs so the tracked truth reflects Slice 12 and the new post-Slice 11 priority freeze.
- Out of Scope:
  - Grant or budget editing flows.
  - Tenant, RBAC, or external IdP administration.
  - Standalone Inbox / Notification center work.
  - Vector retrieval, Org Graph promotion, or broader knowledge-governance expansion.
  - A global capability catalog, search-only capability directory, or deep connector-health explainability.
- Acceptance Criteria:
  - Shared contracts expose `CapabilityResolution` with `descriptor`, `scope_ref`, `execution_state`, `reason_code`, and `explanation`.
  - Runtime explainability reuses governed-runtime truth and supports cost-aware evaluation for already project-bound capabilities.
  - `HubClient` exposes capability resolution parity across local and remote transports, including `estimated_cost`.
  - `apps/remote-hub` returns capability resolutions from the existing `/capabilities` surface with auth and membership enforcement intact.
  - `apps/desktop` updates workspace capability explainability as estimated cost changes for task / automation inputs and shows run `policy_decisions`.
  - Full verification finishes with `cargo test --workspace -- --nocapture`, `pnpm test:ts`, and `pnpm typecheck:ts`.
- Non-functional Constraints:
  - Keep `schemas/` as the only cross-language contract truth.
  - Keep governance evaluation in `crates/governance` / `crates/runtime`; do not introduce UI-local policy state machines.
  - Preserve existing `Run`, `ApprovalRequest`, and `KnowledgeCandidate` state machines.
  - Reuse the existing workspace and run surfaces instead of creating a parallel governance shell.
- MVP Boundary:
  - Only explain capabilities already bound to the current project.
  - Only explain execution state for the current scope and cost; do not add tenant-wide search or management workflows.
  - Reuse `RunDetail.policy_decisions`; do not redesign the run detail contract.
- Human Approval Points:
  - None.
- Source Of Truth Updates:
  - Update this task package.
  - Update `docs/tasks/README.md`.
  - Update `README.md`, `docs/README.md`, `docs/architecture/SAD.md`, and `docs/architecture/ga-implementation-blueprint.md` after implementation and verification complete.
  - Do not add an ADR unless the slice forces a durable boundary change beyond current GA governance semantics.
- Affected Modules:
  - `schemas/governance`
  - `crates/governance`
  - `crates/runtime`
  - `packages/schema-ts`
  - `packages/hub-client`
  - `apps/remote-hub`
  - `apps/desktop`
  - `docs/tasks`
  - owner docs in `README.md` and `docs/`
- Affected Layers:
  - Shared contracts
  - Governed runtime
  - Shared client transport boundary
  - Remote surface assembly
  - Desktop surface assembly
  - Owner documentation
- Risks:
  - Letting the new explainability path drift from the authoritative execution governance logic.
  - Accidentally expanding capability explainability into a broader catalog or tenant-admin feature.
  - Breaking local / remote parity when adding cost-aware inputs.
  - Surfacing run governance outcomes in desktop inconsistently with persisted `policy_decisions`.
- Validation:
  - Add failing Rust, schema-ts, hub-client, remote-hub, and desktop tests first.
  - Re-run workspace Rust tests, TypeScript tests, and TypeScript typecheck before claiming completion.
