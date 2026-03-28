# Slice 14: Desktop Task Workbench

This task package freezes the post-Slice-13 GA priority: turn the current desktop shell into a desktop-first task workbench with explicit `Tasks`, `Runs`, `Inbox`, `Notifications`, and `Connections` surfaces while preserving the existing governed runtime, shared contracts, and detail views.

## Package Files

- [design-note.md](design-note.md)
- [contract-change.md](contract-change.md)
- [implementation-summary.md](implementation-summary.md)
- [verification.md](verification.md)
- [delivery-note.md](delivery-note.md)

## Task Definition

- Goal:
  - Implement the next desktop-first GA workbench slice so the tracked desktop app becomes a focused task execution and follow-up surface instead of one overloaded mixed workspace page.
- Scope:
  - Create this task package and keep Slice 14 design and contract notes here.
  - Reconcile repository entry docs, the GA blueprint, and the visual framework so Slice 13 is marked complete and Slice 14 is the frozen next priority.
  - Add dedicated desktop routes for `Tasks`, `Runs`, `Inbox`, `Notifications`, and `Connections`.
  - Keep `RunView` authoritative for run policy, artifact, trace, approval, and knowledge follow-up.
  - Add one new shared read surface only: `HubClient.listRuns(workspaceId, projectId)`.
  - Implement `listRuns` parity through the local Tauri transport, runtime, remote-hub HTTP route, shared client, and desktop store/UI.
  - Split the current mixed workspace hydration into focused loaders for project context/tasks, runs, inbox, notifications, and connection status.
- Out Of Scope:
  - Full chat semantics, streaming output, `DiscussionSession`, or any chatbot-first desktop redesign.
  - `Agent` / `Team` onboarding or new local-first onboarding objects.
  - `retryRun`, `terminateRun`, or other new run write paths unless promoted by a separate tracked task package.
  - Tenant / RBAC / IdP administration, vector retrieval, Org Graph promotion, or remote admin expansion.
  - New run-row DTOs, board aggregations, pagination, filtering, or a new event contract.
- Acceptance Criteria:
  - Local desktop opens into the demo project `Tasks` route in local mode.
  - The desktop shell exposes dedicated routes for `Tasks`, `Runs`, `Inbox`, `Notifications`, and `Connections`.
  - Creating a task still navigates to run detail, and the resulting run appears in the `Runs` list.
  - Approval-required runs remain actionable through `Inbox`, while `Notifications` stays visible as a read surface.
  - Connection/auth/session state is readable on a dedicated `Connections` page.
  - `HubClient.listRuns(workspaceId, projectId)` works through both local Tauri and remote-hub transports and reuses the existing `RunSummary` contract.
  - Automated tests cover run-list parity, project scoping, ordering, empty state, and the new route surfaces.
- Non-functional Constraints:
  - Keep `schemas/` as the only cross-language contract truth.
  - Keep desktop behavior pull-first; existing `HubEvent` may only trigger refetches.
  - Do not create a second run-summary DTO or desktop-owned transport truth.
  - Do not move business rules out of `crates/runtime`.
- MVP Boundary:
  - Recent runs are project-scoped, ordered by most recently updated first, and returned as a small fixed window.
  - `Tasks` owns formal task creation/start.
  - `Runs` owns recent project runs.
  - `Inbox` owns approval/action handling.
  - `Notifications` is a read/reminder surface.
  - `Connections` owns hub mode, auth/session, and refresh visibility.
- Human Approval Points:
  - None.
- Source Of Truth Updates:
  - Update this task package.
  - Update `README.md`, `docs/README.md`, `docs/architecture/ga-implementation-blueprint.md`, `docs/architecture/VISUAL_FRAMEWORK.md`, and `docs/tasks/README.md`.
  - Add a new ADR only if Slice 14 forces a durable boundary change beyond the existing desktop/local-transport decisions.
- Affected Modules:
  - `docs/tasks`
  - `docs/architecture`
  - `schemas/interop`
  - `packages/schema-ts`
  - `packages/hub-client`
  - `crates/runtime`
  - `apps/remote-hub`
  - `apps/desktop`
  - `apps/desktop/src-tauri`
- Risks:
  - Re-introducing a second run-list DTO instead of reusing `RunSummary`.
  - Expanding the workbench into a broader dashboard or chat surface that exceeds current GA truth.
  - Letting desktop routing diverge between local and remote transport behavior.
  - Treating notifications as an action surface and collapsing them back into Inbox semantics.
- Validation:
  - `cargo test --workspace`
  - `pnpm test:ts`
  - `pnpm typecheck:ts`
  - `pnpm --filter @octopus/desktop smoke:local-host`
