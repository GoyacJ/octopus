# Delivery Note

## What Changed

- Added a desktop-local connection-state model that distinguishes authenticated, auth-required, token-expired, restored-but-disconnected, and memory-only secure-storage states.
- Added a global shell banner so workbench routes can explain read-only or degraded conditions without sending the operator back to `ConnectionsView`.
- Centralized connection-status refresh orchestration in desktop routing and removed route-local `loadConnectionStatus()` decisions from the main workbench views.
- Added Slice 20 regression coverage for the new global degraded-state semantics.

## Why

- Slice 19 proved secure restore and degraded read-only semantics, but the explanation still lived mainly on the connection page. Operators could land directly in `Projects`, `Tasks`, or other workbench routes and see read-only behavior without a shell-level reason.

## User / System Impact

- Remote degraded state is now visible across the desktop workbench shell.
- Recovered connectivity clears the degraded banner on the next route-entry refresh.
- Memory-only secure storage is visible outside the connection page.
- Existing write blocking remains unchanged; the slice only improves state visibility and consistency.

## Risks

- Router-level refresh now participates in more navigation paths, so future route work should not reintroduce view-local connection-status orchestration.
- The banner model is app-local by design; future auth-surface expansion must not treat it as a shared contract.

## Rollback Notes

- Roll back the connection-store state model, route-entry refresh hook, shell banner, and view-loader cleanup together.
- Do not partially restore route-local connection refresh calls while keeping the global banner model; that would reintroduce drift in status semantics.

## Follow-ups

- Freeze the post-GA backlog unless a tracked GA acceptance gap is discovered.
- If approved later, handle refresh token / token rotation, remote admin / tenant / IdP, and deeper desktop remote UX only through new task packages.

## Docs Updated

- Added the Slice 20 task package and GA acceptance matrix.
- Updated the Slice 19 package closeout references so it points to post-Slice-19 work instead of remaining the implied next slice.
- Updated `README.md`, `docs/README.md`, `docs/architecture/ga-implementation-blueprint.md`, and `docs/tasks/README.md` so Slice 19 is verified, Slice 20 is the current verified baseline, and post-GA backlog expansion is frozen by default.

## Tests Included

- Targeted desktop remote-connection regression coverage for global degraded-state convergence.
- `cargo test --workspace`
- `pnpm test:ts`
- `pnpm typecheck:ts`
- `pnpm build:ts`

## ADR Updated

- No new ADR was required.

## Temporary Workarounds

- Refresh token / token rotation remains intentionally out of scope.
- Remote degraded-state recovery still depends on route-entry refresh rather than push-based transport health events.
