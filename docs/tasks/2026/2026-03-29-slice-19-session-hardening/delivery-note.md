# Delivery Note

## Summary

Slice 19 hardens the desktop remote-session path without widening GA scope. Remote login can now persist through an app-local secure vault, desktop startup can restore a valid cached session before route resolution, and disconnected restores degrade to read-only instead of masquerading as a fresh authenticated login.

## Why

By the end of Slice 18, the repository already proved remote desktop login, remembered project entry, and run control. The remaining operator gap was restart resilience: desktop restarts discarded the in-memory token, while the UI could not distinguish auth-invalid restores from temporary transport loss. Slice 19 closes that gap strictly inside the desktop/Tauri boundary.

## User / System Impact

- Desktop remote sessions can survive app restart when OS-backed secure storage is available.
- Cached remote sessions are validated locally and then revalidated remotely before being treated as active.
- Auth-invalid cached sessions are cleared automatically.
- Transport-only restore failures preserve last-known session context and remembered project entry, but keep the surface degraded/read-only.
- Secure-store failures no longer block remote login; the app falls back to memory-only mode with an explicit warning.

## Risks

- Secure-session persistence remains single-profile and app-local; broader multi-profile or multi-tenant handling remains future work.
- Degraded/read-only mode still depends on the existing connection-status refresh cadence rather than streaming state push.
- OS keychain behavior may vary by platform; the memory-only fallback is the compatibility path when secure storage is unavailable.

## Rollback Notes

- Roll back the Tauri secure-session vault, desktop runtime hooks, bootstrap restore ordering, and connection-surface messaging together; partial rollback would leave desktop restore logic calling commands that no longer exist.
- Do not roll back only the owner docs without reverting the code, or the repository truth would again drift from the tracked implementation state.

## Follow-ups

- If refresh tokens or token rotation are later approved, they must be defined as a new slice with explicit remote-hub auth and contract scope.
- If secure-session policy becomes common across more surfaces, promote the rule via ADR instead of duplicating app-local conventions.
- Degraded remote mode UX beyond the connection surface now continues in [Slice 20: Desktop Degraded-State Convergence](../2026-03-29-slice-20-desktop-degraded-state-convergence/README.md) rather than widening Slice 19 retroactively.

## Docs Updated

- Updated `README.md`, `docs/README.md`, `docs/architecture/ga-implementation-blueprint.md`, and `docs/tasks/README.md` so the tracked truth is through Slice 18 and Slice 19 is the next frozen slice.
- Added the Slice 19 task package with design, contract, implementation, verification, and delivery records.

## Tests Included

- Desktop connection-store and surface tests for secure-session persistence and restore behavior.
- Tauri host tests for secure-cache validation and cleanup behavior.
- Full Rust and TypeScript workspace regression gates.

## ADR Updated

- No new ADR was required; the decision remains local to the desktop/Tauri slice.

## Temporary Workarounds

- Refresh tokens and token rotation remain intentionally out of scope.
- `projectId` remains outside the secure vault and stays in non-secret local profile storage.
