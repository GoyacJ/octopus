# Post-GA: Model Center Foundation

This task package records the completed post-GA architecture-foundation slice that formalizes model-center terminology, minimal shared contracts, and owner-doc alignment after the token-lifecycle closeout.

## Closeout Status

- Completed.
- This package is closed out as a doc/schema-only post-GA foundation slice and does not authorize runtime, provider-adapter, or surface implementation.

## Package Files

- [../../../../decisions/0007-model-center-governance-boundary-and-terminology.md](../../../../decisions/0007-model-center-governance-boundary-and-terminology.md)
- [design-note.md](design-note.md)
- [contract-change.md](contract-change.md)
- [adr-trigger-note.md](adr-trigger-note.md)
- [implementation-summary.md](implementation-summary.md)
- [verification.md](verification.md)
- [delivery-note.md](delivery-note.md)

## Task Definition

- Goal:
  - Formalize the first bounded Model Center foundation slice so model governance terminology, shared contracts, and document ownership are stable before any provider adapter or runtime refactor starts.
- Scope:
  - Create and maintain this task package.
  - Add a durable ADR for Model Center boundary and terminology.
  - Freeze the minimum model governance shared contracts under `schemas/governance`.
  - Register the new contracts in `packages/schema-ts`.
  - Update owner docs and indexes to record this as the completed doc/schema-only post-GA architecture foundation and to queue the next design-only candidate.
  - Move the March 27 model-center supplement out of `docs/` root and preserve it as non-normative reference input.
- Out Of Scope:
  - `ProviderAdapter` SPI, provider built-in tool modeling, ToolSearch response redesign, CapabilityResolver runtime refactor, new Rust runtime/governance logic, new remote-hub endpoints, and new desktop surfaces.
  - RBAC, tenant admin, external IdP, SSO, A2A, Org Graph, vector retrieval, or any other Beta/later expansion.
- Acceptance Criteria:
  - PRD/SAD/blueprint/ADR/task package use one canonical model-governance vocabulary based on PRD terms.
  - New shared contracts exist for `ModelProvider`, `ModelCatalogItem`, `ModelProfile`, `TenantModelPolicy`, and `ModelSelectionDecision`.
  - `packages/schema-ts` can parse the new contracts without introducing a parallel source of truth.
  - The supplement no longer resides as a root-level normative-looking document.
- Non-functional Constraints:
  - Keep `schemas/` as the only cross-language contract source.
  - Keep the slice doc/schema-only; no provider connectivity or runtime behavior change.
  - Do not alter the Slice 20 GA acceptance conclusion or the current token-lifecycle scope.
- MVP Boundary:
  - One ADR.
  - One completed doc/schema-only post-GA task package.
  - Five minimal shared contracts.
  - One `schema-ts` registration path.
  - One reference-doc relocation and owner-doc alignment pass.
- Human Approval Points:
  - Any later runtime, transport, provider, or surface consumer slice still requires a new task package and separate approval.
- Source Of Truth Updates:
  - Update this task package.
  - Update `docs/decisions/README.md`, `docs/README.md`, `docs/tasks/README.md`, and `docs/references/README.md`.
  - Update `docs/architecture/SAD.md` and `docs/architecture/ga-implementation-blueprint.md`.
  - Update `README.md` only to keep the tracked-tree summary truthful.
- Affected Modules:
  - `docs/tasks`
  - `docs/decisions`
  - `docs/architecture`
  - `docs/references`
  - `schemas/governance`
  - `packages/schema-ts`
- Affected Layers:
  - Architecture / governance documentation
  - Shared contract layer
  - TypeScript schema consumer layer
- Risks:
  - Expanding the slice into provider adapter or runtime implementation too early.
  - Reintroducing parallel model terminology across docs.
  - Adding schemas that are too detailed before runtime consumers exist.
- Validation:
  - `pnpm --filter @octopus/schema-ts exec vitest run test/contracts.test.ts`
  - `pnpm --filter @octopus/schema-ts typecheck`
