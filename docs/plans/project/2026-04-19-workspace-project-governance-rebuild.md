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

Status: `pending`

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

Status: `pending`

Files:
- Modify: `crates/octopus-infra/src/infra_state.rs`
- Modify: `crates/octopus-infra/src/projects_teams.rs`
- Modify: `crates/octopus-infra/src/auth_users.rs`
- Modify: `crates/octopus-infra/src/workspace_paths.rs`
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

Status: `pending`

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

Status: `pending`

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

Status: `pending`

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

Status: `pending`

Files:
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

Status: `pending`

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
- Action: Move project metadata editing into project settings, including `name`, `description`, `resourceDirectory`, `managerUserId`, `presetCode`, and `leaderAgentId`.
- Done when: project settings owns editable project metadata and the workspace registry no longer hosts those controls.
- Verify: `pnpm -C apps/desktop test -- project-settings-view.test.ts projects-view.test.ts`
- Stop if: metadata editing still requires duplicated save paths between registry and settings.

Step 2:
- Action: Replace the current grants/runtime split with a single capability configuration section that surfaces workspace state, project inherited state, project-disabled state, and project-owned assets for models/tools/agents/teams.
- Done when: the UI no longer shows two separate capability regions for the same resource classes, and users can understand inheritance and project override state from one section.
- Verify: `pnpm -C apps/desktop test -- project-settings-view.test.ts project-runtime-view.test.ts`
- Stop if: the new UI still depends on old grants/runtime dialog concepts instead of the new delta model.

Step 3:
- Action: Add lifecycle controls and approval state to project settings, including archive, restore, delete request, approval status, and delete execution once approved.
- Done when: project settings can fully manage project lifecycle within the new approval model and deep-link from inbox items lands on the correct review state.
- Verify: `pnpm -C apps/desktop test -- project-settings-view.test.ts projects-view.test.ts`
- Stop if: delete execution depends on hidden state that only exists on the registry page.

### Task 8: Wire Workspace And Project Operations Into Notification Center

Status: `pending`

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
- Action: Create a shared desktop notification helper for workspace/project governance actions and route successful saves, archive/restore actions, delete-request actions, approvals, rejections, and workspace settings updates into notification center.
- Done when: all user-visible operations on workspace settings, project registry, and project settings create consistent notification-center entries with route metadata where useful.
- Verify: `pnpm -C apps/desktop test -- notification-store.test.ts projects-view.test.ts project-settings-view.test.ts workspace-settings-view.test.ts`
- Stop if: any mutation flow can bypass the helper and succeed without emitting the required notification-center entry.

Step 2:
- Action: Refresh inbox and local notifications coherently after delete-request creation or approval actions so the operator immediately sees the new state.
- Done when: approval operations update both actionable inbox state and local feedback state without a manual page refresh.
- Verify: `pnpm -C apps/desktop test -- notification-store.test.ts inbox-store.test.ts project-settings-view.test.ts projects-view.test.ts`
- Stop if: the only way to reflect approval state is a full workspace bootstrap reload.

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
