# Project Task Phase 0 Contract And Store Spec

**Goal:** Make Phase 0 concrete enough that OpenAPI, generated schema, and the desktop task store can be implemented without redesigning the model mid-flight.

**Scope:** This document covers only the first implementation slice:

- task transport types
- task list and detail field contracts
- context bundle representation
- attention and failure modeling
- analytics summary shape
- desktop store boundaries

**This document does not cover:** final UI composition, full scheduler implementation, or backend migrations in detail.

---

## 1. Phase 0 Objective

Before building the full task surface, Octopus needs one stable answer to this question:

`What exact fields make a task understandable, runnable, reviewable, and alertable without the client reverse-engineering runtime internals?`

Phase 0 succeeds when:

- OpenAPI can express task semantics directly
- `@octopus/schema` exports those semantics cleanly
- the desktop store shape matches the intended backend model
- later phases can add behavior without changing the core nouns

## 2. Design Rules

1. `Task` is the durable project object.
2. `TaskRun` is the execution projection for one launch.
3. `Context bundle` is a first-class object, not an optional blob string.
4. `Attention` is not the same thing as lifecycle status.
5. `Failure category` is product state, not just error text.
6. `Analytics summary` belongs in transport early, even if the first implementation is small.
7. The desktop store may derive selection and loading state, but it must not derive business meaning from raw runtime strings.

## 3. Recommended Contract Shape

## 3.1 TaskSummary

`TaskSummary` is the list-row contract.

Recommended fields:

- `id`
- `projectId`
- `title`
- `goal`
- `defaultActorRef`
- `status`
- `scheduleSpec`
- `nextRunAt`
- `lastRunAt`
- `latestResultSummary`
- `latestFailureCategory`
- `latestTransition`
- `viewStatus`
- `attentionReasons`
- `activeTaskRunId`
- `analyticsSummary`
- `updatedAt`

Notes:

- `goal` in summary should be short enough for list display; `brief` belongs in detail.
- `viewStatus` should reuse shared `ViewStatus` unless task-specific semantics prove necessary.
- `attentionReasons` carries the actionable meaning; `viewStatus` only signals whether the row needs attention.

## 3.2 TaskDetail

`TaskDetail` extends summary with the fields needed to operate the task.

Recommended additional fields:

- `brief`
- `contextBundle`
- `latestDeliverableRefs`
- `latestArtifactRefs`
- `runHistory`
- `interventionHistory`
- `activeRun`
- `createdBy`
- `updatedBy`
- `createdAt`

Notes:

- `activeRun` should be nullable and should summarize the current run without requiring the client to hit runtime routes first.
- `runHistory` in detail can be paged later, but the contract should already support it.

## 3.3 TaskRunSummary

Recommended fields:

- `id`
- `taskId`
- `triggerType`
- `status`
- `sessionId`
- `conversationId`
- `runtimeRunId`
- `actorRef`
- `startedAt`
- `completedAt`
- `resultSummary`
- `failureCategory`
- `failureSummary`
- `viewStatus`
- `attentionReasons`
- `deliverableRefs`
- `artifactRefs`
- `latestTransition`

## 3.4 TaskInterventionRecord

Recommended fields:

- `id`
- `taskId`
- `taskRunId`
- `type`
- `payload`
- `createdBy`
- `createdAt`
- `appliedToSessionId`
- `status`

Recommended intervention status:

- `accepted`
- `rejected`
- `applied`

This lets the product explain whether an intervention was only recorded or already reflected into the live session.

## 4. Context Bundle Spec

## 4.1 Why this must be first-class

Without a context bundle, a task is only a stored prompt. That weakens scheduled runs, reruns, and explainability.

## 4.2 Recommended types

### TaskContextBundle

Recommended fields:

- `refs`
- `pinnedInstructions`
- `resolutionMode`
- `lastResolvedAt`

### TaskContextRef

Recommended fields:

- `kind`
- `refId`
- `title`
- `subtitle`
- `versionRef`
- `pinMode`

Recommended `kind` values:

- `resource`
- `knowledge`
- `deliverable`

Recommended `pinMode` values:

- `snapshot`
- `follow_latest`

Recommended `resolutionMode` values:

- `explicit_only`
- `explicit_plus_project_defaults`

## 4.3 V1 semantics

- `resource` refs point to project resources.
- `knowledge` refs point to project knowledge entries.
- `deliverable` refs point to prior deliverables or artifact version refs.
- `snapshot` means the run should use the version captured when the task is launched.
- `follow_latest` means the system should resolve the latest version at launch time.

## 4.4 What not to do

- Do not store context bundle as an opaque JSON string in transport.
- Do not make the client expand context refs by calling three unrelated APIs just to render task detail.
- Do not hide context bundle inside `brief`.

## 5. Attention Model

## 5.1 Recommended approach

Use shared `ViewStatus` for the top-level display state:

- `healthy`
- `configured`
- `attention`

Then add task-specific `attentionReasons`:

- `updated`
- `needs_approval`
- `failed`
- `waiting_input`
- `schedule_blocked`
- `takeover_recommended`

This is better than inventing a second generic status enum because:

- the repo already has `ViewStatus`
- list and dashboard surfaces can reuse the same display grammar
- the actionable meaning still lives in the reason array

## 5.2 Recommended fields

At task and task-run level:

- `viewStatus`
- `attentionReasons`
- `attentionUpdatedAt`

## 5.3 Derivation rule

The server owns this derivation.

The client may sort or filter by attention, but must not compute `failed` or `needs_approval` by reading raw run status strings alone.

## 6. Failure Taxonomy

## 6.1 Recommended enum

`TaskFailureCategory`:

- `context_unavailable`
- `permission_blocked`
- `approval_timeout`
- `runtime_error`
- `model_failure`
- `user_canceled`

## 6.2 Recommended fields

At run level:

- `failureCategory`
- `failureSummary`

At task level:

- `latestFailureCategory`

## 6.3 Rule

Every terminal failed run should have a category, even if the low-level error message is also preserved elsewhere.

## 7. Transition Summary

The product needs a lightweight description of the most recent meaningful event without loading full trace.

### TaskStateTransitionSummary

Recommended fields:

- `kind`
- `summary`
- `at`
- `runId`

Recommended `kind` values:

- `created`
- `launched`
- `progressed`
- `waiting_approval`
- `completed`
- `failed`
- `intervened`
- `skipped`

This transition object supports:

- list updates
- notification copy
- recent activity
- dashboard cards

## 8. Analytics Summary

## 8.1 Why it belongs in Phase 0

If analytics is delayed too long, later phases will hardcode task views around only current state and lose the chance to make the object measurable from the start.

## 8.2 Recommended type

### TaskAnalyticsSummary

Recommended fields:

- `runCount`
- `manualRunCount`
- `scheduledRunCount`
- `completionCount`
- `failureCount`
- `takeoverCount`
- `approvalRequiredCount`
- `averageRunDurationMs`
- `lastSuccessfulRunAt`

## 8.3 Scope

- `TaskSummary` should carry a lightweight `analyticsSummary`.
- richer history views can come later.

## 9. Endpoint-Level Recommendations

Recommended task routes should support these response shapes from day one:

- `GET /api/v1/projects/{projectId}/tasks` -> list of `TaskSummary`
- `POST /api/v1/projects/{projectId}/tasks` -> `TaskDetail`
- `GET /api/v1/projects/{projectId}/tasks/{taskId}` -> `TaskDetail`
- `PATCH /api/v1/projects/{projectId}/tasks/{taskId}` -> `TaskDetail`
- `POST /api/v1/projects/{projectId}/tasks/{taskId}/launch` -> `TaskRunSummary`
- `POST /api/v1/projects/{projectId}/tasks/{taskId}/rerun` -> `TaskRunSummary`
- `GET /api/v1/projects/{projectId}/tasks/{taskId}/runs` -> list of `TaskRunSummary`
- `POST /api/v1/projects/{projectId}/tasks/{taskId}/interventions` -> `TaskInterventionRecord`

## 10. Desktop Store Spec

## 10.1 Store split

Use one dedicated store:

- `apps/desktop/src/stores/project_task.ts`

Do not spread task business state across unrelated runtime stores.

## 10.2 Recommended top-level store state

```ts
type ProjectTaskStoreState = {
  listByProjectId: Record<string, TaskSummary[]>
  detailByTaskId: Record<string, TaskDetail>
  runHistoryByTaskId: Record<string, TaskRunSummary[]>
  selectedTaskIdByProjectId: Record<string, string | null>
  filtersByProjectId: Record<string, TaskListFilterState>
  draftsByTaskId: Record<string, TaskEditorDraft>
  createDraftByProjectId: Record<string, TaskEditorDraft | null>
  notificationsByTaskId: Record<string, TaskNotificationView[]>
  loading: {
    list: boolean
    detailByTaskId: Record<string, boolean>
    launchByTaskId: Record<string, boolean>
    saveByTaskId: Record<string, boolean>
  }
}
```

## 10.3 Store-owned derived state

The store may derive:

- selected row
- filter results
- whether detail is stale
- whether a notification has been locally dismissed

The store must not derive:

- failure category
- attention reason
- latest transition meaning
- lifecycle status

## 10.4 Recommended local draft types

### TaskEditorDraft

Recommended fields:

- `title`
- `goal`
- `brief`
- `defaultActorRef`
- `scheduleSpec`
- `contextBundle`

### TaskListFilterState

Recommended fields:

- `status`
- `attentionOnly`
- `actorRef`
- `scheduleMode`
- `query`

### TaskNotificationView

Recommended fields:

- `taskId`
- `reason`
- `summary`
- `at`
- `read`

## 11. Phase 0 Acceptance Checklist

- [ ] OpenAPI includes `TaskContextBundle`, `TaskContextRef`, `TaskFailureCategory`, `TaskStateTransitionSummary`, and `TaskAnalyticsSummary`.
- [ ] Task list transport includes `viewStatus`, `attentionReasons`, and `latestTransition`.
- [ ] Task detail transport includes `contextBundle`, `activeRun`, and `runHistory`.
- [ ] Desktop fixtures can represent all new fields.
- [ ] `project_task` store shape exists and is aligned to the transport model.
- [ ] The client no longer needs to guess task meaning from raw runtime status text.

## 12. Immediate Implementation Order

1. define the new schema objects in `contracts/openapi/src/components/schemas/tasks.yaml`
2. thread them through `contracts/openapi/src/paths/tasks.yaml` and project dashboard schemas
3. regenerate `@octopus/schema`
4. add the matching `project_task` store state and fixtures
5. only then start backend persistence and desktop page work
