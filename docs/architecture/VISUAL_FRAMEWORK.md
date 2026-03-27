# Visual Framework

**Status**: Frozen for the first GA minimum surface
**Last updated**: 2026-03-27

## Purpose

This document freezes the first-round GA surface priorities, minimum page set, information architecture, and shared visual grammar for tracked UI work under `apps/`.

It does not expand PRD scope or architecture boundaries. It only defines how the already-approved GA minimum surface should be presented.

## Current Scope

This first frozen version covers only the minimum GA surfaces required by the blueprint:

- Task creation entry
- Run detail and Trace replay
- Approval Inbox
- Artifact detail
- Shared Knowledge minimum view and promote action
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
2. Task creation
3. Run detail with status, policy, artifact, trace, and knowledge sections
4. Approval Inbox and Notification entry
5. Shared Knowledge minimum view and promote action

If scope pressure appears, keep the order above. Do not drop Hub status, Task create, Run detail, or Approval Inbox in favor of richer secondary views.

## Minimum Page Set

### 1. Workspace Shell

Purpose:

- present current workspace and project context
- surface current hub mode and connection health
- anchor navigation for all minimum GA pages

Required regions:

- workspace name
- project name
- current hub mode (`local` or `remote`)
- connection status badge
- last synchronization / refresh hint

### 2. Task Create

Purpose:

- let a user create and start a minimum task from a formal surface

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

### 3. Run Detail

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

### 4. Approval Inbox

Purpose:

- provide the minimum formal review entry for approvals and blocking items

Required data:

- title
- reason/message
- related run reference
- created time
- current status
- approve / reject actions where allowed

### 5. Artifact Detail

Purpose:

- present the primary execution output and its provenance clearly

Required data:

- artifact content
- artifact type
- provenance source
- trust level
- knowledge-gate status

### 6. Shared Knowledge Minimum View

Purpose:

- show the minimum knowledge loop without claiming a full knowledge-management product

Required data:

- project knowledge space label
- run-related knowledge candidates
- promoted shared assets relevant to the current flow
- promote action where explicit promotion is available
- lineage hint back to source run/artifact

### 7. Notification Entry

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
  - Task Create
  - Run Detail
  - Inbox
  - Knowledge
  - Connections

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
- no shared knowledge assets yet
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
- Knowledge promotion must remain explicit and reversible in wording, even if the current slice only proves promotion.
- Notification is a reminder surface; Inbox is the action surface. Do not collapse them into one list.

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
