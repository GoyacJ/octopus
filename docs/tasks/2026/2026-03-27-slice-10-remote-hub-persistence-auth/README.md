# Slice 10 Remote Hub Persistence Auth

This task package records the GA slice that adds the first remote-hub authentication and persisted access boundary on top of the verified Slice 9 runtime baseline.

## Package Files

- [design-note.md](design-note.md)
- [contract-change.md](contract-change.md)
- [implementation-summary.md](implementation-summary.md)
- [verification.md](verification.md)
- [delivery-note.md](delivery-note.md)

## Task Definition

- Goal:
  - Implement the minimum remote-hub persistence and auth shell so remote HTTP/SSE access can authenticate, enforce workspace membership, and distinguish transport failure from token/session failure without changing the governed runtime execution shell.
- Scope:
  - Create this task package and keep local design, contract, verification, and delivery notes here.
  - Add a dedicated Rust access/auth crate for remote users, workspace membership, and JWT-backed sessions.
  - Persist remote users, workspace membership, and session records in the remote-hub SQLite database.
  - Add login, authenticated session inspection, logout, token-expiry handling, and route-level membership enforcement for remote-hub reads/writes.
  - Extend shared hub/client contracts and TypeScript consumers for auth-aware connection state.
  - Update repository owner docs so Slice 9 and Slice 10 are treated as completed tracked truth and `minimum automation surface` becomes the next priority.
- Out of Scope:
  - Full tenant or RBAC management surfaces.
  - External IdP, OAuth, refresh-token choreography, or provider-backed secret stores.
  - Minimum automation-management UI or API expansion.
  - Changes to runtime orchestration semantics, approvals, or knowledge-promotion policy flow.
- Acceptance Criteria:
  - Remote-hub exposes a minimum login endpoint that returns a JWT-backed session summary without exposing password material afterward.
  - Existing remote-hub write routes require authentication and reject missing, invalid, expired, or workspace-mismatched tokens.
  - `HubConnectionStatus` can distinguish authenticated access, auth required, token expired, and transport disconnect states.
  - `packages/hub-client` remote mode injects bearer tokens and normalizes auth failures into a typed transport/auth error boundary.
  - Desktop shared-client consumers can surface token-expired versus disconnected behavior without introducing automation-management UI.
- Non-functional Constraints:
  - Keep `RunOrchestrator` as the only execution shell.
  - Keep `apps/remote-hub` as HTTP/SSE assembly only.
  - Keep shared contracts in `schemas/`.
  - Keep the slice SQLite-backed and Rust-first.
- MVP Boundary:
  - One bootstrap remote user and workspace membership model.
  - One access-token JWT session model.
  - No multi-tenant admin surface and no automation-management work.
- Human Approval Points:
  - None.
- Source Of Truth Updates:
  - Update this task package.
  - Update `README.md`, `docs/README.md`, and `docs/architecture/ga-implementation-blueprint.md`.
  - Add an ADR only if the access/auth boundary yields a durable repository-wide rule beyond this slice.
- Affected Modules:
  - `apps/remote-hub`
  - `crates/*` new auth/access crate plus workspace wiring
  - `schemas/interop`
  - `packages/schema-ts`
  - `packages/hub-client`
  - `apps/desktop`
  - `docs/tasks`
  - `docs/architecture`
- Affected Layers:
  - Access and identity
  - Remote surface assembly
  - Shared hub/client contracts
  - Desktop shared-client consumption
- Risks:
  - Accidentally letting auth/session logic leak into runtime orchestration.
  - Treating token expiry as generic transport failure.
  - Allowing cross-workspace access through weak path/token validation.
  - Diverging remote contracts from local client contracts.
- Validation:
  - Add failing Rust remote-hub auth integration tests first.
  - Add failing TypeScript contract and client tests first.
  - Re-run `cargo test --workspace` and `pnpm test:ts` before claiming completion.
