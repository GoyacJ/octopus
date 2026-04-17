# Project Task Workbench Design Evaluation

**Goal:** Evaluate the current `Task` implementation direction end to end, using public Claude Cowork materials as external input, then define the product and technical design rules Octopus should adopt before implementation moves forward.

**Scope:** This document covers product positioning, information architecture, execution semantics, governance, runtime integration, scheduling, and rollout boundaries for the project task feature.

**Primary source of truth:** This document complements, but does not replace, [DESIGN.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/design/DESIGN.md) and the implementation plan at [2026-04-17-project-task-workbench-implementation.md](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/plans/task/2026-04-17-project-task-workbench-implementation.md).

---

## 1. Executive Assessment

The current implementation direction is broadly correct.

The key strategic decision already made in the implementation plan, `task as a project asset built on top of the existing runtime`, is the right foundation. It matches our actual product requirements better than either a chat-only mode or a generic automation system.

However, the current plan is still stronger on system decomposition than on product operating rules. After reviewing public Claude Cowork materials, the main gap is clear:

- Cowork is not compelling because it is merely asynchronous.
- Cowork is compelling because it packages async execution, context access, reviewable outputs, human takeover, and enterprise controls into one coherent work unit.

For Octopus, the implication is straightforward:

- We should keep the current task-domain architecture.
- We should strengthen the feature around `context packaging`, `results-first review`, `human intervention`, `notifications`, and `governance`.
- We should avoid turning tasks into either:
  - a hidden runtime implementation detail, or
  - a vague assistant mode with no durable project object model.

## 2. What Cowork Appears To Optimize For

Based on Anthropic's public materials reviewed on **April 17, 2026**, Cowork is positioned around five ideas.

### 2.1 From answers to execution

Anthropic frames the product as the next step after chat and coding assistance: not just answering questions, but handling multi-step work. Public materials emphasize research synthesis, document creation, and data extraction rather than simple prompt-response chat.

### 2.2 Human-in-the-loop first-pass work

Cowork is repeatedly framed as taking the first pass on work that is expensive but reviewable, then routing the human to the calls that actually require judgment. That is a strong fit for legal, research, operations, and similar knowledge workflows.

### 2.3 Tool-connected context

The product story is deeply tied to connectors, organization search, and access to the user's real systems. Context is not treated as static uploads alone. It is live, permission-aware, and increasingly interactive.

### 2.4 Chat plus interface, not chat alone

Anthropic's connector model supports inline cards and fullscreen interactive views. That means Cowork is not just a transcript. It can surface a task board, dashboard, design tool, or editor inside the same flow.

### 2.5 Enterprise rollout as a first-class concern

Anthropic is not selling Cowork only as an end-user feature. The public rollout story includes role-based controls, spend caps, analytics, OpenTelemetry support, and connector tool governance. In other words, management and observability are part of the product, not afterthoughts.

## 3. What We Should Learn From Cowork

### 3.1 The durable unit is the work item, not the chat

This is the biggest lesson.

Cowork's public positioning implies that users think in terms of work being delegated, monitored, reviewed, and taken over. That maps more naturally to a durable task object than to an isolated conversation.

For Octopus, this validates the existing decision:

- `Task` is the durable unit.
- Chat is the execution and intervention surface for a run of that task.

### 3.2 Async execution must still feel inspectable

Users trust async agent work when they can answer:

- what is it doing now
- what context is it using
- what changed
- what needs me
- what came out

This means task detail cannot be a thin metadata page. It must be an inspectable work surface.

### 3.3 Results should outrank process

Cowork's strongest public use cases center on outcomes: draft, redline, extract, compare, summarize, compile. The user is trying to reach an artifact or decision.

For Octopus, the task detail page should therefore privilege:

1. latest result summary
2. deliverable and artifact refs
3. current status and blockers
4. intervention controls
5. only then deeper runtime evidence

### 3.4 Connectors matter because they reduce prompt assembly cost

One of Cowork's real advantages is not merely "access more data." It reduces the user's need to manually gather and restate context every time.

Octopus should mirror this principle, but in our own shape:

- reuse project resources, knowledge, deliverables, and selected runtime context
- let a task declare reusable context sources
- make reruns and scheduled runs rehydrate from those sources

### 3.5 Human takeover should be a normal path, not an exception path

Cowork's design language suggests that agentic work and human steering are part of one flow. That is exactly the right model for us.

Task takeover in Octopus should therefore be:

- visible
- state-preserving
- cheap
- reversible

It should not feel like abandoning one system and opening another.

### 3.6 Governance is part of product quality

The more autonomous the feature feels, the more important these become:

- permission boundaries
- connector controls
- usage analytics
- cost controls
- auditability

If we omit these, the feature may demo well but will be hard to operationalize in a real multi-user environment.

## 4. What We Should Not Copy From Cowork

### 4.1 Do not make tasks an account-level assistant mode

Cowork appears to be organized around the user's Claude environment and connector graph. Octopus is different. Our users are already inside a project-centric workbench with explicit project assets and shared outputs.

So we should not present tasks as:

- a floating global helper
- a private side mode
- an alternate chat persona

We should present tasks as project work objects.

### 4.2 Do not over-index on cloud connector UX in v1

Cowork's connector story is cloud-forward and account-brokered. Octopus has stronger local-first and workspace-root responsibilities. We should learn from the value of reusable context, but we should not block the feature on a broad remote connector surface.

For v1, task context should primarily come from:

- project resources
- project knowledge
- prior deliverables
- selected actor defaults
- explicit user brief

Connector-heavy and interactive-connector-heavy expansion can come later.

### 4.3 Do not make every task conversation private by default

Public Claude materials around enterprise search and projects remain heavily user-authenticated and private-conversation-oriented. Our requirement is different: project tasks should be visible to project members.

That means:

- shared task visibility is correct for Octopus
- run summaries and outputs should be project-visible by default
- private side chats are the wrong default for this feature

### 4.4 Do not let generic research use cases swallow operational clarity

Cowork can afford broad "do work for me" positioning because it is a general product. Octopus cannot rely on ambiguity.

Every task in Octopus should make these things explicit:

- desired outcome
- actor
- schedule
- context sources
- success or completion state
- latest output

## 5. Product Design Recommendation For Octopus

## 5.1 Product thesis

Octopus tasks should be:

`A project-shared work unit that can run asynchronously, produce reviewable outputs, and stay connected to the live conversation when a human needs to steer or take over.`

That is narrower than Cowork, but it is also more durable and more operationally clear.

## 5.2 Primary information architecture

The right top-level shape remains:

- project sidebar entry: `Tasks`
- page layout: `list -> detail`
- detail linked to live conversation when running

This should not become:

- chat first, task second
- dashboard first, task hidden inside panels
- automation page with generic job rows

## 5.3 Task list requirements

The task list should answer, at a glance:

- what is this task for
- who usually runs it
- whether it is manual or scheduled
- what happened last
- what happens next
- whether it needs attention

Recommended row fields:

- title
- short goal or summary
- actor badge
- status
- next run
- last result summary
- attention state badge such as `needs approval`, `failed`, or `updated`

## 5.4 Task detail requirements

The detail page should be the operating surface for one task.

Recommended section order:

1. header: title, status, actor, schedule
2. latest result block
3. output refs: deliverables and artifacts
4. live run summary or blocker state
5. task brief and editable goal
6. context sources
7. run history
8. intervention history
9. deeper ops and trace link

This preserves the results-first rule while still making active work inspectable.

## 5.5 Create-task flow

The create flow should stay opinionated and short.

Required inputs:

- title
- goal
- brief
- default actor
- schedule mode
- selected context sources

Optional but useful:

- expected output type
- success checklist

If we skip expected output shape entirely, tasks will become too vague and reruns will become noisy.

## 5.6 Run lifecycle

Recommended lifecycle:

- `draft`
- `ready`
- `running`
- `waiting_approval`
- `completed`
- `failed`
- `paused`
- `archived`

Recommended active attention states layered on top:

- `updated`
- `needs_takeover`
- `needs_approval`
- `schedule_blocked`

The base lifecycle is durable state. The attention state is short-term operator signal.

## 5.7 Takeover and intervention model

Takeover should open the exact linked conversation or session for the active run.

Interventions should be modeled as explicit actions:

- edit goal
- edit brief
- add steering note
- change actor
- take over
- return to async

These should be visible in task history so the run remains explainable.

## 5.8 Notification model

The current implementation plan underweights this.

Tasks need a clear notification surface because async execution only feels useful if state changes become visible without constant polling.

Recommended v1 notification events:

- run started
- run completed
- run failed
- approval required
- user intervention applied
- next scheduled run skipped because already running

These should feed:

- in-app notifications
- project inbox or message center entries
- task list attention badges

## 5.9 Human review rule

Borrow the "first-pass review" lesson directly.

The product should teach the user that tasks are best for:

- preparing a first draft
- assembling evidence
- extracting structure
- comparing versions
- compiling context

The product should not imply unsupervised final authority on sensitive outcomes.

## 6. Recommended Scope Boundaries

### 6.1 V1 must include

- project-scoped task asset
- list-detail task surface
- manual launch
- recurring and one-off scheduling
- one active run per task
- result summary plus output refs
- live takeover into linked conversation
- editable goal and brief during active run
- project-shared visibility
- notifications and attention states
- basic task analytics at project level

### 6.2 V1 should include if low incremental cost

- expected output template
- schedule preview
- failure categorization
- run duration metrics
- per-task default context bundle presets

### 6.3 Move to later phases

- interactive connector surfaces inside task detail
- complex multi-step workflow graphs
- cross-project task rollups
- private visibility and sharing variants
- advanced queue routing and task dependencies

## 7. Technical Assessment Of The Current Implementation Plan

## 7.1 What is already correct

The implementation plan has four strong architectural decisions that should remain unchanged.

### A. Task as project asset

This is correct and should not be weakened.

### B. Runtime reuse instead of runtime replacement

This is also correct. Tasks should orchestrate existing runtime sessions and runs, not duplicate them.

### C. Separate task-domain persistence

This is correct because durable task semantics differ from raw runtime projections.

### D. Separate task scheduler, not workspace automation revival

This is correct and important. Reusing deleted automation UX or semantics would confuse both the product and the codebase.

## 7.2 What the implementation plan still needs

### A. Stronger context packaging

The plan currently defines scheduling and run state well, but it under-specifies how a task packages reusable context.

We should explicitly support:

- selected project resources
- selected knowledge refs
- selected prior deliverables
- optional pinned instructions

Without this, the task object will not feel meaningfully different from "rerun this conversation."

### B. Stronger notification and attention-state model

The implementation plan should not rely on users refreshing task detail to discover changes.

We need explicit projection fields and events for:

- unread update count or boolean attention state
- reason for attention
- most recent meaningful state transition

### C. Stronger analytics and audit shape

Cowork's public rollout story makes this obvious. Even if Octopus is not shipping enterprise admin controls immediately, the task domain should already emit clean analytics and audit events.

Minimum recommended event families:

- task created and updated
- run launched and completed
- run failed and canceled
- intervention applied
- takeover started
- approval requested and resolved

### D. Clearer schedule semantics

The implementation plan needs explicit rules for:

- time zone ownership
- daylight saving transitions
- pause vs disable
- missed runs after restart
- rerun while schedule exists
- active-run collision behavior

These rules should be fixed in the contract, not improvised in the UI.

### E. Failure classification

A failed run should not only say `failed`.

Recommended failure buckets:

- context unavailable
- permission blocked
- approval timeout
- runtime error
- model failure
- user canceled

This will make notifications, analytics, and support far better.

## 8. Recommended Backend Design Rules

### 8.1 Domain split

Keep this separation:

- runtime = execution truth
- task = durable work object
- scheduler = due-run dispatcher
- notification = user attention surface

Do not collapse them into one server module.

### 8.2 Data ownership

Recommended ownership boundaries:

- `ProjectTaskRecord`: canonical task intent and schedule
- `ProjectTaskRunRecord`: task execution projection
- `ProjectTaskInterventionRecord`: human steering and takeover history
- runtime session and run: live machine execution state
- deliverable and artifact systems: output storage

### 8.3 Context binding

At run start, snapshot:

- actor
- selected context refs
- schedule trigger metadata
- effective config hash when relevant

This keeps the run explainable even if the task later changes.

### 8.4 Output binding

Each run should store:

- latest deliverable refs
- latest artifact refs
- result summary
- completion classification

The task record should cache the latest successful or latest terminal run summary for fast rendering.

### 8.5 Scheduler discipline

The scheduler should be conservative:

- one active run per task
- durable claim or lease
- at-most-one catch-up run
- explicit skip reasoning

This matters more than raw dispatch speed.

## 9. Recommended Frontend Design Rules

### 9.1 List-detail is the right layout

Keep it.

The feature should feel like reviewing and operating project work, not browsing background jobs.

### 9.2 Conversation should be linked, not embedded everywhere

The current recommendation stands:

- task detail is primary
- conversation is the linked live-execution surface

Do not attempt to fully embed the conversation transcript into every task row or detail panel in v1.

### 9.3 Results-first is mandatory

Task detail should never open on trace by default.

If the latest run is completed, the first thing the user sees should be:

- summary
- outputs
- next recommended action

### 9.4 Attention states should be visible in list and sidebar

Users should know a task needs them before they open it.

Recommended visibility:

- row badge in task list
- project navigation count or dot when the module contains attention-worthy tasks
- notification center entries for major transitions

### 9.5 Editing should be local and inline

Goal and brief edits belong inline in task detail. They should not open a large wizard once the task already exists.

## 10. Governance Recommendations

Even if we do not ship full Anthropic-style enterprise controls in v1, the task model should be compatible with them.

Recommended permission split:

- `view_tasks`
- `edit_tasks`
- `run_tasks`
- `manage_task_schedule`
- `take_over_task_runs`

Project module access alone is too coarse if this feature grows.

Recommended future analytics dimensions:

- task count by project
- runs per day
- scheduled vs manual ratio
- completion rate
- average run duration
- takeover rate
- approval-required rate
- failure categories

## 11. Final Recommendation

We should proceed with the current architecture, but with five explicit adjustments.

### Adjustment 1

Treat `Task` as a project operating object, not as a convenience wrapper over chat.

### Adjustment 2

Add a first-class context bundle concept to the task model.

### Adjustment 3

Add notifications and attention states as part of v1, not as later polish.

### Adjustment 4

Add explicit failure categories, schedule rules, and audit event families before backend implementation hardens.

### Adjustment 5

Keep results-first review and live takeover as the defining UX promise.

If we do those five things, Octopus will capture the strongest parts of Cowork's design while still staying true to our own requirements:

- project-scoped instead of account-scoped
- shared work objects instead of private side chats
- runtime-backed instead of duplicated agent infrastructure
- local-first and workbench-oriented instead of connector-first for every flow

## 12. External Inputs Reviewed

The following public Anthropic materials informed this evaluation:

- [The Future of AI at Work: Introducing Cowork](https://www.anthropic.com/webinars/future-of-ai-at-work-introducing-cowork)
- [Deploying Cowork across the Enterprise — with PayPal](https://www.anthropic.com/webinars/deploying-cowork-across-the-enterprise-with-paypal)
- [Claude for Legal Teams](https://www.anthropic.com/webinars/claude-for-legal-teams)
- [Get started with custom connectors using remote MCP](https://support.claude.com/en/articles/11175166-get-started-with-custom-connectors-using-remote-mcp)
- [Use interactive connectors in Claude](https://support.claude.com/en/articles/13454812-use-interactive-connectors-in-claude)
- [View usage analytics for Team and Enterprise plans](https://support.claude.com/en/articles/12883420-view-usage-analytics-for-team-and-enterprise-plans)
- [Use enterprise search](https://support.claude.com/en/articles/12489464-use-enterprise-search)
- [What is the Enterprise plan?](https://support.claude.com/en/articles/9797531-what-is-the-enterprise-plan)
