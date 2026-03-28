# Slice 15: Project Knowledge Index

This task package records the post-Slice-14 calibration and the Slice 15 implementation that adds a project-scoped Shared Knowledge read index without turning the knowledge surface into a new governance action center.

## Package Files

- [design-note.md](design-note.md)
- [contract-change.md](contract-change.md)
- [implementation-summary.md](implementation-summary.md)
- [verification.md](verification.md)
- [delivery-note.md](delivery-note.md)


## Task Definition

- Goal:
  - Implement a project-scoped Shared Knowledge index read surface and calibrate repository fact sources so tracked documentation stops describing Slice 14 as pending work.
- Scope:
  - Create the Slice 15 task package and keep the slice-local design and contract notes here.
  - Add one new shared read surface: `HubClient.getProjectKnowledge(workspaceId, projectId)`.
  - Add one new shared response: `ProjectKnowledgeIndex`, reusing `KnowledgeSummary` for `entries`.
  - Extend the interop-owned local transport contract with `get_project_knowledge`.
  - Implement the project-scoped read path across `packages/schema-ts`, `packages/hub-client`, `crates/runtime`, `apps/remote-hub`, `apps/desktop/src-tauri`, and `apps/desktop`.
  - Add a dedicated desktop `Knowledge` route, navigation entry, store loader, and read-only view.
  - Calibrate owner docs so Slice 14 is no longer described as the next pending priority.
- Out Of Scope:
  - New governance actions on the knowledge page.
  - Workspace-wide knowledge boards, vector retrieval, Org Graph promotion, tenant / RBAC / IdP expansion, or new retry / terminate run actions.
  - A second knowledge-row DTO separate from `KnowledgeSummary`.
  - New event contracts or push-first knowledge synchronization.
- Acceptance Criteria:
  - `ProjectKnowledgeIndex` validates through the shared schema pipeline and parses through shared TypeScript consumers.
  - `HubClient.getProjectKnowledge(workspaceId, projectId)` works through both local and remote transports.
  - Runtime returns project-scoped knowledge entries ordered by `created_at DESC, id DESC`, without cross-workspace or cross-project leakage.
  - Desktop workbench navigation includes `Knowledge` and preserves `RunView` and `Inbox` as the authoritative governance action surfaces.
  - The desktop knowledge page is read-only, shows project knowledge space plus mixed candidate / asset entries, and links back to `RunView` / `Inbox`.
- Non-functional Constraints:
  - Keep `schemas/` as the only cross-language contract truth.
  - Keep transport layers thin; runtime remains the owner of scoping and aggregation semantics.
  - Keep the page pull-first; existing events may only trigger refetch behavior.
  - Preserve truthful tracked-state docs and do not invent a new post-Slice-15 priority unless a tracked owner doc freezes one.
- MVP Boundary:
  - One project-scoped knowledge index per `workspace_id + project_id`.
  - Read-first surface only: space metadata, mixed entries, trust / provenance, and minimal traceability.
  - No new promotion, approval-resolution, or governance mutation entry points from the knowledge page.
- Human Approval Points:
  - None.
- Source Of Truth Updates:
  - Update this task package.
  - Update shared schemas under `schemas/observe` and `schemas/interop`.
  - Update repository entry docs and owner docs whose current-state summary would otherwise remain inaccurate.
  - Add an ADR only if implementation forces a durable boundary change beyond the current runtime / transport / desktop split.
- Affected Modules:
  - `docs/tasks`
  - `docs/architecture`
  - `schemas/observe`
  - `schemas/interop`
  - `packages/schema-ts`
  - `packages/hub-client`
  - `crates/runtime`
  - `apps/remote-hub`
  - `apps/desktop`
  - `apps/desktop/src-tauri`
- Affected Layers:
  - Cross-language contracts
  - Rust runtime / knowledge query layer
  - Local and remote transport assembly
  - Desktop routing, store, and workbench view composition
  - Repository documentation / owner docs
- Risks:
  - Reintroducing a second knowledge-row DTO instead of reusing `KnowledgeSummary`.
  - Letting the desktop knowledge page absorb promotion or approval actions that must remain on `RunView` and `Inbox`.
  - Repeating outdated owner-doc claims that Slice 14 is still pending or that Slice 15 is implemented before verification proves it.
- Validation:
  - `cargo test --workspace`
  - `pnpm test:ts`
  - `pnpm typecheck:ts`
  - `pnpm --filter @octopus/desktop smoke:local-host`
