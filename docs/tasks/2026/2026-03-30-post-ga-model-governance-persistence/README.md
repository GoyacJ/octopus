# Post-GA: Model Governance Persistence

This task package records the first implementation slice that follows the doc/schema-only Model Center foundation and the design-only Model Governance Consumers boundary freeze. The slice is strictly limited to Rust-side persistence truth for provider/catalog/profile/policy records and run-scoped `ModelSelectionDecision` recording.

## Closeout Status

- Completed.
- Implemented and verified as a bounded persistence/runtime-recording slice only.

## Package Files

- [design-note.md](design-note.md)
- [contract-change.md](contract-change.md)
- [implementation-summary.md](implementation-summary.md)
- [verification.md](verification.md)
- [delivery-note.md](delivery-note.md)

## Task Definition

- Goal:
  - Implement the first bounded model-governance consumer slice so Rust persistence owns `ModelProvider` / `ModelCatalogItem` / `ModelProfile` / `TenantModelPolicy`, and runtime can record and read one run-scoped `ModelSelectionDecision` without widening into transport, provider connectivity, or UI work.
- Scope:
  - Create and maintain this implementation task package.
  - Add the minimum SQLite migration required for the model-governance persistence tables and the run-scoped `model_selection_decisions` table.
  - Extend `crates/governance` with durable upsert and read methods for `ModelProvider`, `ModelCatalogItem`, `ModelProfile`, and `TenantModelPolicy`.
  - Extend `crates/runtime` with run-scoped `ModelSelectionDecision` recording and read paths.
  - Add targeted Rust tests in `crates/governance` and `crates/runtime`.
  - Update owner docs so the tracked repository state reflects this implemented slice and keeps transport/UI follow-ons deferred.
- Out Of Scope:
  - New shared schema objects, DTOs, commands, queries, or events.
  - New `apps/remote-hub` endpoints, `packages/hub-client` methods, or desktop/web pages.
  - `ProviderAdapter` SPI, provider connectivity, provider built-in tool modeling, `CapabilityResolver` / `ToolSearch` rewiring, tenant/RBAC/IdP work, vector retrieval, or Org Graph work.
- Acceptance Criteria:
  - `crates/governance` can persist and read `ModelProvider`, `ModelCatalogItem`, `ModelProfile`, and `TenantModelPolicy` against the shared SQLite database.
  - `crates/runtime` can record and read one bounded `ModelSelectionDecision` per run without changing external transport contracts.
  - The slice adds no new schema objects and keeps the existing five model-governance contracts as the only cross-language truth.
  - Focused Rust tests prove persistence and runtime recording behavior, and schema-ts contract verification remains green.
- Non-functional Constraints:
  - Keep `schemas/` as the only cross-language contract source of truth.
  - Keep provider/catalog/profile/policy truth inside `crates/governance`.
  - Keep run-scoped decision persistence inside `crates/runtime`.
  - Keep HTTP assembly, shared client transport, and UI consumers unchanged in this slice.
- MVP Boundary:
  - One shared-database migration.
  - Four governance persistence record types.
  - One run-scoped decision record per run.
  - No transport read models or external APIs.
- Human Approval Points:
  - None unless implementation reveals that existing model-governance contracts are insufficient.
- Source Of Truth Updates:
  - Update this task package.
  - Update `README.md`, `docs/README.md`, `docs/architecture/ga-implementation-blueprint.md`, and `docs/tasks/README.md` after implementation and verification complete.
  - Update the predecessor design-only package only as needed to mark it consumed by this implementation slice.
- Affected Modules:
  - `crates/governance`
  - `crates/runtime`
  - `docs/tasks`
  - owner docs in `README.md` and `docs/`
- Affected Layers:
  - Rust persistence layer
  - Rust runtime orchestration layer
  - Task-package and owner-document layer
- Risks:
  - Letting this persistence slice drift into transport or UI scope.
  - Blurring catalog truth with run-scoped decision truth.
  - Introducing storage behavior that implicitly redesigns provider connectivity or capability resolution.
- Validation:
  - `cargo test -p octopus-governance`
  - `cargo test -p octopus-runtime`
  - `pnpm --filter @octopus/schema-ts exec vitest run test/contracts.test.ts`
  - `pnpm --filter @octopus/schema-ts typecheck`
