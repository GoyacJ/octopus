# Delivery Note

## Summary

Slice 17 is now implemented as the tracked desktop project-scope entry surface. The desktop shell can authenticate into a remote workspace, discover existing projects in that workspace, remember a non-secret project choice locally, and re-enter the existing project-scoped task workbench without changing remote login semantics or inventing new admin flows.

## Why

By the end of Slice 16, desktop could already switch between local and remote hub modes, authenticate against remote-hub, and reuse the shared workbench surfaces. What remained missing was the truthful entry step between “remote session exists” and “project-scoped task routes can open”: desktop had no shared way to list current-workspace projects or remember which project to reopen. Slice 17 closes that gap while keeping auth, workspace discovery, and project CRUD out of scope.

## User / System Impact

- Authenticated remote desktop users without a remembered project now land on `ProjectsView`.
- Selecting a project routes into the existing `/workspaces/:workspaceId/projects/:projectId/tasks` workbench and persists the chosen `projectId`.
- Remembered project ids are reused on later login within the same persisted profile.
- Stale remembered project ids now self-heal by clearing local memory and returning the shell to the workspace project list.
- Local mode keeps the existing demo workbench path.

## Risks

- Remembered project selection is still local profile state, not secure identity state; it should not be treated as authorization truth.
- Direct entry into remembered remote projects still depends on runtime truth being checked on demand from the task route.
- The remote shell remains pull-first; Slice 17 does not add run retry/terminate controls or broader remote state sync.

## Rollback Notes

- Roll back the shared `listProjects` contract, remote route, Tauri mapping, and desktop routing changes together; partial rollback would leave local/remote project-entry behavior inconsistent.
- Roll back the connection-profile `projectId` memory together with the `ProjectsView` bootstrap branch; otherwise desktop would retain a remembered project that no longer has a route owner.

## Follow-ups

- The next candidate priority returns to run control surface, with session hardening after that; both require new task-package freezes before implementation.
- Secure token persistence, workspace discovery, and project CRUD remain separate future slices.

## Docs Updated

- Updated `README.md`, `docs/README.md`, `docs/architecture/ga-implementation-blueprint.md`, and `docs/tasks/README.md` to record Slice 16 and Slice 17 as verified baseline truth.
- Completed the Slice 17 task package with implementation, verification, and delivery artifacts.

## Tests Included

- Rust coverage for workspace-scoped project listing across context, runtime, remote-hub auth enforcement, and desktop local host transport.
- TypeScript coverage for shared `HubClient.listProjects` parity and desktop remote project-selection / remembered-entry behavior.
- Full workspace Rust and TypeScript gate verification.

## ADR Updated

- No new ADR was required; existing schema-first, runtime-owned scoping, and desktop local-vs-remote boundary decisions remain sufficient for this slice.

## Temporary Workarounds

- Remote access tokens remain memory-only; desktop restarts still require signing in again.
- Remembered project validation happens when the project-scoped workbench loads, not through a separate preflight cache or second read model.
