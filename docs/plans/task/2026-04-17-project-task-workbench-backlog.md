# Project Task Workbench Phase Backlog

**Goal:** Turn the task feature design and implementation plan into a delivery backlog that can be executed phase by phase, with explicit priority on `context bundle`, `attention state / notifications`, `failure taxonomy`, and `analytics`.

**Use this document for:** sequencing, scoping, and deciding what must land before later UI or scheduling work starts.

**This document does not replace:** the design evaluation at [2026-04-17-project-task-workbench-design.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/plans/task/2026-04-17-project-task-workbench-design.md) or the full implementation plan at [2026-04-17-project-task-workbench-implementation.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/plans/task/2026-04-17-project-task-workbench-implementation.md). It sequences them.

---

## 1. Planning Rules

1. `Task` stays a project asset. Never collapse the backlog into "just extend conversation."
2. Contract and projection fields for `context bundle`, `attention state`, `failure category`, and `analytics events` must land before advanced UI polish.
3. The frontend must not infer task attention or failure meaning from loose runtime strings. Those must be first-class transport fields.
4. Scheduling is important, but not at the cost of a weak task object. If tradeoffs are needed, strengthen task model and notifications first.
5. Results-first review and live takeover remain the defining user-facing promise in every phase.

## 2. Release Milestones

### Milestone A: Domain Foundation

The task domain is real in transport and persistence, with reusable context, attention state, failure taxonomy, and audit hooks.

### Milestone B: Internal Usable Surface

Project members can create tasks, run them manually, inspect results, and take over active runs.

### Milestone C: Async Operational Surface

Scheduled runs, notifications, and project-level attention states work end to end.

### Milestone D: Operable And Measurable

Dashboard metrics, analytics, and governance coverage are good enough for broader rollout.

## 3. Critical Path

The critical path is:

1. contract hardening
2. persistence projections
3. server service and runtime bridge
4. desktop list-detail shell
5. live execution and takeover
6. scheduler plus notifications
7. analytics and governance hardening

Do not invert this order by building detailed task UI before the domain model is stable.

## 4. Phase Backlog

## Phase 0: Contract Hardening And State Model

**Outcome:** the task feature has a stable type system and transport model that already includes the high-value product semantics.

### Slice 0A: Task contract foundation

**Priority:** P0

**Primary files:**
- `contracts/openapi/src/paths/tasks.yaml`
- `contracts/openapi/src/components/schemas/tasks.yaml`
- `contracts/openapi/src/components/schemas/projects.yaml`
- `contracts/openapi/src/components/schemas/shared.yaml`
- `contracts/openapi/src/root.yaml`
- `packages/schema/src/task.ts`
- `packages/schema/src/shared.ts`
- `packages/schema/src/index.ts`

**Reference spec:**
- `docs/plans/task/2026-04-17-project-task-phase-0-contract-and-store-spec.md`

**Must define now:**
- `TaskContextBundle`
- `TaskContextRef`
- `TaskAttentionState`
- `TaskFailureCategory`
- `TaskLifecycleStatus`
- `TaskRunTriggerType`
- `TaskStateTransitionSummary`
- `TaskNotificationReason`
- `TaskAnalyticsSummary`

**Minimum task detail fields:**
- `contextBundle`
- `attentionState`
- `attentionUpdatedAt`
- `latestFailureCategory`
- `latestTransition`
- `analyticsSummary`

**Acceptance criteria:**
- OpenAPI can describe task list rows without any client-side derived status logic.
- A task run failure can be classified without inspecting raw trace text.
- A scheduled or rerun task can carry reusable context refs as a first-class object.

### Slice 0B: Desktop type and fixture alignment

**Priority:** P0

**Primary files:**
- `apps/desktop/test/openapi-bundler.test.ts`
- `apps/desktop/test/openapi-parity-lib.test.ts`
- `apps/desktop/test/openapi-transport.test.ts`
- `apps/desktop/test/tauri-client-workspace.test.ts`
- `apps/desktop/test/support/workspace-fixture-state.ts`
- `apps/desktop/test/support/workspace-fixture-client.ts`
- `apps/desktop/test/support/workspace-fixture-projects.ts`

**Acceptance criteria:**
- Fixtures can represent tasks with context bundles, attention badges, and failure categories.
- Desktop tests fail if these fields disappear from transport.

### Slice 0C: Store shape lock

**Priority:** P0

**Primary files:**
- `apps/desktop/src/stores/project_task.ts`
- `apps/desktop/src/tauri/workspace_api.ts`
- `apps/desktop/src/tauri/workspace-client.ts`

**Reference spec:**
- `docs/plans/task/2026-04-17-project-task-phase-0-contract-and-store-spec.md`

**Backlog rule:**
- Add placeholder typed store state early, even if not all UI exists yet.

**Minimum store domains:**
- `taskList`
- `taskDetail`
- `contextBundleEditorState`
- `attentionFilters`
- `runHistory`
- `notificationsByTaskId`

**Acceptance criteria:**
- The eventual frontend does not need to redesign store shape after server work lands.

## Phase 1: Persistence And Service Substrate

**Outcome:** the backend can persist and explain tasks without depending on ad hoc runtime scraping.

### Slice 1A: SQLite projections for task and run state

**Priority:** P0

**Primary files:**
- `crates/octopus-core/src/task_records.rs`
- `crates/octopus-core/src/lib.rs`
- `crates/octopus-infra/src/project_tasks.rs`
- `crates/octopus-infra/src/infra_state.rs`
- `crates/octopus-infra/src/lib.rs`
- `crates/octopus-infra/src/split_module_tests.rs`

**Must persist now:**
- task core fields
- context bundle refs
- active attention state
- failure category
- latest transition metadata
- latest analytics counters
- scheduler claim fields

**Acceptance criteria:**
- Task list and task detail can be reconstructed from SQLite projections and linked runtime records.
- Attention and failure information survive restart.

### Slice 1B: Runtime linkage model

**Priority:** P0

**Primary files:**
- `crates/octopus-runtime-adapter/src/persistence.rs`
- `crates/octopus-runtime-adapter/src/session_service.rs`
- `crates/octopus-runtime-adapter/src/execution_events.rs`
- `crates/octopus-runtime-adapter/src/execution_service.rs`
- `crates/octopus-runtime-adapter/src/adapter_tests.rs`

**Backlog rule:**
- Runtime remains the execution truth, but must expose stable task linkage hooks.

**Must capture now:**
- `taskId`
- `taskRunId`
- actor snapshot
- launch trigger
- effective context bundle snapshot

**Acceptance criteria:**
- A task run can be traced to one runtime session and current run state.
- Future UI does not need brittle heuristics to find the linked conversation.

### Slice 1C: Task service and audit event families

**Priority:** P0

**Primary files:**
- `crates/octopus-platform/src/project.rs`
- `crates/octopus-platform/src/lib.rs`
- `crates/octopus-server/src/project_tasks.rs`
- `crates/octopus-server/src/lib.rs`
- `crates/octopus-server/src/dto_mapping.rs`

**Must emit now:**
- task created
- task updated
- task launched
- task completed
- task failed
- task canceled
- intervention applied
- takeover started
- approval requested
- approval resolved

**Acceptance criteria:**
- Audit event families exist before dashboard analytics work starts.
- A task run can be explained from service-level events without reading UI code.

## Phase 2: Base Project Task Surface

**Outcome:** users can discover, create, and inspect tasks through a coherent project page.

### Slice 2A: Navigation and route wiring

**Priority:** P1

**Primary files:**
- `apps/desktop/src/router/index.ts`
- `apps/desktop/src/navigation/menuRegistry.ts`
- `apps/desktop/src/i18n/navigation.ts`
- `apps/desktop/src/locales/en-US.json`
- `apps/desktop/src/locales/zh-CN.json`
- `apps/desktop/src/components/layout/WorkbenchSidebar.vue`
- `apps/desktop/src/composables/project-governance.ts`
- `crates/octopus-server/src/handlers.rs`
- `crates/octopus-server/src/routes.rs`
- `crates/octopus-server/src/lib.rs`

**Acceptance criteria:**
- `Tasks` appears as a first-class project module.
- Route guards and server menu metadata agree on permission mapping.

### Slice 2B: List-detail shell

**Priority:** P1

**Primary files:**
- `apps/desktop/src/views/project/ProjectTasksView.vue`
- `apps/desktop/src/components/task/ProjectTaskListPane.vue`
- `apps/desktop/src/components/task/ProjectTaskDetailPane.vue`
- `apps/desktop/src/components/task/ProjectTaskFiltersBar.vue`
- `apps/desktop/src/stores/project_task.ts`
- `apps/desktop/test/project-tasks-view.test.ts`
- `apps/desktop/test/router.test.ts`
- `apps/desktop/test/layout-shell.test.ts`

**List row must show:**
- title
- actor
- status
- next run
- latest summary
- attention badge

**Detail must show, in this order:**
- header
- latest result summary
- output refs
- live state or blocker state
- brief and goal
- context bundle

**Acceptance criteria:**
- The page reads as a task operating surface, not as a job log.
- Results-first ordering is visible even before takeover ships.

### Slice 2C: Create and edit flow

**Priority:** P1

**Primary files:**
- `apps/desktop/src/components/task/TaskEditorDialog.vue`
- `apps/desktop/src/components/task/TaskContextBundleEditor.vue`
- `apps/desktop/src/stores/project_task.ts`
- `apps/desktop/test/project-tasks-view.test.ts`

**Required inputs:**
- title
- goal
- brief
- default actor
- schedule mode
- context bundle

**Acceptance criteria:**
- A task can be created with reusable context, not only a freeform text brief.

## Phase 3: Live Execution, Intervention, And Takeover

**Outcome:** task runs become steerable and connected to the live conversation surface.

### Slice 3A: Manual launch, rerun, and active-run detail

**Priority:** P1

**Primary files:**
- `crates/octopus-server/src/project_tasks.rs`
- `crates/octopus-server/src/workspace_runtime.rs`
- `apps/desktop/src/stores/project_task.ts`
- `apps/desktop/src/components/task/TaskRunSummaryCard.vue`
- `apps/desktop/src/components/task/TaskRunHistoryList.vue`
- `apps/desktop/test/project-tasks-view.test.ts`
- `apps/desktop/test/project-runtime-view.test.ts`

**Acceptance criteria:**
- Launch and rerun use the bound context bundle and actor snapshot.
- Active run status is visible in task detail without opening trace.

### Slice 3B: Intervention model

**Priority:** P1

**Primary files:**
- `apps/desktop/src/components/task/TaskInterventionComposer.vue`
- `apps/desktop/src/stores/project_task.ts`
- `crates/octopus-server/src/project_tasks.rs`
- `crates/octopus-platform/src/project.rs`

**Interventions to support now:**
- edit goal
- edit brief
- steering note
- change actor

**Acceptance criteria:**
- Interventions are durable records, not ephemeral message hacks.
- Task detail can show who changed what and when.

### Slice 3C: Live takeover

**Priority:** P1

**Primary files:**
- `apps/desktop/src/views/project/ConversationView.vue`
- `apps/desktop/src/stores/project_task.ts`
- `apps/desktop/src/stores/runtime_sessions.ts`
- `apps/desktop/src/stores/runtime_events.ts`
- `apps/desktop/test/project-runtime-view.test.ts`

**Acceptance criteria:**
- Takeover opens the exact linked conversation for the active run.
- The conversation surface shows task context and a back-link to task detail.

## Phase 4: Scheduling, Notifications, And Attention Flow

**Outcome:** async behavior is operationally usable rather than merely implemented.

### Slice 4A: Scheduler and collision semantics

**Priority:** P1

**Primary files:**
- `crates/octopus-server/src/task_scheduler.rs`
- `crates/octopus-server/src/project_tasks.rs`
- `crates/octopus-infra/src/project_tasks.rs`
- `crates/octopus-desktop-backend/src/main.rs`
- `apps/desktop/src-tauri/src/backend.rs`

**Must lock now:**
- timezone ownership
- recurring vs once
- skip-if-running behavior
- catch-up policy
- restart recovery
- skipped-run reason

**Acceptance criteria:**
- A skipped run is visible and explainable, not silently lost.
- One task never ends up with parallel active runs.

### Slice 4B: Notification projection and delivery

**Priority:** P1

**Primary files:**
- `apps/desktop/src/stores/notifications.ts`
- `apps/desktop/src/stores/message-center.ts`
- `apps/desktop/src/stores/project_task.ts`
- `apps/desktop/src/components/task/ProjectTaskListPane.vue`
- `crates/octopus-server/src/project_tasks.rs`
- `crates/octopus-server/src/workspace_runtime.rs`

**Minimum v1 notification reasons:**
- run started
- run completed
- run failed
- approval required
- intervention applied
- scheduled run skipped

**Acceptance criteria:**
- Users do not need to poll the task page to notice important changes.
- Attention badges in the task list and notification center are consistent with backend state.

### Slice 4C: Failure taxonomy surfaced in UX

**Priority:** P1

**Primary files:**
- `apps/desktop/src/components/task/TaskRunSummaryCard.vue`
- `apps/desktop/src/components/task/ProjectTaskDetailPane.vue`
- `apps/desktop/src/stores/project_task.ts`
- `crates/octopus-server/src/dto_mapping.rs`

**Minimum categories:**
- context unavailable
- permission blocked
- approval timeout
- runtime error
- model failure
- user canceled

**Acceptance criteria:**
- Failed runs show category plus next recommended action.
- The frontend does not need to interpret raw error messages as product state.

## Phase 5: Analytics, Dashboard, And Governance Hardening

**Outcome:** the feature is measurable, governable, and safe to broaden.

### Slice 5A: Project dashboard integration

**Priority:** P2

**Primary files:**
- `apps/desktop/src/views/project/ProjectDashboardView.vue`
- `apps/desktop/src/stores/workspace.ts`
- `apps/desktop/src/stores/workspace_actions.ts`
- `crates/octopus-server/src/workspace_runtime.rs`
- `contracts/openapi/src/components/schemas/projects.yaml`

**Minimum dashboard metrics:**
- total tasks
- active tasks
- scheduled vs manual runs
- completion rate
- recent failures
- approval-required count

**Acceptance criteria:**
- Project dashboard can answer whether tasks are working, not just whether they exist.

### Slice 5B: Analytics summary and history

**Priority:** P2

**Primary files:**
- `crates/octopus-server/src/project_tasks.rs`
- `crates/octopus-platform/src/project.rs`
- `crates/octopus-infra/src/project_tasks.rs`
- `apps/desktop/src/components/task/TaskAnalyticsPanel.vue`
- `apps/desktop/src/stores/project_task.ts`

**Acceptance criteria:**
- A task detail page can show run count, success rate, failure mix, average duration, and takeover rate.

### Slice 5C: Governance and regression coverage

**Priority:** P2

**Primary files:**
- `apps/desktop/test/repo-governance.test.ts`
- `apps/desktop/test/projects-view.test.ts`
- `apps/desktop/test/project-tasks-view.test.ts`
- `crates/octopus-server/src/lib.rs`
- `apps/desktop/src/composables/project-governance.ts`

**Must protect now:**
- module permission wiring
- route and menu registration
- localization keys
- task transport parity
- notification reason enum coverage
- failure category enum coverage

**Acceptance criteria:**
- Future task work cannot silently break core project-module wiring or shrink transport semantics.

## 5. Stretch Backlog

These are intentionally out of the critical path:

- interactive connector views inside task detail
- cross-project task rollups
- task dependencies and graphs
- private visibility modes
- multi-run queueing per task
- advanced automation authoring UI

## 6. Suggested Delivery Sequence

If one team is executing this in order, use this sequence:

1. `0A -> 0B -> 0C`
2. `1A -> 1B -> 1C`
3. `2A -> 2B -> 2C`
4. `3A -> 3B -> 3C`
5. `4A -> 4B -> 4C`
6. `5A -> 5B -> 5C`

If work is parallelized, the safest split is:

- backend stream: `0A, 1A, 1B, 1C, 4A, 5B`
- desktop stream: `0B, 0C, 2A, 2B, 2C, 3C, 4B, 4C, 5A`
- integration and governance stream: `3A, 3B, 5C`

## 7. Ship Gates

### Internal dogfood gate

Must include:
- Phase 0
- Phase 1
- Slice `2A`, `2B`
- Slice `3A`, `3B`, `3C`

### Async beta gate

Must include:
- all of Phase 4

### Broad rollout gate

Must include:
- all of Phase 5

## 8. Immediate Next Tasks

The next three implementation tasks should be:

1. add `TaskContextBundle`, `TaskAttentionState`, `TaskFailureCategory`, and analytics summary to OpenAPI and generated schema
2. add matching projection fields in `crates/octopus-core` and `crates/octopus-infra`
3. create the desktop `project_task` store with those fields before building the full task page
