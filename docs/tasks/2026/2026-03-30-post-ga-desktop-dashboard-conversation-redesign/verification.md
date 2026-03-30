# Verification

## Targeted Verification

- `pnpm --filter @octopus/desktop test`
  - Passed
  - Covers the new dashboard-first routing and entry resolution, conversation proposal/confirmation flow, persisted locale/theme preferences, authority-page regressions, and stale remembered remote-project fallback through the desktop suites including `desktop-redesign.test.ts`, `workbench-routes.test.ts`, `remote-connections.test.ts`, `bootstrap-smoke.test.ts`, and `local-mode.test.ts`.
- `pnpm --filter @octopus/desktop typecheck`
  - Passed

## Contract Tests

- No shared-contract changes in this slice.

## Failure Cases

- No-project desktop entry still resolves to `Projects`, while active-project entry now resolves to `Dashboard`.
- Drafting, clarifying, and proposal edits do not create a run; only explicit confirmation against `executionMode = now` creates one formal `Task` / `Run`.
- Stale remembered remote project entry on `Dashboard` or `Conversation` clears the remembered project and falls back to `Projects`.
- Shell reminders and dashboard risk visibility still surface degraded/auth-expired states after the IA change.
- `scheduled` and `event-driven` proposal modes do not silently create unsupported runs; they hand off to the expert `Tasks` path.

## Boundary Cases

- Locale switching updates shell navigation, page titles, empty/error/degraded guidance, and dashboard/conversation copy without translating user-authored content or object identifiers.
- Theme mode persists as `system | light | dark` and preserves semantic status colors for connection, runtime, governance, and knowledge states across light/dark themes.
- `Run Detail`, `Inbox`, and `Knowledge` remain the authoritative formal-object surfaces after the conversation-first redesign.

## Manual Alignment Review

- Reviewed owner-doc alignment across `README.md`, `docs/README.md`, `docs/tasks/README.md`, `docs/decisions/README.md`, `docs/architecture/VISUAL_FRAMEWORK.md`, `docs/architecture/ga-implementation-blueprint.md`, ADR 0008, and this task package.

## Remaining Gaps

- Conversation draft and proposal continuity remains local to the desktop app and does not sync across devices or transports.
- Scheduled and event-driven proposal confirmation still relies on the expert `Tasks` surface instead of a dedicated conversation-native confirmation flow.

## Confidence Level

- High.
