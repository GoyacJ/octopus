# Project Leader And Live Workspace Inheritance Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build project-level `Leader` selection, live workspace inheritance for tools/agents/teams, and tabbed searchable project-setting dialogs without breaking existing project/runtime flows.

**Architecture:** Project metadata owns the single `leaderAgentId` field, while project grant/runtime config stops snapshotting workspace assets and instead stores project-local deltas on top of live workspace state. Effective project scope is always resolved from current workspace tools and active workspace actors, then refined by project exclusions, runtime disables, and tool permission overrides before the conversation surface computes its default actor.

**Tech Stack:** OpenAPI (`contracts/openapi`), generated transport schema (`packages/schema`), Rust server/infra/runtime adapter (`crates/octopus-*`), Vue 3 + Vite + Pinia + Vue Router + Vue I18n (`apps/desktop`), Vitest, Cargo tests.

---

### Task 1: Add Project Leader And Live-Inheritance Transport Shapes

**Files:**
- Modify: `contracts/openapi/src/components/schemas/projects.yaml`
- Modify: `crates/octopus-core/src/lib.rs`
- Modify: `packages/schema/src/project-settings.ts`
- Modify: `packages/schema/src/workspace.ts`
- Test: `crates/octopus-infra/src/projects_teams.rs`
- Test: `apps/desktop/test/tauri-client-workspace.test.ts`

**Step 1: Write the failing tests**

- Add an infra test in `crates/octopus-infra/src/projects_teams.rs` that creates a project with:
  - `leaderAgentId`
  - `assignments.tools.excludedSourceKeys`
  - `assignments.agents.excludedAgentIds`
  - `assignments.agents.excludedTeamIds`
  and asserts `create_project()`, `list_projects()`, and `update_project()` round-trip the new fields.
- Extend `apps/desktop/test/tauri-client-workspace.test.ts` so the workspace project create/update payload assertions include the new `leaderAgentId` and exclusion arrays.

**Step 2: Run the tests to verify they fail**

Run:

```bash
cargo test -p octopus-infra create_project_persists_leader_and_live_inheritance_fields
pnpm -C apps/desktop test -- tauri-client-workspace.test.ts
```

Expected:

- Rust test fails because `ProjectRecord`, `CreateProjectRequest`, `UpdateProjectRequest`, and assignment structs do not yet contain the new fields.
- Vitest fails because the client contract and payload expectations do not match the generated schema.

**Step 3: Write the minimal transport and core implementation**

- In `contracts/openapi/src/components/schemas/projects.yaml`, add:
  - `leaderAgentId?: string` to `CreateProjectRequest`, `UpdateProjectRequest`, and `ProjectRecord`
  - `excludedSourceKeys: string[]` to `ProjectToolAssignments`
  - `excludedAgentIds: string[]` and `excludedTeamIds: string[]` to `ProjectAgentAssignments`
- Keep `ProjectModelAssignments` unchanged in this plan.
- Mirror the same fields in `crates/octopus-core/src/lib.rs`.
- Update `packages/schema/src/project-settings.ts` so runtime settings can later move from “enabled lists” to “disabled lists”:

```ts
export interface ProjectToolSettings {
  disabledSourceKeys: string[]
  overrides: Record<string, ProjectToolPermissionOverride>
}

export interface ProjectAgentSettings {
  disabledAgentIds: string[]
  disabledTeamIds: string[]
}
```

- Keep `packages/schema/src/workspace.ts` as alias/re-export glue only.

**Step 4: Run the tests to verify they pass**

Run:

```bash
pnpm openapi:bundle
pnpm schema:generate
cargo test -p octopus-infra create_project_persists_leader_and_live_inheritance_fields
pnpm -C apps/desktop test -- tauri-client-workspace.test.ts
```

Expected:

- OpenAPI and generated schema complete without drift errors.
- The infra test passes.
- The desktop transport test passes with the new request/response fields.

**Step 5: Commit**

```bash
git add contracts/openapi/src/components/schemas/projects.yaml crates/octopus-core/src/lib.rs packages/schema/src/project-settings.ts packages/schema/src/workspace.ts crates/octopus-infra/src/projects_teams.rs apps/desktop/test/tauri-client-workspace.test.ts
git commit -m "feat: add project leader and live inheritance contracts"
```

### Task 2: Persist Project Leader And Replace Snapshot Grants With Exclusion Deltas

**Files:**
- Modify: `crates/octopus-infra/src/projects_teams.rs`
- Modify: `crates/octopus-server/src/workspace_runtime.rs`
- Test: `crates/octopus-server/src/workspace_runtime.rs`
- Test: `crates/octopus-infra/src/projects_teams.rs`

**Step 1: Write the failing tests**

- In `crates/octopus-server/src/workspace_runtime.rs`, add tests that:
  - trim `leaderAgentId`
  - reject empty-but-present `leaderAgentId`
  - preserve exclusion arrays in `validate_create_project_request()` and `validate_update_project_request()`
- In `crates/octopus-infra/src/projects_teams.rs`, add a test that updates a project from one leader/exclusion set to another and verifies the persisted record changes exactly once.

**Step 2: Run the tests to verify they fail**

Run:

```bash
cargo test -p octopus-server validate_create_project_request_trims_leader_and_exclusions
cargo test -p octopus-infra update_project_rewrites_leader_and_live_inheritance_fields
```

Expected:

- Validation tests fail because `workspace_runtime.rs` does not normalize the new fields.
- Persistence update test fails because the updated project record still ignores `leaderAgentId` and the exclusion arrays.

**Step 3: Write the minimal server/infra implementation**

- Update `validate_create_project_request()` and `validate_update_project_request()` in `crates/octopus-server/src/workspace_runtime.rs` to normalize:
  - `leaderAgentId`
  - `excludedSourceKeys`
  - `excludedAgentIds`
  - `excludedTeamIds`
- Update `InfraWorkspaceService::create_project()` and `InfraWorkspaceService::update_project()` in `crates/octopus-infra/src/projects_teams.rs` to persist `leaderAgentId`.
- Stop deriving `linked_workspace_assets` from “selected snapshot lists” for tools/agents. Leave legacy project-link data intact, but treat inherited project scope as dynamic and not encoded through `linked_workspace_assets`.
- Add a small helper comment near the persistence path explaining that assignments now represent project-local exclusions over live workspace scope.

**Step 4: Run the tests to verify they pass**

Run:

```bash
cargo test -p octopus-server validate_create_project_request_trims_leader_and_exclusions
cargo test -p octopus-infra create_project_persists_leader_and_live_inheritance_fields
cargo test -p octopus-infra update_project_rewrites_leader_and_live_inheritance_fields
```

Expected:

- The validation tests pass.
- Create/update project persistence tests pass.

**Step 5: Commit**

```bash
git add crates/octopus-server/src/workspace_runtime.rs crates/octopus-infra/src/projects_teams.rs
git commit -m "feat: persist project leader and exclusion-based grants"
```

### Task 3: Rewrite Runtime Config Parsing And Effective Project Resolvers

**Files:**
- Modify: `apps/desktop/src/stores/project_settings.ts`
- Modify: `apps/desktop/src/stores/project_setup.ts`
- Modify: `apps/desktop/src/stores/workspace_runtime.ts`
- Modify: `crates/octopus-runtime-adapter/src/registry_overrides.rs`
- Test: `apps/desktop/test/project-settings-view.test.ts`
- Test: `apps/desktop/test/conversation-surface.test.ts`

**Step 1: Write the failing tests**

- Add a project settings view test that starts from an empty `assignments.tools` / `assignments.agents` object and expects the summary to show inherited workspace totals instead of zero.
- Add a conversation surface test that expects inherited workspace agents/teams to remain visible even when the project has no explicit assigned ids.
- Add runtime-adapter validation coverage for:
  - `projectSettings.tools.disabledSourceKeys`
  - `projectSettings.agents.disabledAgentIds`
  - `projectSettings.agents.disabledTeamIds`

**Step 2: Run the tests to verify they fail**

Run:

```bash
pnpm -C apps/desktop test -- project-settings-view.test.ts conversation-surface.test.ts
cargo test -p octopus-runtime-adapter project_settings
```

Expected:

- Vitest fails because current selectors still treat missing assignments/settings as empty instead of inherited.
- Runtime adapter tests fail because the validator only understands `enabled*` arrays.

**Step 3: Write the minimal resolver implementation**

- In `apps/desktop/src/stores/project_settings.ts`:
  - parse `disabledSourceKeys`, `disabledAgentIds`, `disabledTeamIds`
  - resolve effective runtime state as `granted minus disabled`, not `saved enabled list or granted`
- In `apps/desktop/src/stores/project_setup.ts`:
  - compute effective project grants from live workspace state:

```ts
effectiveGrantedTools = workspaceEnabledTools.filter(item => !excludedSourceKeys.includes(item.sourceKey))
effectiveGrantedAgents = workspaceActiveAgents.filter(item => !excludedAgentIds.includes(item.id))
effectiveGrantedTeams = workspaceActiveTeams.filter(item => !excludedTeamIds.includes(item.id))
```

  - keep project-owned actors merged into the effective project actor option list
  - treat “workspace default enabled” as:
    - tools: `managementProjection.assets.filter(entry => entry.enabled)`
    - actors: active workspace-scoped records (`status === 'active'`)
- In `apps/desktop/src/stores/workspace_runtime.ts`, save disabled arrays instead of enabled arrays.
- In `crates/octopus-runtime-adapter/src/registry_overrides.rs`, replace `enabled*` validation with `disabled*` validation and keep tool permission override validation unchanged.

**Step 4: Run the tests to verify they pass**

Run:

```bash
pnpm -C apps/desktop test -- project-settings-view.test.ts conversation-surface.test.ts
cargo test -p octopus-runtime-adapter project_settings
```

Expected:

- Project settings summaries now show inherited counts correctly.
- Conversation options still resolve from live workspace actors plus project-owned actors.
- Runtime adapter validation accepts the new disabled-list shape and rejects invalid ids.

**Step 5: Commit**

```bash
git add apps/desktop/src/stores/project_settings.ts apps/desktop/src/stores/project_setup.ts apps/desktop/src/stores/workspace_runtime.ts crates/octopus-runtime-adapter/src/registry_overrides.rs apps/desktop/test/project-settings-view.test.ts apps/desktop/test/conversation-surface.test.ts
git commit -m "refactor: resolve project scope from live workspace inheritance"
```

### Task 4: Enforce Leader Rules In Server Authorization And Effective Project Scope

**Files:**
- Modify: `crates/octopus-server/src/workspace_runtime.rs`
- Modify: `crates/octopus-server/src/lib.rs`
- Test: `crates/octopus-server/src/workspace_runtime.rs`

**Step 1: Write the failing tests**

- Add server tests that verify:
  - a project leader must belong to the effective granted agent set
  - excluding or runtime-disabling the current leader is rejected
  - project tool/agent authorization helpers resolve live workspace scope even when `linked_workspace_assets` is empty

**Step 2: Run the tests to verify they fail**

Run:

```bash
cargo test -p octopus-server project_leader
cargo test -p octopus-server project_scope_uses_live_workspace_inheritance
```

Expected:

- Leader validation tests fail because the server does not yet know how to evaluate the effective granted actor set.
- Scope resolution tests fail because helper functions still read snapshot ids from `linked_workspace_assets` and old assignments.

**Step 3: Write the minimal server implementation**

- In `crates/octopus-server/src/workspace_runtime.rs`:
  - add helper resolvers for effective project tool/agent/team scope based on:
    - current workspace tool records
    - current workspace agent/team records
    - project exclusion arrays
  - validate `leaderAgentId` against the effective granted agent set on create/update
- Replace `collect_project_agent_ids()`, `collect_project_team_ids()`, and `project_tool_source_keys()` so project authorization uses the effective live scope instead of `linked_workspace_assets` snapshots.
- Keep project membership and module-permission logic in `crates/octopus-server/src/lib.rs` unchanged; Leader is a project runtime orchestration role, not an ACL bypass.

**Step 4: Run the tests to verify they pass**

Run:

```bash
cargo test -p octopus-server project_leader
cargo test -p octopus-server project_scope_uses_live_workspace_inheritance
```

Expected:

- Leader validation passes for valid workspace agents and fails for excluded/disabled ones.
- Project tool and actor scope resolves correctly from live workspace state.

**Step 5: Commit**

```bash
git add crates/octopus-server/src/workspace_runtime.rs crates/octopus-server/src/lib.rs
git commit -m "feat: enforce project leader and live authorization scope"
```

### Task 5: Add Leader Selection To Project Creation And Project Overview Editing

**Files:**
- Modify: `apps/desktop/src/views/workspace/ProjectsView.vue`
- Modify: `apps/desktop/src/components/layout/WorkbenchSidebar.vue`
- Modify: `apps/desktop/src/views/project/useProjectSettings.ts`
- Modify: `apps/desktop/src/views/project/ProjectSettingsView.vue`
- Modify: `apps/desktop/src/locales/zh-CN.json`
- Modify: `apps/desktop/src/locales/en-US.json`
- Test: `apps/desktop/test/projects-view.test.ts`
- Test: `apps/desktop/test/layout-shell.test.ts`
- Test: `apps/desktop/test/project-settings-view.test.ts`

**Step 1: Write the failing tests**

- In `apps/desktop/test/projects-view.test.ts`, add coverage that:
  - create-project form shows a Leader select populated from active workspace agents
  - create submits `leaderAgentId`
  - the default tool/agent/team summary shows “inherit workspace” instead of selected checklists
- In `apps/desktop/test/layout-shell.test.ts`, add the same expectations for the sidebar quick-create popover.
- In `apps/desktop/test/project-settings-view.test.ts`, add coverage for editing Leader from project settings and rejecting a missing/invalid Leader save.

**Step 2: Run the tests to verify they fail**

Run:

```bash
pnpm -C apps/desktop test -- projects-view.test.ts layout-shell.test.ts project-settings-view.test.ts
```

Expected:

- Tests fail because the create forms do not expose any Leader selector and the project settings view does not yet edit `leaderAgentId`.

**Step 3: Write the minimal frontend implementation**

- In `ProjectsView.vue` and `WorkbenchSidebar.vue`:
  - remove preset-based tool/actor snapshot seeding for this feature
  - show a Leader `UiSelect` backed by active workspace agents
  - default project scope copy should explain:
    - tools inherit workspace enabled assets
    - agents/teams inherit active workspace records
- In `useProjectSettings.ts` and `ProjectSettingsView.vue`:
  - surface the current `leaderAgentId`
  - add a dedicated edit action in the overview section
  - enforce that the selected Leader remains part of the effective granted+enabled workspace agents
- Add localized copy for:
  - `Leader`
  - `Inherited from workspace`
  - `Leader must remain enabled`

**Step 4: Run the tests to verify they pass**

Run:

```bash
pnpm -C apps/desktop test -- projects-view.test.ts layout-shell.test.ts project-settings-view.test.ts
```

Expected:

- Project creation surfaces render the Leader picker and inheritance summary.
- Project settings can edit the Leader and block invalid saves.

**Step 5: Commit**

```bash
git add apps/desktop/src/views/workspace/ProjectsView.vue apps/desktop/src/components/layout/WorkbenchSidebar.vue apps/desktop/src/views/project/useProjectSettings.ts apps/desktop/src/views/project/ProjectSettingsView.vue apps/desktop/src/locales/zh-CN.json apps/desktop/src/locales/en-US.json apps/desktop/test/projects-view.test.ts apps/desktop/test/layout-shell.test.ts apps/desktop/test/project-settings-view.test.ts
git commit -m "feat: add project leader selection to create and settings flows"
```

### Task 6: Refactor Tool Grant And Runtime Dialogs To Tabs + Search + Per-Tab Bulk Actions

**Files:**
- Modify: `apps/desktop/src/views/project/useProjectSettings.ts`
- Modify: `apps/desktop/src/views/project/ProjectSettingsView.vue`
- Modify: `apps/desktop/src/locales/zh-CN.json`
- Modify: `apps/desktop/src/locales/en-US.json`
- Test: `apps/desktop/test/project-settings-view.test.ts`

**Step 1: Write the failing tests**

- Add tests that open the grant-tools and runtime-tools dialogs and assert:
  - tabs exist for `builtin`, `skill`, `mcp`
  - a search input filters only the active tab
  - `Select all` and `Clear all` act only on the active tab
  - runtime tools show inherited checked state by default and write `disabledSourceKeys` on uncheck

**Step 2: Run the tests to verify they fail**

Run:

```bash
pnpm -C apps/desktop test -- project-settings-view.test.ts
```

Expected:

- Existing tests fail because the dialogs still use accordion-only layout, no search input, and global bulk selection.

**Step 3: Write the minimal dialog refactor**

- In `useProjectSettings.ts`:
  - track active tool tab and tab-local search query
  - compute filtered entries per tab
  - implement tab-local `selectAll` / `clearAll`
  - change grant/runtime save logic from selected-id lists to exclusion/disabled lists
- In `ProjectSettingsView.vue`:
  - replace tool accordions in dialogs with `UiTabs + UiInput + filtered list`
  - keep permission mode `UiSelect` in runtime tools
  - show `继承工作区` badge/message for default rows

**Step 4: Run the tests to verify they pass**

Run:

```bash
pnpm -C apps/desktop test -- project-settings-view.test.ts
```

Expected:

- Tool dialogs render the 3-tab structure.
- Search and bulk actions are scoped to the active tab.
- Unchecking runtime tools writes disabled deltas instead of replacing the inherited baseline.

**Step 5: Commit**

```bash
git add apps/desktop/src/views/project/useProjectSettings.ts apps/desktop/src/views/project/ProjectSettingsView.vue apps/desktop/src/locales/zh-CN.json apps/desktop/src/locales/en-US.json apps/desktop/test/project-settings-view.test.ts
git commit -m "feat: refactor project tool dialogs with tabs and search"
```

### Task 7: Refactor Actor Grant And Runtime Dialogs To Tabs + Search + Leader Safety

**Files:**
- Modify: `apps/desktop/src/views/project/useProjectSettings.ts`
- Modify: `apps/desktop/src/views/project/ProjectSettingsView.vue`
- Modify: `apps/desktop/src/views/project/ProjectActorsPanel.vue`
- Modify: `apps/desktop/src/locales/zh-CN.json`
- Modify: `apps/desktop/src/locales/en-US.json`
- Test: `apps/desktop/test/project-settings-view.test.ts`

**Step 1: Write the failing tests**

- Extend `project-settings-view.test.ts` so the grant/runtime actor dialogs assert:
  - tabs exist for `数字员工` and `数字团队`
  - search filters the active tab only
  - bulk actions affect the active tab only
  - Leader rows are labeled
  - the user cannot deselect or disable the current Leader without choosing a new Leader first

**Step 2: Run the tests to verify they fail**

Run:

```bash
pnpm -C apps/desktop test -- project-settings-view.test.ts
```

Expected:

- Tests fail because actor dialogs are still two flat checkbox lists with no search, no tabs, and no Leader-specific protection.

**Step 3: Write the minimal dialog refactor**

- In `useProjectSettings.ts`:
  - track actor tab state and query
  - compute filtered agent/team lists
  - make Leader exclusion/disable attempts surface an inline validation error
- In `ProjectSettingsView.vue` and `ProjectActorsPanel.vue`:
  - render agent/team tabs
  - render search input
  - keep project-owned actors visually distinct from inherited workspace actors
  - show a Leader badge on the selected workspace agent
- Save logic:
  - grant dialog writes exclusion deltas
  - runtime dialog writes disabled deltas

**Step 4: Run the tests to verify they pass**

Run:

```bash
pnpm -C apps/desktop test -- project-settings-view.test.ts
```

Expected:

- Actor dialogs render the 2-tab structure with search.
- Per-tab bulk actions work.
- Leader protection prevents invalid exclusion/disable states.

**Step 5: Commit**

```bash
git add apps/desktop/src/views/project/useProjectSettings.ts apps/desktop/src/views/project/ProjectSettingsView.vue apps/desktop/src/views/project/ProjectActorsPanel.vue apps/desktop/src/locales/zh-CN.json apps/desktop/src/locales/en-US.json apps/desktop/test/project-settings-view.test.ts
git commit -m "feat: refactor project actor dialogs with leader-safe inheritance"
```

### Task 8: Default New Conversations To Leader And Preserve Manual Actor Override

**Files:**
- Modify: `apps/desktop/src/views/project/ConversationView.vue`
- Modify: `apps/desktop/src/stores/project_setup.ts`
- Test: `apps/desktop/test/conversation-surface.test.ts`

**Step 1: Write the failing tests**

- Add conversation tests that verify:
  - when a valid `leaderAgentId` exists, the composer defaults to `agent:<leaderAgentId>`
  - users can still switch to another inherited workspace agent or team
  - if the leader becomes invalid, the composer falls back to the first effective actor without crashing

**Step 2: Run the tests to verify they fail**

Run:

```bash
pnpm -C apps/desktop test -- conversation-surface.test.ts
```

Expected:

- Tests fail because `seedComposerSelections()` still defaults to the first actor option and ignores project Leader.

**Step 3: Write the minimal conversation implementation**

- In `project_setup.ts`, add a helper that resolves the preferred conversation actor:
  - `leaderAgentId` first
  - else first effective agent
  - else first effective team
- In `ConversationView.vue`, seed `selectedActorValue` from that helper instead of `actorOptions[0]`.
- Preserve current behavior where users can change the actor per conversation session after default seeding.

**Step 4: Run the tests to verify they pass**

Run:

```bash
pnpm -C apps/desktop test -- conversation-surface.test.ts
```

Expected:

- New conversations default to the project Leader.
- Manual actor changes still work.
- Invalid leader data falls back cleanly.

**Step 5: Commit**

```bash
git add apps/desktop/src/views/project/ConversationView.vue apps/desktop/src/stores/project_setup.ts apps/desktop/test/conversation-surface.test.ts
git commit -m "feat: default project conversations to the leader actor"
```

### Task 9: Full Regression, Schema Checks, And Final Cleanup

**Files:**
- Modify: `apps/desktop/test/project-settings-view.test.ts`
- Modify: `apps/desktop/test/projects-view.test.ts`
- Modify: `apps/desktop/test/layout-shell.test.ts`
- Modify: `apps/desktop/test/conversation-surface.test.ts`
- Modify: `apps/desktop/test/tauri-client-workspace.test.ts`
- Modify: `crates/octopus-server/src/workspace_runtime.rs`
- Modify: `crates/octopus-infra/src/projects_teams.rs`
- Modify: `docs/plans/project/2026-04-18-project-leader-live-inheritance-implementation.md`

**Step 1: Write the last failing regression coverage**

- Add any missing regression tests for:
  - leader removal fallback
  - inherited tool disable persistence
  - inherited actor disable persistence
  - create/update project transport parity
- Update the plan file itself only if implementation discoveries change the agreed architecture.

**Step 2: Run the full verification suite and capture failures**

Run:

```bash
pnpm schema:check
pnpm -C apps/desktop test -- project-settings-view.test.ts projects-view.test.ts layout-shell.test.ts conversation-surface.test.ts tauri-client-workspace.test.ts
pnpm -C apps/desktop typecheck
cargo test -p octopus-server
cargo test -p octopus-infra
cargo test -p octopus-runtime-adapter
```

Expected:

- Any remaining drift appears now instead of after merge.
- Type errors and stale tests surface before the final cleanup commit.

**Step 3: Apply the minimal cleanup**

- Remove dead preset-seeding branches that only existed for snapshot-style tool/actor assignment.
- Remove helper names or comments that still refer to “selected assigned tools/actors” when the new model is exclusion-based inheritance.
- Re-run only the failing commands from Step 2 until clean.

**Step 4: Run the final green verification**

Run:

```bash
pnpm openapi:bundle
pnpm schema:generate
pnpm schema:check
pnpm -C apps/desktop typecheck
pnpm -C apps/desktop test -- project-settings-view.test.ts projects-view.test.ts layout-shell.test.ts conversation-surface.test.ts tauri-client-workspace.test.ts
cargo test -p octopus-server
cargo test -p octopus-infra
cargo test -p octopus-runtime-adapter
```

Expected:

- OpenAPI bundle and generated schema are up to date.
- Desktop typecheck and targeted Vitest suites are green.
- Rust server/infra/runtime-adapter tests are green.

**Step 5: Commit**

```bash
git add contracts/openapi/src/components/schemas/projects.yaml packages/schema/src/project-settings.ts packages/schema/src/workspace.ts crates/octopus-core/src/lib.rs crates/octopus-server/src/workspace_runtime.rs crates/octopus-server/src/lib.rs crates/octopus-infra/src/projects_teams.rs crates/octopus-runtime-adapter/src/registry_overrides.rs apps/desktop/src/views/workspace/ProjectsView.vue apps/desktop/src/components/layout/WorkbenchSidebar.vue apps/desktop/src/views/project/useProjectSettings.ts apps/desktop/src/views/project/ProjectSettingsView.vue apps/desktop/src/views/project/ProjectActorsPanel.vue apps/desktop/src/views/project/ConversationView.vue apps/desktop/src/stores/project_settings.ts apps/desktop/src/stores/project_setup.ts apps/desktop/src/stores/workspace_runtime.ts apps/desktop/src/locales/zh-CN.json apps/desktop/src/locales/en-US.json apps/desktop/test/project-settings-view.test.ts apps/desktop/test/projects-view.test.ts apps/desktop/test/layout-shell.test.ts apps/desktop/test/conversation-surface.test.ts apps/desktop/test/tauri-client-workspace.test.ts
git commit -m "feat: add project leader and live workspace inheritance"
```

## Implementation Notes

- Do not add a second workspace-level `enabled` flag for agents/teams in this plan. Treat active workspace-scoped actor records as the inherited default baseline.
- Do not make Leader an ACL bypass. Project Leader is the highest project runtime orchestrator, but human session authorization and project membership rules remain unchanged.
- Do not hand-edit `contracts/openapi/octopus.openapi.yaml` or `packages/schema/src/generated.ts`.
- Keep `docs/design/DESIGN.md` as the source of truth for dialog density, tabs, search rows, and badge treatment.
- Keep tool dialogs and actor dialogs inside the existing `UiDialog` shell; do not introduce a new modal system.

## Acceptance Criteria

- A project can choose one active workspace agent as `Leader` during create and edit.
- New project conversations default to the Leader, while still allowing manual actor override to another inherited agent or team.
- Project tools, agents, and teams follow workspace changes automatically.
- Project-level edits store only exclusions, disables, and tool permission overrides, not full workspace snapshots.
- Tool dialogs use `内置工具 / 技能 / MCP` tabs with search and tab-local bulk actions.
- Actor dialogs use `数字员工 / 数字团队` tabs with search and tab-local bulk actions.
- The current Leader cannot be excluded or disabled without first selecting a replacement Leader.
