# Slice 20: Desktop Degraded-State Convergence

This task package defines the post-Slice-19 desktop-only convergence slice for making degraded remote-session semantics visible and consistent across the entire workbench shell.

## Package Files

- [design-note.md](design-note.md)
- [contract-change.md](contract-change.md)
- [implementation-summary.md](implementation-summary.md)
- [verification.md](verification.md)
- [delivery-note.md](delivery-note.md)
- [ga-acceptance-matrix.md](ga-acceptance-matrix.md)

## Task Definition

- Goal:
  - Converge the desktop remote degraded-state experience so restored-but-disconnected, auth-invalid, and secure-storage-memory-only conditions are visible and consistent across the full workbench rather than only in `ConnectionsView`.
- Scope:
  - Create and maintain this Slice 20 task package.
  - Keep owner docs truthful after Slice 19 verification and establish Slice 20 as the next tracked desktop-only slice.
  - Add an explicit app-local degraded-state model in `apps/desktop/src/stores/connection.ts`.
  - Add a global shell banner and read-only explanation path in `apps/desktop/src/App.vue`.
  - Centralize desktop connection-status refresh orchestration so bootstrap restore, profile apply, login, logout, and workbench route entry use one shared path.
  - Keep the existing write gating unchanged while making degraded reasons visible in `Projects / Tasks / Runs / Knowledge / Inbox / Notifications / Automation Detail / RunView`.
- Out Of Scope:
  - Any `schemas/`, `packages/schema-ts`, `packages/hub-client`, or `apps/remote-hub` public-surface change.
  - Refresh tokens, token rotation, RBAC, tenant admin, IdP integration, vector retrieval, Org Knowledge Graph promotion, or Beta-scope expansion.
  - New write actions, new remote routes, or auth DTO redesign.
- Acceptance Criteria:
  - Desktop state distinguishes at least `authenticated`, `auth_required`, `token_expired`, `restored_but_disconnected`, and `memory_only_storage`.
  - A restored remote session that lands on a non-`/connections` route still shows a global degraded/read-only explanation in the shell.
  - Secure-storage-unavailable login shows a global warning outside `ConnectionsView` without blocking sign-in or sign-out.
  - Reconnected/authenticated route refresh clears the degraded banner.
  - Remote profile/workspace changes do not leak prior degraded-state or remembered-project context into the next profile.
  - Auth-invalid restore still falls back to `/connections`.
- Non-functional Constraints:
  - Keep the slice app-local to `apps/desktop` and `apps/desktop/src-tauri`.
  - Reuse existing Vue/Pinia/router patterns and keep Composition API boundaries intact.
  - Preserve current read-only write gating semantics.
- MVP Boundary:
  - One explicit degraded-state model.
  - One global shell banner surface.
  - One shared connection-status refresh orchestration path.
  - No shared-contract or remote-hub expansion.
- Human Approval Points:
  - None.
- Source Of Truth Updates:
  - Update this task package.
  - Update `README.md`, `docs/README.md`, `docs/architecture/ga-implementation-blueprint.md`, and `docs/tasks/README.md` to mark Slice 19 as verified and Slice 20 as the next tracked task package.
  - Add a GA acceptance matrix under this task package and update owner docs only if the matrix changes tracked GA conclusions.
- Affected Modules:
  - `docs/tasks`
  - `docs/architecture`
  - `apps/desktop`
- Affected Layers:
  - Repository documentation / owner docs
  - Desktop app shell
  - Desktop router/bootstrap orchestration
  - Desktop connection store
- Risks:
  - Letting route-local loaders and the new global orchestration compete and create duplicate or stale connection-status updates.
  - Turning a local UX convergence slice into an accidental auth/contract redesign.
  - Leaving degraded-state sticky across profile or workspace transitions.
- Validation:
  - `pnpm --filter @octopus/desktop exec vitest run test/remote-connections.test.ts`
  - `cargo test --workspace`
  - `pnpm test:ts`
  - `pnpm typecheck:ts`
  - `pnpm build:ts`
