# Delivery Note

## What Changed

- Replaced the old desktop `Tasks-first` primary path with a project-scoped `Dashboard` and `Conversation` entry model, while keeping `Runs`, `Inbox`, and `Knowledge` as the formal authority surfaces.
- Added app-local preference state for `locale: zh-CN | en-US` and `themeMode: system | light | dark`, plus a bounded desktop translation dictionary and semantic light/dark design tokens.
- Reworked the desktop shell navigation into primary `Dashboard / Conversation / Runs / Inbox / Knowledge` and secondary `Projects / Connections / Models / Preferences`, with `Tasks` retained as an expert-only direct-execution tool.
- Added an app-local conversation/proposal store and a governed conversation workspace where explicit confirmation is the only path that creates a formal `Task` / `Run`.
- Re-ranked dashboard, reminder, runs, run detail, inbox, knowledge, connections, models, and tasks surfaces so the user sees operational context first and system complexity second.
- Added focused regression coverage for the new routing, shell IA, conversation behavior, preference persistence, and stale remembered remote-project fallback.

## Why

- The tracked GA workbench proved the governed runtime and authority surfaces, but the first desktop action still exposed formal runtime objects too early. This redesign moves the complexity into a governed proposal flow so users can begin with intent and refinement rather than task object configuration.

## User / System Impact

- Operators now land on `Dashboard` for active projects, resume work from context/reminder signals, and move into `Conversation` to clarify intent before execution.
- Formal backend truth is unchanged: `Task`, `Run`, `InboxItem`, `Notification`, and `ProjectKnowledgeIndex` remain the shared truth, and run creation still uses the existing execution APIs.
- Locale and theme preferences persist locally across restart and apply to navigation, guidance, empty/error/degraded copy, and redesigned shell surfaces without translating user-authored content.

## Risks

- Future work could incorrectly treat the app-local conversation store as shared truth unless later slices introduce schema-first contracts explicitly.
- Reminder visibility could drift if later UI work re-promotes `Notifications` into a competing primary surface instead of keeping shell/dashboard reminder semantics coherent.
- The current scheduled/event-driven handoff depends on the expert `Tasks` surface, so later automation-entry refinement must preserve explicit-confirmation semantics.

## Rollback Notes

- Roll back the owner-doc alignment, ADR, shell navigation/routing changes, preferences/conversation stores, new views, and regression tests together.
- Do not partially restore `Tasks-first` routing without also reverting the dashboard/conversation shell copy and remembered-project default-route logic.

## Follow-ups

- If cross-device conversation continuity is later required, create a separate schema-first slice for shared draft/proposal truth.
- If scheduled or event-driven execution needs a first-class conversation confirmation path, add it as a bounded follow-on instead of reintroducing automation-first creation semantics.

## Docs Updated

- `README.md`
- `docs/README.md`
- `docs/architecture/VISUAL_FRAMEWORK.md`
- `docs/architecture/ga-implementation-blueprint.md`
- `docs/decisions/README.md`
- `docs/tasks/README.md`
- `docs/decisions/0008-desktop-dashboard-plus-conversation-first-interaction-model.md`
- This task package

## Tests Included

- `pnpm --filter @octopus/desktop test`
- `pnpm --filter @octopus/desktop typecheck`

## ADR Updated

- Added ADR 0008 for the durable `Dashboard + Conversation first` desktop interaction model.

## Temporary Workarounds

- Conversation drafts and execution proposals remain localStorage-backed desktop state only.
- Reminder detail still routes through the lighter `Notifications` view for deeper inspection, but reminder prominence now lives in the shell and dashboard rather than primary navigation.
