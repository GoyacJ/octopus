# App Template Marketplace Implementation Plan

> **For Codex:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task.

**Goal:** Build one app-level template marketplace as the only template discovery and publishing surface, unify builtin/workspace-shared/remote templates under one rule set, and let workspaces import templates while existing business pages keep blank-instance creation.

**Architecture:** The UI entry and browsing route are app-level (`/templates`), but the active workspace remains the execution context for import, publish, usage projection, and workspace-shared template storage. The backend should introduce a dedicated template marketplace domain that aggregates builtin bundles, active-workspace shared templates, and remote marketplace providers behind one contract, while business pages stop owning template browsing logic and only manage instances plus `import from template` / `publish as template` entry points.

**Tech Stack:** `Vue 3 + Vite + Pinia + Vue Router + Vue I18n + Tauri 2`, `@octopus/ui`, Tailwind CSS + design tokens, OpenAPI-first HTTP contracts, `@octopus/schema`, `crates/octopus-server`, `crates/octopus-infra`, SQLite metadata + filesystem payload storage under workspace root, Vitest, Rust unit/integration tests.

---

## Scope

- In scope:
  - App-level route, entry button, and search-overlay entry for the template marketplace.
  - Unified template domain covering `数字员工`, `数字团队`, `项目`, `资源`, `知识`, `工具`.
  - Tool subcategories `内置工具`, `Skill`, `MCP` inside the marketplace.
  - Migration of builtin agent/team templates and current builtin/skill/mcp catalog templates into the marketplace domain.
  - Workspace-shared template publishing from existing instances.
  - Workspace import flow from template to instance.
  - Search, filter, sort, detail view, favorite, like, download placeholder, usage/download metrics display.
  - Remote marketplace provider abstraction with empty/stub download implementation for this phase.
  - Removal of duplicated template browsing logic from old pages after the new marketplace becomes canonical.
- Out of scope:
  - Forcing business pages to become template-first.
  - Fully implemented remote package download/install.
  - Public cloud sync, moderation, template review workflow, or org-wide approval workflow.
  - Cross-workspace personal templates as a new ownership tier in this phase.
  - Replacing current instance editors with a new marketplace-only editor.

## Confirmed Product Decisions

- The template marketplace is an `app`-level module, not a workspace page.
- The current workspace still matters as the status projection for `已导入`, `已启用`, `版本不匹配`, and publish target.
- Old template logic should be extracted into the marketplace module and removed from the existing business pages once migration is complete.
- Existing business pages keep `新建空白实例`.
- Existing instances can be published as templates.
- Default publish target is `当前工作区共享模板`.
- The top-level entry lives in the workbench topbar, left of `设置`.
- The UI can borrow Notion's information architecture and interaction restraint, but must still follow `docs/design/DESIGN.md`.

## Design Summary

### 1. Product Mental Model

The marketplace is the only place where users discover, compare, favorite, like, publish, and import templates. A workspace never becomes the source of template definitions; it only stores workspace-shared templates and imports instances from templates. Business pages keep instance CRUD and operational configuration.

This creates one clean split:

- Marketplace owns template definitions and template rules.
- Business pages own instances and runtime configuration.

### 2. Route And Entry Model

- Add app-level route: `app-templates` at `/templates`.
- Add a topbar button in `WorkbenchTopbar.vue` immediately left of `topbar-settings-button`.
- Add a search-overlay navigation item so the route is discoverable from `Cmd/Ctrl+K`.
- Do not add a left-sidebar primary nav item in phase 1. The feature is global but should stay lightweight in chrome until usage validates a permanent nav slot.

### 3. Information Architecture

The marketplace page should use a list/detail shell:

- Header:
  - Title: `模板市场`
  - Subtitle: explain app-level discovery plus current workspace context.
  - Workspace context chip: current workspace name and active import target.
  - Primary actions: `上传模板`, `仅看我的模板`, `仅看当前工作区已导入`.
- Filter row:
  - Keyword search
  - Category tabs: `数字员工`, `数字团队`, `项目`, `资源`, `知识`, `工具`
  - Source filters: `全部`, `内置`, `工作区共享`, `市场`
  - Tool subtype filters when category is `工具`: `全部`, `内置工具`, `Skill`, `MCP`
  - Sort: `推荐`, `最新`, `最热`, `最多使用`
- List pane:
  - Template cards with source badge, category badge, summary, tags, metrics, current workspace status.
- Detail pane:
  - Overview
  - Detailed content
  - Dependencies
  - Version and source metadata
  - Workspace status projection
  - Actions: `导入到当前工作区`, `收藏`, `点赞`, `下载`, `发布新版本`

### 4. Notion-Inspired, Octopus-Compliant UX

Keep the structure disciplined and dense, but do not copy Notion's visual dialect:

- Use Octopus warm neutral surfaces, whisper borders, calm spacing, and the existing accent system from `docs/design/DESIGN.md`.
- Prefer soft section headers, precise metadata rows, and stable detail panels instead of decorative hero blocks.
- Keep motion understated. Use selection-state polish and list/detail continuity, not novelty animation.
- Make template cards feel like a curated catalog, not a dashboard of widgets.

### 5. Supported Template Categories

Every template record has one top-level category:

- `agent`
- `team`
- `project`
- `resource`
- `knowledge`
- `tool`

Tool templates carry one subtype:

- `builtin`
- `skill`
- `mcp`

### 6. Common Template Metadata

Every template must expose one common summary shape:

- `templateId`
- `category`
- `toolSubtype` when relevant
- `source`
- `scope`
- `name`
- `summary`
- `description`
- `tags`
- `authorLabel`
- `version`
- `updatedAt`
- `favoriteCount`
- `likeCount`
- `downloadCount`
- `usageCount`
- `isFavorited`
- `isLiked`
- `currentWorkspaceStatus`
- `detailPreview`

### 7. Category Payload Model

Each category keeps its own payload instead of forcing one over-general schema:

- Agent template payload:
  - agent profile, prompt, tags, default model strategy, builtin tool keys, skill ids, mcp server names, capability policy.
- Team template payload:
  - leader/member composition, team prompt/personality, agent dependencies, tool dependencies.
- Project template payload:
  - project shell defaults, recommended agents/teams/resources/knowledge/tools, optional scaffold metadata.
- Resource template payload:
  - resource type, access/config skeleton, policy defaults, indexing/import defaults.
- Knowledge template payload:
  - knowledge structure, labels/taxonomy, import strategy, retrieval defaults.
- Tool template payload:
  - builtin tool registration reference, skill bundle reference/content, or MCP package config.

### 8. Publish And Import Flows

#### Publish Existing Instance As Template

Business pages keep a `发布为模板` action on eligible instances:

- Agent pages publish agent instances.
- Team pages publish team instances.
- Tools pages publish skill and MCP definitions.
- Resource, knowledge, and project pages publish their own asset definitions once their payload mapping exists.

Publish flow:

1. User selects `发布为模板`.
2. A marketplace publish dialog opens with current workspace preselected as the target scope.
3. User fills template metadata: name, summary, tags, detailed content, version note.
4. Backend normalizes the source instance into category-specific template payload.
5. Runtime-only, secret, and workspace-private values are removed or rewritten to safe references.
6. Result is stored as a `workspace_shared` template in the active workspace.

#### Import Template Into Current Workspace

Import flow:

1. User opens a template detail.
2. User clicks `导入到当前工作区`.
3. Backend routes the request by category and produces or updates the correct business instance type.
4. Usage metrics increment and the workspace status projection updates.

### 9. Favorite, Like, Download, And Usage Semantics

Split user preference from shared metrics:

- `收藏` is a personal app-level preference and should live in host preferences.
- `点赞` is a workspace-scoped template reaction so counts remain shared inside the active workspace.
- `下载量` and `使用量` belong to the template metrics projection.
- `下载` for remote templates is a reserved action in this phase:
  - UI button exists.
  - Store action and API contract exist.
  - The actual remote package materialization may return a stub result and a clear `coming_soon` state.

## Persistence And Ownership Model

### 1. Storage Responsibilities

Respect the repository persistence rules:

- Template metadata and query fields belong in SQLite.
- Template payload bodies and snapshot files belong on disk.
- New workspace storage path: `data/templates/`.
- Template records reference `storage_path`, `content_hash`, `byte_size`, `content_type`, timestamps, owner scope, and source metadata.

### 2. Ownership Model

- `builtin`
  - Shipped with the app/runtime bundle.
  - Readonly.
- `workspace_shared`
  - Published from instances in the active workspace.
  - Stored in that workspace.
  - Importable by that workspace.
- `marketplace_remote`
  - Search result returned by a remote provider.
  - Readonly in phase 1, with placeholder download.

### 3. App-Level UI, Workspace-Scoped Data Plane

The route is app-level, but the transport should remain adapter-first and workspace-scoped:

- Desktop UI uses `workspace-client.ts`, not bare `fetch`.
- HTTP routes should live under `/api/v1/workspace/templates*` so they keep active workspace semantics and avoid inventing a second business transport boundary.
- App-level personal preferences should extend the existing host preferences flow.

This keeps the app-level UX without violating the current adapter and persistence model.

## Contract And API Design

### 1. New Workspace Template Marketplace Endpoints

Add a dedicated workspace template API family:

- `GET /api/v1/workspace/templates`
  - Aggregated list query with category/source/sort/search filters.
- `GET /api/v1/workspace/templates/{templateId}`
  - Full detail plus current workspace status projection.
- `POST /api/v1/workspace/templates/publish`
  - Publish current instance to `workspace_shared`.
- `POST /api/v1/workspace/templates/{templateId}/import`
  - Import template into current workspace.
- `POST /api/v1/workspace/templates/{templateId}/like`
  - Add like reaction.
- `DELETE /api/v1/workspace/templates/{templateId}/like`
  - Remove like reaction.
- `POST /api/v1/workspace/templates/{templateId}/download`
  - Placeholder/stub for remote downloads in this phase.

### 2. Host Preferences Extension

Extend host preferences so favorites stay app-level:

- `favoriteTemplateIds: string[]`
- optional UI preference keys for remembered marketplace filters if needed later

This should reuse the existing `/api/v1/host/preferences` flow instead of inventing a second host storage surface.

## Migration Strategy

### 1. Canonical Source Transition

Current duplicated sources:

- builtin agent/team templates from the agent bundle path
- builtin/skill/mcp browsing in `useAgentCenter.ts`
- tool catalog browsing in `ToolsView`

Target state:

- the template marketplace becomes the only place that browses templates
- old pages only expose:
  - `从模板导入`
  - `发布为模板`
  - blank instance creation

### 2. Migration Rules

- Do not keep parallel template list UIs after the marketplace is live.
- Do not keep agent/team builtin templates mixed inside instance stores as if they were normal instances.
- Do not let business pages keep their own template-specific search or sorting rules.
- Reuse current import/copy code paths where they are already correct, but call them through the new template marketplace service instead of directly from page-local logic.

### 3. Phasing

- Phase 1:
  - template domain contract
  - app route and UI shell
  - builtin agent/team/tool migration
  - workspace_shared publish/import for agent/team/tool
- Phase 2:
  - project/resource/knowledge payload mapping and importers
  - richer detail rendering and dependency display
- Phase 3:
  - remote provider integration and real download pipeline

## Risks Or Open Questions

- Project/resource/knowledge template payloads need exact normalization rules; stop if a category cannot safely round-trip between instance and template.
- Remote marketplace provider contract is undefined; phase 1 must isolate it behind an interface and return empty data without blocking the rest of the system.
- Builtin templates currently piggyback on agent/team and catalog flows; stop if removing their legacy visibility breaks existing management scenarios that still need readonly context.
- Template like counts need a workspace-scoped reaction model; stop if current auth/session identity is insufficient to produce stable per-user reactions.
- Sensitive fields in skill/MCP/resource templates must not leak into workspace_shared template bodies; stop if redaction/reference semantics are unclear.

## Execution Rules

- Do not start implementation until each task has exact files, acceptance, verification, and stop conditions.
- Do not keep two sources of truth for template browsing after migration.
- Keep app-level UI semantics and workspace-scoped data semantics separate.
- Follow OpenAPI-first ordering for every `/api/v1/*` change.
- Use `data/templates/` plus SQLite metadata instead of introducing ad-hoc JSON caches or frontend persistence.
- Execute in small batches and update task status in place after each batch.

## Task Ledger

### Task 1: Define Canonical Template Contracts

Status: `pending`

Files:
- Create: `contracts/openapi/src/components/schemas/template-marketplace.yaml`
- Create: `contracts/openapi/src/paths/template-marketplace.yaml`
- Modify: `contracts/openapi/src/root.yaml`
- Create: `packages/schema/src/template-marketplace.ts`
- Modify: `packages/schema/src/index.ts`
- Modify: `packages/schema/src/shell.ts`

Preconditions:
- The approved product decisions in this plan remain unchanged.

Step 1:
- Action: Define transport schemas for template summary, template detail, filters, import, publish, like, and download placeholder responses in OpenAPI.
- Done when: Every workspace template endpoint and every payload shape is represented in `contracts/openapi/src/**` without inline large schemas.
- Verify: `pnpm openapi:bundle`
- Stop if: A payload belongs to UI-only behavior and cannot be expressed as a transport contract without inventing fake backend fields.

Step 2:
- Action: Generate and expose domain helpers in `packages/schema/src/template-marketplace.ts`, plus host preference extensions for favorites in `packages/schema/src/shell.ts`.
- Done when: Desktop code can import typed marketplace helpers from `@octopus/schema` without reading `generated.ts` directly.
- Verify: `pnpm schema:generate && pnpm schema:check`
- Stop if: Generated transport types and handwritten domain exports drift or duplicate ownership.

Notes:
- Keep transport contracts under OpenAPI and domain/UI helpers under handwritten schema files.

### Task 2: Build Backend Template Marketplace Domain And Storage

Status: `pending`

Files:
- Create: `crates/octopus-infra/src/template_marketplace.rs`
- Modify: `crates/octopus-infra/src/lib.rs`
- Modify: `crates/octopus-infra/src/bootstrap.rs`
- Modify: `crates/octopus-infra/src/workspace_paths.rs`
- Modify: `crates/octopus-infra/src/split_module_tests.rs`
- Modify: `crates/octopus-infra/src/agent_bundle/builtin.rs`
- Modify: `crates/octopus-infra/src/projects_teams.rs`
- Modify: `crates/octopus-infra/src/resources_skills.rs`

Preconditions:
- Task 1 transport/domain contracts are generated and checked in.

Step 1:
- Action: Add workspace path initialization for `data/templates/` and implement SQLite plus filesystem storage for workspace-shared template metadata and payload files.
- Done when: Workspace bootstrap creates the template storage path and infra code can persist/reload workspace-shared template records with filesystem-backed content.
- Verify: `cargo test -p octopus-infra template_marketplace`
- Stop if: Storage design requires a new root-level persistence class outside current workspace ownership rules.

Step 2:
- Action: Implement one marketplace registry service that aggregates builtin templates, workspace-shared templates, and remote provider results into one queryable list/detail model.
- Done when: Backend can return a filtered, sorted marketplace list and detail object without page-local composition in the desktop app.
- Verify: `cargo test -p octopus-infra template_marketplace_aggregation`
- Stop if: Builtin agent/team/tool templates cannot be represented without continuing to masquerade as normal workspace instances.

Step 3:
- Action: Implement category-aware publish and import services for agent/team/tool templates, reusing existing copy/import logic where appropriate.
- Done when: The domain can publish agent/team/tool instances as workspace-shared templates and import those templates back into workspace instances through one service surface.
- Verify: `cargo test -p octopus-infra template_marketplace_publish_import`
- Stop if: Instance-to-template normalization leaks secrets, runtime-only state, or unstable identifiers.

Notes:
- Keep remote provider integration behind an interface that can return `[]` in phase 1.

### Task 3: Expose Template Marketplace HTTP Routes

Status: `pending`

Files:
- Modify: `crates/octopus-server/src/routes.rs`
- Modify: `crates/octopus-server/src/workspace_runtime.rs`
- Modify: `apps/desktop/src/tauri/workspace-client.ts`
- Modify: `apps/desktop/src/tauri/shared.ts`

Preconditions:
- Tasks 1 and 2 are complete.

Step 1:
- Action: Add workspace template list/detail/publish/import/like/download routes and map them to the new infra service.
- Done when: Every declared template route in OpenAPI exists in the server transport and uses typed request/response handling.
- Verify: `pnpm schema:check:routes && cargo test -p octopus-server workspace_templates`
- Stop if: A route needs to bypass workspace authentication or adapter conventions.

Step 2:
- Action: Extend the desktop workspace adapter with typed marketplace methods.
- Done when: Stores can call `workspace-client.ts` for marketplace list/detail/publish/import/like/download without assembling URLs in views.
- Verify: `pnpm schema:check:adapters && pnpm -C apps/desktop typecheck`
- Stop if: The desktop app would need to call both `shell.ts` and `workspace-client.ts` for the same template business action.

### Task 4: Add The App-Level Marketplace Route And Store

Status: `pending`

Files:
- Create: `apps/desktop/src/views/app/TemplateMarketplaceView.vue`
- Create: `apps/desktop/src/views/app/useTemplateMarketplaceView.ts`
- Create: `apps/desktop/src/stores/template_marketplace.ts`
- Modify: `apps/desktop/src/router/index.ts`
- Modify: `apps/desktop/src/i18n/navigation.ts`
- Modify: `apps/desktop/src/components/layout/WorkbenchTopbar.vue`
- Modify: `apps/desktop/src/components/layout/WorkbenchSearchOverlay.vue`
- Modify: `apps/desktop/src/stores/shell.ts`
- Modify: `apps/desktop/src/locales/zh-CN.json`
- Modify: `apps/desktop/src/locales/en-US.json`
- Test: `apps/desktop/test/template-marketplace-view.test.ts`
- Test: `apps/desktop/test/search-overlay.test.ts`
- Test: `apps/desktop/test/layout-shell.test.ts`

Preconditions:
- Task 3 adapter surface is available.

Step 1:
- Action: Add the app-level route, topbar trigger, and search-overlay entry for the marketplace.
- Done when: Users can open `/templates` from the topbar and from global search while staying inside the workbench shell.
- Verify: `pnpm -C apps/desktop test -- test/layout-shell.test.ts test/search-overlay.test.ts`
- Stop if: Route guard behavior forces a workspace route param and blocks the app-level page.

Step 2:
- Action: Implement the marketplace Pinia store and route view with active workspace context, search/filter/sort state, detail selection, favorite handling, and adapter calls.
- Done when: The page can load aggregated templates, switch categories, show detail, and trigger import/publish/like/download actions through one store.
- Verify: `pnpm -C apps/desktop test -- test/template-marketplace-view.test.ts && pnpm -C apps/desktop typecheck`
- Stop if: The view starts reading raw API payloads directly instead of going through the store/view-model.

### Task 5: Build Marketplace UI Components And Detail Rendering

Status: `pending`

Files:
- Create: `apps/desktop/src/views/template-marketplace/TemplateMarketplaceFilters.vue`
- Create: `apps/desktop/src/views/template-marketplace/TemplateMarketplaceList.vue`
- Create: `apps/desktop/src/views/template-marketplace/TemplateMarketplaceCard.vue`
- Create: `apps/desktop/src/views/template-marketplace/TemplateMarketplaceDetail.vue`
- Create: `apps/desktop/src/views/template-marketplace/TemplatePublishDialog.vue`
- Create: `apps/desktop/src/views/template-marketplace/template-marketplace-copy.ts`
- Test: `apps/desktop/test/template-marketplace-view.test.ts`

Preconditions:
- Task 4 page/store shell exists.

Step 1:
- Action: Build the filter row, card list, and detail panel using `@octopus/ui` list/detail and panel patterns while following `docs/design/DESIGN.md`.
- Done when: The page renders all six categories, tool subfilters, source badges, metrics, and workspace-status callouts without page-local visual drift.
- Verify: `pnpm -C apps/desktop test -- test/template-marketplace-view.test.ts`
- Stop if: The UI needs new primitives that belong in `@octopus/ui` instead of business-page components.

Step 2:
- Action: Add the publish dialog and detail-action affordances for import, favorite, like, and placeholder download.
- Done when: Users can open a publish dialog from the marketplace and see all expected action states in the detail panel.
- Verify: `pnpm -C apps/desktop test -- test/template-marketplace-view.test.ts`
- Stop if: The dialog needs to edit category-specific instance fields that belong in the original business page editors.

### Task 6: Migrate Agent And Team Templates Out Of Agent Center

Status: `pending`

Files:
- Modify: `apps/desktop/src/stores/agent.ts`
- Modify: `apps/desktop/src/stores/team.ts`
- Modify: `apps/desktop/src/views/agents/useAgentCenter.ts`
- Modify: `apps/desktop/src/views/agents/AgentCenterView.vue`
- Modify: `apps/desktop/src/views/agents/AgentListPanel.vue`
- Modify: `apps/desktop/src/views/agents/TeamListPanel.vue`
- Test: `apps/desktop/test/agent-center.test.ts`
- Test: `apps/desktop/test/support/workspace-fixture-state.ts`

Preconditions:
- Tasks 2 through 5 are working for agent/team template display and import.

Step 1:
- Action: Remove builtin template browsing as a first-class list concern from agent/team instance stores and route that discovery through the marketplace.
- Done when: Agent Center manages only instances plus import/publish entry points, and builtin templates no longer appear as if they were normal workspace instances.
- Verify: `pnpm -C apps/desktop test -- test/agent-center.test.ts`
- Stop if: Removing builtin template rows breaks project/workspace readonly instance views that are not actually template-related.

Step 2:
- Action: Add `从模板导入` and `发布为模板` entry points in Agent Center.
- Done when: Users can still start from templates and publish instances without browsing template catalogs inside Agent Center.
- Verify: `pnpm -C apps/desktop test -- test/agent-center.test.ts`
- Stop if: Publish flow requires agent/team editor changes that would create a second template-metadata editor.

### Task 7: Migrate Tool Templates And Remove Duplicated Catalog Template Browsing

Status: `pending`

Files:
- Modify: `apps/desktop/src/views/workspace/ToolsView.vue`
- Modify: `apps/desktop/src/views/workspace/useToolsView.ts`
- Modify: `apps/desktop/src/stores/catalog.ts`
- Modify: `apps/desktop/src/stores/catalog_actions.ts`
- Modify: `apps/desktop/src/stores/catalog_management.ts`
- Test: `apps/desktop/test/tools-view.test.ts`
- Test: `apps/desktop/test/catalog-store.test.ts`

Preconditions:
- Tool templates render correctly in the new marketplace.

Step 1:
- Action: Reframe ToolsView as installed/configured tool management only and remove template-marketplace-style browsing from it.
- Done when: ToolsView still manages builtin/skill/mcp assets already in the workspace, but template discovery is delegated to the marketplace.
- Verify: `pnpm -C apps/desktop test -- test/tools-view.test.ts test/catalog-store.test.ts`
- Stop if: Installed tool configuration and template discovery are still coupled inside one store and cannot be separated cleanly.

Step 2:
- Action: Add `从模板导入` and `发布为模板` entry points for skill and MCP assets.
- Done when: Skill/MCP definitions can be turned into workspace-shared templates and new tool instances can be created from marketplace templates.
- Verify: `pnpm -C apps/desktop test -- test/tools-view.test.ts`
- Stop if: Template publishing requires secrets or runtime-specific values that cannot be safely normalized.

### Task 8: Add Project, Resource, And Knowledge Template Payload Support

Status: `pending`

Files:
- Modify: `packages/schema/src/template-marketplace.ts`
- Modify: `crates/octopus-infra/src/template_marketplace.rs`
- Modify: `apps/desktop/src/stores/resource.ts`
- Modify: `apps/desktop/src/stores/knowledge.ts`
- Modify: `apps/desktop/src/views/workspace/WorkspaceResourcesView.vue`
- Modify: `apps/desktop/src/views/workspace/WorkspaceKnowledgeView.vue`
- Modify: `apps/desktop/src/views/project/ProjectResourcesView.vue`
- Modify: `apps/desktop/src/views/project/ProjectKnowledgeView.vue`
- Modify: `apps/desktop/src/views/workspace/ProjectsView.vue`
- Test: `apps/desktop/test/resources-view.test.ts`
- Test: `apps/desktop/test/workspace-resources-view.test.ts`
- Test: `apps/desktop/test/knowledge-view.test.ts`

Preconditions:
- Core marketplace registry and publish/import infrastructure exist.

Step 1:
- Action: Define payload normalization and import rules for project/resource/knowledge templates.
- Done when: Each category has a stable template payload that can be published from an instance and imported back into a workspace-managed asset.
- Verify: `cargo test -p octopus-infra template_marketplace_project_resource_knowledge`
- Stop if: Any category still lacks a safe boundary between editable instance fields and template-safe fields.

Step 2:
- Action: Add publish/import entry points to the corresponding business pages.
- Done when: These pages keep blank-instance creation while exposing the marketplace flow consistently.
- Verify: `pnpm -C apps/desktop test -- test/resources-view.test.ts test/workspace-resources-view.test.ts test/knowledge-view.test.ts`
- Stop if: Page-local editors begin taking ownership of template metadata beyond the publish dialog.

### Task 9: Add Favorites, Likes, Metrics, And Download Placeholder

Status: `pending`

Files:
- Modify: `packages/schema/src/shell.ts`
- Modify: `crates/octopus-server/src/routes.rs`
- Modify: `crates/octopus-server/src/workspace_runtime.rs`
- Modify: `crates/octopus-infra/src/template_marketplace.rs`
- Modify: `apps/desktop/src/stores/shell.ts`
- Modify: `apps/desktop/src/stores/template_marketplace.ts`
- Test: `apps/desktop/test/template-marketplace-view.test.ts`
- Test: `apps/desktop/test/settings-view.test.ts`

Preconditions:
- Template registry and UI actions exist.

Step 1:
- Action: Persist favorites in host preferences and implement workspace-scoped like reactions plus metrics projection.
- Done when: Favorite toggles survive app restart, like counts are shared through workspace data, and the marketplace UI shows all four requested metrics.
- Verify: `pnpm -C apps/desktop test -- test/template-marketplace-view.test.ts test/settings-view.test.ts && cargo test -p octopus-infra template_marketplace_reactions`
- Stop if: Stable user identity for like reactions is unavailable in the current session model.

Step 2:
- Action: Add the placeholder download action and state model for remote templates.
- Done when: Remote templates expose a working download button and typed result, even if the underlying operation intentionally returns a stubbed `coming_soon` state.
- Verify: `pnpm -C apps/desktop test -- test/template-marketplace-view.test.ts`
- Stop if: The placeholder action would require committing to an incorrect storage or provider protocol.

### Task 10: Remove Legacy Duplicate Paths And Run Full Verification

Status: `pending`

Files:
- Modify: `apps/desktop/src/views/agents/useAgentCenter.ts`
- Modify: `apps/desktop/src/views/workspace/useToolsView.ts`
- Modify: `apps/desktop/src/views/agents/AgentResourceCatalogPanel.vue`
- Modify: `apps/desktop/test/agent-center.test.ts`
- Modify: `apps/desktop/test/tools-view.test.ts`
- Modify: `apps/desktop/test/template-marketplace-view.test.ts`

Preconditions:
- Tasks 1 through 9 pass in isolation.

Step 1:
- Action: Delete or neutralize obsolete template-specific tabs, cards, and duplicated store branches after the marketplace fully covers the user flows.
- Done when: There is one canonical template browsing surface and no user-facing duplicate template catalogs remain.
- Verify: `pnpm -C apps/desktop test -- test/agent-center.test.ts test/tools-view.test.ts test/template-marketplace-view.test.ts`
- Stop if: A legacy path still exposes functionality not yet migrated into the marketplace.

Step 2:
- Action: Run cross-layer verification for contracts, desktop, and Rust backend.
- Done when: The full feature passes the required repo checks for the touched surfaces.
- Verify: `pnpm schema:check && pnpm check:desktop && cargo test -p octopus-infra && cargo test -p octopus-server`
- Stop if: Any failure indicates a hidden second source of truth for templates.

## Batch Checkpoint Format

After each execution batch, append a short checkpoint using this shape:

```md
## Checkpoint YYYY-MM-DD HH:MM

- Batch: Task 1 Step 1 -> Task 1 Step 2
- Completed:
  - short list
- Verification:
  - `command` -> pass or fail
- Blockers:
  - none
- Next:
  - Task 2 Step 1
```
