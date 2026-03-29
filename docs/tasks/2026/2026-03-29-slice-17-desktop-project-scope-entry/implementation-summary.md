# Implementation Summary

## What Changed

- Calibrated tracked owner docs so Slice 16 is recorded as verified baseline truth and Slice 17 is recorded as the implemented desktop project-scope entry slice across:
  - `README.md`
  - `docs/README.md`
  - `docs/architecture/ga-implementation-blueprint.md`
  - `docs/tasks/README.md`
  - this Slice 17 task package
- Extended the interop-owned local hub transport contract with one additive command:
  - `hub:list_projects`
- Added shared client parity for workspace-scoped project listing:
  - `HubClient.listProjects(workspaceId): Promise<Project[]>`
  - local and remote transport adapters both normalize against the existing shared `Project` schema
- Extended `crates/domain-context` and `crates/runtime` with a read-only workspace-scoped project list surface ordered by `updated_at DESC, id DESC`.
- Added `GET /api/workspaces/:workspaceId/projects` to `apps/remote-hub` and kept existing auth plus workspace-membership enforcement intact.
- Added the matching `hub:list_projects` Tauri mapping in `apps/desktop/src-tauri` so desktop local and remote modes stay transport-parity aligned.
- Adjusted the desktop shell so:
  - `ConnectionsView` remains connection/session authority only
  - new `ProjectsView` owns workspace-scoped project discovery and entry
  - remote authenticated bootstrap without a remembered project lands on `/workspaces/:workspaceId/projects`
  - remote authenticated bootstrap with a remembered project lands on `/workspaces/:workspaceId/projects/:projectId/tasks`
  - stale remembered project ids are cleared and degraded back to `ProjectsView`
- Extended the desktop connection profile with an optional non-secret `projectId` while keeping remote access tokens memory-only.

## Key Decisions Preserved

- `schemas/` remains the only cross-language contract truth; Slice 17 reuses the existing `Project` schema instead of inventing a second project row DTO.
- Runtime owns workspace scoping and ordering semantics; transport layers stay thin and only forward the shared query.
- `ConnectionsView` still does not absorb project selection; project choice happens only after authenticated workspace entry.
- Remote session semantics remain `workspace_id + email + password`; Slice 17 does not widen auth, workspace discovery, or admin scope.

## Notable Follow-through

- Desktop navigation now exposes `Projects` whenever a workspace scope exists, while `Tasks / Runs / Knowledge` only appear when a current project scope exists.
- Remembered remote project selection is intentionally stored as optional profile metadata instead of a secret, and it is cleared when runtime truth proves the project no longer exists in the workspace.
- Local mode remains on the existing demo workbench path; Slice 17 only adds the remote project-entry branch.
