# GA Foundation Repo Skeleton

This task package records the first controlled implementation step for the GA rebuild: initialize the monorepo skeleton, freeze shared-contract ownership in `schemas/`, and prepare the repository for `Slice 1` without claiming runtime functionality already exists.

## Package Files

- [../../../../decisions/0002-json-schema-source-of-truth-and-generation-boundary.md](../../../../decisions/0002-json-schema-source-of-truth-and-generation-boundary.md)
- [design-note.md](design-note.md)
- [contract-change.md](contract-change.md)
- [implementation-summary.md](implementation-summary.md)
- [verification.md](verification.md)
- [delivery-note.md](delivery-note.md)

## Task Definition

- Goal: Establish the initial monorepo skeleton and shared-contract foundation required to start GA development without skipping schema-first or repo-boundary controls.
- Scope:
  - Create the task package and related ADR.
  - Add root workspace manifests for Rust and pnpm governance.
  - Initialize `apps/`, `crates/`, `packages/`, and `schemas/` as tracked top-level directories.
  - Add placeholder JSON Schemas for the first-priority GA object set and required strong state enums.
  - Update entry docs whose current-state statements would become inaccurate after this change.
- Out of Scope:
  - Runtime logic, orchestration, adapters, UI pages, or MCP execution.
  - Full field-level schema design for GA objects.
  - Beta-only contracts such as `DiscussionSession`, `ResidentAgentSession`, `A2A`, or Org Graph promotion flows.
  - Consumer generation pipelines, codegen tooling, or first concrete crate/package members.
- Acceptance Criteria:
  - The repository contains tracked root manifests plus top-level skeleton directories.
  - Each new top-level directory that needs local AI guidance has a local `AGENTS.md`, and placeholder subdirectories are not padded with unnecessary instruction files.
  - `schemas/` contains parseable placeholder JSON Schemas for the planned GA object groups and required state enums.
  - An ADR records JSON Schema as the source-of-truth format and defines generation as downstream-only.
  - Root entry docs and architecture notes no longer claim that the top-level implementation skeleton is absent.
- Non-functional Constraints:
  - Preserve doc-first truthfulness.
  - Do not imply runnable implementation where none exists.
  - Keep links relative inside repository docs.
  - Keep contracts intentionally minimal and reviewable.
- MVP Boundary: This task stops at skeleton initialization, contract-source freezing, and placeholder schemas. It does not enter `Slice 1` behavior or member implementation.
- Human Approval Points: None beyond the already approved implementation plan.
- Source Of Truth Updates:
  - Task package under `docs/tasks/`
  - ADR under `docs/decisions/`
  - Entry-state updates in `README.md`, `AGENTS.md`, and `docs/architecture/SAD.md`
- Affected Modules:
  - Repository root governance
  - Documentation indexes
  - Top-level monorepo skeleton
  - Shared contract source layout
- Affected Layers:
  - Documentation
  - Repo structure
  - Shared contract layer
- Risks:
  - Overstating skeleton files as implementation.
  - Freezing overly detailed schemas too early.
  - Leaving stale tracked-state wording in owner docs.
- Validation:
  - Confirm expected directories and files exist.
  - Parse JSON Schemas.
  - Parse root manifests.
  - Review the diff against scope and exclusions.
