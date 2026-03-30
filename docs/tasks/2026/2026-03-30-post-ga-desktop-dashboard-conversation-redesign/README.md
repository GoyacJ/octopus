# Post-GA: Desktop Dashboard + Conversation Redesign

This task package records the post-GA desktop redesign that replaces the frozen `Tasks-first` workbench stance with a `Dashboard -> Conversation -> formal execution` primary path.

## Closeout Status

- Completed and verified.
- This package is closed out as a bounded desktop-only redesign that keeps `Task` and `Run` as formal system truth while moving initiation into an app-local `Dashboard + Conversation` flow without adding shared conversation contracts.

## Package Files

- [design-note.md](design-note.md)
- [contract-change.md](contract-change.md)
- [implementation-summary.md](implementation-summary.md)
- [verification.md](verification.md)
- [delivery-note.md](delivery-note.md)

## Task Definition

- Goal:
  - Redesign the desktop primary path so the operational home becomes `Dashboard`, work initiation becomes governed `Conversation`, and formal `Task` / `Run` creation occurs only after explicit confirmation.
- Scope:
  - Create and maintain this implementation task package.
  - Update owner docs and add one ADR so the post-GA desktop target state is explicitly authorized before implementation.
  - Add desktop app-local preference state for `locale` and `themeMode`, with persisted `zh-CN | en-US` and `system | light | dark`.
  - Replace project entry routing so an active project lands on `Dashboard`, with `Conversation` as the primary initiation surface.
  - Add a project-scoped `Dashboard` surface for context, continue-work, system summary, and reminder visibility.
  - Add a project-scoped `Conversation` surface with app-local draft/proposal state, explicit execution confirmation, and a bridge into existing task/run APIs.
  - Downgrade `Tasks` into an expert-only direct execution surface and move `Notifications` out of first-class primary navigation.
  - Re-rank `Runs`, `Run Detail`, `Inbox`, and `Knowledge` to match the new mental model without changing cross-language contracts.
  - Add focused desktop tests for routing, conversation confirmation behavior, i18n/theme preferences, and the adapted authority pages.
- Out Of Scope:
  - New shared schemas, DTOs, routes, events, or cross-language conversation persistence.
  - New remote-hub endpoints or new `packages/hub-client` methods for conversation drafts.
  - Cross-device draft sync, real LLM chat orchestration, or new execution/governance backend semantics.
  - Tenant / RBAC administration, provider connectivity, or unrelated post-GA backlog work.
- Acceptance Criteria:
  - The default active-project desktop route becomes `Dashboard`, while no-project entry still lands on `Projects`.
  - `Conversation` becomes the primary project-scoped initiation flow, and drafting/revision alone does not create a run.
  - Explicit confirmation creates exactly one formal `Task` / `Run` through the existing execution APIs.
  - `Dashboard`, `Conversation`, shell navigation, empty/error/degraded states, and preferences support both `zh-CN` and `en-US`.
  - Theme mode supports `system`, `light`, and `dark`, and persists locally across restart.
  - `Runs`, `Run Detail`, `Inbox`, and `Knowledge` still act as authority surfaces after the redesign.
- Non-functional Constraints:
  - Keep `apps/desktop` as app-local assembly only.
  - Keep formal runtime truth in the existing `Task`, `RunDetail`, `InboxItem`, `Notification`, and `ProjectKnowledgeIndex` contracts.
  - Keep conversation draft/proposal state app-local only in this slice.
  - Preserve degraded/read-only semantics already established by Slice 20.
- MVP Boundary:
  - App-local preferences store.
  - App-local conversation/proposal store.
  - New `Dashboard` and `Conversation` pages.
  - Refactored shell navigation and route defaults.
  - Adapted existing authority pages and tests.
- Human Approval Points:
  - None. The redesign plan has already been approved in-session; implementation should stop only if hidden contract pressure appears.
- Source Of Truth Updates:
  - Update this task package.
  - Update `README.md`, `docs/README.md`, `docs/tasks/README.md`, `docs/decisions/README.md`, `docs/architecture/VISUAL_FRAMEWORK.md`, and `docs/architecture/ga-implementation-blueprint.md`.
  - Add one ADR for the desktop primary interaction model.
- Affected Modules:
  - `apps/desktop`
  - `docs/architecture`
  - `docs/decisions`
  - `docs/tasks`
  - repository entry docs
- Affected Layers:
  - Desktop shell and view layer
  - Desktop app-local state layer
  - Owner-doc and task-package layer
- Risks:
  - Letting the desktop redesign silently create new cross-language truth.
  - Reintroducing `Tasks-first` behavior through entry-route or navigation leftovers.
  - Blurring `Inbox` actions and `Notifications` reminders during visual consolidation.
  - Overpromising conversation intelligence beyond the current tracked backend.
- Validation:
  - `pnpm --filter @octopus/desktop test`
  - `pnpm --filter @octopus/desktop typecheck`
