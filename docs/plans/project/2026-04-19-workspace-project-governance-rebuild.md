# Workspace And Project Governance Rebuild Implementation Plan

> **For Codex:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task.

**Goal:** Rebuild workspace settings, project registry, project settings, and project deletion governance around clean long-term contracts instead of layering compatibility patches onto the pre-launch implementation.

**Architecture:** Workspace settings becomes the canonical workspace-level surface for workspace profile, storage root, and default capability availability. Project settings becomes the canonical project configuration surface for project metadata, human responsibility, capability deltas, preset selection, and lifecycle controls, while workspace console project management becomes a registry and governance surface for create/view/archive/delete-request/负责人分配. Project capability resolution is simplified to `workspace enabled baseline + project-owned assets - project disabled deltas`, and project deletion becomes a first-class business approval flow backed by targeted inbox items plus local notification-center events.

**Tech Stack:** OpenAPI (`contracts/openapi`), generated transport schema (`packages/schema`), Rust core/server/infra/access-control (`crates/octopus-*`), Vue 3 + Vite + Pinia + Vue Router + Vue I18n + Tauri (`apps/desktop`), Vitest, Cargo tests.

---

## Scope

- In scope:
  - add a workspace settings surface as the first console tab
  - add workspace name/avatar/mapped-directory editing plus first-entry mapped-directory selection
  - add required human `managerUserId` / 项目负责人 semantics without reusing `leaderAgentId`
  - rebuild project capability configuration around workspace inheritance plus project deltas
  - move project metadata/config editing into `project-settings`
  - reduce `workspace-console-projects` to registry, lifecycle, and governance actions
  - add archived/non-archived project subtabs
  - add project delete approval requests with project-admin or system-admin approval
  - route approvals into user-targeted inbox items and route local UI feedback into notification center
  - fix first login/register avatar persistence
- Out of scope:
  - compatibility layers for old `assignments`, `linkedWorkspaceAssets`, or old grants/runtime dialogs
  - data migration for historical project records
  - introducing a brand new ACL product beyond deriving project-admin eligibility from existing permission data
  - remote workspace filesystem migration beyond showing a read-only mapped directory if the host cannot move it safely

## Baseline Decisions

- This repository is still pre-launch. Prefer deletion, replacement, and simplification over compatibility shims.
- `ProjectRecord.assignments` and `ProjectRecord.linkedWorkspaceAssets` are treated as obsolete design and should be removed from the primary project-editing model instead of being preserved.
- `ProjectSettingsView` is the only project configuration surface. `ProjectsView` may still create projects and execute governance shortcuts, but it must not host full metadata/capability editing.
- Workspace-level enabled capability state is the upper bound. If a workspace capability is disabled, a project cannot enable it.
- Project capability state is stored as project deltas over the workspace baseline plus project-owned assets.
- Project deletion requires the project to be archived first.
- Inbox is the shared business approval surface and must become user-targeted. Notification center remains a local feedback/history surface for the current desktop user.
- `leaderAgentId` remains the default digital actor concept. It is not the same thing as the required human `managerUserId`.

## Risks Or Open Questions

- If execution finds critical server/runtime flows that still require `assignments` or `linkedWorkspaceAssets`, stop and enumerate them before reintroducing any compatibility layer.
- If the current access-control data cannot resolve concrete approver user ids for `project.manage`, stop and decide whether to add explicit project-admin bindings before continuing approval work.
- If changing the workspace mapped directory cannot safely move the local workspace root in one explicit operation, stop instead of saving a disconnected display path that is not the real source of truth.
- If project deletion cannot safely scope and remove project-owned files, blobs, resources, and projections, stop and define the exact deletion boundary before adding the final destructive route.

## Execution Rules

- Do not preserve the current grants/runtime split UI by renaming labels; replace the interaction model with a unified capability configuration surface.
- Do not keep `project-settings` owner-only. Rebuild route access around permission-based governance once the new project configuration boundary is in place.
- Do not wire delete approval only into notification toasts. Approval requests are incomplete until inbox items are created for eligible approvers.
- Do not add page-local fetch calls. All frontend business flows must stay behind stores and the existing adapter boundary.
- Do not hand-edit `contracts/openapi/octopus.openapi.yaml` or `packages/schema/src/generated.ts`.
- Execute in small batches and update this plan in place after every batch.

## Task Ledger

### Task 1: Replace Transport Contracts With The New Governance Model

Status: `done`

Files:
- Modify: `contracts/openapi/src/components/schemas/projects.yaml`
- Modify: `contracts/openapi/src/components/schemas/workspace.yaml`
- Modify: `contracts/openapi/src/components/schemas/auth.yaml`
- Modify: `contracts/openapi/src/components/schemas/misc.yaml`
- Modify: `contracts/openapi/src/paths/projects.yaml`
- Modify: `contracts/openapi/src/paths/workspace.yaml`
- Modify: `contracts/openapi/src/paths/system-auth.yaml`
- Modify: `crates/octopus-core/src/lib.rs`
- Modify: `packages/schema/src/project-settings.ts`
- Modify: `packages/schema/src/workspace.ts`
- Regenerate: `contracts/openapi/octopus.openapi.yaml`
- Regenerate: `packages/schema/src/generated.ts`
- Test: `apps/desktop/test/tauri-client-workspace.test.ts`
- Test: `apps/desktop/test/notification-schema.test.ts`

Preconditions:
- Baseline decisions above are accepted, especially removing old project grant snapshot semantics.

Step 1:
- Action: Redefine the public transport shapes so they match the new product boundary:
  - add workspace settings update/read shapes for `name`, `avatar`, `mappedDirectory`, and `mappedDirectoryDefault`
  - add `managerUserId` and `presetCode` to project transport
  - keep `leaderAgentId` only as the digital actor field
  - remove project transport reliance on `assignments` and `linkedWorkspaceAssets`
  - reshape `ProjectSettingsConfig` into delta-only capability settings
  - add project deletion request transport shapes and targeted inbox metadata
  - add `GET /api/v1/workspace/personal-center/profile`
  - add bootstrap-admin mapped-directory input for first-entry workspace setup
- Done when: OpenAPI source files describe the new workspace/project governance model without compatibility aliases for the removed project grant snapshot fields.
- Verify: `pnpm openapi:bundle && pnpm schema:generate && pnpm schema:check`
- Stop if: removing the old project fields breaks another canonical HTTP contract that still depends on them and the dependency boundary is not yet understood.

Step 2:
- Action: Update transport-facing tests and schema aliases so the desktop client compiles against the new contract surface.
- Done when: the workspace client tests assert the new request/response fields and no desktop transport code references removed project grant snapshot fields.
- Verify: `pnpm -C apps/desktop test -- tauri-client-workspace.test.ts notification-schema.test.ts`
- Stop if: generated schema exposes a shape mismatch that cannot be resolved in handwritten alias files without reintroducing parallel truth sources.

### Task 2: Persist Workspace Settings And The Simplified Project Record

Status: `done`
Current step: `Completed`

Files:
- Modify: `crates/octopus-core/src/lib.rs`
- Modify: `crates/octopus-infra/src/infra_state.rs`
- Modify: `crates/octopus-infra/src/projects_teams.rs`
- Modify: `crates/octopus-infra/src/auth_users.rs`
- Modify: `crates/octopus-infra/src/workspace_paths.rs`
- Modify: `crates/octopus-platform/src/workspace.rs`
- Modify: `crates/octopus-server/src/handlers.rs`
- Modify: `crates/octopus-server/src/routes.rs`
- Modify: `crates/octopus-server/src/workspace_runtime.rs`
- Test: `crates/octopus-infra/src/projects_teams.rs`
- Test: `crates/octopus-infra/src/auth_users.rs`
- Test: `crates/octopus-server/src/workspace_runtime.rs`

Preconditions:
- Task 1 contract generation is complete and the new Rust core types compile.

Step 1:
- Action: Persist workspace `name`, `avatar`, and `mappedDirectory`, and thread the mapped-directory source of truth through workspace root/path initialization so the configured directory is the real data/storage root.
- Done when: workspace summary and workspace update routes read/write a real mapped-directory-backed workspace root, and bootstrap-admin can set the initial mapped directory.
- Verify: `cargo test -p octopus-infra auth_users && cargo test -p octopus-server workspace_summary`
- Stop if: the mapped-directory move would silently diverge from the actual workspace root on disk.

Step 2:
- Action: Persist `managerUserId`, `presetCode`, and the cleaned project metadata model in infra/server validation, while removing create/update/list persistence dependency on the removed project snapshot grant fields.
- Done when: project create/update/list flows round-trip `managerUserId`, `presetCode`, `leaderAgentId`, and `resourceDirectory` without reading or writing the removed project snapshot grant fields.
- Verify: `cargo test -p octopus-infra projects_teams && cargo test -p octopus-server validate_create_project_request`
- Stop if: a server runtime flow still assumes project capability state lives on `ProjectRecord` instead of project settings resolution.

Step 3:
- Action: Add `GET current profile` and use the stored user avatar as the canonical profile hydration source rather than relying on access-control user summaries.
- Done when: the server exposes a current-profile read route returning the stored avatar and profile update routes keep the same shape.
- Verify: `cargo test -p octopus-server personal_center_profile && cargo test -p octopus-infra auth_users`
- Stop if: the stored avatar source is incomplete and would still force the frontend to keep a stale previous-avatar fallback.

### Task 3: Add Project Delete Approval As A Targeted Inbox Workflow

Status: `done`
Current step: `Completed`

Files:
- Modify: `crates/octopus-infra/src/access_control.rs`
- Modify: `crates/octopus-infra/src/artifacts_inbox_knowledge.rs`
- Modify: `crates/octopus-infra/src/projects_teams.rs`
- Modify: `crates/octopus-server/src/routes.rs`
- Modify: `crates/octopus-server/src/workspace_runtime.rs`
- Test: `crates/octopus-infra/src/access_control.rs`
- Test: `crates/octopus-infra/src/artifacts_inbox_knowledge.rs`
- Test: `crates/octopus-infra/src/projects_teams.rs`
- Test: `crates/octopus-server/src/workspace_runtime.rs`

Preconditions:
- Task 2 persistence for project metadata and current profile is in place.

Step 1:
- Action: Add a project deletion request domain object plus project routes for create/list/approve/reject/delete, and gate destructive delete so only archived projects with an approved request can be deleted.
- Done when: direct project deletion is impossible without an approved deletion request and archived status, and the approval record stores requester, approver, status, and timestamps.
- Verify: `cargo test -p octopus-server project_delete_request && cargo test -p octopus-infra projects_teams`
- Stop if: the exact destructive boundary for project-owned resources, blobs, runtime projections, or knowledge artifacts is still unclear.

Step 2:
- Action: Resolve eligible approvers from project admins (`project.manage`) plus `system.owner` and `system.admin`, then create user-targeted inbox items for those approvers.
- Done when: inbox items are visible only to eligible approvers, a single approval resolves the request, and remaining inbox items are closed automatically.
- Verify: `cargo test -p octopus-infra access_control && cargo test -p octopus-infra artifacts_inbox_knowledge && cargo test -p octopus-server inbox`
- Stop if: access-control resolution cannot produce concrete approver user ids from current project policy data.

Step 3:
- Action: Expose approval-friendly payloads and route metadata so the desktop can deep-link approvers into the right project settings or project registry surface.
- Done when: inbox items include actionable route and label metadata for approve/reject review flows.
- Verify: `cargo test -p octopus-infra artifacts_inbox_knowledge && cargo test -p octopus-server project_delete_request`
- Stop if: the approval action requires multi-step routing state that the current inbox record shape cannot express cleanly.

### Task 4: Refactor Desktop Adapters And Stores Around The New Contracts

Status: `done`
Current step: `Completed`

Files:
- Modify: `apps/desktop/src/tauri/workspace_api.ts`
- Modify: `apps/desktop/src/tauri/workspace-client.ts`
- Modify: `apps/desktop/src/stores/workspace.ts`
- Modify: `apps/desktop/src/stores/workspace_actions.ts`
- Modify: `apps/desktop/src/stores/workspace_runtime.ts`
- Modify: `apps/desktop/src/stores/project_settings.ts`
- Modify: `apps/desktop/src/stores/project_setup.ts`
- Modify: `apps/desktop/src/stores/user-profile.ts`
- Modify: `apps/desktop/src/stores/auth.ts`
- Modify: `apps/desktop/src/stores/inbox.ts`
- Modify: `apps/desktop/src/composables/project-governance.ts`
- Modify: `apps/desktop/test/support/workspace-fixture.ts`
- Modify: `apps/desktop/test/support/workspace-fixture-bootstrap.ts`
- Modify: `apps/desktop/test/support/workspace-fixture-client.ts`
- Modify: `apps/desktop/test/support/workspace-fixture-projects.ts`
- Modify: `apps/desktop/test/support/workspace-fixture-runtime.ts`
- Test: `apps/desktop/test/tauri-client-workspace.test.ts`
- Test: `apps/desktop/test/auth-store.test.ts`
- Test: `apps/desktop/test/inbox-store.test.ts`
- Test: `apps/desktop/test/project-governance.test.ts`
- Test: `apps/desktop/test/workspace-access-control-store.test.ts`

Preconditions:
- Tasks 1 through 3 are available through the desktop transport layer or fixture mocks.

Step 1:
- Action: Replace store/adaptor usage of removed project snapshot grant fields with the clean inheritance model, add workspace settings actions, add deletion-request actions, and expose permission helpers for project settings access and delete approval review.
- Done when: desktop stores resolve project capabilities from workspace-enabled baseline plus project-owned assets minus project disabled deltas, and no store references removed project snapshot grant fields.
- Verify: `pnpm -C apps/desktop test -- tauri-client-workspace.test.ts project-governance.test.ts workspace-access-control-store.test.ts`
- Stop if: store-level capability resolution still depends on route-local view logic instead of shared selectors.

Step 2:
- Action: Replace first-login avatar hydration with current-profile loading after login/register and make inbox bootstrap user-targeted rather than workspace-global.
- Done when: the current user avatar survives first registration/login refresh and inbox only shows items relevant to the active user.
- Verify: `pnpm -C apps/desktop test -- auth-store.test.ts inbox-store.test.ts app-auth-gate.test.ts`
- Stop if: the frontend still needs to retain `previous?.avatar` fallback to avoid losing the avatar.

Step 3:
- Action: Update desktop fixtures so all project, workspace, inbox, and approval tests exercise the new contract and inheritance model.
- Done when: fixture builders create workspace settings, project manager, preset, targeted inbox, and deletion request data without any legacy snapshot grant payloads.
- Verify: `pnpm -C apps/desktop test -- tauri-client-workspace.test.ts inbox-store.test.ts project-settings-view.test.ts projects-view.test.ts`
- Stop if: fixture drift hides a missing production codepath rather than exposing it.

### Task 5: Add Workspace Settings As The First Console Tab

Status: `done`
Current step: `Completed`

Files:
- Create: `apps/desktop/src/views/workspace/WorkspaceSettingsView.vue`
- Create: `apps/desktop/test/workspace-settings-view.test.ts`
- Modify: `apps/desktop/src/views/workspace/WorkspaceConsoleView.vue`
- Modify: `apps/desktop/src/navigation/menuRegistry.ts`
- Modify: `apps/desktop/src/router/index.ts`
- Modify: `apps/desktop/src/i18n/navigation.ts`
- Modify: `apps/desktop/src/components/layout/WorkbenchTopbar.vue`
- Modify: `apps/desktop/src/components/layout/WorkbenchSidebar.vue`
- Modify: `apps/desktop/src/components/auth/AuthGateForm.vue`
- Modify: `apps/desktop/src/views/auth/AuthLoginView.vue`
- Modify: `apps/desktop/src/locales/en-US.json`
- Modify: `apps/desktop/src/locales/zh-CN.json`
- Test: `apps/desktop/test/router.test.ts`
- Test: `apps/desktop/test/startup-router-install.test.ts`
- Test: `apps/desktop/test/workspace-settings-view.test.ts`

Preconditions:
- Task 4 exposes workspace settings read/write actions and current mapped-directory defaults.

Step 1:
- Action: Add a `workspace settings` console route and menu entry in first position, then render a document-style settings page for workspace name, avatar, and mapped directory.
- Done when: `workspace-console` lands on workspace settings first, the tab order is stable, and the workspace page edits the canonical workspace settings contract instead of route-local state.
- Verify: `pnpm -C apps/desktop test -- router.test.ts startup-router-install.test.ts workspace-settings-view.test.ts`
- Stop if: menu/access-control ordering prevents a stable first tab without hardcoding around hidden-menu behavior.

Step 2:
- Action: Add the first-entry mapped-directory selector to the workspace bootstrap/auth flow with the current default mapped directory prefilled.
- Done when: bootstrap-admin UI can choose the mapped directory before first login completes and the same directory appears in workspace settings after bootstrap.
- Verify: `pnpm -C apps/desktop test -- app-auth-gate.test.ts auth-store.test.ts workspace-settings-view.test.ts`
- Stop if: the first-entry flow cannot share the same mapped-directory validation rules as workspace settings.

Step 3:
- Action: Reflect workspace name/avatar updates in shell chrome so the new workspace settings surface has visible product impact.
- Done when: sidebar/topbar use the updated workspace display data without requiring manual refresh.
- Verify: `pnpm -C apps/desktop test -- workspace-settings-view.test.ts router.test.ts`
- Stop if: shell chrome reads a different workspace source than the workspace store summary.

### Task 6: Rebuild Workspace Console Project Management As A Registry And Governance Surface

Status: `done`
Current step: `Completed`

Files:
- Modify: `apps/desktop/src/stores/workspace.ts`
- Modify: `apps/desktop/src/stores/workspace_actions.ts`
- Modify: `apps/desktop/src/views/workspace/ProjectsView.vue`
- Modify: `apps/desktop/src/components/projects/ProjectResourceDirectoryField.vue`
- Modify: `apps/desktop/src/locales/en-US.json`
- Modify: `apps/desktop/src/locales/zh-CN.json`
- Test: `apps/desktop/test/projects-view.test.ts`

Preconditions:
- Tasks 4 and 5 provide manager-user options, lifecycle actions, delete-request actions, and workspace settings defaults.

Step 1:
- Action: Reduce `ProjectsView` to a registry page with:
  - minimal create flow
  - active/archived subtabs
  - read-only summary rows
  - open-project-settings action
  - archive/restore/delete-request shortcuts
  - project-manager assignment shortcut
- Done when: the page no longer edits full project metadata/capabilities inline and archived projects are visibly separated from active projects.
- Verify: `pnpm -C apps/desktop test -- projects-view.test.ts`
- Stop if: project creation still depends on editing fields that are supposed to live only in project settings.

Step 2:
- Action: Enforce delete-request visibility and behavior so only archived projects can request or execute delete, while active projects only show archive/open settings actions.
- Done when: archived rows expose delete review/request actions and active rows do not expose destructive delete actions.
- Verify: `pnpm -C apps/desktop test -- projects-view.test.ts`
- Stop if: lifecycle behavior diverges between project registry and project settings and there is no shared source of truth.

### Task 7: Rebuild Project Settings Into The Canonical Project Configuration Surface

Status: `done`

Files:
- Modify: `apps/desktop/src/views/project/ProjectSettingsView.vue`
- Modify: `apps/desktop/src/views/project/useProjectSettings.ts`
- Modify: `apps/desktop/src/views/project/ProjectBasicsPanel.vue`
- Modify: `apps/desktop/src/views/project/ProjectMembersPanel.vue`
- Create: `apps/desktop/src/views/project/ProjectCapabilitiesPanel.vue`
- Create: `apps/desktop/src/views/project/ProjectLifecyclePanel.vue`
- Modify: `apps/desktop/src/components/projects/ProjectResourceDirectoryField.vue`
- Modify: `apps/desktop/src/locales/en-US.json`
- Modify: `apps/desktop/src/locales/zh-CN.json`
- Test: `apps/desktop/test/project-settings-view.test.ts`
- Test: `apps/desktop/test/project-runtime-view.test.ts`

Preconditions:
- Task 4 selectors expose the new capability inheritance model and deletion request state.

Step 1:
- Status: `done`
- Action: Move project metadata editing into project settings, including `name`, `description`, `resourceDirectory`, `managerUserId`, `presetCode`, and `leaderAgentId`.
- Done when: project settings owns editable project metadata and the workspace registry no longer hosts those controls.
- Verify: `pnpm -C apps/desktop test -- project-settings-view.test.ts projects-view.test.ts`
- Stop if: metadata editing still requires duplicated save paths between registry and settings.

Step 2:
- Status: `done`
- Action: Replace the current grants/runtime split with a single capability configuration section that surfaces workspace state, project inherited state, project-disabled state, and project-owned assets for models/tools/agents/teams.
- Done when: the UI no longer shows two separate capability regions for the same resource classes, and users can understand inheritance and project override state from one section.
- Verify: `pnpm -C apps/desktop test -- project-settings-view.test.ts project-runtime-view.test.ts`
- Stop if: the new UI still depends on old grants/runtime dialog concepts instead of the new delta model.

Step 3:
- Status: `done`
- Action: Add lifecycle controls and approval state to project settings, including archive, restore, delete request, approval status, and delete execution once approved.
- Done when: project settings can fully manage project lifecycle within the new approval model and deep-link from inbox items lands on the correct review state.
- Verify: `pnpm -C apps/desktop test -- project-settings-view.test.ts projects-view.test.ts`
- Stop if: delete execution depends on hidden state that only exists on the registry page.

### Task 8: Wire Workspace And Project Operations Into Notification Center

Status: `done`
Current step: `Completed`

Files:
- Create: `apps/desktop/src/composables/useWorkspaceProjectNotifications.ts`
- Modify: `apps/desktop/src/views/workspace/WorkspaceSettingsView.vue`
- Modify: `apps/desktop/src/views/workspace/ProjectsView.vue`
- Modify: `apps/desktop/src/views/project/useProjectSettings.ts`
- Modify: `apps/desktop/src/stores/notifications.ts`
- Test: `apps/desktop/test/notification-store.test.ts`
- Test: `apps/desktop/test/projects-view.test.ts`
- Test: `apps/desktop/test/project-settings-view.test.ts`
- Test: `apps/desktop/test/workspace-settings-view.test.ts`

Preconditions:
- Tasks 5 through 7 expose the final mutation flows that need notification-center integration.

Step 1:
- Status: `done`
- Action: Create a shared desktop notification helper for workspace/project governance actions and route successful saves, archive/restore actions, delete-request actions, approvals, rejections, and workspace settings updates into notification center.
- Done when: all user-visible operations on workspace settings, project registry, and project settings create consistent notification-center entries with route metadata where useful.
- Verify: `pnpm -C apps/desktop test -- notification-store.test.ts projects-view.test.ts project-settings-view.test.ts workspace-settings-view.test.ts`
- Stop if: any mutation flow can bypass the helper and succeed without emitting the required notification-center entry.

Step 2:
- Status: `done`
- Action: Refresh inbox and local notifications coherently after delete-request creation or approval actions so the operator immediately sees the new state.
- Done when: approval operations update both actionable inbox state and local feedback state without a manual page refresh.
- Verify: `pnpm -C apps/desktop test -- notification-store.test.ts inbox-store.test.ts project-settings-view.test.ts projects-view.test.ts`
- Stop if: the only way to reflect approval state is a full workspace bootstrap reload.

### Task 9: Finish Project Settings Governance Access Around Permission-Based Navigation

Status: `done`
Current step: `Completed`

Files:
- Modify: `apps/desktop/src/composables/project-governance.ts`
- Modify: `apps/desktop/src/components/layout/WorkbenchSidebar.vue`
- Test: `apps/desktop/test/project-governance.test.ts`
- Test: `apps/desktop/test/router.test.ts`
- Test: `apps/desktop/test/layout-shell.test.ts`

Preconditions:
- Tasks 4 through 8 are complete and `project-settings` already owns canonical project configuration plus lifecycle review flows.

Step 1:
- Status: `done`
- Action: Replace the remaining member-only sidebar/navigation assumption with permission-based governance access so non-member project reviewers can still discover and open `project-settings` from shell navigation when they hold `project.manage`, `system.admin`, or `system.owner`.
- Done when: the shell exposes a stable `project-settings` navigation entry for governance reviewers even when they are not project members, and no project-governance helper still encodes `project-settings` as an owner-only route.
- Verify: `pnpm -C apps/desktop exec vitest run test/project-governance.test.ts test/layout-shell.test.ts -t "shows the project settings entry for governance reviewers who are not project members"`
- Stop if: exposing project settings in shell navigation would also leak non-settings project modules for non-member reviewers and there is no clean way to constrain the visible surface to settings only.

Step 2:
- Status: `done`
- Action: Re-verify route guards and redirect behavior so permission-based reviewers can deep-link into project settings while non-member users without governance permission still fall back to workspace/project-safe routes.
- Done when: router behavior, shell navigation, and shared project-governance helpers all agree on who can open `project-settings`, with no stale owner-only helper semantics left behind.
- Verify: `pnpm -C apps/desktop exec vitest run test/project-governance.test.ts test/router.test.ts test/layout-shell.test.ts`
- Stop if: router and shell navigation would require separate duplicated authorization rules instead of one shared project-governance contract.

## Batch Checkpoint Format

After each batch, append a short checkpoint using this shape:

```md
## Checkpoint YYYY-MM-DD HH:MM

- Batch: Task 1 Step 1 -> Task 1 Step 2
- Completed: short list
- Verification:
  - `command` -> pass or fail
- Blockers:
  - none
- Next:
  - Task 2 Step 1
```

## Checkpoint 2026-04-19 01:42

- Batch: Task 1 Step 1 -> Task 1 Step 2
- Completed:
  - added workspace settings transport fields for `avatar`, `mappedDirectory`, and `mappedDirectoryDefault`
  - added `PATCH /api/v1/workspace` and `GET /api/v1/workspace/personal-center/profile`
  - added project `managerUserId` / `presetCode` transport fields plus project deletion request transport/routes
  - added targeted inbox transport metadata and refreshed desktop transport assertions for the new contract surface
- Verification:
  - `pnpm openapi:bundle` -> pass
  - `pnpm schema:generate` -> pass
  - `pnpm schema:check` -> pass
  - `pnpm -C apps/desktop test -- tauri-client-workspace.test.ts notification-schema.test.ts` -> pass
- Blockers:
  - none
- Next:
  - Task 2 Step 1

## Checkpoint 2026-04-19 02:14

- Batch: Task 2 Step 1 -> Task 2 Step 3
- Completed:
  - added workspace update/current-profile service methods plus `PATCH /api/v1/workspace` and `GET /api/v1/workspace/personal-center/profile`
  - persisted workspace `mappedDirectory`, workspace avatar metadata, and canonical `mappedDirectoryDefault` through infra config/state
  - validated bootstrap/update mapped-directory input against the real workspace root instead of allowing a disconnected display-only path
  - persisted project `managerUserId` and `presetCode` through infra DB load/save and server request normalization without reintroducing legacy grant snapshot persistence
  - updated compile-broken infra/server tests and fixture literals for the new transport fields
- Verification:
  - `cargo test -p octopus-infra auth_users` -> pass
  - `cargo test -p octopus-server workspace_summary` -> pass
  - `cargo test -p octopus-infra projects_teams` -> pass

## Checkpoint 2026-04-19 09:11

- Batch: Task 5 Step 1
- Completed:
  - stabilized `workspace-settings-view.test.ts` so it waits for shell bootstrap to expose the loopback directory picker before editing and saving workspace settings
  - updated the shell navigation assertion so footer workspace-console entry now lands on `workspace-console-settings`, matching the new first-tab routing
  - verified the new workspace settings route remains the first authorized console surface while the document-style page saves canonical workspace fields through the workspace store
- Verification:
  - `pnpm -C apps/desktop exec vitest run test/workspace-settings-view.test.ts` -> pass
  - `pnpm -C apps/desktop exec vitest run test/layout-shell.test.ts -t "navigates to the first console workspace surface from the footer workspace menu"` -> pass
  - `pnpm -C apps/desktop exec vitest run test/router.test.ts test/startup-router-install.test.ts test/workspace-settings-view.test.ts` -> pass
  - `pnpm -C apps/desktop exec vitest run test/layout-shell.test.ts` -> pass
- Blockers:
  - none
- Next:
  - Task 5 Step 2

## Checkpoint 2026-04-19 09:19

- Batch: Task 5 Step 2
- Completed:
  - added first-launch mapped-directory selection to `AuthGateForm.vue`, prefilled from the bootstrap workspace default and reusing the same directory-picker pattern as workspace settings for loopback workspaces
  - extended `auth.registerOwner()` to forward the selected `mappedDirectory` through `bootstrapAdmin`, and updated the fixture bootstrap-admin path to persist the chosen directory into the canonical workspace summary
  - added regression coverage proving first-owner registration stores the selected mapped directory and that the auth gate persists it into the canonical workspace summary used by workspace settings
- Verification:
  - `pnpm -C apps/desktop exec vitest run test/auth-store.test.ts -t "stores the selected mapped directory during first-owner registration"` -> pass
  - `pnpm -C apps/desktop exec vitest run test/app-auth-gate.test.ts -t "prefills the bootstrap mapped directory and persists the selected directory into the canonical workspace summary"` -> pass
  - `pnpm -C apps/desktop exec vitest run test/app-auth-gate.test.ts test/auth-store.test.ts test/workspace-settings-view.test.ts` -> pass
- Blockers:
  - none
- Next:
  - Task 5 Step 3
  - `cargo test -p octopus-server validate_create_project_request` -> pass
  - `cargo test -p octopus-server personal_center_profile` -> pass
- Blockers:
  - none
- Next:
  - Task 3 Step 1

## Checkpoint 2026-04-19 03:02

- Batch: Task 3 Step 1
- Completed:
  - added project deletion request persistence, service methods, and guarded delete flow in infra/platform
  - added project deletion request routes for list/create/approve/reject plus `DELETE /api/v1/projects/:project_id`
  - enforced archived-plus-approved gating before destructive project delete
  - scoped destructive cleanup across project-owned DB rows, runtime session projections, managed resource storage, and project resource directories
  - added infra/server regression tests covering request lifecycle and guarded deletion
- Verification:
  - `cargo test -p octopus-server project_delete_request` -> pass
  - `cargo test -p octopus-infra projects_teams` -> pass
- Blockers:
  - none
- Next:
  - Task 3 Step 2

## Checkpoint 2026-04-19 04:06

- Batch: Task 3 Step 2
- Completed:
  - added infra access-control resolution for concrete project deletion approver user ids from scoped `project.manage` users plus `system.owner` and `system.admin`
  - fanned out project deletion requests into targeted inbox items for eligible approvers and closed remaining inbox items automatically after a single review
  - relaxed delete-review route authorization so scoped project approvers can approve or reject without being the project owner
  - filtered inbox responses and pet reminder counts to the current user so delete-approval items are only visible to their intended approvers
  - added regression tests covering approver resolution, targeted inbox fanout/closure, scoped approver review, and user-filtered inbox responses
- Verification:
  - `cargo test -p octopus-infra access_control` -> pass
  - `cargo test -p octopus-infra artifacts_inbox_knowledge` -> pass
  - `cargo test -p octopus-server inbox` -> pass
  - `cargo test -p octopus-server project_delete_request_approve_route_allows_project_scoped_admin_reviewers` -> pass
- Blockers:
  - none
- Next:
  - Task 3 Step 3

## Checkpoint 2026-04-19 04:20

- Batch: Task 3 Step 3
- Completed:
  - added project deletion inbox route metadata that deep-links approvers into `/workspaces/:workspaceId/projects/:projectId/settings`
  - added a stable delete-review action label so desktop inbox entries can render a clear approval CTA
  - locked the new metadata with infra and server regression coverage on deletion-request creation and inbox fanout
- Verification:
  - `cargo test -p octopus-infra artifacts_inbox_knowledge` -> pass
  - `cargo test -p octopus-server project_delete_request` -> pass
- Blockers:
  - none
- Next:
  - Task 4 Step 1

## Checkpoint 2026-04-19 03:29

- Batch: Task 4 Step 1
- Completed:
  - added desktop workspace-store `updateWorkspace(...)` action so workspace settings writes can reuse the shared adapter boundary and immediately sync workspace summary plus overview cache
  - removed legacy `assignments` resubmission from project archive and restore flows so lifecycle changes no longer depend on removed project snapshot grant payloads
  - added a store-level regression test covering workspace settings sync and the no-legacy-payload archive path
  - extended the shared workspace fixture client with workspace settings update support so store tests exercise the real desktop adapter/store boundary
- Verification:
  - `pnpm -C apps/desktop test -- workspace-store-actions.test.ts` -> pass
  - `pnpm -C apps/desktop test -- tauri-client-workspace.test.ts project-governance.test.ts workspace-access-control-store.test.ts workspace-store-actions.test.ts` -> pass
- Blockers:
  - Step 1 still has broad legacy usage in `project_setup.ts`, `useProjectSettings.ts`, `ProjectsView.vue`, and fixture builders that continue to depend on `assignments` / `linkedWorkspaceAssets`
- Next:
  - continue Task 4 Step 1 by moving remaining capability resolution off legacy project fields and into shared selector/store logic

## Checkpoint 2026-04-19 03:30

- Batch: Task 4 Step 1
- Completed:
  - extracted shared project capability selectors in `project_setup.ts` that resolve granted tools, agents, and teams from explicit exclusion lists instead of requiring a legacy `ProjectRecord.assignments` shape
  - updated `useProjectSettings.ts` actor-grant save validation to use the new shared selectors rather than constructing route-local fake project records
  - added selector regression coverage in `project-setup.test.ts` and re-ran `project-settings-view.test.ts` to keep the project settings surface green after the selector refactor
- Verification:
  - `pnpm -C apps/desktop test -- project-setup.test.ts project-settings-view.test.ts` -> pass
- Blockers:
  - Step 1 still has first-class legacy state writes in `useProjectSettings.ts`, `ProjectsView.vue`, `project_setup.ts` preset seed output, and fixture project builders that emit `assignments` / `linkedWorkspaceAssets`
- Next:
  - continue Task 4 Step 1 by replacing remaining legacy project create/update payload construction with project-settings/runtime-delta-driven state

## Checkpoint 2026-04-19 03:39

- Batch: Task 4 Step 1
- Completed:
  - removed legacy `assignments` and `linkedWorkspaceAssets` writes from `ProjectsView.vue` project create and metadata update payloads
  - switched project registry capability summaries in `ProjectsView.vue` to use workspace model inheritance plus project runtime `modelSettings` instead of draft or selected `assignments` snapshots
  - fixed project preset creation so `modelSettings` are snapshotted before reactive selection resets, allowing documentation and engineering presets to persist runtime model settings after create
  - added `projects-view.test.ts` regression coverage asserting project create/update paths no longer submit legacy grant payload fields and that preset model settings are saved separately
- Verification:
  - `pnpm -C apps/desktop test -- projects-view.test.ts` -> pass
  - `pnpm -C apps/desktop test -- projects-view.test.ts project-settings-view.test.ts project-setup.test.ts` -> pass
- Blockers:
  - Task 4 Step 1 still has legacy field construction in `useProjectSettings.ts`, `project_setup.ts` preset seed types/builders, `WorkbenchSidebar.vue`, and desktop fixture project builders/state
- Next:
  - continue Task 4 Step 1 by removing remaining `assignments`-based grant save/reset logic from `useProjectSettings.ts` and then collapse preset seed and fixture builders away from legacy project snapshot fields

## Checkpoint 2026-04-19 04:12

- Batch: Task 4 Step 1
- Completed:
  - confirmed the new grant-save paths were already sending the correct runtime patch payloads for project grant models and grant tools
  - fixed `project-settings-view.test.ts` to wait for async dialog saves to finish before reopening grant dialogs, so the assertions now follow the actual save lifecycle instead of reading stale pre-save UI state
  - removed temporary runtime and fixture debug logging added during diagnosis
- Verification:
  - `pnpm -C apps/desktop exec vitest run test/project-settings-view.test.ts` -> pass
  - `pnpm -C apps/desktop exec vitest run test/project-settings-view.test.ts test/project-setup.test.ts test/projects-view.test.ts test/project-governance.test.ts test/workspace-access-control-store.test.ts test/tauri-client-workspace.test.ts` -> pass
- Blockers:
  - Task 4 Step 1 still has remaining legacy desktop references to `assignments` / `linkedWorkspaceAssets` outside the grant-save path, especially in preset and fixture compatibility seams
- Next:
  - continue Task 4 Step 1 by removing remaining legacy desktop references and fixture dependencies on project snapshot grant fields

## Checkpoint 2026-04-19 04:18

- Batch: Task 4 Step 1
- Completed:
  - removed legacy preset-seed `assignments` output from `project_setup.ts` so preset seeding now only returns runtime `modelSettings`
  - removed sidebar quick-create submission of `assignments`, keeping project creation on the cleaned project metadata contract and saving preset model defaults through runtime settings only
  - updated sidebar shell coverage to assert the created project no longer retains legacy `assignments` and that preset model seeding is reflected through project settings instead
- Verification:
  - `pnpm -C apps/desktop exec vitest run test/project-setup.test.ts test/projects-view.test.ts test/layout-shell.test.ts` -> pass
- Blockers:
  - Task 4 Step 1 still has legacy fixture and view-test setup that mutates `project.assignments` directly in `agent-center.test.ts`, `conversation-surface.test.ts`, and workspace fixture builders/state
- Next:
  - continue Task 4 Step 1 by replacing remaining desktop test/fixture dependence on `project.assignments` with project-settings/runtime-config setup

## Checkpoint 2026-04-19 04:18 (continued)

- Batch: Task 4 Step 1
- Completed:
  - added `workspace-fixture-project-settings.ts` so desktop tests can mutate project runtime settings through the same runtime-config source of truth that stores read in production
  - migrated the newly failing `conversation-surface.test.ts` and `agent-center.test.ts` state transforms away from direct project actor/model assignment mutation and onto runtime `projectSettings` updates
  - narrowed `useAgentCenter.ts` project resource tabs to project-owned agent/team consumers so workspace skill and MCP entries no longer leak into project-scoped resource tabs
- Verification:
  - `pnpm -C apps/desktop exec vitest run test/conversation-surface.test.ts -t "shows localized setup guidance and skips session creation when live project scope has no models or actors"` -> pass
  - `pnpm -C apps/desktop exec vitest run test/conversation-surface.test.ts -t "shows model-specific setup guidance when the project has actors but no model assignment"` -> pass
  - `pnpm -C apps/desktop exec vitest run test/conversation-surface.test.ts -t "scopes the composer model and actor selectors to the effective live project scope"` -> pass
  - `pnpm -C apps/desktop exec vitest run test/agent-center.test.ts -t "shows only effective project resources inside the project resource tabs"` -> pass
  - `pnpm -C apps/desktop exec vitest run test/agent-center.test.ts test/conversation-surface.test.ts` -> pass
  - `pnpm -C apps/desktop exec vitest run test/project-settings-view.test.ts test/layout-shell.test.ts` -> pass
- Blockers:
  - Task 4 Step 1 still has residual legacy project snapshot references in `conversation-surface.test.ts` seeded-model fallback coverage, `project-settings-view.test.ts`, and the fixture builders/state that still derive runtime project config from `project.assignments` / `linkedWorkspaceAssets`
- Next:
  - continue Task 4 Step 1 by removing the remaining fixture-builder and view-test dependence on legacy project snapshot grant fields, then re-run the broader Task 4 Step 1 desktop verification slice

## Checkpoint 2026-04-19 06:53

- Batch: Task 4 Step 1
- Completed:
  - removed `assignments` and `linkedWorkspaceAssets` emission from `workspace-fixture-projects.ts` so created and updated fixture `ProjectRecord`s now stay on the cleaned project metadata contract
  - replaced `workspace-fixture-state.ts` project defaults and runtime project config seeding so `proj-redesign` capability defaults come from explicit runtime `projectSettings` instead of deriving from legacy `project.assignments`
  - updated `project-settings-view.test.ts` to assert the fixture project record no longer retains legacy `assignments`, and rewrote the seeded-model fallback coverage in `conversation-surface.test.ts` to remove project model settings through runtime config rather than mutating `project.assignments`
- Verification:
  - `pnpm -C apps/desktop exec vitest run test/project-settings-view.test.ts` -> pass
  - `pnpm -C apps/desktop exec vitest run test/projects-view.test.ts test/layout-shell.test.ts test/project-settings-view.test.ts test/conversation-surface.test.ts test/agent-center.test.ts` -> pass
- Blockers:
  - Task 4 Step 1 still keeps legacy compatibility handling around runtime project settings enabled lists in desktop store/view logic, and there is still stale legacy wording in at least one conversation test name
- Next:
  - continue Task 4 Step 1 by moving remaining project grant/runtime resolution off legacy runtime compat fields in store/view code, then remove the leftover stale legacy test wording once the behavior is fully selector-driven

## Checkpoint 2026-04-19 08:24

- Batch: Task 4 Step 1
- Completed:
  - removed desktop parser/selector dependence on legacy runtime `enabledSourceKeys`, `enabledAgentIds`, and `enabledTeamIds` in `project_settings.ts`, so workspace grant resolution now ignores old enabled-list compat fields
  - rewired `useProjectSettings.ts` grant save flows to persist only `disabled*` project deltas through the normal runtime save actions, and preserved hidden disabled deltas when runtime dialogs save a partial visible subset
  - removed the dedicated legacy grant-save store actions from `workspace_runtime.ts` and `workspace.ts`
  - updated `workspace-fixture-state.ts` runtime defaults so `proj-redesign` starts from disabled-delta project settings instead of seeding legacy enabled lists
  - added `project-setup.test.ts` regression coverage proving legacy enabled lists are ignored by the project capability selectors
- Verification:
  - `pnpm -C apps/desktop exec vitest run test/project-setup.test.ts` -> pass
  - `pnpm -C apps/desktop exec vitest run test/project-setup.test.ts test/workspace-store-actions.test.ts test/project-settings-view.test.ts test/conversation-surface.test.ts` -> pass
  - `pnpm -C apps/desktop exec vitest run test/tauri-client-workspace.test.ts test/project-governance.test.ts test/workspace-access-control-store.test.ts test/project-setup.test.ts test/workspace-store-actions.test.ts test/project-settings-view.test.ts test/conversation-surface.test.ts` -> pass
- Blockers:
  - Task 4 Step 1 still has residual legacy naming and interaction assumptions around the split grant/runtime UI, even though the underlying store persistence no longer uses enabled-list compat fields
- Next:
  - continue Task 4 Step 1 by cleaning the remaining stale legacy test wording and checking whether any desktop selectors or fixtures still assume the pre-rebuild grant/runtime split semantics before moving on to later Task 4 items

## Checkpoint 2026-04-19 08:39

- Batch: Task 4 Step 1
- Completed:
  - renamed the remaining stale `project-setup.test.ts` selector specs so they describe the current inheritance rule `workspace baseline + project-owned assets - disabled/excluded deltas` instead of referencing removed legacy project assignments
  - renamed the residual `project-settings-view.test.ts` summaries/save-flow specs so they describe capability inheritance and delta-only project settings persistence without implying the old compatibility model is still part of store behavior
  - re-checked the remaining desktop hits for `grants` / `runtime` split wording and kept the current `ProjectSettingsView.vue` section structure deferred to Task 7 Step 2, because that is the planned UI unification batch rather than a Task 4 Step 1 store-contract fix
- Verification:
  - `pnpm -C apps/desktop exec vitest run test/project-setup.test.ts test/project-settings-view.test.ts` -> pass
- Blockers:
  - none for this wording-cleanup batch; the remaining split-section UI labels are intentionally deferred to Task 7 Step 2
- Next:
  - start Task 4 Step 2 by tracing current-profile hydration after login/register and confirming inbox bootstrap is now scoped to the active user instead of the whole workspace

## Checkpoint 2026-04-19 08:54

- Batch: Task 4 Step 2
- Completed:
  - added `getCurrentUserProfile()` to the desktop workspace client and transport layer so authenticated desktop flows can hydrate the current user from `GET /api/v1/workspace/personal-center/profile`
  - rewired `user-profile.ts` to load the canonical current profile document instead of reconstructing it from access-control summaries, which removes the `previous?.avatar` fallback dependency and keeps profile alerts driven by the returned password state
  - changed auth success paths in `auth.ts` (`bootstrapAuth`, `login`, `registerOwner`, `connectWorkspace`) to hydrate current profile and inbox immediately after session establishment or restoration
  - made `inbox.ts` user-targeted by tracking the active session user per connection, filtering records by `targetUserId`, and forcing a reload when the same workspace connection changes users
  - removed pre-auth inbox bootstrap from `App.vue` so the app no longer loads inbox state before the authenticated user is known
  - updated the desktop workspace fixture client to expose `profile.getCurrentUserProfile()` and to keep fixture auth/profile behavior aligned with the authenticated session user
  - added transport regression coverage for `GET /api/v1/workspace/personal-center/profile`
- Verification:
  - `pnpm -C apps/desktop exec vitest run test/auth-store.test.ts test/inbox-store.test.ts` -> pass
  - `pnpm -C apps/desktop exec vitest run test/tauri-client-workspace.test.ts test/auth-store.test.ts test/inbox-store.test.ts test/app-auth-gate.test.ts` -> pass
- Blockers:
  - none in Step 2; the previous avatar fallback is no longer needed for first-login and restored-session hydration
- Next:
  - continue Task 4 Step 3 by checking remaining fixture builders and desktop approval/inbox tests against the new targeted inbox and workspace settings contract surface

## Checkpoint 2026-04-19 08:58

- Batch: Task 4 Step 3
- Completed:
  - extended the `projects-view.test.ts` preset-seeding regression to assert the created project record persists `presetCode`
  - confirmed the failure was a real production gap in `ProjectsView.vue`, where the create payload omitted `presetCode` even though fixture builders now round-trip it
  - updated the project creation flow in `ProjectsView.vue` so non-`general` presets are sent through the canonical create-project contract while runtime preset model seeding remains unchanged
- Verification:
  - `pnpm -C apps/desktop exec vitest run test/projects-view.test.ts --testNamePattern "updates preset summary and uses preset seeding without exposing advanced lists"` -> pass
  - `pnpm -C apps/desktop exec vitest run test/projects-view.test.ts test/project-settings-view.test.ts test/inbox-store.test.ts test/tauri-client-workspace.test.ts` -> pass
- Blockers:
  - none in this batch; fixture drift is now exposing and closing production gaps rather than masking them
- Next:
  - continue Task 4 Step 3 by checking the remaining project-creation entrypoints for the same `presetCode` contract omission, starting with `WorkbenchSidebar.vue`, and add regression coverage before changing production code

## Checkpoint 2026-04-19 09:00

- Batch: Task 4 Step 3
- Completed:
  - added quick-create regression coverage in `layout-shell.test.ts` asserting sidebar project creation persists the selected `presetCode`
  - confirmed the same contract omission in `WorkbenchSidebar.vue` via a failing focused test before production edits
  - updated sidebar quick-create submission to pass non-`general` presets through the canonical create-project payload while preserving the existing runtime preset seeding flow
  - completed the remaining Step 3 fixture/contract follow-through for the two desktop project creation entrypoints covered by the current UI
- Verification:
  - `pnpm -C apps/desktop exec vitest run test/layout-shell.test.ts --testNamePattern "creates a project from the sidebar quick-create popover and lands on project settings"` -> pass
  - `pnpm -C apps/desktop exec vitest run test/layout-shell.test.ts test/projects-view.test.ts test/project-settings-view.test.ts test/inbox-store.test.ts test/tauri-client-workspace.test.ts` -> pass
- Blockers:
  - none; Task 4 is ready to close for the current contract/fixture scope
- Next:
  - start Task 5 Step 1 by reviewing the current console route/menu order and the workspace-shell layout needed to make workspace settings the first console tab

## Checkpoint 2026-04-19 09:28

- Batch: Task 5 Step 3
- Completed:
  - updated shell workspace label resolution so loopback workspaces keep the localized default label until the canonical workspace name diverges from the connection label, then switch chrome to the saved workspace name immediately
  - refreshed the sidebar workspace trigger and footer workspace menu intro to use the canonical workspace avatar with an initial fallback, sourced from the active workspace summary
  - extended the workspace settings view regression to save avatar changes and assert the new workspace name/avatar appear in topbar and sidebar without a manual reload
  - aligned fixture workspace updates to persist saved avatar uploads as data URLs and kept loopback label regressions consistent with the intended shell-label fallback behavior
- Verification:
  - `pnpm -C apps/desktop exec vitest run test/workspace-settings-view.test.ts` -> pass
  - `pnpm -C apps/desktop exec vitest run test/layout-shell.test.ts -t "keeps the topbar workspace breadcrumb aligned with the sidebar label for loopback workspaces|renders translated local workspace labels and connection dots in the footer workspace menu"` -> pass
  - `pnpm -C apps/desktop exec vitest run test/workspace-settings-view.test.ts test/router.test.ts` -> pass
  - `pnpm -C apps/desktop exec vitest run test/layout-shell.test.ts` -> pass
  - `pnpm -C apps/desktop exec vitest run test/app-auth-gate.test.ts test/auth-store.test.ts` -> pass
- Blockers:
  - none
- Next:
  - start Task 6 Step 1 by reviewing `ProjectsView.vue` and its tests against the registry-only workspace governance scope

## Checkpoint 2026-04-19 09:33

- Batch: Task 6 Step 1
- Completed:
  - rewrote the first `projects-view.test.ts` assertions so the registry defaults to a read-only selected-project detail and no longer expects inline metadata edit controls for existing projects
  - added a focused red-green regression proving active and archived projects are separated by registry tabs and that archived projects surface restore/settings actions only after switching tabs
  - updated `ProjectsView.vue` to introduce active/archived registry tabs, keep creation in an explicit create mode, and show existing project details as read-only registry cards plus capability summaries instead of editable metadata fields
  - aligned the archive/restore registry test flow with the new tabbed behavior so archived projects move out of the active list immediately after lifecycle changes
- Verification:
  - `pnpm -C apps/desktop exec vitest run test/projects-view.test.ts` -> pass
- Blockers:
  - `managerUserId` shortcut and delete-request shortcut are still not surfaced in the registry; Task 6 Step 1 is in progress until those governance actions are added without reintroducing metadata editing
- Next:
  - continue Task 6 Step 1 by wiring a lightweight manager assignment shortcut and reviewing whether deletion-request affordances should land now or remain for Task 6 Step 2 gating work

## Checkpoint 2026-04-19 09:43

- Batch: Task 6 Step 1
- Completed:
  - added a focused registry regression asserting the selected project manager can be reassigned from `ProjectsView.vue` without reopening inline metadata editing
  - updated `ProjectsView.vue` to load workspace access-control members for the registry manager shortcut, wait to render the shortcut until member options are available, and reuse the canonical `UpdateProjectRequest` shape when saving `managerUserId`
  - kept the registry surface read-only for full project metadata while making the manager shortcut usable against the existing project update contract
- Verification:
  - `pnpm -C apps/desktop exec vitest run test/projects-view.test.ts -t "updates the selected project manager from the registry shortcut without reopening full metadata editing|renders a lightweight registry detail with capability summaries and an advanced settings entry"` -> pass
  - `pnpm -C apps/desktop exec vitest run test/projects-view.test.ts` -> pass
- Blockers:
  - delete-request shortcut and its active-vs-archived visibility rules are still pending; Task 6 Step 1 remains in progress until that registry governance action lands
- Next:
  - continue Task 6 Step 1 by adding delete-request affordances for archived projects only, then use Step 2 to tighten any remaining destructive-action visibility gaps

## Checkpoint 2026-04-19 09:55

- Batch: Task 6 Step 1
- Completed:
  - added shared workspace-store actions for loading and creating project deletion requests, then wired `ProjectsView.vue` to load deletion-request state alongside dashboard/runtime data for the selected project
  - extended the desktop fixture state/client with archived-only deletion-request semantics so registry tests now exercise the real project governance flow instead of a UI-only stub
  - surfaced an archived-project deletion-request shortcut in the workspace registry, including pending-status display after request creation, while keeping active projects free of destructive delete actions
  - fixed a real shared-store race where successful deletion-request loading cleared unrelated lifecycle errors such as “last active project cannot be archived”
- Verification:
  - `pnpm -C apps/desktop exec vitest run test/projects-view.test.ts -t "shows delete-request actions only for archived projects and creates a pending request from the registry shortcut"` -> pass
  - `pnpm -C apps/desktop exec vitest run test/projects-view.test.ts` -> pass
- Blockers:
  - none for Step 1; Step 2 still needs tighter delete-review / execute-delete visibility so archived projects expose the full governance actions and active projects remain non-destructive
- Next:
  - continue Task 6 Step 2 by deciding which archived-only destructive controls belong in the registry now (`approve/reject` review or final `delete`) versus which should stay exclusive to project settings in Task 7

## Checkpoint 2026-04-19 10:05

- Batch: Task 6 Step 2
- Completed:
  - added shared workspace-store review actions for project deletion requests so registry and future project settings surfaces can approve or reject pending requests through the same store boundary
  - updated `ProjectsView.vue` to surface archived-only approve/reject actions for pending deletion requests when the current operator has delete-review authority, while still keeping active projects free of destructive delete actions
  - preserved the final destructive delete button for archived projects only after an approval exists and added localized review failure copy for the new governance branch
  - locked the new registry review branch with a focused red-green regression that promotes a pending archived deletion request into the final delete state after approval
- Verification:
  - `pnpm -C apps/desktop exec vitest run test/projects-view.test.ts -t "shows deletion review actions for archived projects with a pending request and promotes approved requests to final delete"` -> pass
  - `pnpm -C apps/desktop exec vitest run test/projects-view.test.ts` -> pass
- Blockers:
  - none
- Next:
  - start Task 7 Step 1 by moving the remaining editable project metadata out of the registry and into `ProjectSettingsView.vue`

## Checkpoint 2026-04-19 10:25

- Batch: Task 7 Step 1
- Completed:
  - mounted `ProjectBasicsPanel` inside `ProjectSettingsView.vue` and wired it to the canonical `useProjectSettings.saveBasics()` flow so project settings now owns `name`, `description`, `resourceDirectory`, `managerUserId`, and `presetCode`
  - kept `leaderAgentId` editing in project settings through the existing leader dialog and updated overview copy to clarify that lifecycle actions remain in the workspace registry
  - removed editable manager controls from `ProjectsView.vue`, leaving the registry as a read-only summary surface with manager/preset badges plus the existing settings deep-link
  - aligned desktop locale copy and the project-settings regression with the real resource-directory picker behavior instead of typing into a read-only input
- Verification:
  - `pnpm -C apps/desktop exec vitest run test/project-settings-view.test.ts -t "saves project metadata from project settings instead of the workspace registry"` -> pass
  - `pnpm -C apps/desktop exec vitest run test/projects-view.test.ts -t "renders a lightweight registry detail with capability summaries and an advanced settings entry|keeps project metadata editing out of the registry and routes operators to project settings instead"` -> pass
  - `pnpm -C apps/desktop exec vitest run test/project-settings-view.test.ts test/projects-view.test.ts` -> pass
- Blockers:
  - none
- Next:
  - start Task 7 Step 2 by collapsing the duplicated grants/runtime mental model into a single capability configuration surface

## Checkpoint 2026-04-19 10:37

- Batch: Task 7 Step 2
- Completed:
  - added `ProjectCapabilitiesPanel.vue` and replaced the separate grant/runtime summary regions in `ProjectSettingsView.vue` with a single capability configuration section
  - exposed unified per-resource capability cards for models, tools, agents, and teams, each summarizing workspace-granted scope, project-owned assets, enabled count, and disabled count from shared `useProjectSettings.ts` selectors
  - added focused page-level regression coverage proving the old split sections are gone and the new capability section surfaces inheritance/disabled-state summaries while existing dialogs still remain usable
- Verification:
  - `pnpm -C apps/desktop exec vitest run test/project-settings-view.test.ts -t "renders document sections instead of tabs and keeps runtime inputs inside dialogs|shows capability inheritance and project-disabled summaries in a single capability section"` -> pass
  - `pnpm -C apps/desktop exec vitest run test/project-settings-view.test.ts -t "supports select all and clear all actions across grant dialogs|persists actor grant and runtime refinements through delta-only settings saves|saves runtime model quota from the runtime dialog only|saves grant model selection without routing through project metadata updates"` -> pass
  - `pnpm -C apps/desktop exec vitest run test/project-settings-view.test.ts test/project-runtime-view.test.ts` -> pass
- Blockers:
  - Step 2 is still in progress because the capability cards currently route into the legacy grant/runtime dialogs; the next batch needs to collapse those entrypoints into a true delta-model editing flow before this step can be marked done
- Next:
  - continue Task 7 Step 2 by unifying model/tool/actor dialog concepts behind the new capability section and removing grant/runtime terminology from the edit flows

## Checkpoint 2026-04-19 10:55

- Batch: Task 7 Step 2
- Completed:
  - collapsed the old model/tool/actor grant-vs-runtime entrypoints into three unified capability dialogs in `ProjectSettingsView.vue`, each with a shared `workspace baseline` / `project rules` scope switch instead of separate legacy dialogs
  - rewired `ProjectCapabilitiesPanel.vue` to expose one edit action per capability card and updated `useProjectSettings.ts` to manage unified dialog state while preserving the existing store save boundaries behind the scenes
  - updated desktop locale copy and regressions so the new capability flow is tested through the unified dialogs rather than the removed grant/runtime entrypoints
- Verification:
  - `pnpm -C apps/desktop exec vitest run test/project-settings-view.test.ts -t "renders document sections and keeps project capability inputs inside unified capability dialogs|keeps unified capability dialogs inside the shared scrollable dialog shell|saves project model quota from the unified capability dialog only|saves workspace model baseline selection without routing through project metadata updates"` -> pass
  - `pnpm -C apps/desktop exec vitest run test/project-settings-view.test.ts test/project-runtime-view.test.ts` -> pass
- Blockers:
  - none for Step 2
- Next:
  - start Task 7 Step 3 by moving archive / restore / delete-request lifecycle controls and approval state into `ProjectSettingsView.vue`

## Checkpoint 2026-04-19 11:07

- Batch: Task 7 Step 3
- Completed:
  - added a dedicated `ProjectLifecyclePanel.vue` and moved archive, restore, delete-request creation, approval review, and final delete execution into `ProjectSettingsView.vue` / `useProjectSettings.ts`
  - added deletion-request loading and review deep-link handling (`?review=deletion-request`) in project settings, including redirecting back to the workspace project registry after final delete
  - removed registry-owned lifecycle action buttons from `ProjectsView.vue`, keeping the workspace projects surface read-only and routing pending review shortcuts into project settings
  - updated locale copy and desktop tests to reflect the new lifecycle ownership boundary
- Verification:
  - `pnpm -C apps/desktop exec vitest run test/project-settings-view.test.ts test/projects-view.test.ts` -> pass
  - `pnpm -C apps/desktop exec vitest run test/project-settings-view.test.ts test/projects-view.test.ts test/project-runtime-view.test.ts` -> pass
- Blockers:
  - none for Task 7
- Next:
  - start Task 8 by wiring workspace/project lifecycle operations into notification center flows from the new settings-owned lifecycle surface

## Checkpoint 2026-04-19 12:40

- Batch: Task 8 Step 1 -> Task 8 Step 2
- Completed:
  - added `useWorkspaceProjectNotifications.ts` to centralize workspace/project governance notification-center entries and route metadata
  - wired workspace settings save, project creation, project-settings saves, archive/restore, delete-request create/review, and final delete into the shared notification flow
  - refreshed inbox state immediately after delete-request creation and approval/rejection so project governance actions update both inbox and local notifications without a full workspace reload
  - extended desktop tests to assert notification records and inbox refresh behavior for governance mutations
- Verification:
  - `pnpm -C apps/desktop exec vitest run test/notification-store.test.ts test/inbox-store.test.ts test/projects-view.test.ts test/project-settings-view.test.ts test/workspace-settings-view.test.ts` -> pass
- Blockers:
  - none
- Next:
  - Task 9 Step 1

## Checkpoint 2026-04-19 14:48

- Batch: Task 9 Step 1 -> Task 9 Step 2
- Completed:
  - added a shared `canShowProjectInShell()` governance helper so shell visibility now follows the same permission-based project-settings contract as route access
  - updated `WorkbenchSidebar.vue` to keep active projects visible for non-member governance reviewers, but constrain those reviewers to the `project-settings` entry instead of leaking member-only project modules
  - routed non-member governance reviewers from collapsed project cards into `project-settings` while keeping sidebar delete affordances member-scoped
  - removed the last stale owner-only helper semantics for `project-settings` and added shell regressions covering both visibility and navigation for non-member governance reviewers
- Verification:
  - `pnpm -C apps/desktop exec vitest run test/project-governance.test.ts test/layout-shell.test.ts -t "keeps project shell navigation visible for members and governance reviewers|shows the project settings entry for governance reviewers who are not project members|routes governance reviewers to project settings when they open a non-member project from the sidebar"` -> pass
  - `pnpm -C apps/desktop exec vitest run test/project-governance.test.ts test/router.test.ts test/layout-shell.test.ts` -> pass
- Blockers:
  - none
- Next:
  - all currently planned tasks are complete; await review or a new execution batch

## Checkpoint 2026-04-19 15:15

- Batch: Merge-prep verification repair
- Completed:
  - re-ran merge-prep verification and found two stale `octopus-infra` test-support mismatches that were outside the functional Task 1-9 batches
  - updated `crates/octopus-infra/src/agent_assets.rs` test table setup so project export/import coverage uses the current `projects` schema with `manager_user_id` and `preset_code`
  - updated the deletion-request tests in `crates/octopus-infra/src/projects_teams.rs` to bootstrap a real owner session before creating and approving archived-project deletion requests, matching the current approver-resolution contract
- Verification:
  - `pnpm openapi:bundle` -> pass
  - `pnpm schema:generate` -> pass
  - `pnpm schema:check` -> pass
  - `pnpm -C apps/desktop exec vitest run test/tauri-client-workspace.test.ts test/auth-store.test.ts test/app-auth-gate.test.ts test/inbox-store.test.ts test/project-governance.test.ts test/workspace-access-control-store.test.ts test/workspace-settings-view.test.ts test/projects-view.test.ts test/project-settings-view.test.ts test/router.test.ts test/layout-shell.test.ts test/project-setup.test.ts test/notification-store.test.ts` -> pass
  - `cargo test -p octopus-server` -> pass
  - `cargo test -p octopus-infra` -> pass
- Blockers:
  - none
- Next:
  - commit branch changes, merge `goya/workspace-project-governance-rebuild` into `main`, then remove the worktree and branch
