# Design Note

## Problem

The tracked GA baseline proves JWT access tokens, persisted remote sessions, desktop secure cache restore, and degraded/read-only handling. It does not yet prove refresh-token support, token rotation, replay detection, or a bounded recovery path once the cached access token expires.

## Goal

Close the remote token lifecycle loop without widening the auth boundary beyond the existing remote-hub shell and desktop connection surface.

## Acceptance Frame

- `login -> access token + refresh token + session summary`
- `expired access -> refresh once -> retry once`
- `rotated refresh replay -> revoke family -> re-login required`
- `logout -> current session family revoked`
- `desktop restart -> local expiry check -> refresh before route resolution when eligible`

## Architecture Decision

- Keep access tokens as short-lived JWTs used by protected routes and SSE.
- Introduce an opaque rotating refresh token that is only exchanged on `/api/auth/refresh`.
- Persist only refresh-token hashes server-side.
- Group refresh chains into a `session family` created per login.
- Treat refresh-token replay as a session anomaly that revokes the active family.
- Keep `HubConnectionStatus.auth_state` unchanged; transport and desktop UX continue to map hard auth failure onto the existing `auth_required` / `token_expired` semantics.

## Flow

1. `POST /api/auth/login` authenticates the user, creates a new session family plus initial refresh record, and returns `access_token + refresh_token + refresh_expires_at + session`.
2. Protected routes continue to accept only bearer access tokens.
3. When access fails with `401/token_expired`, the shared client calls `/api/auth/refresh` once for that remote profile.
4. Refresh validates the presented token against the current active record, atomically marks it rotated, creates the replacement token record, and returns a fresh token pair.
5. The failing request is replayed once with the fresh access token. If it fails again with auth, the client clears tokens and surfaces the stable auth error.
6. If a rotated refresh token is presented again, the service revokes the full session family and all further refresh or protected-route access fails until re-login.
7. Desktop startup restore first checks cached access expiry locally. If access is still valid, it can continue to `/api/auth/session` validation. If access is expired but refresh is still locally valid, it refreshes first and only then resolves the entry route.

## Persistence Model

- Keep `auth_sessions` as the access-token session anchor used by JWT `sid`.
- Add refresh-token persistence that records:
  - token id
  - session family id
  - session id
  - token hash
  - issued_at
  - expires_at
  - rotated_at
  - replaced_by_token_id
  - revoked_at
- Add family-level revocation metadata so replay can invalidate the active chain.

## Error Semantics

- Missing or invalid bearer token still maps to `auth_required`.
- Expired access token still maps to `token_expired`.
- Refresh failure clears auth state only when it is a hard auth failure, not when the transport is temporarily disconnected.
- If refresh-specific differentiation is required internally, it should stay in service/client logic or expand `HubAuthError.error_code` only without changing `HubConnectionStatus.auth_state`.

## Out-of-Scope Guardrails

- No cross-account session management.
- No all-device logout.
- No RBAC, tenant admin, IdP, or SSO.
- No new desktop auth pages.
