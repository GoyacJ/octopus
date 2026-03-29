# Delivery Note

## Summary

Slice 16 is now implemented as the tracked desktop remote connection and session surface. The desktop shell no longer assumes a local-host-only bootstrap path; it can now persist a non-secret remote profile, authenticate against the existing remote-hub auth routes, and reuse the same workbench IA after login without inventing a second client or contract truth.

## Why

By the end of Slice 15, the governed runtime loop, remote transport, remote auth routes, desktop local host, task workbench, and project knowledge index were all already tracked and verified. The main remaining GA gap was that desktop still booted through a fixed local host path and could not treat remote connection/session state as a first-class surface. Slice 16 closes that gap without expanding into secure credential storage, richer run controls, tenant administration, or knowledge-board work.

## User / System Impact

- Desktop now remembers whether the shell should use local or remote hub mode, plus the remote base URL, workspace, and email.
- Remote login and logout now happen from `ConnectionsView` and reuse the existing remote-hub auth routes.
- Authenticated remote sessions reuse the existing workbench surfaces and land on the workspace inbox route rather than a fabricated project route.
- Remote sessions that are missing, expired, or disconnected now surface distinct states on the connection page and keep the workbench read-only when authentication is not valid.
- Local mode continues to land on the existing demo task workbench path.

## Risks

- Remote access tokens remain memory-only, so desktop restarts intentionally require signing in again.
- The remote default landing route is workspace-scoped inbox, because the stored profile does not persist a canonical project identifier in this slice.
- The workbench remains pull-first; Slice 16 does not introduce live remote boards, retry/terminate controls, or richer remote-only state sync.

## Rollback Notes

- Roll back the desktop connection store, bootstrap route resolution, and `ConnectionsView` changes together; partial rollback would leave the shell with mismatched route/bootstrap assumptions.
- Roll back `RemoteHubAuthClient` together with the desktop remote-auth flow that consumes it; the shared workbench client should remain unchanged.

## Follow-ups

- Secure credential persistence or restart-stable remote login should be promoted through a separate slice dedicated to keychain/secure-store behavior.
- Any future remote-first run controls, workspace admin, or knowledge-board surfaces should remain separate from Slice 16 so this slice stays focused on connection/session parity only.

## Docs Updated

- Updated `README.md`, `docs/README.md`, `docs/architecture/ga-implementation-blueprint.md`, and `docs/tasks/README.md` to freeze Slice 16 as the next formal priority.
- Completed the Slice 16 task package with implementation, verification, and delivery artifacts.

## Tests Included

- `packages/hub-client` contract coverage for auth login/session/logout and auth-aware error normalization.
- Desktop remote-connection coverage for unauthenticated bootstrap, login, logout, token-expiry handling, and local/remote switching.
- Full workspace Rust and TypeScript gate verification.

## ADR Updated

- No new ADR was required; the existing schema-first, shared-client, and desktop/local-vs-remote boundary decisions remain sufficient for this slice.

## Temporary Workarounds

- Remote sessions do not survive application restart because secure token persistence is intentionally out of scope for Slice 16.
- Remote authenticated bootstrap lands on workspace inbox until a later, explicitly tracked slice decides whether desktop should persist richer remote project-selection state.
