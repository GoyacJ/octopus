# Implementation Summary

## What Changed

- Added one new shared read surface for the workbench: `HubClient.listRuns(workspaceId, projectId)`.
- Extended the interop-owned local transport contract with `list_runs` and consumed the contract change from:
  - `packages/schema-ts`
  - `packages/hub-client`
  - `apps/desktop/src-tauri`
- Implemented project-scoped recent-run parity across both runtime transports:
  - `crates/runtime` now exposes a project-scoped recent-run query ordered by `updated_at DESC, id DESC`
  - `apps/remote-hub` now exposes the matching project-scoped HTTP read route
  - `apps/desktop/src-tauri` now dispatches the matching local host command
- Split the desktop shell into explicit workbench routes and focused views for:
  - `Tasks`
  - `Runs`
  - `Inbox`
  - `Notifications`
  - `Connections`
- Reworked the desktop store so route views load only the data they own instead of relying on one mixed workspace page hydration path.
- Preserved existing run detail and automation detail ownership while updating bootstrap/default navigation to land local mode in the demo `Tasks` route.
- Added and aligned desktop, shared TypeScript, local-host, and remote-hub tests for the new route split and `listRuns` parity.

## Key Decisions Preserved

- `RunSummary` remains the only shared recent-run row contract; no second desktop-owned DTO was introduced.
- Desktop behavior remains pull-first; existing `HubEvent` subscriptions only trigger refetches.
- `RunView` remains the authoritative surface for run policy decisions, artifacts, trace, approvals, and knowledge follow-up.
- No new write paths were added for retry, terminate, chat, streaming, onboarding, tenant admin, vector retrieval, or Org Graph promotion.
- Business rules remain in `crates/runtime`; remote-hub, Tauri local host, and the desktop shell only consume the shared runtime and contract truth.

## Notable Follow-through

- Updated the desktop bootstrap smoke path and read-only happy-path assertions to match the new IA split, with approvals now exercised from `Inbox` instead of the `Tasks` surface.
- Tightened `apps/desktop/src/app.ts` route typing to satisfy `vue-tsc` under Vue Router 4's `RouteRecordRaw` contract.
