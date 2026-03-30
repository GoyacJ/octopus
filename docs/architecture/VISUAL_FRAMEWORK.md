# Visual Framework

**Status**: Revised post-GA desktop framework; GA minimum-surface constraints remain preserved where not explicitly replaced below
**Last updated**: 2026-03-30

## Purpose

This document freezes the first-round GA surface priorities, minimum page set, information architecture, and shared visual grammar for tracked UI work under `apps/`.

It does not expand PRD scope or architecture boundaries. It only defines how the already-approved GA minimum surface should be presented.

## Current Scope

This revised version defines the approved post-GA desktop interaction model for the tracked desktop shell:

- project-scoped `Dashboard`
- project-scoped `Conversation`
- project-scoped `Runs`
- project-scoped `Knowledge`
- workspace-scoped `Inbox`
- workspace-scoped `Projects`
- secondary `Connections`, `Models`, and `Preferences`
- shell-level reminder handling for notifications and degraded-state warnings

This version still does **not** define:

- a full cross-device conversation platform
- mobile-first layouts
- deep responsive variants beyond the desktop shell
- a repository-wide permanent design system for every future app
- a generic consumer-chat product surface

## Surface Priorities

The tracked post-GA desktop implementation must prioritize pages in this order:

1. Workspace / Project shell, reminder state, and preferences
2. `Dashboard` as the operational home
3. `Conversation` as the primary governed initiation path
4. `Run Detail`, `Runs`, and `Inbox` as authority surfaces
5. `Knowledge` as the provenance/read surface
6. `Connections`, `Models`, `Tasks`, and other expert tools as secondary surfaces

If scope pressure appears, keep `Dashboard`, `Conversation`, `Run Detail`, and `Inbox` coherent before enriching secondary tools.

## Minimum Page Set

### 1. Workspace Shell

Purpose:

- present current workspace and project context
- surface current hub mode, connection health, locale, theme, and reminder state
- anchor navigation for all primary and secondary desktop surfaces

Required regions:

- workspace name
- project name
- current hub mode (`local` or `remote`)
- connection status badge
- last synchronization / refresh hint
- locale preference
- theme preference
- reminder visibility for notifications and degraded state

### 2. Dashboard

Purpose:

- act as the default active-project landing page
- resume context, surface operational truth, and start or continue work

Required regions:

- context rail with workspace/project/hub/auth/read-only/locale/theme state
- one natural-language hero input
- continue-work modules for recent conversations, runs, and pending items
- system summary for in-progress runs, pending approvals, failed runs, and new knowledge
- risk/reminder region for auth expiry, degraded connection, blocked approvals, low-trust outputs, and write failures

Required states:

- idle
- populated operational state
- degraded/read-only
- empty recent activity

Constraint:

- `Dashboard` is operational and contextual, not a metrics-heavy BI surface.

### 3. Conversation

Purpose:

- let users form intent through governed Q&A instead of direct formal object entry
- keep execution proposal visible before any formal run exists

Required regions:

- conversation turn history
- guided clarification prompts
- persistent `Execution Proposal` side panel
- explicit execution confirmation control

Required stages:

- `drafting`
- `proposal_ready`
- `execution_started`
- `follow_up`

Explicit rule:

- drafting or revising must never auto-create a run
- formal execution begins only on explicit confirmation

### 4. Runs

Purpose:

- provide a focused recent-run follow-up surface ranked by what needs attention now

Required data:

- `RunSummary` rows only
- run title
- run status
- capability identifier or task context summary
- updated time
- direct navigation to run detail
- attention-oriented ordering or grouping

Required states:

- populated recent list
- empty recent list
- refetch after confirmed execution or `run.updated`

### 5. Run Detail

Purpose:

- give one authoritative page for run state, result, governance, and recovery context

Required sections:

- conclusion / current state
- result
- governance
- diagnosis
- recovery actions when allowed

Required states:

- created
- running
- waiting approval
- completed
- failed
- terminated

### 6. Approval Inbox

Purpose:

- provide the minimum formal review entry for approvals and blocking items

Required data:

- title
- reason/message
- related run reference
- created time
- current status
- approve / reject actions where allowed

### 7. Project Knowledge Index

Purpose:

- show the project-level Shared Knowledge loop with meaning, provenance, trust, and traceability before raw identifiers

Required data:

- project knowledge space label
- mixed project-visible knowledge candidates and shared assets
- trust / provenance state per entry
- source run or candidate traceability hint
- navigation back to `Run Detail` and `Inbox`

### 8. Secondary Tool Surfaces

Purpose:

- preserve dedicated environment/governance and expert execution surfaces without making them the first user entry

Includes:

- `Projects`
- `Connections`
- `Models`
- `Preferences`
- expert `Tasks`

## Information Architecture

The desktop shell should use a stable two-level IA:

- Global shell
  - Workspace context
  - Project context
  - Hub status
  - Reminder state
  - Primary navigation
  - Secondary tools and preferences
- Content area
  - Dashboard
  - Conversation
  - Runs
  - Run Detail
  - Inbox
  - Knowledge
  - Projects
  - Connections
  - Models
  - Preferences
  - expert Tasks

The stable primary IA is `Dashboard / Conversation / Runs / Inbox / Knowledge`.

The stable secondary IA is `Projects / Connections / Models / Preferences`, with `Tasks` available as an expert-only direct execution path.

Notifications remain reminder context and must not compete with `Inbox` as a primary work destination.

## Shared Visual Grammar

### 1. Tone

- operational, not marketing
- clear system truth over decorative flourish
- readable status and provenance before secondary chrome

### 2. Layout

- desktop-first composition with a persistent shell
- one primary content column with optional secondary summary rail
- avoid dashboard sprawl; each page should answer one operational question clearly
- the default active-project route should be `Dashboard`
- `Conversation` may use a two-column layout with proposal context on the side

### 3. Hierarchy

- context first: workspace, project, hub mode
- intent second: dashboard state, conversation proposal, next action
- formal object third: run, artifact, approval, knowledge
- diagnostics fourth: policy decisions, trace, lineage

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

- Conversation drafting and proposal editing must remain explicit; do not auto-start tasks on turn entry or field edit.
- Task/run creation must remain explicit; formal execution starts only on confirmation.
- Approval actions must require deliberate user action and show the resulting run state after completion.
- Trace is explanatory, not decorative. Do not hide it behind development-only affordances.
- Knowledge index pages remain read-only; promotion requests stay on `Run Detail`, and approval resolution stays on `Inbox`.
- Notification is a reminder surface; Inbox is the action surface. Do not collapse them into one list.
- Connections is a dedicated visibility surface for hub mode, auth/session, and refresh state. Do not bury it only inside the mixed workspace body.
- `Tasks` may remain available for expert direct execution, but it must not be the default landing page or the only supported initiation path.

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
