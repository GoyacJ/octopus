# Claude-Inspired Project Conversation Deliverable Workbench Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Rebuild Octopus chat around a Claude-like `Project -> Conversation -> Deliverable` experience while keeping the existing durable `Session -> Run -> Subrun` runtime substrate, so users can preview, edit, version, promote, and fork generated outputs without the current split between trace, knowledge, and artifact metadata views.

**Architecture:** Keep `project`, `session`, `run`, `subrun`, mailbox, and trace as the backend execution truth. Refactor the public contract and desktop surface so the primary user-facing objects become `Project`, `Conversation`, and `Deliverable`, with `Context` and `Ops` as secondary planes. Because the product is not launched, prefer breaking refactors and incompatible cleanup over compatibility shims, alias fields, or temporary dual models.

**Tech Stack:** Vue 3, Vite, Pinia, Vue Router, Vue I18n, Tauri 2, Rust (`octopus-server`, `octopus-platform`, `octopus-runtime-adapter`, `octopus-infra`), OpenAPI-first transport, generated `@octopus/schema`, SQLite projections, disk-backed artifact storage under `data/artifacts`, JSONL runtime events.

---

## Read Before Starting

- Root `AGENTS.md`
- `docs/AGENTS.md`
- `docs/design/DESIGN.md`
- `docs/plans/chat/2026-04-16-claude-inspired-project-conversation-deliverable-design.md`
- `docs/api-openapi-governance.md`
- `docs/runtime_config_api.md`
- `contracts/openapi/AGENTS.md`
- `docs/plans/runtime/agent-runtime-rebuild-design.md`

## Branch And Compatibility Policy

- This planning document was created on `main` per user instruction.
- Default execution assumption for follow-up Codex work is also `main` unless the user changes that requirement.
- The product is not launched. Optimize for a clean model, not for preserving legacy APIs, query params, enums, or store shapes.
- Do not add compatibility aliases such as `ArtifactRecord + ArtifactDetailRecord` if one canonical transport shape is enough.
- Do not preserve legacy UI modes merely to avoid route or store churn.
- When an old abstraction conflicts with the target model, delete it in the same task that replaces it.

## Current Architecture Summary

### Existing strengths

- Runtime already has durable `Session`, `Run`, and `Subrun` containers, plus typed `trace`, `approval`, `memory`, `mailbox`, and `background` projection fields in [crates/octopus-core/src/lib.rs](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/crates/octopus-core/src/lib.rs).
- The desktop conversation surface is already a two-pane workbench in [apps/desktop/src/views/project/ConversationView.vue](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/apps/desktop/src/views/project/ConversationView.vue).
- Artifact IDs already flow through runtime messages and the shell keeps artifact selection state in [apps/desktop/src/stores/shell.ts](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/apps/desktop/src/stores/shell.ts).
- Artifact storage already follows the correct persistence rule: body on disk, metadata in SQLite, with existing storage helpers in [crates/octopus-runtime-adapter/src/persistence.rs](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/crates/octopus-runtime-adapter/src/persistence.rs) and [crates/octopus-infra/src/infra_state.rs](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/crates/octopus-infra/src/infra_state.rs).

### Current gaps

- The transport contract only exposes a thin `ArtifactRecord` summary in [contracts/openapi/src/components/schemas/projects.yaml](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/contracts/openapi/src/components/schemas/projects.yaml) and [packages/schema/src/artifact.ts](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/packages/schema/src/artifact.ts). There is no canonical detail, version, preview, edit, promote, or fork contract.
- The right pane in [apps/desktop/src/components/layout/ConversationContextPane.vue](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/apps/desktop/src/components/layout/ConversationContextPane.vue) is an inspector with six parallel tabs (`summary`, `memories`, `artifacts`, `resources`, `tools`, `timeline`) instead of a deliverable-first workspace.
- `packages/schema/src/workbench.ts` already contains a richer artifact concept, but it drifts from the real OpenAPI transport instead of acting as the same model.
- Knowledge, trace, and artifacts are split into separate pages and stores, so the user does not get a single clear workflow of “project context -> conversation execution -> output review/edit”.
- There is no first-class artifact version lineage tied to `conversationId`, `sessionId`, `runId`, `sourceMessageId`, and `parentArtifactId`.

## Fixed Decisions

1. Keep backend execution nouns as `Session`, `Run`, and `Subrun`. Do not rename runtime internals to chase UI copy.
2. Use `Deliverable` as the primary user-facing label for generated outputs; keep `artifact` as the technical backend object name where needed.
3. The right-side workbench has exactly three primary modes:
   - `deliverable`
   - `context`
   - `ops`
4. Conversations stay isolated. Cross-conversation reuse comes from explicit promotion into project knowledge or memory, not from replaying other chats into the prompt by default.
5. Deliverables are first-class objects with:
   - summary metadata
   - content-aware preview
   - version history
   - explicit promotion to knowledge
   - explicit fork to a new conversation
6. Artifact bodies remain on disk; SQLite stores metadata, refs, hashes, lineage, and projection state.
7. Because compatibility is not required, remove stale transport names, route state, fixtures, and store branches in-place instead of layering adapters around them.

## Global Checklist

- [ ] OpenAPI exposes canonical deliverable summary, detail, version, edit, promote, and fork contracts.
- [ ] Runtime and infra persist deliverable versions and lineage without duplicate blob storage.
- [ ] Server, platform, and desktop adapter expose typed deliverable APIs with browser/Tauri parity.
- [ ] Desktop stores model `deliverable`, `context`, and `ops` directly.
- [ ] Conversation right rail supports preview, version selection, editing, and save-as-new-version.
- [ ] Project-level deliverable list/detail and promotion flows exist.
- [ ] Trace survives as an ops surface, not as the primary output-viewing surface.
- [ ] All conflicting legacy model definitions, enums, and UI branches are removed.

## Global Exit Condition

The work is complete only when:

- a user can generate an output in a conversation, open it in the right rail, preview it, edit it, save a new version, promote it to knowledge, and optionally fork it into a new conversation;
- the same deliverable is explainable from `projectId`, `conversationId`, `sessionId`, `runId`, message linkage, and version lineage;
- there are no compatibility-only transport aliases or store branches left behind;
- the focused Rust and desktop test suites pass after the cleanup.

## Delivery Order

- Task 1 to Task 3 are the contract and substrate foundation. Do not start desktop UX work before they are green locally.
- Task 4 and Task 5 reshape the conversation workbench.
- Task 6 turns deliverables into a project-level surface.
- Task 7 is mandatory cleanup, not optional polish.

## Task 1: Replace thin artifact transport with deliverable-first contracts

**Files:**
- Modify: `contracts/openapi/src/components/schemas/projects.yaml`
- Modify: `contracts/openapi/src/components/schemas/runtime.yaml`
- Modify: `contracts/openapi/src/paths/projects.yaml`
- Modify: `contracts/openapi/src/paths/runtime.yaml`
- Modify: `packages/schema/src/artifact.ts`
- Modify: `packages/schema/src/knowledge.ts`
- Modify: `packages/schema/src/workbench.ts`
- Modify: `apps/desktop/test/openapi-bundler.test.ts`
- Modify: `apps/desktop/test/openapi-transport.test.ts`
- Modify: `apps/desktop/test/tauri-client-workspace.test.ts`
- Generated: `contracts/openapi/octopus.openapi.yaml`
- Generated: `packages/schema/src/generated.ts`

**Checklist:**
- [ ] Define canonical deliverable transport types: summary, detail, version summary, version content, create-version input, promote input, fork input.
- [ ] Add project-scoped and artifact-scoped operations to list deliverables, fetch detail, fetch versions, fetch version content, save a new version, promote to knowledge, and fork to a conversation.
- [ ] Extend runtime message or session detail transport so the active conversation can resolve artifact version refs without view-local guessing.
- [ ] Reconcile `packages/schema/src/workbench.ts` with the generated transport instead of keeping a second artifact model.
- [ ] Remove thin transport naming that only preserves legacy ambiguity.

**Step 1: Write the failing transport tests**

Run or update:

```bash
pnpm -C apps/desktop exec vitest run \
  test/openapi-bundler.test.ts \
  test/openapi-transport.test.ts \
  test/tauri-client-workspace.test.ts
```

Expected: FAIL because the current contract only exposes artifact summary metadata and has no detail/version/action API.

**Step 2: Update the OpenAPI source of truth**

- Hand-edit only `contracts/openapi/src/**`.
- Keep schemas in `components/schemas/*.yaml` and path operations in `paths/*.yaml`.
- Do not place large inline response bodies in path files.

**Step 3: Regenerate bundled and generated transport**

Run:

```bash
pnpm openapi:bundle
pnpm schema:generate
pnpm schema:check
```

Expected: PASS with updated bundled OpenAPI and generated schema output.

**Step 4: Reconcile feature-based schema exports**

- Make `packages/schema/src/artifact.ts` and `packages/schema/src/knowledge.ts` thin feature-based exports over generated types.
- Delete or rewrite conflicting handwritten interfaces in `packages/schema/src/workbench.ts` if they duplicate the new canonical model.

**Step 5: Re-run the focused transport tests**

Run:

```bash
pnpm -C apps/desktop exec vitest run \
  test/openapi-bundler.test.ts \
  test/openapi-transport.test.ts \
  test/tauri-client-workspace.test.ts
```

Expected: PASS.

**Step 6: Commit**

```bash
git add contracts/openapi/src packages/schema/src apps/desktop/test
git commit -m "refactor: replace thin artifact transport with deliverable contracts"
```

**Exit criteria:**

- Deliverable detail, version, and action contracts are represented in OpenAPI and generated schema.
- There is one canonical transport model, not a summary-only model plus handwritten shadow types.
- No compatibility-only alias fields were added.

## Task 2: Persist deliverable versions, lineage, and promotion state in infra and runtime

**Files:**
- Modify: `crates/octopus-core/src/lib.rs`
- Modify: `crates/octopus-core/src/asset_records.rs`
- Modify: `crates/octopus-infra/src/infra_state.rs`
- Modify: `crates/octopus-infra/src/artifacts_inbox_knowledge.rs`
- Modify: `crates/octopus-infra/src/projects_teams.rs`
- Modify: `crates/octopus-runtime-adapter/src/persistence.rs`
- Modify: `crates/octopus-runtime-adapter/src/session_service.rs`
- Modify: `crates/octopus-runtime-adapter/src/execution_events.rs`
- Modify: `crates/octopus-runtime-adapter/src/memory_runtime.rs`
- Modify: `crates/octopus-runtime-adapter/src/adapter_tests.rs`

**Checklist:**
- [ ] Add canonical persistence records for deliverable summary and version lineage.
- [ ] Persist artifact bodies only on disk under `data/artifacts`.
- [ ] Persist metadata in SQLite: `projectId`, `conversationId`, `sessionId`, `runId`, `sourceMessageId`, `parentArtifactId`, `contentHash`, `byteSize`, `contentType`, `latestVersion`, `promotionState`.
- [ ] Make deliverable detail recoverable from SQLite projections plus disk-backed content only.
- [ ] Record version lineage and promotion provenance in runtime events or projection refs where needed.
- [ ] Remove any artifact fallback path that depends on ad hoc JSON blobs instead of the canonical store.

**Step 1: Write the failing persistence tests**

Add or extend `crates/octopus-runtime-adapter/src/adapter_tests.rs` to cover:

- creating a deliverable with version `1`;
- saving version `2` without overwriting `1`;
- loading detail plus version history after reload;
- promoting a deliverable into knowledge while preserving lineage.

Run:

```bash
cargo test -p octopus-runtime-adapter artifact -- --nocapture
```

Expected: FAIL because current persistence does not model version history and promotion lineage as first-class state.

**Step 2: Implement SQLite and disk storage changes**

- Add or revise tables for deliverable summaries and versions in `crates/octopus-infra/src/infra_state.rs`.
- Keep migration logic explicit and clean; do not leave deprecated columns or temporary write paths if they are no longer needed.
- Update core Rust structs in `crates/octopus-core/src/lib.rs` and `crates/octopus-core/src/asset_records.rs`.

**Step 3: Wire runtime persistence and projection**

- Update runtime persistence helpers in `crates/octopus-runtime-adapter/src/persistence.rs`.
- Project deliverable refs into session and run detail in `crates/octopus-runtime-adapter/src/session_service.rs`.
- Record version and promotion events in `crates/octopus-runtime-adapter/src/execution_events.rs`.

**Step 4: Re-run focused Rust tests**

Run:

```bash
cargo test -p octopus-runtime-adapter
cargo test -p octopus-infra
```

Expected: PASS.

**Step 5: Commit**

```bash
git add crates/octopus-core/src crates/octopus-infra/src crates/octopus-runtime-adapter/src
git commit -m "refactor: persist deliverable versions and lineage"
```

**Exit criteria:**

- Deliverable detail and version history survive reload and restart.
- Bodies are stored once on disk, not duplicated in SQLite rows.
- Promotion and lineage can be explained from canonical metadata, not inferred from UI-only state.

## Task 3: Expose deliverable APIs through platform, server, and desktop adapters

**Files:**
- Modify: `crates/octopus-platform/src/artifact.rs`
- Modify: `crates/octopus-platform/src/workspace.rs`
- Modify: `crates/octopus-platform/src/runtime.rs`
- Modify: `crates/octopus-server/src/workspace_runtime.rs`
- Modify: `apps/desktop/src/tauri/workspace_api.ts`
- Modify: `apps/desktop/src/tauri/runtime_api.ts`
- Modify: `apps/desktop/src/tauri/workspace-client.ts`
- Modify: `apps/desktop/test/tauri-client-runtime.test.ts`
- Modify: `apps/desktop/test/tauri-client-workspace.test.ts`
- Modify: `apps/desktop/test/openapi-transport.test.ts`

**Checklist:**
- [ ] Add typed adapter methods for project deliverable list/detail/version/action operations.
- [ ] Keep browser-host and Tauri-host contract shape identical.
- [ ] Ensure runtime event transport can refresh active deliverable state without view-specific polling hacks.
- [ ] Route all desktop API usage through the existing adapter layer.
- [ ] Delete old adapter methods or response shaping that only served the thin artifact summary model.

**Step 1: Write the failing adapter tests**

Run:

```bash
pnpm -C apps/desktop exec vitest run \
  test/tauri-client-workspace.test.ts \
  test/tauri-client-runtime.test.ts \
  test/openapi-transport.test.ts
```

Expected: FAIL because `workspaceClient` and `runtime` adapters do not yet expose typed deliverable detail/version/edit/promotion flows.

**Step 2: Update platform traits and server handlers**

- Add or revise deliverable APIs in `crates/octopus-platform/src/artifact.rs`, `crates/octopus-platform/src/workspace.rs`, and `crates/octopus-platform/src/runtime.rs`.
- Implement the new HTTP handlers in `crates/octopus-server/src/workspace_runtime.rs`.
- Remove handler branches that preserve the old summary-only transport if they conflict with the new model.

**Step 3: Update desktop adapter surfaces**

- Extend `apps/desktop/src/tauri/workspace_api.ts`, `apps/desktop/src/tauri/runtime_api.ts`, and `apps/desktop/src/tauri/workspace-client.ts`.
- Keep all deliverable calls behind the adapter boundary; no direct `fetch` from stores or views.

**Step 4: Re-run adapter and transport tests**

Run:

```bash
pnpm -C apps/desktop exec vitest run \
  test/tauri-client-workspace.test.ts \
  test/tauri-client-runtime.test.ts \
  test/openapi-transport.test.ts
```

Expected: PASS.

**Step 5: Commit**

```bash
git add crates/octopus-platform/src crates/octopus-server/src apps/desktop/src/tauri apps/desktop/test
git commit -m "refactor: expose deliverable detail and actions through adapters"
```

**Exit criteria:**

- Desktop can fetch and mutate deliverables only through typed adapters.
- Browser and Tauri hosts expose the same deliverable contract.
- No server-side fallback shaping remains for the old artifact summary path.

## Task 4: Refactor desktop state around deliverable, context, and ops

**Files:**
- Modify: `apps/desktop/src/stores/shell.ts`
- Modify: `apps/desktop/src/stores/artifact.ts`
- Modify: `apps/desktop/src/stores/runtime.ts`
- Modify: `apps/desktop/src/stores/runtime_sessions.ts`
- Modify: `apps/desktop/src/stores/knowledge.ts`
- Modify: `apps/desktop/test/shell-store.test.ts`
- Modify: `apps/desktop/test/runtime-store.test.ts`
- Modify: `apps/desktop/test/conversation-surface.test.ts`
- Modify: `apps/desktop/test/router.test.ts`

**Checklist:**
- [ ] Replace the current multi-tab inspector focus state with `deliverable | context | ops`.
- [ ] Extend the artifact store to hold detail, version list, selected version, edit draft, loading, and save state.
- [ ] Keep conversation selection isolated by `conversationId`; do not leak state across conversations.
- [ ] Move derived workbench selection logic into stores instead of duplicating it inside views.
- [ ] Remove obsolete store branches for `summary`, `memories`, `resources`, `tools`, and `timeline` as top-level pane identities.
- [ ] State shape and defaults match the companion design spec, especially `Deliverable` as the primary selected work surface after output generation.

**Step 1: Write the failing store tests**

Run:

```bash
pnpm -C apps/desktop exec vitest run \
  test/shell-store.test.ts \
  test/runtime-store.test.ts \
  test/conversation-surface.test.ts \
  test/router.test.ts
```

Expected: FAIL because current stores and route state still model the old inspector tabs and summary-only artifact state.

**Step 2: Refactor shell and artifact state**

- Update [apps/desktop/src/stores/shell.ts](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/apps/desktop/src/stores/shell.ts) to store the new workbench mode and active deliverable version state.
- Extend [apps/desktop/src/stores/artifact.ts](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/apps/desktop/src/stores/artifact.ts) with detail/version/edit actions.

**Step 3: Reconcile runtime and knowledge store interactions**

- Update [apps/desktop/src/stores/runtime.ts](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/apps/desktop/src/stores/runtime.ts) and [apps/desktop/src/stores/runtime_sessions.ts](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/apps/desktop/src/stores/runtime_sessions.ts) so artifact selection hydrates detail and version state cleanly.
- Keep [apps/desktop/src/stores/knowledge.ts](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/apps/desktop/src/stores/knowledge.ts) focused on promoted knowledge, not generic artifact browsing.

**Step 4: Re-run store tests**

Run:

```bash
pnpm -C apps/desktop exec vitest run \
  test/shell-store.test.ts \
  test/runtime-store.test.ts \
  test/conversation-surface.test.ts \
  test/router.test.ts
```

Expected: PASS.

**Step 5: Commit**

```bash
git add apps/desktop/src/stores apps/desktop/test
git commit -m "refactor: align desktop state with deliverable workbench"
```

**Exit criteria:**

- Stores directly represent `deliverable`, `context`, and `ops`.
- Active deliverable detail and version state are store-driven, not assembled ad hoc in views.
- No stale top-level pane identity remains in route or shell state.

## Task 5: Rebuild the conversation right rail into a deliverable-first workbench

**Files:**
- Create: `apps/desktop/src/components/conversation/ArtifactPreviewPanel.vue`
- Create: `apps/desktop/src/components/conversation/ArtifactVersionList.vue`
- Modify: `apps/desktop/src/components/conversation/ConversationMessageBubble.vue`
- Modify: `apps/desktop/src/components/layout/ConversationContextPane.vue`
- Modify: `apps/desktop/src/views/project/ConversationView.vue`
- Modify: `apps/desktop/src/locales/en-US.json`
- Modify: `apps/desktop/src/locales/zh-CN.json`
- Modify: `apps/desktop/test/conversation-surface.test.ts`
- Modify: `apps/desktop/test/trace-view.test.ts`
- Modify: `apps/desktop/test/knowledge-view.test.ts`

**Checklist:**
- [ ] `Deliverable` tab shows a real preview, not metadata only.
- [ ] Preview mode is selected by content type or preview kind: markdown, plain text, JSON/code, image, file fallback.
- [ ] Version list is visible and switching versions updates the preview.
- [ ] Editable deliverables allow inline editing and save-as-new-version.
- [ ] `Context` tab holds linked resources, selected memory, and promotion context.
- [ ] `Ops` tab holds trace, approvals, worker status, background run state, and tool history.
- [ ] The UI uses the existing workbench language from `docs/design/DESIGN.md`.
- [ ] The UI follows `docs/plans/chat/2026-04-16-claude-inspired-project-conversation-deliverable-design.md` for hierarchy, empty states, right-rail mode emphasis, and inline editing behavior.

**Step 1: Write the failing component and view tests**

Run:

```bash
pnpm -C apps/desktop exec vitest run \
  test/conversation-surface.test.ts \
  test/trace-view.test.ts \
  test/knowledge-view.test.ts
```

Expected: FAIL because the current right pane is still a generic inspector with metadata-only artifact handling.

**Step 2: Build preview and version components**

- Create `ArtifactPreviewPanel.vue` for content-aware preview and editing.
- Create `ArtifactVersionList.vue` for version selection and lineage display.
- Use shared `@octopus/ui` primitives only; do not introduce ad hoc UI libraries.

**Step 3: Rebuild the pane structure**

- Update `ConversationContextPane.vue` to use the three new workbench modes.
- Update `ConversationView.vue` and `ConversationMessageBubble.vue` so selecting an artifact opens the deliverable tab and hydrates the selected version cleanly.

**Step 4: Re-run the focused UI tests**

Run:

```bash
pnpm -C apps/desktop exec vitest run \
  test/conversation-surface.test.ts \
  test/trace-view.test.ts \
  test/knowledge-view.test.ts
```

Expected: PASS.

**Step 5: Commit**

```bash
git add apps/desktop/src/components apps/desktop/src/views apps/desktop/src/locales apps/desktop/test
git commit -m "refactor: rebuild conversation rail around deliverables"
```

**Exit criteria:**

- A user can open a deliverable from a message and inspect it in a real preview surface.
- Version switching works inside the conversation rail.
- Trace and approvals still exist, but no longer dominate the primary output-review flow.

## Task 6: Add project-level deliverable surface and explicit promote or fork flows

**Files:**
- Create: `apps/desktop/src/views/project/ProjectDeliverablesView.vue`
- Modify: `apps/desktop/src/router/index.ts`
- Modify: `apps/desktop/src/navigation/menuRegistry.ts`
- Modify: `apps/desktop/src/components/layout/WorkbenchSearchOverlay.vue`
- Modify: `apps/desktop/src/views/project/ProjectDashboardView.vue`
- Modify: `apps/desktop/src/views/project/ProjectKnowledgeView.vue`
- Modify: `apps/desktop/src/stores/artifact.ts`
- Modify: `apps/desktop/src/locales/en-US.json`
- Modify: `apps/desktop/src/locales/zh-CN.json`
- Modify: `apps/desktop/test/router.test.ts`
- Modify: `apps/desktop/test/search-overlay.test.ts`
- Modify: `apps/desktop/test/dashboard-overview.test.ts`
- Modify: `apps/desktop/test/knowledge-view.test.ts`

**Checklist:**
- [ ] Add a project-level deliverable list/detail page.
- [ ] Add explicit `Promote to Knowledge` action with visible state.
- [ ] Add explicit `Fork to Conversation` action that creates or redirects to a new conversation seeded from the selected deliverable.
- [ ] Add dashboard and search entry points for deliverables.
- [ ] Keep `Trace` as an advanced ops route, not the only place to inspect execution results.
- [ ] The page follows the companion design spec's list/detail and Notion-style workbench rules instead of introducing a separate visual dialect.

**Step 1: Write the failing navigation and search tests**

Run:

```bash
pnpm -C apps/desktop exec vitest run \
  test/router.test.ts \
  test/search-overlay.test.ts \
  test/dashboard-overview.test.ts \
  test/knowledge-view.test.ts
```

Expected: FAIL because the project surface still centers dashboard, knowledge, resources, and trace without a first-class deliverable page and action flow.

**Step 2: Add the project deliverables route and page**

- Add the route in `apps/desktop/src/router/index.ts`.
- Update search and dashboard entry points.
- Keep route naming and menu placement consistent with the workbench shell.

**Step 3: Add promote and fork UX**

- Wire the deliverable store actions into both conversation and project deliverable surfaces.
- Promotion should produce project knowledge state, not a duplicate shadow list.
- Fork should create a clean conversation path, not mutate the current conversation in place.

**Step 4: Re-run the focused route and surface tests**

Run:

```bash
pnpm -C apps/desktop exec vitest run \
  test/router.test.ts \
  test/search-overlay.test.ts \
  test/dashboard-overview.test.ts \
  test/knowledge-view.test.ts
```

Expected: PASS.

**Step 5: Commit**

```bash
git add apps/desktop/src apps/desktop/test
git commit -m "feat: add project deliverable surface and promote or fork flows"
```

**Exit criteria:**

- Deliverables are first-class at the project level, not only inside one conversation pane.
- Promotion and fork are explicit user actions with clear routes and resulting state.
- Knowledge browsing and deliverable browsing have distinct responsibilities.

## Task 7: Delete obsolete model drift, compatibility code, and stale tests

**Files:**
- Modify: `packages/schema/src/index.ts`
- Modify: `packages/schema/src/workbench.ts`
- Modify: `packages/schema/src/shell.ts`
- Modify: `apps/desktop/src/components/layout/ConversationContextPane.vue`
- Modify: `apps/desktop/src/stores/shell.ts`
- Modify: `apps/desktop/src/stores/artifact.ts`
- Modify: `apps/desktop/src/stores/runtime.ts`
- Modify: `apps/desktop/test/*` (only the files that still encode removed behavior)

**Checklist:**
- [ ] Remove stale enum values, route query handling, and store branches from the old inspector model.
- [ ] Remove conflicting handwritten artifact interfaces if the generated transport now owns the model.
- [ ] Delete unused test fixtures and assertions that preserve summary-only behavior.
- [ ] Remove dead UI copy for obsolete tabs and routes.
- [ ] Ensure no `TODO`, `legacy`, or compatibility comments remain in the touched paths unless they describe a real future phase.

**Step 1: Run targeted searches for stale behavior**

Run:

```bash
rg -n "summary'|memories'|resources'|tools'|timeline'|ArtifactRecord|legacy artifact|selectedArtifactId" \
  apps/desktop/src packages/schema/src apps/desktop/test
```

Expected: identify stale branches that should be deleted or rewritten.

**Step 2: Delete or rewrite obsolete code paths**

- Remove old pane identity handling.
- Remove duplicated artifact models and temporary transport glue.
- Rewrite any tests that still encode the previous inspector mental model.

**Step 3: Run the focused desktop suite**

Run:

```bash
pnpm -C apps/desktop exec vitest run \
  test/shell-store.test.ts \
  test/runtime-store.test.ts \
  test/conversation-surface.test.ts \
  test/router.test.ts \
  test/search-overlay.test.ts \
  test/knowledge-view.test.ts \
  test/trace-view.test.ts
```

Expected: PASS.

**Step 4: Commit**

```bash
git add packages/schema/src apps/desktop/src apps/desktop/test
git commit -m "refactor: remove obsolete chat inspector and artifact drift"
```

**Exit criteria:**

- No compatibility-only code path remains in the touched transport, store, or UI files.
- There is one clear model for deliverables across transport, runtime, and desktop.
- Search results for the removed pane identities only match intentional migration history in docs or unrelated fixtures.

## Final Verification

Run:

```bash
pnpm openapi:bundle
pnpm schema:generate
pnpm schema:check
cargo test -p octopus-platform
cargo test -p octopus-server
cargo test -p octopus-runtime-adapter
cargo test -p octopus-infra
pnpm -C apps/desktop exec vitest run \
  test/openapi-bundler.test.ts \
  test/openapi-transport.test.ts \
  test/tauri-client-workspace.test.ts \
  test/tauri-client-runtime.test.ts \
  test/shell-store.test.ts \
  test/runtime-store.test.ts \
  test/conversation-surface.test.ts \
  test/router.test.ts \
  test/search-overlay.test.ts \
  test/dashboard-overview.test.ts \
  test/knowledge-view.test.ts \
  test/trace-view.test.ts
```

Expected:

- all commands pass;
- generated artifacts are up to date;
- no compatibility-only changes remain staged for later.

## Manual QA Checklist

- [ ] Create or open a project conversation.
- [ ] Generate an output that produces a deliverable.
- [ ] Open the deliverable from the message bubble.
- [ ] Switch versions in the right rail.
- [ ] Edit the deliverable and save a new version.
- [ ] Promote the deliverable to knowledge.
- [ ] Fork the deliverable into a new conversation.
- [ ] Open the same deliverable from the project-level deliverables page.
- [ ] Confirm the `Ops` pane still shows trace and approvals.
- [ ] Confirm another conversation in the same project does not silently inherit transcript context unless knowledge or memory was explicitly promoted.

## Notes For The Implementing Codex Agent

- Execute this plan on `main` unless the user explicitly changes branch policy.
- Make breaking changes directly. Do not preserve obsolete contracts just to avoid editing more files.
- Follow OpenAPI-first order strictly: edit `contracts/openapi/src/**`, bundle, generate schema, then update Rust and desktop code.
- Keep persistence responsibilities intact: config in `config/`, structured state in SQLite, append-only runtime events in `runtime/events/*.jsonl`, artifact bodies on disk.
- Keep desktop surfaces adapter-first and store-first. Views must not call bare `fetch`.
- For Task 4 through Task 6, treat `docs/plans/chat/2026-04-16-claude-inspired-project-conversation-deliverable-design.md` as a binding UI contract, not optional inspiration.
- If a task reveals a conflicting design that was not captured here, update this plan before continuing the implementation.
