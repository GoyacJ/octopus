# Octopus Module Area Design Guide

This guide extends `docs/design/DESIGN.md`.
`DESIGN.md` remains the visual source of truth for color, type, radius, spacing, motion, and shared components.
This document defines how complex module content areas should be structured.

The image exploration showed a useful direction: Octopus modules should feel like operational workbenches, not generic settings pages.
Each major module should make the active object, live state, related work, and next action visible in the same viewport.

## 1. Source Summary

### Generated Module Patterns

The generated module previews consistently used these patterns:

- a persistent shell: left navigation, compact top context bar, and stable runtime/connection status
- a signal strip near the top for metrics, health, counts, and immediate risk
- a primary work zone for the module's main activity
- a secondary zone for related work such as tasks, runs, versions, or recent events
- a right inspector for the selected entity, permissions, context, and actions
- dense tables, queues, timelines, boards, matrices, and file trees instead of repeated summary cards
- local-first cues such as connection, sync, storage, runtime health, and trace status

The previews used teal as an active execution accent.
Do not copy that palette directly.
In Octopus implementation, use the canonical Calm Intelligence tokens and Octopus accent from `docs/design/DESIGN.md`.
The reusable part is the information architecture.

### Current Project Facts

- `docs/design/DESIGN.md` defines the desktop language, page archetypes, tokens, and governance checklist.
- `apps/desktop/src/layouts/WorkbenchLayout.vue` provides the persistent shell, topbar, main canvas, sidebar, and search overlay.
- `packages/ui/src/components/UiPageShell.vue` provides standard, wide, and full page widths plus density presets.
- `packages/ui/src/components/UiListDetailWorkspace.vue`, `UiListDetailShell.vue`, and `UiInspectorPanel.vue` already provide a list/detail/inspector foundation.
- `packages/ui/src/components/UiPanelFrame.vue` provides panel framing for page sections.
- `apps/desktop/src/router/index.ts` defines the main module surfaces: workspace overview, console, project dashboard, conversations, deliverables, tasks, agents, resources, knowledge, trace, settings, access control, personal center, connections, and app settings.
- `apps/desktop/src/navigation/menuRegistry.ts` groups navigation into app, workspace, console, project, and access-control sections.
- `apps/desktop/src/views/project/ProjectDashboardView.vue` already aggregates project metrics, users, tool ranking, model/resource breakdowns, conversations, activity, and deliverables.
- `apps/desktop/src/views/project/ConversationView.vue` already binds conversation messages to actor selection, model options, permission mode, resources, artifacts, queue state, approvals, and runtime context.
- `apps/desktop/src/views/workspace/ModelsView.vue` already follows a strong list/detail configuration pattern.
- `apps/desktop/src/views/workspace/useToolsView.ts` already models built-in tools, skills, MCP servers, skill files, imports, copies, and config editing.
- `apps/desktop/src/stores/workspace-access-control.ts` already exposes users, org units, roles, permissions, bindings, data policies, resource policies, menu policies, sessions, audit records, resources, and current authorization.

## 2. Design Gap

The current design system defines the visual language and page archetypes.
The missing layer is the module-area composition rule.

Most modules know what data they manage, but their content areas can still degrade into one of these weaker shapes:

- a page header followed by unrelated panels
- list/detail pages where the detail panel becomes the whole product story
- dashboards that show counts but hide the active work
- configuration pages that separate state, validation, permissions, and actions too far apart
- conversation/runtime pages where queue, approvals, tools, artifacts, and trace are present but not organized as one operational sidecar

The target is a repeatable module workbench.
Every module should answer four questions in one scan:

1. What is selected or active?
2. What is happening now?
3. What is blocked or risky?
4. What can the user do next?

## 3. Module Workbench Anatomy

Use this anatomy for high-density modules.
It can be implemented with existing shared primitives first.

### 3.1 Context Bar

The topbar should keep global and route-level context visible:

- workspace selector
- project selector when the module is project-scoped
- global search
- connection state
- runtime health
- account and notifications

Do not repeat this information as page-local hero content.
Module headers should be short.

### 3.2 Signal Strip

Place compact signals directly below the module header when the module has live or aggregate state.

Use signals for:

- session count
- active agents or teams
- task counts
- pending approvals
- token or budget usage
- resources and knowledge counts
- connection, sync, storage, or health state

Signals are not decorative metric cards.
Each signal must have a reason to exist and should link to the relevant area when possible.

### 3.3 Primary Work Zone

The primary work zone contains the module's main job:

- conversation stream
- task board
- model table
- skill file editor
- resource preview
- permission matrix
- project activity timeline

This zone should be visually dominant.
It should not be interrupted by unrelated cards.

### 3.4 Secondary Work Zone

The secondary zone keeps related work near the primary zone:

- recent runs
- trace events
- deliverable versions
- queue lanes
- activity timeline
- related resources
- validation results
- affected consumers

Use this zone to reduce navigation between modules.
Do not turn it into a duplicate dashboard.

### 3.5 Inspector Rail

Use a persistent right inspector when the page has a selected entity or live execution context.

The inspector should show:

- selected identity or object summary
- ownership and status
- model route, credential, or budget where relevant
- permissions and access state
- linked tools, skills, MCP servers, resources, and knowledge
- approvals, trace, sessions, and audit status
- primary actions for the selected object

Keep the inspector narrow and scannable.
Prefer grouped rows, chips, and short status labels over paragraphs.

### 3.6 Action Placement

Primary actions belong near the thing they change:

- create/import actions in the module toolbar
- save/validate/probe actions in the selected detail or inspector
- approve/deny actions inside the approval block
- promote/fork actions inside the deliverable inspector
- send/queue controls inside the composer

Avoid putting all actions in the page header.

## 4. Layout Templates

### 4.1 Command Center

Use for workspace overview and project dashboard.

Structure:

```text
Header
Signal strip
Main grid:
  Activity or run timeline
  Board, ranking, or live work panel
  Recent deliverables / trace / resources
Inspector or status rail when a project is active
```

Best for:

- `WorkspaceOverviewView`
- `ProjectDashboardView`

Rules:

- show live work, not only aggregate counts
- pair metrics with recent runs, task state, and blocked approvals
- keep connection and runtime health visible
- make project cards actionable, not just descriptive

### 4.2 Runtime Conversation

Use for conversations and agent execution.

Structure:

```text
Top controls: model, actor, permission, runtime state
Message stream
Docked composer
Right runtime sidecar:
  Queue
  Active run
  Pending approval
  Tool calls
  Artifacts
  Resources / knowledge
  Trace
```

Best for:

- `ConversationView`

Rules:

- distinguish scheduled turns from executing runs
- show approval state next to the action it blocks
- show artifacts and resources as active context, not as hidden attachments
- keep trace reachable from the current run

### 4.3 Registry Manager

Use for agents, teams, models, tools, skills, MCP servers, and access resources.

Structure:

```text
Toolbar: search, filters, scope, create/import actions
Left or center list/table/grid
Detail editor
Inspector summary for validation, permissions, consumers, and actions
```

Best for:

- `WorkspaceAgentsView`
- `ProjectAgentsView`
- `ModelsView`
- `ToolsView`
- access-control management pages

Rules:

- preserve selection while filtering
- put validation and permission state beside the editable fields
- expose consumers and project assignment in the same view
- use tables for comparable records and cards only when visual identity matters

### 4.4 File And Artifact Workspace

Use for resources, knowledge, deliverables, and artifact versions.

Structure:

```text
Scope and import toolbar
Directory or collection tree
Content preview
Metadata and version inspector
Promotion / fork / save-version actions
```

Best for:

- `ProjectResourcesView`
- `WorkspaceResourcesView`
- `ProjectKnowledgeView`
- `WorkspaceKnowledgeView`
- `ProjectDeliverablesView`

Rules:

- keep tree, preview, and version metadata in one viewport
- show storage, hash, type, owner, scope, and updated time near the preview
- make promotion and fork state visible
- do not make users navigate away to understand version history

### 4.5 Governance Matrix

Use for access control and permissions.

Structure:

```text
Members or subjects list
Policy / role / permission matrix
Selected user or policy inspector:
  effective permissions
  visible menus
  resource grants
  sessions
  audit events
```

Best for:

- `AccessControlView`
- `AccessMembersView`
- `AccessPermissionsView`
- `AccessGovernanceView`

Rules:

- separate assigned roles from effective permissions
- show inherited, partial, allowed, and denied states distinctly
- keep audit and active sessions near the selected subject
- make menu visibility and resource grants visible as first-class results

## 5. Module-Specific Direction

### Workspace Overview

Keep the overview as an operational landing page.
It should show workspace health, active projects, recent runs, token use, activity, connection/runtime state, and storage/sync state.

Current `WorkspaceOverviewView.vue` already has metrics, project cards, recent conversations, project token usage, and activity.
The next improvement is to make recent runs and connection/runtime state more prominent and to reduce dependence on generic record cards.

### Project Command Center

The project dashboard should become the project operating room.
It should combine metrics, run timeline, task board, deliverable versions, recent trace events, and an active inspector.

Current `ProjectDashboardView.vue` has the data foundation.
The content area should be rearranged so active work and blocked approvals are visible before long analytical summaries.

### Conversation Runtime

The conversation page should behave like a runtime console.
The stream is primary, but the sidecar should consistently show queue, active run, pending approval, tool calls, artifacts, resources, knowledge, and trace.

Current `ConversationView.vue` already has most state.
The improvement is stronger grouping and a more stable right runtime sidecar.

### Agents And Teams

Agent and team management should use a registry plus detail editor.
Cards are useful for identity, but project assignment, tools, skills, MCP servers, prompts, and team membership should remain visible while editing.

Current agent center state already covers workspace/project scope, tabs, filters, import/export, avatars, prompts, tools, skills, MCP servers, and team members.
The content area should standardize filter rail, registry grid, and detail inspector.

### Models And Runtime Config

Models should remain table-first.
The selected model inspector should show provider, upstream model, surfaces, capability chips, credential state, validation, budget, traffic classes, reservation strategy, and probe result.

Current `ModelsView.vue` is already closest to the generated direction.
It should become the reference implementation for table plus configuration inspector pages.

### Tools, Skills, And MCP

Tools should not be a plain catalog only.
The content area should combine catalog list, file tree or server config, editor/preview, metadata, consumers, permissions, and enabled state.

Current `useToolsView.ts` already models this data.
The layout should make skill file editing and MCP config inspection first-class.

### Resources, Knowledge, And Deliverables

These modules should share a file-and-artifact workspace.
Use tree, preview, metadata, version history, promotion, and fork actions in one viewport.

Current resource and deliverable stores already support detail, content, children, promotion, versions, draft saving, promote, and fork.
The layout should reduce module switching between resources, knowledge, and deliverables.

### Access Control

Access control should be matrix-first.
Members, role presets, capability bundles, policies, menus, resources, sessions, and audit records should connect through the selected subject or selected policy.

Current access-control state already contains the needed governance data.
The content area should emphasize effective results, not only editable definitions.

## 6. Shared Component Implications

Prefer extending `@octopus/ui` before adding page-local layouts.

Potential shared additions:

- `UiModuleWorkbench`: common shell for toolbar, primary zone, secondary zone, and optional inspector
- `UiSignalStrip`: compact top metrics and health signals
- `UiInspectorRail`: persistent right rail with grouped status sections and pinned actions
- `UiRuntimeSidecar`: queue, run, approval, tools, artifacts, resources, and trace blocks
- `UiOperationalBoard`: task/status lanes with stable density and drag-ready structure
- `UiVersionTimeline`: artifact and deliverable version history
- `UiPermissionMatrix`: allowed, denied, partial, and inherited permission states

Add these only when two or more modules need the same structure.
Do not create a component for one page only.

## 7. Implementation Rules

- Keep `docs/design/DESIGN.md` tokens authoritative.
- Use existing `@octopus/ui` primitives first.
- Use `UiPageShell width="full"` for command centers, conversation runtime, and governance matrices when horizontal density is required.
- Use `UiListDetailWorkspace` for simple registry pages.
- Use a custom module workbench only when a page needs three zones or a persistent inspector.
- Use tables for comparable operational records.
- Use cards for identity objects, summaries, or objects where visual scanning matters.
- Use right inspectors for selected-object state and actions.
- Do not nest cards inside cards.
- Do not introduce page-local palettes.
- Do not hide approval, credential, permission, blocked, or runtime state behind secondary navigation.
- Do not make dense modules depend on long prose descriptions.

## 8. Suggested Migration Order

1. Treat `ModelsView` as the first reference for table plus inspector refinement.
2. Refactor `ConversationView` sidecar into stable runtime groups.
3. Redesign `ProjectDashboardView` into the command-center layout.
4. Align `ToolsView` around catalog plus file/config editor plus metadata inspector.
5. Align resources, knowledge, and deliverables around the file-and-artifact workspace pattern.
6. Redesign access control around member list, permission matrix, and selected subject inspector.
7. Extract shared components only after two modules prove the same pattern.

## 9. Acceptance Checklist

For every redesigned module content area:

- the active workspace and project are visible
- the selected object is visible
- live or recent activity is visible
- blocked, pending, invalid, denied, or unhealthy states are visible
- the primary next action is near its object
- the right inspector, if present, explains the selected object without navigation
- the module uses canonical tokens and shared UI primitives
- the page still works in dark mode
- the layout avoids overlapping text at desktop widths
- the page can be scanned without reading paragraphs
