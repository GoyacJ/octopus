# Slice 17: Desktop Project Scope Entry

This task package freezes the post-Slice-16 next priority as Slice 17, which adds workspace-scoped project discovery, selection, memory, and entry for the desktop shell without changing the current remote login semantics or inventing new admin flows.

## Package Files

- [design-note.md](design-note.md)
- [contract-change.md](contract-change.md)
- [implementation-summary.md](implementation-summary.md)
- [verification.md](verification.md)
- [delivery-note.md](delivery-note.md)

## Task Definition

- Goal:
  - Implement the minimum desktop project-scope entry surface so authenticated remote users can discover existing projects in the current workspace, pick one, remember it locally, and re-enter the tracked workbench through the existing project-scoped task routes.
- Scope:
  - Create and maintain the Slice 17 task package and freeze the post-Slice-16 owner-doc priority here.
  - Calibrate owner docs so Slice 16 is described as the verified baseline instead of a pending target.
  - Add one shared read surface: `HubClient.listProjects(workspaceId)`.
  - Extend the interop-owned local transport contract with `list_projects`.
  - Reuse the existing `Project` schema as the only shared DTO for project list items.
  - Extend `crates/domain-context`, `crates/runtime`, `apps/remote-hub`, `apps/desktop/src-tauri`, `packages/schema-ts`, and `packages/hub-client` with workspace-scoped project list parity.
  - Add a dedicated desktop `Projects` route and view for authenticated remote mode without a remembered project.
  - Extend the desktop connection/profile store with optional non-secret `projectId` memory while keeping remote token storage memory-only.
- Out Of Scope:
  - Changes to remote login semantics, workspace discovery, project create/edit/delete, run retry/terminate, secure token persistence, RBAC, IdP, tenant administration, or Org Graph promotion.
  - New shared schemas for login/session or a second project row DTO separate from `Project`.
  - Moving project selection into `ConnectionsView`.
- Acceptance Criteria:
  - `HubClient.listProjects(workspaceId)` works through both local and remote transports and validates against the existing shared `Project` schema.
  - Runtime returns workspace-scoped projects ordered by `updated_at DESC, id DESC` without leaking other workspaces.
  - `apps/remote-hub` exposes `GET /api/workspaces/:workspaceId/projects` and enforces existing auth and workspace membership semantics.
  - Desktop remote mode without a remembered project lands on `/workspaces/:workspaceId/projects`.
  - Selecting a project enters `/workspaces/:workspaceId/projects/:projectId/tasks` and persists the chosen `projectId` in the desktop connection profile.
  - Remembered remote project selection restores direct entry into the project workbench on later login within the same running app session.
  - Stale remembered project ids degrade cleanly by clearing the local memory and returning the user to the `Projects` surface.
- Non-functional Constraints:
  - Keep `schemas/` as the only cross-language contract truth.
  - Keep transport layers thin; runtime owns scoping and ordering semantics.
  - Keep `ConnectionsView` focused on connection/session authority only.
  - Preserve truthful owner-doc state for Slice 16 and Slice 17.
- MVP Boundary:
  - One workspace-scoped project list.
  - One `Projects` page.
  - One optional persisted `projectId` on the desktop connection profile.
  - One bootstrap rule that distinguishes local mode, remote authenticated with remembered project, remote authenticated without remembered project, and remote missing session.
- Human Approval Points:
  - None.
- Source Of Truth Updates:
  - Update this task package.
  - Update `README.md`, `docs/README.md`, `docs/architecture/ga-implementation-blueprint.md`, and `docs/tasks/README.md` so Slice 16 is described as the verified baseline and Slice 17 is the frozen next priority, then as implemented once verification passes.
  - Add an ADR only if implementation forces a durable architecture change beyond the existing runtime / transport / desktop split.
- Affected Modules:
  - `docs/tasks`
  - `docs/architecture`
  - `schemas/interop`
  - `packages/schema-ts`
  - `packages/hub-client`
  - `crates/domain-context`
  - `crates/runtime`
  - `apps/remote-hub`
  - `apps/desktop`
  - `apps/desktop/src-tauri`
- Affected Layers:
  - Repository documentation / owner docs
  - Cross-language contracts
  - Rust context/runtime query layer
  - Local and remote transport assembly
  - Desktop routing, persistence, and workbench view composition
- Risks:
  - Letting project selection bleed into connection/session semantics.
  - Inventing a second project DTO or desktop-only transport truth.
  - Keeping stale remembered project state and silently routing users into broken project scopes.
- Validation:
  - `cargo test --workspace`
  - `pnpm test:ts`
  - `pnpm typecheck:ts`
