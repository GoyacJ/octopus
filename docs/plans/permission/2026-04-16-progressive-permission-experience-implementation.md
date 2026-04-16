# Progressive Permission Experience Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the current enterprise-first access-control center with one progressive permission experience that supports personal, team, and enterprise usage in the same product without any explicit mode switch.

**Architecture:** Refactor the current access-control implementation into a cleaner target model before launch. Keep RBAC, role binding, data policy, resource policy, session, and audit as the authorization core, but explicitly decouple route authorization from menu policy. Introduce first-class access-section authorization, a system role template catalog, grouped business-facing capability bundles, and a progressive access experience summary. Reframe desktop IA into `Members / Access / Governance`, make menu policy part of governance rather than navigation authority, and remove transitional compatibility layers that would become technical debt.

**Tech Stack:** OpenAPI contracts in `contracts/openapi/src/**`, generated transport in `packages/schema/src/generated.ts`, feature schema exports in `packages/schema/src/*`, Rust access-control domain and service code in `crates/octopus-core`, `crates/octopus-platform`, `crates/octopus-infra`, and `crates/octopus-server`, plus Vue 3 + Vite + Pinia + Vue Router + Vue I18n + Tauri 2 desktop surfaces in `apps/desktop/src/**`.

---

## Read Before Starting

- Root governance: `AGENTS.md`
- Docs governance: `docs/AGENTS.md`
- UI source of truth: `docs/design/DESIGN.md`
- HTTP contract policy: `docs/api-openapi-governance.md`
- OpenAPI source rules: `contracts/openapi/AGENTS.md`
- Current access shell: `apps/desktop/src/views/workspace/AccessControlView.vue`
- Current access store: `apps/desktop/src/stores/workspace-access-control.ts`
- Current access schema: `packages/schema/src/access-control.ts`
- Current access transport: `contracts/openapi/src/components/schemas/access-control.yaml`
- Current server wiring: `crates/octopus-platform/src/access_control.rs`, `crates/octopus-infra/src/access_control.rs`, `crates/octopus-server/src/handlers.rs`, `crates/octopus-server/src/routes.rs`

## Fixed Product Decisions

- There is no personal mode switch, team mode switch, or enterprise mode switch.
- The system always uses one permission core and one data model.
- Complexity is revealed progressively from actual workspace state, actual grants, and actual administrative intent.
- `Personal Center` remains the place for self profile, self preferences, and personal runtime settings. It does not become the workspace permission center.
- The access experience is reorganized into three user-facing surfaces:
  - `Members`: who is here and what their primary access level is
  - `Access`: preset-first role assignment and grouped business actions
  - `Governance`: organization structure, custom roles, policies, resources, menus, sessions, and audit
- Access-center route authorization becomes a first-class semantic model. It is not derived from menu visibility.
- Raw permission codes, menu policies, resource policies, and org structure are not the first thing a normal user sees.
- Menu policy is a governance capability and presentation policy only. It does not define access-center IA or route access.
- Raw enterprise entities remain available for governance workflows, but they no longer dictate the top-level product structure.
- The initial preset catalog is intentionally small: `owner`, `admin`, `member`, `viewer`, `auditor`.
- User-facing preset labels do not equal raw role codes. All system-managed roles use a reserved `system.*` namespace, including `system.owner`.
- Custom roles remain supported, but they move behind governance and are never the first default path.
- Because the product is not launched yet, the implementation should prefer clean replacement and refactor over compatibility shims.

## Non-Goals

- Do not rewrite RBAC into a new ABAC or policy DSL.
- Do not add a separate persistence technology or frontend truth source.
- Do not collapse `Personal Center` and workspace access control into one screen.
- Do not remove enterprise governance entities from the backend.
- Do not hand-edit `contracts/openapi/octopus.openapi.yaml` or `packages/schema/src/generated.ts`.
- Do not keep pre-launch compatibility routes, transport fields, or guard branches that exist only to preserve the current unfinished IA.

## Current Baseline

- Desktop access control is currently an enterprise-first tabbed shell with `users / org / roles / policies / menus / resources / sessions`.
- The backend already has a real authorization and access-control core with users, org units, positions, groups, roles, role bindings, data policies, resource policies, menu policies, protected resources, sessions, and audit.
- The current store eagerly loads almost every admin dataset once the access center is entered.
- The current route and menu wiring uses raw access-control menus as the primary UX source.
- There is strong evidence of a default builtin `owner` role, but not yet a small practical preset suite for ordinary collaboration.
- Product positioning already spans personal, team, and enterprise usage, so the UX needs to scale across all three without fragmenting the model.

## Target Authorization Model

- Access-center routing is governed by explicit semantic section grants:
  - `access.members.read`
  - `access.assign.read`
  - `access.governance.read`
- Section grants are derived from effective permissions, roles, and policies in the authorization service, not from menu ids.
- Menu policy remains available for workspace navigation and governance configuration, but it is not the authorization model for the access center.
- The access shell and router guards must consume section grants as first-class transport fields.
- `Members` and `Access` should be reachable without loading governance-heavy datasets.

## Role And Template Model

- Introduce first-class system role templates for `owner`, `admin`, `member`, `viewer`, and `auditor`.
- Separate user-facing templates from raw assignable roles:
  - `AccessRoleTemplate` describes product presets
  - `AccessRoleRecord` describes actual RBAC roles
- All platform-managed roles use a reserved `system.*` namespace:
  - `system.owner`
  - `system.admin`
  - `system.member`
  - `system.viewer`
  - `system.auditor`
- Custom roles are first-class but explicitly marked as `custom`.
- Pre-launch refactor is allowed to rename the current `owner` role implementation to `system.owner` so the role model is internally consistent before release.

## Codex Execution Checklist

- [ ] Create or use a dedicated worktree before implementation.
- [ ] Read the governance docs listed above before touching contracts, routes, or UI.
- [ ] Keep OpenAPI-first order: edit `contracts/openapi/src/**` -> `pnpm openapi:bundle` -> `pnpm schema:generate` -> adapters/stores/server/tests.
- [ ] Preserve adapter-first desktop architecture. No business view may call raw `fetch`.
- [ ] Keep browser-host and Tauri-host public contract shapes identical.
- [ ] Reuse `@octopus/ui` primitives and `docs/design/DESIGN.md` interaction language.
- [ ] Avoid introducing business-page-specific colors, ad-hoc spacing, or third-party UI patterns.
- [ ] Prefer coherent target-state refactor over additive transitional layering.
- [ ] Avoid eager loading the full governance dataset on first open of the access center.
- [ ] Remove temporary transition helpers before final verification.

## Global Exit Conditions

- A one-person workspace can use access management without seeing enterprise jargon or raw policy tables first.
- A small team workspace can add members and assign sensible presets without building org units, custom roles, or menu policy rules.
- An enterprise workspace can still access org structure, custom roles, policies, resources, sessions, and audit from a dedicated governance surface.
- The same workspace can grow from simple to complex without any explicit mode migration.
- Access-center routing is governed by first-class semantic section authorization, not menu visibility.
- The internal role model is consistent: system-managed roles use `system.*`, custom roles use their own namespace, and preset labels are UI-facing abstractions.
- No compatibility-only routes, transport fields, or guard branches remain in the final implementation.
- Access-center navigation no longer depends on raw menu policy definitions as its primary UX model.
- Desktop tests, schema checks, and targeted Rust verification all pass.

## Task 1: Add First-Class Access Experience Contracts

**Files:**
- Modify: `contracts/openapi/src/components/schemas/access-control.yaml`
- Modify: `contracts/openapi/src/paths/access-control.yaml`
- Modify: `packages/schema/src/access-control.ts`
- Modify: `packages/schema/src/index.ts`
- Modify: `apps/desktop/test/openapi-transport.test.ts`

**Implement:**
- Add one lightweight transport response for the new UX entry layer, `AccessExperienceResponse`.
- Add transport types for:
  - `AccessExperienceSummary`
  - `AccessSectionGrant`
  - `AccessRoleTemplate`
  - `AccessRolePreset`
  - `AccessCapabilityBundle`
- Extend `AccessRoleRecord` with additive metadata:
  - `source: system | custom`
  - `editable: boolean`
- Remove the assumption that access-center authorization comes from menu visibility.
- Add an additive endpoint `GET /api/v1/access/experience` that returns:
  - summary flags derived from real workspace state
  - first-class section grants for `members / access / governance`
  - the system template catalog
  - the preset catalog
  - grouped capability bundles for the `Access` surface
  - counts needed for empty states and progressive disclosure

**Required fields:**
- `AccessExperienceSummary`:
  - `experienceLevel`: `personal` | `team` | `enterprise`
  - `memberCount`
  - `hasOrgStructure`
  - `hasCustomRoles`
  - `hasAdvancedPolicies`
  - `hasMenuGovernance`
  - `hasResourceGovernance`
  - `recommendedLandingSection`
- `AccessSectionGrant`:
  - `section`
  - `allowed`
- `AccessRoleTemplate`:
  - `code`
  - `name`
  - `description`
  - `managedRoleCodes`
  - `editable`
- `AccessRolePreset`:
  - `code`
  - `name`
  - `description`
  - `recommendedFor`
  - `templateCodes`
  - `capabilityBundleCodes`
- `AccessCapabilityBundle`:
  - `code`
  - `name`
  - `description`
  - `permissionCodes`

**Verification:**
```bash
pnpm openapi:bundle
pnpm schema:generate
pnpm schema:check
pnpm -C apps/desktop exec vitest run test/openapi-transport.test.ts
```

**Done when:**
- The frontend can consume one stable target-state transport shape for progressive access UX.
- Section authorization is expressed explicitly in transport instead of being implied by menu state.

## Task 2: Refactor The Role Model Into System Templates Plus Custom Roles

**Files:**
- Modify: `crates/octopus-core/src/lib.rs`
- Modify: `crates/octopus-platform/src/access_control.rs`
- Modify: `crates/octopus-infra/src/access_control.rs`
- Modify: `crates/octopus-server/src/handlers.rs`

**Implement:**
- Add backend support for role source metadata matching `system | custom`.
- Introduce first-class system role templates and align the persisted role model with them.
- Rename the current owner implementation to `system.owner`.
- Reserve the `system.*` role-code namespace for platform-managed roles.
- Reject create or update operations from custom role flows when they try to use `system.*`.
- Keep the domain model explicit:
  - templates are product presets
  - roles are RBAC units
  - presets can map to one or more templates or managed roles

**Required backend rules:**
- `system.owner` remains full-access and is never downgraded by this change.
- User-facing preset labels are not used as raw role codes.
- Custom role creation remains in the same table, but cannot occupy the `system.*` namespace.

**Verification:**
```bash
cargo test -p octopus-infra access_control
cargo test -p octopus-server access
```

**Done when:**
- The backend can distinguish system and custom roles cleanly.
- The role model no longer relies on implicit legacy naming assumptions.

## Task 3: Expose Experience Summary, Templates, Presets, And Section Grants

**Files:**
- Modify: `crates/octopus-platform/src/access_control.rs`
- Modify: `crates/octopus-infra/src/access_control.rs`
- Modify: `crates/octopus-server/src/handlers.rs`
- Modify: `crates/octopus-server/src/routes.rs`

**Implement:**
- Seed and repair system-managed roles idempotently:
  - `system.owner`
  - `system.admin`
  - `system.member`
  - `system.viewer`
  - `system.auditor`
- Compute `AccessExperienceSummary` from actual state, not a stored mode flag.
- Compute `AccessSectionGrant[]` from effective permissions and policy rules.
- Add grouped capability bundles by mapping raw permission codes into a small business-facing catalog.
- Expose a preset catalog where:
  - preset `owner` maps to `system.owner`
  - other presets map to `system.*` managed roles or templates
- Keep raw governance endpoints only where they still represent target-state governance entities.

**Required backend rules:**
- Summary logic is deterministic and does not depend on frontend inference.
- Section grants are derived from semantic authorization rules on the server.
- Menu policy is not consulted for access-center route authorization.
- Missing system roles are created or repaired idempotently.

**Verification:**
```bash
cargo test -p octopus-infra access_control
cargo test -p octopus-server access
```

**Done when:**
- The backend exposes a stable preset catalog and summary without breaking existing RBAC behavior.
- A newly bootstrapped workspace gets sensible system presets on a clean role model.

## Task 4: Split Desktop Access Loading Into Entry Data And Governance Data

**Files:**
- Modify: `apps/desktop/src/tauri/workspace_api.ts`
- Modify: `apps/desktop/src/tauri/workspace-client.ts`
- Modify: `apps/desktop/src/stores/workspace-access-control.ts`
- Create: `apps/desktop/test/workspace-access-control-store.test.ts`

**Implement:**
- Add client support for `getAccessExperience`.
- Split current store loading into at least two layers:
  - `ensureAuthorizationContext` plus `loadExperience`
  - `loadGovernanceData`
- If `loadAdminData` is kept temporarily during refactor, stop using it as the default first-load path immediately and remove it before final verification.
- Derive stable helpers from the experience response:
  - `accessSectionGrants`
  - `recommendedAccessSection`
  - `isGovernanceEmpty`
  - `presetCards`
  - `capabilityBundlesByCode`
- Only load org, policy, resource, menu, session, and audit datasets when the governance surface is actually opened.

**Required store guarantees:**
- Route guards continue to work.
- The store exposes first-class section grants rather than inferring access from menu data.
- The new shell does not block on the full enterprise dataset.

**Verification:**
```bash
pnpm -C apps/desktop exec vitest run test/workspace-access-control-store.test.ts test/openapi-transport.test.ts
```

**Done when:**
- Opening the access center no longer eagerly fetches every enterprise admin dataset.
- The store exposes stable section-grant, template, and preset state for the new shell.

## Task 5: Replace Access Routing With First-Class Section Routes

**Files:**
- Modify: `apps/desktop/src/router/index.ts`
- Modify: `apps/desktop/src/navigation/menuRegistry.ts`
- Modify: `apps/desktop/test/router.test.ts`
- Modify: `apps/desktop/test/startup-router-install.test.ts`

**Implement:**
- Add new first-class routes:
  - `/workspaces/:workspaceId/access-control/members`
  - `/workspaces/:workspaceId/access-control/access`
  - `/workspaces/:workspaceId/access-control/governance`
- Make `/access-control` redirect to the store-provided recommended section.
- Keep main sidebar `访问控制` entry intact.
- Remove the old raw child-route model for `users / org / roles / policies / menus / resources / sessions` from the top-level access shell.
- Guard the new top-level routes with first-class section grants.
- Delete compatibility-only redirects and obsolete route branches that exist only because of the current unfinished IA.

**Required routing rules:**
- New top-level sections are guarded by derived section grants.
- New routing does not bypass the current authorization model.
- The final route tree reflects the target IA directly, not a wrapper around legacy child routes.

**Verification:**
```bash
pnpm -C apps/desktop exec vitest run test/router.test.ts test/startup-router-install.test.ts
```

**Done when:**
- New top-level access routes are stable and guarded.
- The router no longer carries compatibility-only access-center route branches.

## Task 6: Replace Raw Menu Tabs With A Fixed Access Shell

**Files:**
- Modify: `apps/desktop/src/views/workspace/AccessControlView.vue`
- Modify: `apps/desktop/src/locales/en-US.json`
- Modify: `apps/desktop/src/locales/zh-CN.json`
- Modify: `apps/desktop/test/access-centers.test.ts`

**Implement:**
- Stop rendering the top-level access shell tabs directly from `availableAccessControlMenus`.
- Replace them with a fixed, calm, workbench-style section switcher for `Members / Access / Governance`.
- Keep the main sidebar `访问控制` entry, but decouple the shell from raw access-control submenu shape.
- Change the access shell load behavior so first entry loads authorization context plus experience data, not full governance data.

**Required shell rules:**
- The first screen must read like a workbench surface, not an enterprise admin tab dump.
- Primary access navigation must no longer be menu-policy-shaped.
- The shell must still cooperate with existing route guards and workspace bootstrap.

**Verification:**
```bash
pnpm -C apps/desktop exec vitest run test/access-centers.test.ts test/router.test.ts
```

**Done when:**
- The access shell presents three fixed sections instead of seven raw admin tabs.
- The shell still opens reliably under current workspace bootstrap flow.

## Task 7: Build The Members Surface

**Files:**
- Create: `apps/desktop/src/views/workspace/access-control/AccessMembersView.vue`
- Modify: `apps/desktop/src/views/workspace/access-control/useAccessControlSelection.ts`
- Modify: `apps/desktop/src/views/workspace/access-control/useAccessControlNotifications.ts`
- Modify: `apps/desktop/src/locales/en-US.json`
- Modify: `apps/desktop/src/locales/zh-CN.json`
- Modify: `apps/desktop/test/access-centers.test.ts`

**Implement:**
- Create a member-first workbench surface that answers:
  - who belongs to this workspace
  - whether they are active
  - what their primary preset or effective access level is
  - whether they participate in an org structure
- Show friendly empty and starter states for single-user workspaces.
- Keep quick actions practical:
  - create user
  - assign preset
  - open advanced details
- Do not expose org tree editing, menu policy, or raw permission code grids on first render.

**Required presentation rules:**
- Use quiet page framing aligned with `docs/design/DESIGN.md`.
- Prefer list/detail or stacked workbench blocks over admin-console tables.
- Avoid exposing enterprise-only jargon when the workspace is still personal-like.

**Verification:**
```bash
pnpm -C apps/desktop exec vitest run test/access-centers.test.ts
```

**Done when:**
- A personal workspace sees a simple member list and access summary.
- A team workspace can manage people without entering governance first.

## Task 8: Build The Access Surface Around Managed Presets And Capability Bundles

**Files:**
- Create: `apps/desktop/src/views/workspace/access-control/AccessPermissionsView.vue`
- Modify: `apps/desktop/src/views/workspace/access-control/AccessControlRolesView.vue`
- Modify: `apps/desktop/src/stores/workspace-access-control.ts`
- Modify: `apps/desktop/src/locales/en-US.json`
- Modify: `apps/desktop/src/locales/zh-CN.json`
- Modify: `apps/desktop/test/access-centers.test.ts`

**Implement:**
- Make presets the primary path for everyday assignment.
- Present grouped business actions instead of raw permission codes by default.
- Provide a clear but secondary path into custom roles.
- Show the relationship between:
  - preset
  - included capability bundles
  - system templates
  - managed role codes behind the preset
- Keep advanced editing for raw custom roles outside the default flow.

**Initial bundle catalog guidance:**
- `workspace_governance`
- `member_management`
- `project_and_resource_access`
- `automation_and_tools`
- `security_and_audit`

**Required UX rules:**
- A normal admin should be able to grant practical access without reading permission code strings.
- Preset cards must explain intent, not implementation detail.
- Menu policy must not appear on this surface.

**Verification:**
```bash
pnpm -C apps/desktop exec vitest run test/access-centers.test.ts test/workspace-access-control-store.test.ts
```

**Done when:**
- A small team can run day-to-day membership and access changes from one screen.
- Custom role management is still available but no longer mandatory for common cases.

## Task 9: Fold Advanced Administration Into Governance

**Files:**
- Create: `apps/desktop/src/views/workspace/access-control/AccessGovernanceView.vue`
- Modify: `apps/desktop/src/views/workspace/access-control/AccessControlOrgView.vue`
- Modify: `apps/desktop/src/views/workspace/access-control/AccessControlPoliciesView.vue`
- Modify: `apps/desktop/src/views/workspace/access-control/AccessControlMenusView.vue`
- Modify: `apps/desktop/src/views/workspace/access-control/AccessControlResourcesView.vue`
- Modify: `apps/desktop/src/views/workspace/access-control/AccessControlSessionsView.vue`
- Modify: `apps/desktop/src/views/workspace/access-control/AccessControlRolesView.vue`
- Modify: `apps/desktop/src/locales/en-US.json`
- Modify: `apps/desktop/src/locales/zh-CN.json`
- Modify: `apps/desktop/test/access-centers.test.ts`

**Implement:**
- Create a governance hub that groups advanced areas into one calm surface instead of seven equal-priority top tabs.
- Reuse existing advanced views where possible instead of rewriting them all at once.
- Group advanced sections under stable headings:
  - organization
  - custom roles
  - policies
  - menu governance
  - protected resources
  - sessions and audit
- Keep governance visible, but allow it to render a low-noise empty state when a workspace has not grown into advanced administration yet.

**Required governance rules:**
- Governance remains fully capable for enterprise users.
- Advanced sections do not dominate the initial experience for personal and team workspaces.
- Existing enterprise data operations remain accessible behind the grouped surface.

**Verification:**
```bash
pnpm -C apps/desktop exec vitest run test/access-centers.test.ts test/router.test.ts
```

**Done when:**
- Enterprise users can still reach every advanced entity.
- Personal and team users are not forced through enterprise governance concepts up front.

## Task 10: Migrate Fixtures, Copy, And Final Route/Test Cleanup

**Files:**
- Modify: `apps/desktop/test/support/workspace-fixture-client.ts`
- Modify: `apps/desktop/test/support/workspace-fixture-state.ts`
- Modify: `apps/desktop/test/access-centers.test.ts`
- Modify: `apps/desktop/test/router.test.ts`
- Modify: `apps/desktop/test/openapi-transport.test.ts`
- Modify: `apps/desktop/src/i18n/copy.ts`
- Modify: `apps/desktop/src/i18n/navigation.ts`
- Modify: `apps/desktop/src/locales/en-US.json`
- Modify: `apps/desktop/src/locales/zh-CN.json`

**Implement:**
- Update fixtures to return:
  - experience summary
  - section grants
  - role templates
  - managed presets
  - capability bundles
  - enterprise-heavy states for governance coverage
  - personal and team states for progressive disclosure coverage
- Rewrite tests that currently assert the old seven-tab IA.
- Add coverage for:
  - section-level route guarding
  - reserved `system.*` role namespace behavior
  - absence of compatibility-only access route branches
- Make copy calmer and more practical:
  - no enterprise-overclaim for basic workspaces
  - no raw permission jargon in the primary flow
- Keep locale keys organized and avoid one-off text literals in views.

**Verification:**
```bash
pnpm -C apps/desktop exec vitest run test/access-centers.test.ts test/router.test.ts test/openapi-transport.test.ts
```

**Done when:**
- Test fixtures cover personal, team, and enterprise progression.
- Copy matches the new progressive access model.

## Rollout And Migration Notes

- Pre-launch refactor is allowed to replace the current unfinished access IA directly.
- Rename the current owner implementation to `system.owner` as part of the role-model cleanup.
- Seed system-managed roles idempotently during service initialization or access bootstrap.
- Remove compatibility-only access routes, redirect branches, and guard logic instead of carrying them forward.
- Menu policy records remain valid governance data, but they must not act as access-center route authorization.
- If a later phase wants invite workflows, SCIM, or enterprise directory sync, layer them into `Members` or `Governance` after this plan, not during it.

## Final Verification Checklist

- [ ] `pnpm openapi:bundle`
- [ ] `pnpm schema:generate`
- [ ] `pnpm schema:check`
- [ ] `pnpm -C apps/desktop exec vitest run test/openapi-transport.test.ts`
- [ ] `pnpm -C apps/desktop exec vitest run test/workspace-access-control-store.test.ts`
- [ ] `pnpm -C apps/desktop exec vitest run test/access-centers.test.ts`
- [ ] `pnpm -C apps/desktop exec vitest run test/router.test.ts test/startup-router-install.test.ts`
- [ ] `pnpm check:frontend`
- [ ] `cargo test -p octopus-infra access_control`
- [ ] `cargo test -p octopus-server access`
- [ ] New top-level sections are guarded via first-class semantic section grants.
- [ ] Custom role flows reject reserved managed role codes under `system.*`.
- [ ] The router no longer carries compatibility-only access child routes or redirects.
- [ ] Personal-like fixture lands on a simple access surface.
- [ ] Team-like fixture can manage members and presets without governance.
- [ ] Enterprise-like fixture can still reach org, policy, resource, menu, session, and audit operations.
- [ ] Menu policy no longer determines access-center route authorization.
