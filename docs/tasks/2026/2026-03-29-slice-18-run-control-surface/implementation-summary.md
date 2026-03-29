# Implementation Summary

## What Changed

- Froze Slice 18 as the formal post-Slice-17 priority across:
  - `README.md`
  - `docs/README.md`
  - `docs/architecture/ga-implementation-blueprint.md`
  - `docs/tasks/README.md`
  - this Slice 18 task package
- Added schema-first run control commands under `schemas/runtime/`:
  - `run-retry-command.schema.json`
  - `run-terminate-command.schema.json`
- Extended `schemas/interop/local-hub-transport.schema.json` and `schemas/interop/local-hub-transport.json` with:
  - `retry_run`
  - `terminate_run`
- Extended `packages/schema-ts` to export:
  - `RunRetryCommand`
  - `RunTerminateCommand`
  - `parseRunRetryCommand(...)`
  - `parseRunTerminateCommand(...)`
  - updated local-hub transport command typing for the two new command keys
- Extended `packages/hub-client` so both local and remote adapters now expose:
  - `retryRun(command): Promise<RunDetail>`
  - `terminateRun(command): Promise<RunDetail>`
  - the remote adapter maps these to `POST /api/runs/:run_id/retry` and `POST /api/runs/:run_id/terminate`
- Extended `apps/remote-hub` with authenticated run control surface routes:
  - `POST /api/runs/{run_id}/retry`
  - `POST /api/runs/{run_id}/terminate`
  - both routes enforce workspace membership and reject path/body mismatch with explicit `400`
- Extended `apps/desktop/src-tauri` with:
  - `hub_retry_run`
  - `hub_terminate_run`
  - matching local transport command names `hub:retry_run` and `hub:terminate_run`
- Updated `crates/runtime` so public `terminate_run(...)` now mirrors `retry_run(...)` and reloads the authoritative `RunExecutionReport` after synchronizing automation `TriggerDelivery` state.
- Extended the desktop hub store and `RunView`:
  - new run mutation helpers for retry/terminate
  - fixed terminate reason constant `desktop_operator_stopped`
  - post-mutation refresh of current run, project runs, workspace inbox, workspace notifications, and project knowledge index
  - minimal `Retry Run` and `Terminate Run` buttons only in `RunView`
- Added or updated contract, runtime, remote-hub, and desktop coverage for Slice 18 in:
  - `packages/schema-ts/test/contracts.test.ts`
  - `packages/hub-client/test/hub-client.contract.test.ts`
  - `crates/runtime/tests/schema_contracts.rs`
  - `crates/runtime/tests/slice1_runtime.rs`
  - `crates/runtime/tests/slice3_automation.rs`
  - `apps/remote-hub/tests/http_surface.rs`
  - `apps/remote-hub/tests/auth_surface.rs`
  - `apps/desktop/test/happy-path.test.ts`

## Key Decisions Preserved

- `RunDetail` remains the single authority response for run control mutations; `Run`, `RunSummary`, and `RunDetail` were not widened for list-level actions.
- Slice 18 stays scoped to `retry` and `terminate`; approval-driven `run.resume` remains on the existing Inbox/Approval path.
- `Terminate Run` still uses the fixed audit reason `desktop_operator_stopped`; no confirm modal or free-form reason entry was introduced.
- Run control surface remains detail-view only in `RunView`; `RunsView` and list/bulk actions remain out of scope.
- Session hardening, secure token persistence, tenant/RBAC work, vector retrieval, workspace boards, and chat/streaming remain explicitly out of scope.

## Notable Follow-through

- `apps/remote-hub` coverage now proves success and rejection cases for run control, including completed-run terminate rejection, path/body mismatch, auth-required, and workspace-forbidden branches.
- Desktop run control coverage now proves authenticated happy paths, read-only disablement, refresh behavior, and error-banner behavior without local optimistic drift.
- While closing workspace gates, the desktop remote-connection suite exposed a stale session-fixture expiry timestamp; that fixture was updated to a stable far-future value so route-resolution tests no longer fail based on wall-clock date rather than behavior.
