# Slice 19: Session Hardening

This task package freezes Slice 19 as the desktop-only session hardening slice, limited to secure remote-session persistence, startup restore, invalidation cleanup, and disconnected degradation for the existing remote connection surface.

## Package Files

- [design-note.md](design-note.md)
- [contract-change.md](contract-change.md)
- [implementation-summary.md](implementation-summary.md)
- [verification.md](verification.md)
- [delivery-note.md](delivery-note.md)

## Task Definition

- Goal:
  - Harden desktop remote session handling so the app can securely restore a valid remote session across restarts, clear invalid cache state deterministically, and degrade read-only when the hub is temporarily unreachable without widening GA scope.
- Scope:
  - Create and maintain this Slice 19 task package and correct owner docs so the tracked baseline is truthful through Slice 18.
  - Add an app-local secure session vault in `apps/desktop/src-tauri` for remote `access_token` plus the last valid `HubSession` summary, bound to `baseUrl + workspaceId + email`.
  - Add app-local Tauri commands for loading, saving, and clearing the secure remote session cache.
  - Extend the desktop connection runtime and bootstrap flow to restore cached remote sessions before route resolution.
  - Keep desktop login, logout, profile apply, and session refresh flows synchronized with the secure session cache.
  - Add degraded/read-only UX and warnings when restore succeeds locally but remote transport validation fails or secure storage is unavailable.
- Out Of Scope:
  - Any `schemas/` updates, `packages/schema-ts` changes, `packages/hub-client` API expansion, or new remote-hub HTTP routes.
  - Refresh tokens, token rotation, remote-hub auth redesign, RBAC, IdP integration, vector retrieval, or Org Knowledge Graph work.
  - Moving `projectId` into secure storage or widening the local profile schema beyond existing non-secret metadata.
- Acceptance Criteria:
  - Desktop login persists the remote access token and session summary through the app-local secure-session hooks when storage is available.
  - Desktop bootstrap restores a non-expired, profile-bound cached session before default-route resolution and revalidates it against the existing remote auth surface.
  - Auth-invalid restore outcomes clear both in-memory and persisted session state and fall back to `/connections`.
  - Transport-only restore failures preserve the cached session summary and remembered project entry while leaving the hub surface explicitly degraded/read-only.
  - If secure storage is unavailable, remote login still works in memory-only mode and the connection surface shows a warning instead of breaking sign-in.
- Non-functional Constraints:
  - Keep the secure-session cache app-local to desktop/Tauri; it is not a cross-surface shared contract.
  - Keep remote-hub auth, shared DTOs, and route-level membership semantics unchanged.
  - Preserve truthful owner-doc state: the current tracked baseline is through Slice 18, and Slice 19 is the next frozen slice.
- MVP Boundary:
  - One secure cache record holding `accessToken + session`.
  - One profile-binding rule on `baseUrl + workspaceId + email`.
  - One degraded/read-only message path in the connection surface.
  - No refresh-token support and no multi-profile secure vault.
- Human Approval Points:
  - None.
- Source Of Truth Updates:
  - Update this task package.
  - Update `README.md`, `docs/README.md`, `docs/architecture/ga-implementation-blueprint.md`, and `docs/tasks/README.md` so the repository truth is through Slice 18 and Slice 19 is the next frozen slice.
  - Add an ADR only if secure storage decisions become durable repository-wide guidance beyond this desktop slice.
- Affected Modules:
  - `docs/tasks`
  - `docs/architecture`
  - `apps/desktop`
  - `apps/desktop/src-tauri`
- Affected Layers:
  - Repository documentation / owner docs
  - Desktop bootstrap and connection runtime
  - Tauri app-local host assembly
  - Desktop UX for remote connection state
- Risks:
  - Accidentally treating an app-local storage hook as a shared contract and widening scope into `schemas/` or remote-hub APIs.
  - Restoring a stale or mismatched token into the wrong remote profile.
  - Misclassifying transport failure as auth failure and needlessly discarding a reusable cached session summary.
- Validation:
  - `pnpm --filter @octopus/desktop exec vitest run test/remote-connections.test.ts`
  - `pnpm --filter @octopus/desktop exec vitest run test/happy-path.test.ts`
  - `pnpm --filter @octopus/hub-client test -- test/hub-client.contract.test.ts`
  - `cargo test --workspace`
  - `pnpm test:ts`
  - `pnpm typecheck:ts`
