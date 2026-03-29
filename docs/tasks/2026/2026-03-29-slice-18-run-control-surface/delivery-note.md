# Delivery Note

## Summary

Slice 18 is now implemented as the minimum cross-surface run control slice. Retry and terminate are exposed through shared schemas, the local and remote hub transports, the remote-hub HTTP surface, the Tauri desktop host, and the desktop `RunView`, while keeping session hardening and other deferred GA scope out of this cut.

## Why

By the end of Slice 17, the governed SQLite runtime loop, remote auth and persistence shell, local desktop host, and workspace/project read surfaces were already tracked and verified. The main remaining operator gap was that retry and terminate existed in the runtime core but were not formally exposed through shared contracts or either desktop/remote surface. Slice 18 closes that gap without widening into bulk actions, session hardening, or new run DTO shape.

## User / System Impact

- Operators can now retry a retryable failed run directly from desktop `RunView`.
- Operators can now terminate an allowed in-flight or failed run directly from desktop `RunView`.
- Local and remote transports now share the same command schemas and return the same authoritative `RunDetail` shape after mutation.
- Remote-hub now enforces auth, workspace membership, and path/body consistency for run control commands instead of relying on runtime-only behavior.
- Automation-backed terminate now keeps `TriggerDelivery` state aligned with run state through the public runtime surface.

## Risks

- Run control remains pull-refresh based; Slice 18 does not add streaming or optimistic UI updates.
- Terminate currently records the fixed reason `desktop_operator_stopped`; richer operator input remains deferred.
- Controls remain detail-view only; operators still cannot act from the project runs list or perform bulk actions.

## Rollback Notes

- Roll back the schema, client, remote-hub, Tauri, and desktop `RunView` changes together; partial rollback would leave one or more surfaces advertising commands that another layer no longer understands.
- Roll back the runtime `terminate_run(...)` delivery synchronization together with the surface routes that depend on the authoritative `RunDetail` response after terminate.

## Follow-ups

- Session hardening and secure token persistence remain the next explicit candidate slice after Slice 18.
- Any future list-level or bulk run controls should be introduced as a separate slice so `RunSummary` widening and list ergonomics stay explicit.
- If remote run control later requires push updates, that work should be coordinated with the existing workspace/run event surface rather than added ad hoc.

## Docs Updated

- Updated `README.md`, `docs/README.md`, `docs/architecture/ga-implementation-blueprint.md`, and `docs/tasks/README.md` to freeze Slice 18 as the next formal priority.
- Completed the Slice 18 task package with implementation, verification, and delivery artifacts.

## Tests Included

- Shared schema parser and local-transport command-map coverage.
- Shared hub-client contract coverage for local and remote retry/terminate commands.
- Runtime coverage for retry/terminate behavior and automation delivery synchronization.
- Remote-hub HTTP/auth coverage for run control success, auth, workspace-forbidden, and invalid-transition branches.
- Desktop `RunView` coverage for retry, terminate, read-only disablement, refresh, and error-banner behavior.
- Full Rust workspace and TypeScript workspace gate verification.

## ADR Updated

- No new ADR was required; the existing schema-first, shared-client, runtime-governance, and desktop remote-connection boundaries remain sufficient for this slice.

## Temporary Workarounds

- Session hardening and secure token persistence remain intentionally deferred.
- `Terminate Run` continues to use a fixed operator reason constant until a later tracked slice adds confirmation or richer audit input.
