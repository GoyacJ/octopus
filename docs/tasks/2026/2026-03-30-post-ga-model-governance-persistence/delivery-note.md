# Delivery Note

## Delivery Note

- What Changed:
  - Added SQLite-backed persistence for `ModelProvider`, `ModelCatalogItem`, `ModelProfile`, and `TenantModelPolicy` in `crates/governance`.
  - Added a shared runtime migration for model-governance persistence tables plus `model_selection_decisions`.
  - Added run-scoped `ModelSelectionDecisionRecord` recording, readback, and run-report loading in `crates/runtime`.
  - Added focused Rust integration tests and updated owner docs/task indexes to reflect the completed slice and the next queued read-only transport design package.
- Why:
  - The model-governance foundation needs one implemented persistence slice before any transport or surface consumer work can start safely.
- User / System Impact:
  - No external API or UI behavior changed.
  - The repository now has durable Rust-side truth for model-governance records and one bounded decision record per run.
- Risks:
  - Future transport work could still over-expand into write paths or provider connectivity if it does not preserve this slice boundary.
- Rollback Notes:
  - Revert the runtime migration and Rust persistence changes together if the slice must be backed out.
- Follow-ups:
  - Promote the queued read-only transport design package before adding any hub routes, hub-client readers, or desktop model-center consumers.
  - Stop and create a bounded contract-change if any later transport slice proves that the current five contracts are insufficient.
- Docs Updated:
  - Yes. Owner docs, the predecessor design-only package, and this task package were updated to reflect completed persistence and the next queued read-only transport design slice.
- Tests Included:
  - `cargo test -p octopus-governance`
  - `cargo test -p octopus-runtime`
  - `pnpm --filter @octopus/schema-ts exec vitest run test/contracts.test.ts`
  - `pnpm --filter @octopus/schema-ts typecheck`
- ADR Updated:
  - None planned.
- Temporary Workarounds:
  - Read-only consumers remain deferred until a later approved slice.
