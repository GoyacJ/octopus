# Delivery Note

## Summary

This slice adds the first post-GA remote token-lifecycle hardening pass. It stays inside the approved scope: refresh-token support, rotating refresh persistence, transport-managed retry, and desktop secure-restore closure, without widening into RBAC, IdP, tenant admin, or collaboration work.

## Why

The GA baseline proves remote login and secure restore, but it still forces a full re-login whenever the cached access token expires. The next bounded improvement is to close that lifecycle gap without widening into broader identity or tenant work.

## What Changed

- Added schema-first refresh contracts under `schemas/interop`, regenerated the shared TypeScript contract surface, and extended `HubLoginResponse` to return `refresh_token`.
- Hardened `crates/access-auth` with session-family-based rotating refresh tokens, server-side refresh hash storage, replay-triggered family revocation, logout family revocation, and a dedicated refresh-token migration.
- Added `POST /api/auth/refresh` to `apps/remote-hub`, while keeping the app layer as HTTP assembly over the auth crate.
- Extended `packages/hub-client` so remote HTTP requests and SSE subscriptions perform at most one refresh attempt plus one replay/reconnect, with per-client in-flight refresh coordination and hard-auth cleanup.
- Completed the desktop restore path in `apps/desktop` and `apps/desktop/src-tauri` so secure vault persistence now carries `access_token + refresh_token + refresh expiry + session summary`, startup can refresh before route resolution, hard-auth failure clears secure and in-memory state, and transport-only failures preserve the existing degraded/read-only path.

## Risks

- Refresh orchestration can accidentally introduce retry loops if not tightly bounded.
- Session-family revocation must be atomic enough that replay is treated as a hard failure.
- Desktop must not clear valid refresh state on transport-only failures.

## Verification Status

- Targeted verification passed for remote-hub auth surface, schema-ts contracts, hub-client transport behavior, desktop remote-connections behavior, and desktop secure-vault persistence.
- Workspace gates passed: `cargo test --workspace`, `pnpm test:ts`, and `pnpm typecheck:ts`.

## Temporary Workarounds / Residuals

- The refreshed access-token issuance path currently clamps the renewed session TTL to at least one second inside the auth crate so negative-TTL test fixtures can still verify refresh behavior deterministically.
- The Rust workspace still emits non-blocking `dead_code` warnings for a few persisted auth-record fields that remain intentionally present in SQL-backed models.

## Follow-ups

- Multi-device/account session administration remains future work.
- RBAC, IdP, and broader remote admin remain out of scope.
- Any durable auth-governance conclusion beyond this slice should be considered for ADR promotion later.
