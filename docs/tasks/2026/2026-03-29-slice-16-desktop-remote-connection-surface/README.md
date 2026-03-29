# Slice 16: Desktop Remote Connection & Session Surface

This task package freezes the post-Slice-15 next priority as Slice 16, which adds a first-class desktop remote connection and session surface without expanding the current GA workbench IA or inventing a second client contract truth.

## Package Files

- [design-note.md](design-note.md)
- [contract-change.md](contract-change.md)
- [implementation-summary.md](implementation-summary.md)
- [verification.md](verification.md)
- [delivery-note.md](delivery-note.md)

## Task Definition

- Goal:
  - Implement the minimum desktop remote connection and session surface so the tracked desktop shell can select local or remote hub access, authenticate against the existing remote-hub auth routes, and degrade cleanly back to `/connections` when a remote session is missing or expired.
- Scope:
  - Create and maintain the Slice 16 task package and freeze the post-Slice-15 owner-doc priority here.
  - Add `RemoteHubAuthClient` plus `createRemoteHubAuthClient(options)` in `packages/hub-client`, reusing the existing hub auth schemas.
  - Add a desktop connection/profile store that persists only non-secret remote profile data: `mode`, `baseUrl`, `workspaceId`, and `email`.
  - Keep the access token in memory only for this slice and wire it into the shared remote hub client.
  - Update desktop bootstrap so `/` resolves to the local task workbench in local mode, the remote workbench in authenticated remote mode, or `/connections` when remote profile exists without a valid session.
  - Upgrade `ConnectionsView` into the authority surface for local/remote switching, remote login/logout, session display, and auth-aware state messaging.
  - Reuse the existing workbench routes, `HubClient` abstraction, and `hub` store for all read/write surfaces after login.
- Out Of Scope:
  - Secure credential persistence, keychain integration, refresh tokens, or cross-restart remote session retention.
  - New `schemas/` files or changes to existing auth schemas.
  - New desktop page families, a second remote-only DTO layer, or a second remote-only workbench.
  - Push-first live boards, run retry / terminate, workspace admin, tenant / RBAC / IdP expansion, workspace-wide knowledge boards, or Org Graph work.
- Acceptance Criteria:
  - `packages/hub-client` exposes `RemoteHubAuthClient.login`, `RemoteHubAuthClient.getCurrentSession`, and `RemoteHubAuthClient.logout` using the existing shared auth DTOs and auth-aware error normalization.
  - Desktop can persist a local/remote connection profile without persisting the remote access token.
  - Local mode continues to bootstrap into the existing demo task workbench path.
  - Remote mode without a valid in-memory session boots into `/connections` and shows `auth_required` / `token_expired` / `disconnected` distinctly.
  - Remote login reuses the existing workbench IA and shared hub store instead of creating parallel pages or transport-specific DTOs.
  - Remote logout returns the shell to a read-only remote state and does not leave stale token state active.
- Non-functional Constraints:
  - Keep `schemas/` as the only cross-language DTO truth.
  - Keep `apps/remote-hub` thin and unchanged unless tests prove a client-facing expectation mismatch.
  - Keep token storage memory-only in this slice.
  - Preserve truthful owner-doc state: Slice 16 is frozen as next priority here before any implementation claim is made.
- MVP Boundary:
  - One connection authority view.
  - One persisted remote profile shape.
  - One in-memory remote access token.
  - One shared workbench route family for both local and remote modes.
- Human Approval Points:
  - None.
- Source Of Truth Updates:
  - Update this task package.
  - Update `README.md`, `docs/README.md`, and `docs/architecture/ga-implementation-blueprint.md` so Slice 16 is the frozen next priority rather than an implied future candidate.
  - Update `docs/tasks/README.md` to register this package.
  - Add an ADR only if implementation forces a durable new boundary beyond the existing shared client parity rules.
- Affected Modules:
  - `docs/tasks`
  - `docs/architecture`
  - `packages/hub-client`
  - `apps/desktop`
- Affected Layers:
  - Repository documentation / owner docs
  - Shared TypeScript transport/auth client layer
  - Desktop bootstrap, connection state, and workbench surface composition
- Risks:
  - Accidentally persisting secrets or smuggling auth state into app-local DTO truth.
  - Inventing a fake default remote project just to keep the existing task route entrypoint.
  - Letting remote connection work expand into tenant / admin / secure-credential scope.
- Validation:
  - `cargo test --workspace`
  - `pnpm test:ts`
  - `pnpm typecheck:ts`
  - `pnpm --filter @octopus/desktop test -- --runInBand`
