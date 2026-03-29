# Slice 18: Run Control Surface

This task package freezes Slice 18 as the minimum run control surface across runtime contracts, local and remote transport, and the desktop `RunView`, limited to `retry` and `terminate`.

## Package Files

- [design-note.md](design-note.md)
- [contract-change.md](contract-change.md)
- [implementation-summary.md](implementation-summary.md)
- [verification.md](verification.md)
- [delivery-note.md](delivery-note.md)

## Task Definition

- Goal:
  - Implement the minimum cross-surface run control chain so desktop users can retry failed runs and terminate allowed in-flight runs through the shared `HubClient` boundary without changing run DTO truth or broadening GA scope.
- Scope:
  - Create and maintain this Slice 18 task package and freeze owner-doc priority here.
  - Add shared runtime command schemas for `retry_run` and `terminate_run`.
  - Extend the interop-owned local transport contract with `retry_run` and `terminate_run`.
  - Extend `packages/schema-ts` and `packages/hub-client` with shared parser and client parity.
  - Extend `crates/runtime`, `apps/remote-hub`, and `apps/desktop/src-tauri` so both local and remote transports expose the same run control semantics.
  - Add the minimum control surface to `apps/desktop/src/views/RunView.vue` only.
  - Refresh current run, project runs, workspace inbox / notifications, and project knowledge after successful control actions.
- Out Of Scope:
  - Session hardening, secure token persistence, run-list bulk actions, chat / streaming, workspace boards, tenant / RBAC / IdP, vector retrieval, Org Graph promotion, or any new list-row DTO fields.
  - New run DTO variants separate from the existing `RunDetail`.
  - New approval-resume commands; approval-driven `run.resume` stays on the existing Inbox / Approval path.
- Acceptance Criteria:
  - `HubClient.retryRun(command)` and `HubClient.terminateRun(command)` work through both local and remote transports and return the shared `RunDetail`.
  - `retry` accepts only `{ run_id }`; `terminate` accepts only `{ run_id, reason }`.
  - Remote hub exposes `POST /api/runs/:run_id/retry` and `POST /api/runs/:run_id/terminate` with auth, workspace membership enforcement, path/body mismatch rejection, and explicit invalid-transition `4xx` responses.
  - Runtime preserves delivery synchronization for automation-origin runs on both retry and terminate paths.
  - Desktop `RunView` shows `Retry Run` only when the current run is retryable, `Terminate Run` only when the current run is terminable, and blocks execution in read-only or unauthenticated states.
- Non-functional Constraints:
  - Keep `schemas/` as the only cross-language contract truth.
  - Keep runtime as the owner of run-transition semantics and delivery synchronization.
  - Keep run controls scoped to `RunView`; do not add list-level control buttons in Slice 18.
  - Preserve truthful owner-doc state: Slice 18 is frozen for run control surface, and session hardening is deferred.
- MVP Boundary:
  - One `retry` command.
  - One `terminate` command with fixed desktop reason `desktop_operator_stopped`.
  - One `RunView` control block.
  - No new run-summary fields and no batch actions.
- Human Approval Points:
  - None.
- Source Of Truth Updates:
  - Update this task package.
  - Update `README.md`, `docs/README.md`, `docs/architecture/ga-implementation-blueprint.md`, and `docs/tasks/README.md` so Slice 18 is frozen and session hardening is explicitly deferred.
  - Add an ADR only if implementation changes durable run / delivery synchronization boundaries or `RunView` authority rules.
- Affected Modules:
  - `docs/tasks`
  - `docs/architecture`
  - `schemas/runtime`
  - `schemas/interop`
  - `packages/schema-ts`
  - `packages/hub-client`
  - `crates/runtime`
  - `apps/remote-hub`
  - `apps/desktop`
  - `apps/desktop/src-tauri`
- Affected Layers:
  - Repository documentation / owner docs
  - Cross-language contracts
  - Runtime public surface
  - Local and remote transport assembly
  - Desktop store and `RunView`
- Risks:
  - Returning inconsistent post-action run shapes between retry and terminate.
  - Letting desktop or transport layers redefine retry / terminate eligibility instead of consuming runtime truth.
  - Forgetting automation delivery synchronization on terminate and leaving run / delivery state drift.
- Validation:
  - `cargo test --workspace`
  - `pnpm test:ts`
  - `pnpm typecheck:ts`
