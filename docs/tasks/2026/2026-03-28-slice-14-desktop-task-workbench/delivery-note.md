# Delivery Note

## Summary

Slice 14 is now implemented as the tracked desktop task workbench. The desktop shell no longer centers one mixed workspace page; it now exposes explicit `Tasks`, `Runs`, `Inbox`, `Notifications`, and `Connections` surfaces backed by a shared project-scoped `listRuns` read path that works through both local Tauri and remote-hub transports.

## Why

The repository had already proved the governed runtime, approval surfaces, explainability reads, shared clients, and real desktop local host. The remaining gap was navigability and parity of the desktop-first GA surface. This slice closes that gap without expanding product semantics into chat, streaming, new onboarding objects, or new governance write paths.

## User / System Impact

- Local desktop now lands on the demo project `Tasks` route by default.
- Task creation still transitions into the existing run-detail flow.
- Recent project runs are now directly visible through a dedicated `Runs` route.
- Approval actions remain available, but only on the dedicated `Inbox` route.
- Notifications remain visible as a read-only reminder surface.
- Connection mode, auth state, and refresh state are now readable on a dedicated `Connections` page.

## Risks

- `listRuns` is intentionally a small fixed recent window with no pagination or filtering in this slice.
- Desktop routing now depends on canonical workspace-scoped `Inbox` and `Notifications` routes plus compatibility redirects from the older project-scoped paths.
- The workbench remains pull-first, so users should not expect optimistic board behavior or new live event shapes from this slice.

## Rollback Notes

- Roll back the shared `listRuns` contract, both transport implementations, and the desktop `Runs` surface together; partial rollback would break local/remote parity.
- Route-split rollback should include the default landing redirect, new views, and store loader split together to avoid leaving tests or navigation in a mixed state.

## Follow-ups

- Any future expansion of run actions such as retry or terminate should be promoted through a separate task package instead of extending Slice 14 implicitly.
- Pagination, filtering, or richer run board behavior should remain deferred until a later slice explicitly widens the read model.

## Docs Updated

- Slice 14 task package updated with implementation, verification, and delivery records.
- Repository entry docs, GA blueprint, visual framework, and task index were already aligned earlier in the slice.

## Tests Included

- Shared TypeScript contract/client coverage for `RunSummary[]` parsing and `HubClient.listRuns`.
- Local-host and remote-hub parity tests for project scoping, ordering, and empty-state behavior.
- Desktop route, bootstrap, local-mode, and happy-path coverage for the new workbench IA.
- Full workspace Rust and TypeScript gate verification.

## ADR Updated

- No new ADR was required; the existing local/remote transport and desktop boundary decisions remain sufficient.

## Temporary Workarounds

- The canonical approval interaction moved to `Inbox`, so route-compatibility redirects remain in place for older project-scoped URLs during this slice.
- The recent-runs list is intentionally fixed-window and refetch-driven until a later tracked slice promotes richer list semantics.
