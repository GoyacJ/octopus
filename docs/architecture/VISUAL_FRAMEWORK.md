# Visual Framework

**Status**: Frozen for the first GA minimum surface through Slice 15 project knowledge index
**Last updated**: 2026-03-29

## Purpose

This document freezes the first-round GA surface priorities, minimum page set, information architecture, and shared visual grammar for tracked UI work under `apps/`.

It does not expand PRD scope or architecture boundaries. It only defines how the already-approved GA minimum surface should be presented.

## Current Scope

This frozen version covers only the minimum GA surfaces required by the blueprint plus the tracked Slice 14 and Slice 15 desktop IA refinements:

- Task creation entry
- Recent project runs
- Run detail and Trace replay
- Approval Inbox
- Artifact detail
- Project-scoped Shared Knowledge read index
- Workspace / Project context presentation
- Hub Connections status
- Notification intake

This version does **not** define:

- a full board or multi-view collaboration experience
- mobile-first layouts
- deep responsive variants beyond the minimum desktop shell
- a shared component library or design-token system for every future app

## Surface Priorities

The first tracked UI implementation must prioritize pages in this order:

1. Workspace / Project shell and Hub Connection status
2. Task creation and recent run follow-up
3. Run detail with status, policy, artifact, trace, and knowledge sections
4. Approval Inbox and Notification entry
5. Project-scoped Shared Knowledge read index

If scope pressure appears, keep the order above. Do not drop Hub status, Task create, Runs, Run detail, or Approval Inbox in favor of richer secondary views.

## Minimum Page Set

### 1. Workspace Shell

Purpose:

- present current workspace and project context
- surface current hub mode and connection health
- anchor navigation for all minimum GA workbench pages

Required regions:

- workspace name
- project name
- current hub mode (`local` or `remote`)
- connection status badge
- last synchronization / refresh hint

### 2. Tasks

Purpose:

- let a user create and start a minimum task from a formal surface
- act as the default local-mode landing page for the current demo project

Required fields:

- title
- instruction
- capability selector or identifier input
- action payload editor appropriate to the minimum supported task flow
- estimated cost hint

Required states:

- idle
- submitting
- validation error
- success with navigation to the created run

### 3. Runs

Purpose:

- provide a focused recent-run follow-up surface for the current project

Required data:

- `RunSummary` rows only
- run title
- run status
- capability identifier or task context summary
- updated time
- direct navigation to run detail

Required states:

- populated recent list
- empty recent list
- refetch after task creation or `run.updated`

### 4. Run Detail

Purpose:

- give one authoritative page for run state, result, governance, and recovery context

Required sections:

- run summary
- status timeline or ordered state blocks
- approval status and policy decisions
- artifact result
- trace stream or ordered trace list
- knowledge candidate / recalled asset summary
- recovery actions when allowed

Required states:

- created
- running
- waiting approval
- completed
- failed
- terminated

### 5. Approval Inbox

Purpose:

- provide the minimum formal review entry for approvals and blocking items

Required data:

- title
- reason/message
- related run reference
- created time
- current status
- approve / reject actions where allowed

### 6. Artifact Detail

Purpose:

- present the primary execution output and its provenance clearly

Required data:

- artifact content
- artifact type
- provenance source
- trust level
- knowledge-gate status

### 7. Project Knowledge Index

Purpose:

- show the minimum project-level Shared Knowledge loop without claiming a full knowledge-management product

Required data:

- project knowledge space label
- mixed project-visible knowledge candidates and shared assets
- trust / provenance state per entry
- source run or candidate traceability hint
- navigation back to `Run Detail` and `Inbox`

### 8. Notification Entry

Purpose:

- expose reminders separately from Inbox facts

Required data:

- notification title
- message
- dedupe-aware status
- linked run or approval target

## Information Architecture

The minimum desktop shell should use a stable two-level IA:

- Global shell
  - Workspace context
  - Project context
  - Hub status
  - Primary navigation
- Content area
  - Tasks
  - Runs
  - Run Detail
  - Inbox
  - Connections
  - Notifications
  - Knowledge

The stable desktop workbench IA is `Tasks / Runs / Knowledge / Inbox / Notifications / Connections`.

The first tracked shell should prefer direct pages over nested modal flows. Approval decisions may use inline actions or simple dialogs, but the authoritative state must remain visible on the page after action completion.

## Shared Visual Grammar

### 1. Tone

- operational, not marketing
- clear system truth over decorative flourish
- readable status and provenance before secondary chrome

### 2. Layout

- desktop-first composition with a persistent shell
- one primary content column with optional secondary summary rail
- avoid dashboard sprawl; each page should answer one operational question clearly
- the default workbench route should be `Tasks`, not a mixed dashboard page

### 3. Hierarchy

- context first: workspace, project, hub mode
- object second: task, run, artifact, approval, knowledge
- diagnostics third: policy decisions, trace, lineage

### 4. Status Presentation

Every formal object state shown in the UI must use:

- a text label
- a visual variant
- a short explanatory hint when ambiguity exists

Do not rely on color alone to distinguish:

- approval pending vs rejected
- trusted vs low-trust artifact output
- completed vs completed-with-gated-knowledge outcome

### 5. Provenance And Trust

Provenance and trust are first-class visual signals.

Artifact and knowledge surfaces must show:

- source (`builtin`, `mcp_connector`, or equivalent)
- trust level
- knowledge gate outcome

Low-trust output must look materially different from verified shared knowledge and must never appear as if it has already been promoted.

### 6. Empty And Error States

Empty states must describe what formal object is missing, not use generic filler.

Examples:

- no inbox items
- no notifications
- no shared knowledge entries yet
- hub disconnected

Error states must preserve the user’s context and identify whether the failure came from:

- connection/transport
- runtime/governance
- approval decision
- validation

## Interaction Rules

- Task submission must remain explicit; do not auto-start tasks on field edit.
- Approval actions must require deliberate user action and show the resulting run state after completion.
- Trace is explanatory, not decorative. Do not hide it behind development-only affordances.
- Knowledge index pages remain read-only; promotion requests stay on `Run Detail`, and approval resolution stays on `Inbox`.
- Notification is a reminder surface; Inbox is the action surface. Do not collapse them into one list.
- Connections is a dedicated visibility surface for hub mode, auth/session, and refresh state. Do not bury it only inside the mixed workspace body.

## Out Of Scope

- changing product scope defined by `docs/product/PRD.md`
- changing architecture boundaries defined by `docs/architecture/SAD.md`
- introducing Beta-only pages into the GA default path
- freezing a long-term brand or design-system token library

## Update Trigger

Update this document when the repository needs to freeze or revise:

- GA page priorities
- the minimum page set
- shell/navigation hierarchy
- shared visual grammar for status, trust, and provenance
