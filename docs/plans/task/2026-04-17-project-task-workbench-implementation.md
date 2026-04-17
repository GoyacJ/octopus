# Project Task Workbench Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build an Octopus-native project task feature inspired by Claude's Cowork behavior, but modeled as a first-class `Task` asset inside each project. A task must support async-first execution, explicit live takeover, editable goal/brief during active work, project-shared visibility, results-first completion surfaces, and full scheduling in v1.

**Architecture:** Keep the existing runtime as the execution truth. A `Task` owns project-scoped planning metadata such as goal, brief, default actor, schedule, visibility, and latest result summary. Each execution creates a `TaskRun` that binds to one runtime `sessionId` plus the latest runtime `runId` snapshot. Inline edits and takeover actions are stored as `TaskIntervention` records and appended into the same active runtime session when the task is already running. Scheduling is implemented as a task-domain dispatcher, not as a revival of the removed workspace automation product surface.

**Tech Stack:** Vue 3, Vite, Pinia, Vue Router, Vue I18n, Tauri 2, Rust (`octopus-server`, `octopus-platform`, `octopus-infra`, `octopus-runtime-adapter`), OpenAPI-first transport, generated `@octopus/schema`, SQLite projections, JSONL runtime events, disk-backed artifact and deliverable storage.

---

## Read Before Starting

 - Root `AGENTS.md`
 - `docs/AGENTS.md`
 - `docs/design/DESIGN.md`
 - `docs/api-openapi-governance.md`
 - `contracts/openapi/AGENTS.md`
 - `docs/plans/task/2026-04-17-project-task-workbench-design.md`
 - `docs/plans/task/2026-04-17-project-task-workbench-backlog.md`
 - `docs/plans/task/2026-04-17-project-task-phase-0-contract-and-store-spec.md`
 - `docs/plans/chat/2026-04-16-claude-inspired-project-conversation-deliverable-design.md`
 - `docs/plans/chat/2026-04-16-claude-inspired-project-conversation-deliverable-implementation.md`
 - `docs/plans/2026-04-17-workspace-automations-removal-design.md`

## Locked Product Decisions

1. Use `Task` as the user-facing noun. Do not ship the feature as `Cowork`.
2. Tasks are project-scoped assets. Every project owns its own task list and detail views.
3. The primary task surface is `list -> detail`, not a chat-only mode.
4. Execution is mixed-mode: async-first, with user takeover available at any time.
5. Each task stores a default assignee actor, and the user may change it before a rerun or during active work.
6. The completion surface is results-first: summary plus deliverable and artifact refs before trace or low-level runtime detail.
7. During an active run, editing task goal or brief continues the same runtime session instead of spawning a replacement task object.
8. Task visibility is project-shared in v1.
9. Full scheduling is included in v1.
10. Tasks wrap existing runtime sessions and runs. Do not introduce a second execution engine.

## Non-Goals

- No cross-project global task board in v1.
- No private or user-only task visibility model in v1.
- No separate task chat runtime or shadow event stream.
- No dependency on the removed workspace automation UX.
- No silent mutation of an already running task because a future schedule was edited; schedule edits apply to future runs unless the user explicitly intervenes in the active run.

## Target Domain Model

### Task model

- `ProjectTaskRecord`
- Core fields:
  - `id`
  - `projectId`
  - `title`
  - `goal`
  - `brief`
  - `defaultActorRef`
  - `lifecycleStatus` = `active | paused | archived`
  - `executionMode` = `async_first`
  - `scheduleSpec`
  - `nextRunAt`
  - `lastRunAt`
  - `activeTaskRunId`
  - `latestResultSummary`
  - `latestDeliverableRefs`
  - `latestArtifactRefs`
  - `createdBy`
  - `updatedBy`
  - timestamps

### Run model

- `ProjectTaskRunRecord`
- Core fields:
  - `id`
  - `taskId`
  - `projectId`
  - `triggerType` = `manual | scheduled | rerun`
  - `status` = `queued | running | waiting_approval | completed | failed | canceled`
  - `sessionId`
  - `conversationId`
  - `runtimeRunId`
  - `actorRef`
  - `startedAt`
  - `completedAt`
  - `resultSummary`
  - `deliverableRefs`
  - `artifactRefs`
  - `failureSummary`

### Intervention model

- `ProjectTaskInterventionRecord`
- Core fields:
  - `id`
  - `taskId`
  - `taskRunId`
  - `type` = `edit_goal | edit_brief | comment | change_actor | take_over | resume_async`
  - `payload`
  - `appliedToSessionId`
  - `createdBy`
  - `createdAt`

### Schedule model

- `TaskScheduleSpec`
- Core fields:
  - `mode` = `manual | once | recurring`
  - `timezone`
  - `startsAt`
  - `rrule`
  - `skipIfRunning`
  - `catchUpPolicy` = `skip | enqueue_one`
  - `enabled`

## Execution Semantics

- A task is durable across many runs.
- A task has at most one active run at a time.
- Launching or rerunning a task creates a fresh `TaskRun` and a fresh runtime session snapshot for that run.
- Live takeover never creates a second active task run. It opens the current run's linked conversation and continues the same session.
- Goal and brief edits during an active run are stored as interventions and forwarded into the same session as structured user intent updates.
- Schedule edits affect future runs only. The currently active run remains bound to its captured launch snapshot plus explicit interventions.
- Task detail shows runtime approval and progress state, but runtime remains the source of truth for approvals, messages, and subruns.

## Current Repo Anchors

- Project routes already exist in `apps/desktop/src/router/index.ts`; add a sibling `project-tasks` route instead of inventing a top-level surface.
- Project navigation is registered in `apps/desktop/src/navigation/menuRegistry.ts` and mirrored in `crates/octopus-server/src/handlers.rs`; both sides need a new `menu-project-tasks` entry.
- Project module permission wiring in `apps/desktop/src/composables/project-governance.ts` and `crates/octopus-server/src/lib.rs` currently does not include `tasks`.
- The current conversation workbench in `apps/desktop/src/views/project/ConversationView.vue` already handles runtime actor selection, progress, approvals, and trace-aware execution; task takeover should reuse that surface instead of re-implementing chat inside the task page.
- Runtime session and run data already exist across `apps/desktop/src/stores/runtime*.ts`, `crates/octopus-server/src/workspace_runtime.rs`, and `crates/octopus-runtime-adapter/src/*`; tasks should compose these layers rather than replace them.
- `crates/octopus-platform/src/project.rs` is intentionally thin and is the right place to introduce a dedicated project task service.
- `crates/octopus-infra/src/infra_state.rs` and `crates/octopus-infra/src/projects_teams.rs` are the current project persistence anchors; task projections should follow the same SQLite-first pattern.

## Global Checklist

- [ ] OpenAPI exposes canonical task, task run, intervention, and schedule contracts.
- [ ] Shared schema exports task types from a feature file under `packages/schema/src/task.ts`.
- [ ] SQLite stores project task, run, intervention, and schedule projection state without duplicating artifact bodies.
- [ ] Server routes, permissions, and menu metadata recognize `tasks` as a first-class project module.
- [ ] A scheduler can dispatch due task runs and recover correctly after restart.
- [ ] Desktop navigation, route guards, and list-detail UX exist for project tasks.
- [ ] Tasks support reusable context bundles over project resources, knowledge, and prior deliverables.
- [ ] Task detail supports manual run, rerun, pause/resume, edit brief, change actor, and live takeover.
- [ ] Task runs expose attention state, failure category, and notification-worthy transitions.
- [ ] Completion view is results-first and links back to the task's deliverables and artifacts.
- [ ] Dashboard counters and recent activity include tasks.
- [ ] Task analytics and audit events exist for launches, completions, failures, interventions, and takeovers.
- [ ] No old automation UX or task-specific shadow runtime is introduced.

## Global Exit Condition

The feature is complete only when a project member can create a task, assign an actor, schedule it, let it run automatically or trigger it manually, edit the brief while it is running, take over in the linked conversation, review a results-first completion summary with output refs, rerun it later, and see the task reflected in project navigation and dashboard counts.

## Delivery Order

- Task 1 and Task 2 define the transport and persistence foundation.
- Task 3 adds permission-aware server behavior and runtime bridging.
- Task 4 adds scheduling and background dispatch.
- Task 5 and Task 6 deliver the desktop UX.
- Task 7 finishes dashboard integration, governance, and cleanup.

## Task 1: Define canonical task contracts and schema exports

**Files:**
- Create: `contracts/openapi/src/paths/tasks.yaml`
- Create: `contracts/openapi/src/components/schemas/tasks.yaml`
- Modify: `contracts/openapi/src/root.yaml`
- Modify: `contracts/openapi/src/components/schemas/projects.yaml`
- Modify: `contracts/openapi/src/components/schemas/shared.yaml`
- Modify: `docs/openapi-audit.md`
- Create: `packages/schema/src/task.ts`
- Modify: `packages/schema/src/index.ts`
- Modify: `packages/schema/src/shared.ts`
- Modify: `apps/desktop/test/openapi-bundler.test.ts`
- Modify: `apps/desktop/test/openapi-parity-lib.test.ts`
- Modify: `apps/desktop/test/openapi-transport.test.ts`
- Modify: `apps/desktop/test/tauri-client-workspace.test.ts`
- Generated: `contracts/openapi/octopus.openapi.yaml`
- Generated: `packages/schema/src/generated.ts`

**Checklist:**
- [ ] Add project-scoped task endpoints for list, create, detail, patch, runs, interventions, and launch or rerun actions.
- [ ] Define canonical task transport types: `TaskSummary`, `TaskDetail`, `TaskRunSummary`, `TaskRunDetail`, `TaskInterventionRecord`, `TaskScheduleSpec`, `TaskContextBundle`, `TaskLaunchInput`, `TaskPatchInput`, and `TaskListFilter`.
- [ ] Extend project dashboard transport with task counters and recent-task summary blocks.
- [ ] Extend project permission defaults and overrides to include `tasks`.
- [ ] Include attention state, failure category, and notification-facing transition fields in task and task-run transport.
- [ ] Keep handwritten shared schema files feature-based; do not accumulate task definitions in `packages/schema/src/index.ts`.

**Verification:**

```bash
pnpm -C apps/desktop exec vitest run \
  test/openapi-bundler.test.ts \
  test/openapi-parity-lib.test.ts \
  test/openapi-transport.test.ts \
  test/tauri-client-workspace.test.ts

pnpm openapi:bundle
pnpm schema:generate
pnpm schema:check
```

**Exit criteria:**

- The OpenAPI source of truth contains task transport and project dashboard task counters.
- `@octopus/schema` exports task types from `packages/schema/src/task.ts`.
- Browser and Tauri workspace clients can be typed against the new task contract without view-local interfaces.

## Task 2: Persist task, run, intervention, and schedule projections

**Files:**
- Create: `crates/octopus-core/src/task_records.rs`
- Modify: `crates/octopus-core/src/lib.rs`
- Create: `crates/octopus-infra/src/project_tasks.rs`
- Modify: `crates/octopus-infra/src/lib.rs`
- Modify: `crates/octopus-infra/src/infra_state.rs`
- Modify: `crates/octopus-infra/src/projects_teams.rs`
- Modify: `crates/octopus-infra/src/split_module_tests.rs`
- Modify if needed for runtime linkage: `crates/octopus-runtime-adapter/src/persistence.rs`
- Modify if needed for runtime linkage: `crates/octopus-runtime-adapter/src/session_service.rs`
- Modify if needed for runtime linkage: `crates/octopus-runtime-adapter/src/execution_events.rs`
- Modify if needed for runtime linkage: `crates/octopus-runtime-adapter/src/adapter_tests.rs`

**Checklist:**
- [ ] Add SQLite tables and migrations for project tasks, task runs, task interventions, and scheduler claim state.
- [ ] Store only metadata and refs in SQLite; keep artifact and deliverable bodies in their existing disk-backed stores.
- [ ] Persist `activeTaskRunId`, `latestResultSummary`, `nextRunAt`, and latest output refs on the task record for fast list rendering.
- [ ] Persist run-to-runtime linkage with `sessionId`, `conversationId`, `runtimeRunId`, actor snapshot, and status projection fields.
- [ ] Persist task context bundle refs so reruns and scheduled runs can reconstruct intended context without prompt duplication.
- [ ] Persist intervention history so active-session edits are replayable in audit and detail views.
- [ ] Persist attention state, failure category, and last meaningful transition metadata for notifications and list badges.
- [ ] Ensure restart-safe reconstruction of task list and detail projections from SQLite plus existing runtime records.

**Verification:**

```bash
cargo test -p octopus-core
cargo test -p octopus-infra
cargo test -p octopus-runtime-adapter
```

**Exit criteria:**

- Task list and detail state survive reload and restart.
- Output refs remain metadata-only and point to existing artifact or deliverable storage.
- There is one canonical persistence path for task state instead of ad hoc JSON caches.

## Task 3: Add project task service, permissions, server routes, and runtime bridge

**Files:**
- Modify: `crates/octopus-platform/src/project.rs`
- Modify: `crates/octopus-platform/src/lib.rs`
- Modify if needed: `crates/octopus-platform/src/workspace.rs`
- Create: `crates/octopus-server/src/project_tasks.rs`
- Modify: `crates/octopus-server/src/lib.rs`
- Modify: `crates/octopus-server/src/routes.rs`
- Modify: `crates/octopus-server/src/handlers.rs`
- Modify: `crates/octopus-server/src/dto_mapping.rs`
- Modify: `crates/octopus-server/src/workspace_runtime.rs`
- Modify if needed for runtime metadata tags: `crates/octopus-runtime-adapter/src/session_service.rs`
- Modify if needed for runtime metadata tags: `crates/octopus-runtime-adapter/src/execution_service.rs`

**Checklist:**
- [ ] Introduce a dedicated `ProjectTaskService` interface under the project domain instead of bloating `WorkspaceService`.
- [ ] Add server handlers for task CRUD, run launch and rerun, intervention append, run history, and schedule preview.
- [ ] Add `tasks` to project module permission resolution and request-to-module mapping.
- [ ] Add `menu-project-tasks` to server-delivered menu metadata.
- [ ] Reuse existing runtime session creation paths when launching a task run.
- [ ] When a task run is active, hydrate task detail from the linked runtime session and current run projection instead of maintaining a second live-execution store.
- [ ] Surface `waiting_approval` and similar runtime states directly on the task run projection.
- [ ] Emit clean task-domain audit and analytics events for run launch, completion, failure, intervention, approval, and takeover.

**Verification:**

```bash
cargo test -p octopus-server
pnpm -C apps/desktop exec vitest run test/tauri-client-workspace.test.ts
```

**Exit criteria:**

- Server authorization recognizes tasks as a project module.
- Launching a task creates a linked runtime session without inventing a parallel runtime.
- Task detail can explain its current live state from server projections alone.

## Task 4: Implement v1 task scheduling and dispatch

**Files:**
- Create: `crates/octopus-server/src/task_scheduler.rs`
- Modify: `crates/octopus-server/src/lib.rs`
- Modify: `crates/octopus-server/src/project_tasks.rs`
- Modify: `crates/octopus-infra/src/project_tasks.rs`
- Modify if needed: `crates/octopus-platform/src/project.rs`
- Modify: `crates/octopus-desktop-backend/src/main.rs`
- Modify: `apps/desktop/src-tauri/src/backend.rs`

**Checklist:**
- [ ] Add a scheduler loop that polls due task rows and claims them safely.
- [ ] Support one-off and recurring schedules from `TaskScheduleSpec`.
- [ ] Enforce one active run per task; if a task is already running and `skipIfRunning=true`, advance the schedule without starting a second run.
- [ ] Implement restart recovery so the dispatcher recalculates the next due run after process restart.
- [ ] Respect `catchUpPolicy`, but cap recovery to at most one enqueued catch-up run.
- [ ] Record skipped and collided schedule events so the user can understand why a task did not run.
- [ ] Keep the scheduler task-domain-specific; do not reuse deleted workspace automation UX state or routes.

**Verification:**

```bash
cargo test -p octopus-server task_scheduler -- --nocapture
cargo test -p octopus-infra project_tasks -- --nocapture
```

**Exit criteria:**

- Due task runs start automatically.
- Duplicate dispatch is prevented by durable claim logic.
- Scheduler behavior after restart is deterministic and test-covered.

## Task 5: Deliver desktop navigation, route guard, list-detail shell, and typed client APIs

**Files:**
- Modify: `apps/desktop/src/router/index.ts`
- Modify: `apps/desktop/src/navigation/menuRegistry.ts`
- Modify: `apps/desktop/src/i18n/navigation.ts`
- Modify: `apps/desktop/src/locales/en-US.json`
- Modify: `apps/desktop/src/locales/zh-CN.json`
- Modify: `apps/desktop/src/components/layout/WorkbenchSidebar.vue`
- Modify: `apps/desktop/src/composables/project-governance.ts`
- Modify: `apps/desktop/src/tauri/workspace_api.ts`
- Modify: `apps/desktop/src/tauri/workspace-client.ts`
- Create: `apps/desktop/src/stores/project_task.ts`
- Create: `apps/desktop/src/views/project/ProjectTasksView.vue`
- Create: `apps/desktop/src/components/task/ProjectTaskListPane.vue`
- Create: `apps/desktop/src/components/task/ProjectTaskDetailPane.vue`
- Create: `apps/desktop/src/components/task/ProjectTaskFiltersBar.vue`
- Modify: `apps/desktop/test/router.test.ts`
- Modify: `apps/desktop/test/layout-shell.test.ts`
- Create: `apps/desktop/test/project-tasks-view.test.ts`
- Modify: `apps/desktop/test/tauri-client-workspace.test.ts`
- Modify: `apps/desktop/test/support/workspace-fixture-state.ts`
- Modify: `apps/desktop/test/support/workspace-fixture-client.ts`
- Modify: `apps/desktop/test/support/workspace-fixture-projects.ts`

**Checklist:**
- [ ] Add a `project-tasks` route and `menu-project-tasks` navigation entry.
- [ ] Extend route-to-project-module mapping so task pages are permission-guarded like other project modules.
- [ ] Add typed workspace client calls for task list, task detail, create, patch, launch, rerun, and intervention submission.
- [ ] Expose task attention state and context bundle data through the adapter so list and detail views do not infer them locally.
- [ ] Implement a list-detail layout that matches `docs/design/DESIGN.md` and reuses `@octopus/ui`.
- [ ] List rows must show status, actor, next run, last run, and latest result summary at a glance.
- [ ] List rows must also surface notification-worthy attention states such as `failed`, `needs approval`, and `updated`.
- [ ] Detail loading and optimistic updates should live in a dedicated Pinia store instead of leaking task state into unrelated runtime stores.

**Verification:**

```bash
pnpm -C apps/desktop exec vitest run \
  test/router.test.ts \
  test/layout-shell.test.ts \
  test/project-tasks-view.test.ts \
  test/tauri-client-workspace.test.ts
```

**Exit criteria:**

- Project members with task permission can reach the task page from the sidebar.
- List and detail data are typed end-to-end through the adapter boundary.
- The UX is task-first, not a thin wrapper around the conversation page.

## Task 6: Implement task detail actions, results-first completion, and live takeover

**Files:**
- Modify: `apps/desktop/src/views/project/ConversationView.vue`
- Modify: `apps/desktop/src/stores/project_task.ts`
- Modify if needed: `apps/desktop/src/stores/runtime_sessions.ts`
- Modify if needed: `apps/desktop/src/stores/runtime_events.ts`
- Create: `apps/desktop/src/components/task/TaskRunSummaryCard.vue`
- Create: `apps/desktop/src/components/task/TaskScheduleCard.vue`
- Create: `apps/desktop/src/components/task/TaskInterventionComposer.vue`
- Create: `apps/desktop/src/components/task/TaskRunHistoryList.vue`
- Create: `apps/desktop/src/components/task/TaskOutputsPanel.vue`
- Modify: `apps/desktop/test/project-tasks-view.test.ts`
- Modify: `apps/desktop/test/project-runtime-view.test.ts`

**Checklist:**
- [ ] Show result summary and output refs first whenever the latest run is completed.
- [ ] Show live run state, actor, approvals, and progress summary when a run is active.
- [ ] Allow inline edits for goal and brief, storing them as interventions on the active run.
- [ ] Allow actor changes for future reruns and explicit actor-change interventions for the active run.
- [ ] Show the task's bound context bundle in detail so the user can understand what the run is using.
- [ ] Provide a clear takeover action that opens the linked `ConversationView` with the same task run session.
- [ ] Add a back-link or task context chip inside `ConversationView` so takeover does not strand the user in a detached chat.
- [ ] Preserve run history in the detail rail so a task can be understood without opening trace first.
- [ ] Surface failure category and next recommended action when a run fails.

**Verification:**

```bash
pnpm -C apps/desktop exec vitest run \
  test/project-tasks-view.test.ts \
  test/project-runtime-view.test.ts
```

**Exit criteria:**

- A running task can be steered without destroying its current execution context.
- Takeover uses the same runtime session.
- Completion state is centered on outputs, not trace internals.

## Task 7: Add dashboard integration, governance coverage, and cleanup

**Files:**
- Modify: `apps/desktop/src/views/project/ProjectDashboardView.vue`
- Modify: `apps/desktop/src/stores/workspace.ts`
- Modify: `apps/desktop/src/stores/workspace_actions.ts`
- Modify: `apps/desktop/test/projects-view.test.ts`
- Modify: `apps/desktop/test/repo-governance.test.ts`
- Modify if needed: `apps/desktop/test/project-deliverables-view.test.ts`
- Modify if needed: `crates/octopus-server/src/workspace_runtime.rs`
- Modify if needed: `contracts/openapi/src/components/schemas/projects.yaml`

**Checklist:**
- [ ] Add task counts and recent-task summaries to project dashboard snapshots.
- [ ] Ensure dashboard links route into `project-tasks`.
- [ ] Add governance assertions so future project-module additions keep `tasks` wired through menu registry, locale labels, server handlers, and route guards.
- [ ] Add project-level task analytics blocks such as scheduled vs manual runs, completion rate, and recent failure counts.
- [ ] Confirm no deleted automation UX concepts leaked into naming, routes, or state shapes.
- [ ] Remove temporary task-specific compatibility branches or duplicated summary logic introduced during earlier tasks.

**Verification:**

```bash
pnpm -C apps/desktop exec vitest run \
  test/projects-view.test.ts \
  test/repo-governance.test.ts \
  test/project-deliverables-view.test.ts

cargo test -p octopus-server
```

**Exit criteria:**

- Dashboard surfaces task state without making tasks a second-class page.
- Governance tests protect the route/menu/permission wiring.
- The final code path is clean and does not depend on removed automation terminology or duplicate runtime state.

## Final Verification Sweep

Run this full sweep before calling the implementation complete:

```bash
pnpm openapi:bundle
pnpm schema:generate
pnpm schema:check

pnpm -C apps/desktop exec vitest run \
  test/openapi-bundler.test.ts \
  test/openapi-parity-lib.test.ts \
  test/openapi-transport.test.ts \
  test/tauri-client-workspace.test.ts \
  test/router.test.ts \
  test/layout-shell.test.ts \
  test/project-tasks-view.test.ts \
  test/project-runtime-view.test.ts \
  test/projects-view.test.ts \
  test/repo-governance.test.ts

cargo test -p octopus-core
cargo test -p octopus-infra
cargo test -p octopus-runtime-adapter
cargo test -p octopus-server
```

## Implementation Notes

- Prefer a clean task-domain model over thin aliases around conversations.
- Do not put task schema definitions directly into `packages/schema/src/index.ts`.
- Do not let business pages call bare `fetch`; all desktop API access must go through the existing workspace adapter boundary.
- Keep task UI inside the shared design system and existing layout grammar from `docs/design/DESIGN.md`.
- If runtime metadata tagging is needed, keep it minimal and additive so runtime remains broadly reusable outside the task surface.
