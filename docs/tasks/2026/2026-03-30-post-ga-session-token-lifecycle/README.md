# Post-GA: Remote Session Token Lifecycle Hardening

This task package freezes the first post-GA auth hardening slice after the Slice 20 GA acceptance baseline. The slice is strictly limited to remote session token lifecycle closure across `schemas/`, `crates/access-auth`, `apps/remote-hub`, `packages/hub-client`, `apps/desktop`, and `apps/desktop/src-tauri`.

## Closeout Status

- In progress.
- This slice is the approved post-GA follow-on to the Slice 20 GA baseline.

## Package Files

- [design-note.md](design-note.md)
- [contract-change.md](contract-change.md)
- [verification.md](verification.md)
- [delivery-note.md](delivery-note.md)

## Task Definition

- Goal:
  - Add a bounded refresh-token and session-family hardening slice so remote desktop sessions can refresh, rotate, recover, and revoke safely without widening beyond the current remote auth boundary.
- Scope:
  - Create and maintain this post-GA task package and the minimum owner-doc updates needed to register the slice.
  - Extend `schemas/interop` with refresh-token input/output contracts and the login-response refresh fields.
  - Extend `crates/access-auth` with refresh-token persistence, session-family semantics, one-time token rotation, replay-triggered family revocation, and configurable refresh TTL.
  - Add `POST /api/auth/refresh` to `apps/remote-hub` while keeping auth orchestration in Rust services and HTTP assembly in the app shell.
  - Extend `packages/hub-client` so refresh is transport-managed for request/replay and SSE reconnect.
  - Extend `apps/desktop` and `apps/desktop/src-tauri` so startup restore can use a valid refresh token, hard auth failure clears secure state, and degraded transport handling stays separate from auth failure.
- Out Of Scope:
  - RBAC, tenant administration, external IdP, SSO, Org Graph, vector retrieval, or any new Beta collaboration surface.
  - New desktop page families or a wider redesign of `Connections` / banner / read-only semantics.
  - New workspace-management or account-management APIs.
- Acceptance Criteria:
  - Login returns both `access_token` and `refresh_token`, plus refresh-expiry metadata and the existing `HubSession`.
  - A valid refresh token can rotate once and produce a fresh token pair after the access token expires.
  - Refresh-token replay after rotation revokes the current session family and forces full re-login.
  - Remote client request paths refresh at most once and replay at most once per failing operation.
  - Desktop restart can recover from `expired access + valid refresh`, while revoked or expired refresh state clears secure cache and returns to `/connections`.
- Non-functional Constraints:
  - Keep `apps/remote-hub` as HTTP/SSE assembly only.
  - Keep shared contract truth in `schemas/`.
  - Keep raw tokens out of browser local storage; only remote-hub responses and the desktop secure vault may hold token plaintext.
  - Keep `HubConnectionStatus.auth_state` limited to `authenticated | auth_required | token_expired`.
- MVP Boundary:
  - One session family per login.
  - One rotating opaque refresh-token chain with server-side hash storage.
  - One in-flight refresh coordinator per remote profile in the shared client.
  - One startup-restore path that prefers refresh before route resolution when access has expired locally.
- Human Approval Points:
  - None beyond the approved implementation plan unless scope needs to expand.
- Source Of Truth Updates:
  - Update this task package.
  - Update `README.md`, `docs/README.md`, and `docs/tasks/README.md`.
  - Optionally update `docs/architecture/ga-implementation-blueprint.md` only to register the next post-GA priority without changing PRD or SAD semantics.
- Affected Modules:
  - `schemas/interop`
  - `packages/schema-ts`
  - `crates/access-auth`
  - `apps/remote-hub`
  - `packages/hub-client`
  - `apps/desktop`
  - `apps/desktop/src-tauri`
- Affected Layers:
  - Shared contract layer
  - Rust auth and persistence layer
  - Remote hub HTTP assembly
  - Shared TypeScript transport layer
  - Desktop/Tauri restore and secure-session layer
- Risks:
  - Over-widening the slice into account management or auth-state UX redesign.
  - Accidentally allowing duplicate refresh attempts or infinite replay loops.
  - Treating transport disconnect as hard auth failure and clearing reusable session state.
- Validation:
  - `cargo test --workspace`
  - `pnpm test:ts`
  - `pnpm typecheck:ts`
