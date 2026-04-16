# Personal Pet Agent Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a single-system personal pet assistant for each user in a workspace, with owner-derived permissions, personal memory and knowledge, proactive reminder bubbles, and a Personal Center management surface.

**Architecture:** Model the pet as a `scope=personal` agent asset plus a pet-specific extension instead of introducing a second agent runtime. Keep pet identity, learning state, and reminder preferences at user scope, then let the same pet enter a project execution context when needed with a permission ceiling derived from the owner user and the target project. Reuse the existing notification and runtime memory planes instead of adding pet-only side stores.

**Tech Stack:** Vue 3, Vite, Pinia, Vue Router, Vue I18n, Tauri 2, Rust (`octopus-server`, `octopus-infra`, `octopus-runtime-adapter`), OpenAPI-first transport, generated `@octopus/schema`, SQLite projections, runtime JSONL and durable memory.

---

## Read Before Starting

- `docs/api-openapi-governance.md`
- `docs/design/DESIGN.md`
- `docs/plans/runtime/agent-runtime-rebuild-design.md`
- Root `AGENTS.md`

## Architecture Design

### 1. One Agent System

- The pet is a `personal` agent asset, not a parallel runtime stack.
- Add generic ownership metadata to agent records:
  - `owner_user_id`
  - `asset_role`, with phase-1 values `default | pet`
- Keep pet-only presentation and growth state in a dedicated pet extension/read model instead of bloating the base agent record.

### 2. Pet Identity And Lifecycle

- Every user gets exactly one stable pet per workspace.
- New users receive one random species from the existing 18-value `PetSpecies` set during user bootstrap.
- Existing users are backfilled once with a deterministic selection derived from stable identifiers so migration is idempotent and testable.
- Species assignment is stable after creation; later customization changes appearance or copy, not identity.

### 3. Scope Model

- Public scope model for user-facing assets remains:
  - `personal`
  - `project`
  - `workspace`
- Runtime memory keeps its richer internal scopes:
  - `user-private`
  - `agent-private`
  - `project-shared`
  - `workspace-shared`
  - `team-shared`

### 4. Session Model

- The pet always exists at workspace and user scope.
- Introduce two pet conversation contexts:
  - `home`: user + workspace + pet, no project required
  - `project-context`: user + workspace + pet + project
- The same pet identity is reused in both contexts.
- Project context is an execution lens, not ownership.
- `home` requires runtime transport support that does not force a `projectId`; implement this through an optional `projectId` in the runtime session contract or a dedicated home-session route, but keep one shared runtime session model.

### 5. Permission Model

- The pet never has more power than its owner user.
- Compute:

`effectivePetPermission = min(ownerWorkspaceOrProjectCeiling, petPreferenceCeiling, selectedActorEnvelope)`

- The Personal Center permission selector is a narrowing preference only.
- The server derives the true owner ceiling from the authenticated session and project governance.
- The client must not send an owner override.
- The owner ceiling must be applied inside runtime session policy compilation, not as a client-side convention.
- Reuse the existing session permission clamp path by feeding owner-derived limits into the same server-side clamp that already applies actor and runtime-config ceilings.

### 6. Learning And Growth Model

- Reuse existing runtime durable memory:
  - `user-private` for user preferences, habits, goals
  - `agent-private` for pet-specific service strategies and learned assistance patterns
  - `project-shared` only when explicitly promoted into project context
- Do not add pet-only JSON caches.
- Treat knowledge and memory differently:
  - `knowledge`: curated reference material
  - `memory`: learned durable facts, preferences, and validated outcomes
- Hermes-inspired behavior to keep:
  - separate user profile memory from pet self-memory
  - allow deeper user modeling without making every chat message permanent
  - promote successful workflows into reusable skills or procedures instead of raw conversation replay

### 7. Proactive Reminder Model

- Phase 1 allows proactive reminders and deep links only.
- Phase 1 does not allow automatic execution.
- Reuse the existing host notification plane for TTL, unread state, and route deep links.
- The pet host becomes a presentation surface over notification-derived runtime and inbox signals.
- For Phase 1, the pet bubble consumes `NotificationRecord` only.
- Runtime completion and inbox events may appear in the pet bubble only after they are materialized into `NotificationRecord` entries with `routeTo`.
- Direct inbox polling is not the proactive event source for Phase 1.
- Bubble reminders must:
  - appear near the left-bottom pet anchor
  - expire automatically after a configurable TTL
  - keep the underlying notification or inbox item after the bubble disappears
  - deep link to the target route on click
  - render from exactly one ephemeral presentation channel at a time, so the same notification is not shown simultaneously in both the global toast viewport and the pet bubble

### 8. Personal Center Model

- Personal Center is the canonical pet management surface.
- It must show:
  - identity and appearance
  - model and permission ceiling preference
  - activity and growth statistics
  - memory, knowledge, and resource counts
  - proactive reminder preferences
- Keep generic agent center clean by hiding or clearly excluding `asset_role=pet` records from default browsing.

## Functional Design

### User-Facing Behaviors

- A newly added user opens the workspace and immediately has one personal pet.
- The pet is visible from the sidebar host even when no project is selected.
- The pet can start a home conversation with no project selected.
- When the user enters a project, the pet can continue helping there, but execution is clamped to the project permission ceiling.
- When a relevant runtime completion, inbox event, or system notification arrives, the pet shows a small bubble with a label, short copy, and a clickable jump target.
- The bubble disappears after TTL, but the event remains available through the existing notification or inbox surfaces.
- The Personal Center pet tab shows the current pet status, learning metrics, reminder settings, and personal asset counts.

### Phase-1 Non-Goals

- No automatic execution started by the pet
- No multi-pet ownership
- No workspace-shared pet identity
- No pet-specific secondary persistence system
- No decorative pet-only visual system outside `docs/design/DESIGN.md`
- No pet breeding, leveling game loops, or marketplace customization

## Delivery Notes

- Follow OpenAPI-first workflow for every `/api/v1/*` change.
- Never hand-edit `contracts/openapi/octopus.openapi.yaml` or `packages/schema/src/generated.ts`.
- Prefer extending existing stores and adapters over adding parallel ones.
- Keep commits small and phase-local.

## Delivery Phases

### Phase 1: Proactive Reminder And Deep-Link MVP (Ship First)

This is the first user-visible slice and must land before any auto-execution work is considered.

- Use the existing pet host plus the existing notification plane to ship a narrow reminder loop first.
- Phase-1 scope is strictly:
  - left-bottom sidebar pet bubble
  - TTL-based auto-dismiss
  - click-through deep link via `routeTo`
  - preserve the source notification or inbox item after the bubble disappears
  - drive bubble rendering from `NotificationRecord` rather than direct inbox polling
  - ensure a reminder is rendered in exactly one transient presentation channel at a time
  - no automatic execution, no implicit approval, no background task start
- Implement `Task 5` first for this slice.
- Keep backend change for this slice minimal. Prefer reusing existing notification fields over inventing pet-only transport.
- If inbox items must become proactive in phase 1, bridge them into notifications first instead of adding a second reminder transport.
- If any phase-1 implementation shortcut conflicts with the long-term ownership model, isolate it behind store or component seams that are replaced during `Task 1` to `Task 3`.

### Phase 2: Ownership And Transport Foundation

- Execute `Task 1` and `Task 2`.
- Move pet transport and persistence from shared workspace or project semantics to current-user personal semantics.
- Backfill existing users safely before enabling broader pet management UX.
- Keep `asset_role=pet` out of generic agent list and actor-selection surfaces by default during the same phase, so pets do not leak into project actor management before Personal Center is complete.

### Phase 3: Session And Permission Unification

- Execute `Task 3`.
- Introduce `home` and `project-context` pet conversations without changing pet identity.
- Enforce owner-derived permission ceilings on the server.

### Phase 4: Personal Knowledge Plane

- Execute `Task 4`.
- Align personal knowledge with existing personal resources and durable memory responsibilities.

### Phase 5: Personal Center Dashboard And Preferences

- Execute `Task 6`.
- Make Personal Center the canonical pet management surface after the identity, permission, and reminder foundations are stable.

## Task 1: Rebuild the transport contract around a user-owned personal pet

**Files:**
- Modify: `contracts/openapi/src/components/schemas/workspace.yaml`
- Modify: `contracts/openapi/src/paths/workspace.yaml`
- Modify: `contracts/openapi/src/paths/projects.yaml`
- Modify: `packages/schema/src/shared.ts`
- Modify: `packages/schema/src/workspace-plane.ts`
- Modify: `packages/schema/src/knowledge.ts`
- Modify: `apps/desktop/test/repo-governance.test.ts`
- Modify: `apps/desktop/test/tauri-client-workspace.test.ts`
- Generated: `contracts/openapi/octopus.openapi.yaml`
- Generated: `packages/schema/src/generated.ts`

**Step 1: Write the failing test**

Add transport assertions for:

- current-user pet home snapshot semantics on `/api/v1/workspace/pet`
- current-user pet project-context semantics on `/api/v1/projects/{projectId}/pet`
- a new pet dashboard summary endpoint under workspace scope
- personal knowledge fields on `KnowledgeRecord`: `scope`, `visibility`, `ownerUserId`
- owner-aware pet binding fields and any new pet dashboard schema

**Step 2: Run test to verify it fails**

Run: `pnpm -C apps/desktop test -- test/repo-governance.test.ts test/tauri-client-workspace.test.ts`

Expected: FAIL because the current OpenAPI contract still exposes workspace or project pet snapshots without explicit personal ownership semantics and knowledge records still lack personal scope metadata.

**Step 3: Write minimal implementation**

Update the OpenAPI source of truth so that:

- pet routes are defined as current-user projections, not shared scope assets
- pet dashboard data is available through a dedicated workspace route
- knowledge transport records support personal scope and owner metadata
- generated schema aliases point to the new transport surface without creating a handwritten parallel truth source

Then run:

`pnpm openapi:bundle`

`pnpm schema:generate`

**Step 4: Run test to verify it passes**

Run: `pnpm schema:check && pnpm -C apps/desktop test -- test/repo-governance.test.ts test/tauri-client-workspace.test.ts`

Expected: PASS

**Step 5: Commit**

```bash
git add contracts/openapi/src/components/schemas/workspace.yaml contracts/openapi/src/paths/workspace.yaml contracts/openapi/src/paths/projects.yaml contracts/openapi/octopus.openapi.yaml packages/schema/src/shared.ts packages/schema/src/workspace-plane.ts packages/schema/src/knowledge.ts packages/schema/src/generated.ts apps/desktop/test/repo-governance.test.ts apps/desktop/test/tauri-client-workspace.test.ts
git commit -m "feat: define personal pet and knowledge transport contracts"
```

## Task 2: Persist the pet as a personal agent asset and bootstrap one pet per user

**Files:**
- Modify: `crates/octopus-core/src/asset_records.rs`
- Modify: `crates/octopus-core/src/lib.rs`
- Modify: `crates/octopus-platform/src/workspace.rs`
- Modify: `crates/octopus-infra/src/agent_assets.rs`
- Modify: `crates/octopus-infra/src/auth_users.rs`
- Modify: `crates/octopus-infra/src/infra_state.rs`
- Modify: `crates/octopus-infra/src/projects_teams.rs`
- Modify: `crates/octopus-runtime-adapter/src/actor_manifest.rs`
- Modify: `crates/octopus-server/src/workspace_runtime.rs`
- Test: `crates/octopus-infra/src/agent_assets.rs`
- Test: `crates/octopus-runtime-adapter/src/adapter_tests.rs`
- Test: `crates/octopus-server/src/workspace_runtime.rs`

**Step 1: Write the failing test**

Add Rust coverage for:

- `AgentRecord` persistence with `scope=personal`, `owner_user_id`, and `asset_role=pet`
- pet extension persistence with species, display defaults, and owner linkage
- database-level uniqueness for one pet per `(workspace_id, owner_user_id)`
- one-time pet bootstrap for newly created users
- deterministic backfill for existing users without a pet
- owner-aware pet snapshot and binding lookup keyed by current user
- generic agent listing and project actor queries excluding `asset_role=pet` by default

**Step 2: Run test to verify it fails**

Run: `cargo test -p octopus-infra --locked && cargo test -p octopus-runtime-adapter --locked && cargo test -p octopus-server --locked`

Expected: FAIL because agent records do not yet carry owner metadata, current pet tables are still workspace or project keyed, generic agent loaders do not understand pet ownership metadata, and no user bootstrap path creates a pet.

**Step 3: Write minimal implementation**

Implement the backend model:

- add generic agent ownership metadata
- add a pet extension table or equivalent normalized persistence for pet-only fields
- enforce the single-pet invariant with a database-level unique index or equivalent uniqueness constraint keyed by workspace and owner user
- replace shared pet binding and presence keys with owner-aware keys
- route all user bootstrap and migration backfill through one idempotent `ensure_personal_pet_for_user` path
- bootstrap one pet per user from the 18-species registry
- keep `PetProfile` as a projection derived from agent plus pet extension, not a second source of truth
- add pet dashboard summary projection fields needed by Personal Center
- update runtime-adapter SQL readers, writers, and fixture inserts for the new agent ownership columns
- exclude `asset_role=pet` from generic agent list/read models by default; fetch the pet through dedicated current-user pet projections instead of the general agent catalog

**Step 4: Run test to verify it passes**

Run: `cargo test -p octopus-infra --locked && cargo test -p octopus-runtime-adapter --locked && cargo test -p octopus-server --locked`

Expected: PASS

**Step 5: Commit**

```bash
git add crates/octopus-core/src/asset_records.rs crates/octopus-core/src/lib.rs crates/octopus-platform/src/workspace.rs crates/octopus-infra/src/agent_assets.rs crates/octopus-infra/src/auth_users.rs crates/octopus-infra/src/infra_state.rs crates/octopus-infra/src/projects_teams.rs crates/octopus-runtime-adapter/src/actor_manifest.rs crates/octopus-runtime-adapter/src/adapter_tests.rs crates/octopus-server/src/workspace_runtime.rs
git commit -m "feat: persist personal pet agents per user"
```

## Task 3: Implement home and project-context pet sessions with owner-derived permission ceilings

**Files:**
- Modify: `contracts/openapi/src/components/schemas/runtime.yaml`
- Modify: `contracts/openapi/src/paths/runtime.yaml`
- Generated: `contracts/openapi/octopus.openapi.yaml`
- Generated: `packages/schema/src/generated.ts`
- Modify: `crates/octopus-core/src/runtime_policy.rs`
- Modify: `crates/octopus-runtime-adapter/src/session_policy.rs`
- Modify: `crates/octopus-runtime-adapter/src/session_service.rs`
- Modify: `crates/octopus-server/src/workspace_runtime.rs`
- Modify: `apps/desktop/src/stores/pet.ts`
- Modify: `apps/desktop/src/stores/runtime_sessions.ts`
- Modify: `apps/desktop/src/tauri/runtime_api.ts`
- Modify: `apps/desktop/test/runtime-store.test.ts`
- Modify: `apps/desktop/test/tauri-client-workspace.test.ts`

**Step 1: Write the failing test**

Add coverage for:

- runtime transport support for a pet home session that does not require a project-bound `CreateRuntimeSessionInput`
- opening a pet home conversation without a selected project
- entering a project-context pet conversation without changing pet identity
- permission clamping to the minimum of owner ceiling, pet preference, and selected actor envelope
- denying project-only capabilities when the user is outside the project or lacks module access

**Step 2: Run test to verify it fails**

Run: `pnpm -C apps/desktop test -- test/runtime-store.test.ts test/tauri-client-workspace.test.ts && cargo test -p octopus-runtime-adapter --locked`

Expected: FAIL because the current runtime contract still requires `projectId`, the current client requires `projectId` for `ensureConversation`, and permission mode still comes directly from pet preference instead of owner-derived ceilings.

**Step 3: Write minimal implementation**

Implement:

- OpenAPI-first runtime contract changes for home pet sessions before adapter or client code
- home pet session creation with no project dependency
- project-context bindings that reuse the same pet identity
- server-side owner ceiling derivation from authenticated session and project governance
- feed the owner ceiling into the existing server-side session permission clamp path
- client-side pet preference as a narrowing input only
- clear separation between home and project-context conversation bindings

**Step 4: Run test to verify it passes**

Run: `pnpm openapi:bundle && pnpm schema:generate && pnpm schema:check && pnpm -C apps/desktop test -- test/runtime-store.test.ts test/tauri-client-workspace.test.ts && cargo test -p octopus-runtime-adapter --locked`

Expected: PASS

**Step 5: Commit**

```bash
git add contracts/openapi/src/components/schemas/runtime.yaml contracts/openapi/src/paths/runtime.yaml contracts/openapi/octopus.openapi.yaml packages/schema/src/generated.ts crates/octopus-core/src/runtime_policy.rs crates/octopus-runtime-adapter/src/session_policy.rs crates/octopus-runtime-adapter/src/session_service.rs crates/octopus-server/src/workspace_runtime.rs apps/desktop/src/stores/pet.ts apps/desktop/src/stores/runtime_sessions.ts apps/desktop/src/tauri/runtime_api.ts apps/desktop/test/runtime-store.test.ts apps/desktop/test/tauri-client-workspace.test.ts
git commit -m "feat: add owner-derived pet home and project contexts"
```

## Task 4: Extend knowledge into a full personal, project, and workspace plane

**Files:**
- Modify: `crates/octopus-core/src/lib.rs`
- Modify: `crates/octopus-platform/src/workspace.rs`
- Modify: `crates/octopus-infra/src/infra_state.rs`
- Modify: `crates/octopus-infra/src/artifacts_inbox_knowledge.rs`
- Modify: `crates/octopus-server/src/workspace_runtime.rs`
- Modify: `apps/desktop/src/stores/knowledge.ts`
- Modify: `apps/desktop/src/views/workspace/WorkspaceKnowledgeView.vue`
- Modify: `apps/desktop/src/views/project/ProjectKnowledgeView.vue`
- Modify: `apps/desktop/test/knowledge-view.test.ts`

**Step 1: Write the failing test**

Add assertions for:

- personal knowledge records visible only to the owner user
- workspace knowledge view showing a personal section analogous to resources
- project knowledge view supporting personal, project, and workspace filtering
- authorization rejecting non-owner access to personal knowledge

**Step 2: Run test to verify it fails**

Run: `pnpm -C apps/desktop test -- test/knowledge-view.test.ts && cargo test -p octopus-server --locked`

Expected: FAIL because knowledge records currently have no `scope`, `visibility`, or `owner_user_id`, and the UI store only loads workspace and project partitions.

**Step 3: Write minimal implementation**

Implement:

- a scope and visibility model for knowledge aligned with resources
- owner-based authorization for personal knowledge
- workspace and project knowledge UI partitions and filters
- store support for personal knowledge projections without introducing a duplicate cache model

**Step 4: Run test to verify it passes**

Run: `pnpm -C apps/desktop test -- test/knowledge-view.test.ts && cargo test -p octopus-server --locked`

Expected: PASS

**Step 5: Commit**

```bash
git add crates/octopus-core/src/lib.rs crates/octopus-platform/src/workspace.rs crates/octopus-infra/src/infra_state.rs crates/octopus-infra/src/artifacts_inbox_knowledge.rs crates/octopus-server/src/workspace_runtime.rs apps/desktop/src/stores/knowledge.ts apps/desktop/src/views/workspace/WorkspaceKnowledgeView.vue apps/desktop/src/views/project/ProjectKnowledgeView.vue apps/desktop/test/knowledge-view.test.ts
git commit -m "feat: add personal knowledge scope for pet owners"
```

## Task 5: Add proactive reminder bubbles and deep links on top of the existing notification plane

**Execution Priority:** Ship this task first as the phase-1 slice, then rebind it onto the personal pet model introduced by later tasks.

**Files:**
- Modify: `apps/desktop/src/App.vue`
- Create: `apps/desktop/src/components/pet/DesktopPetBubble.vue`
- Modify: `apps/desktop/src/components/pet/DesktopPetHost.vue`
- Modify: `apps/desktop/src/components/pet/DesktopPetChat.vue`
- Modify: `apps/desktop/src/components/layout/WorkbenchSidebar.vue`
- Modify: `apps/desktop/src/stores/notifications.ts`
- Modify: `apps/desktop/src/stores/pet.ts`
- Modify: `apps/desktop/src/locales/zh-CN.json`
- Modify: `apps/desktop/src/locales/en-US.json`
- Create: `apps/desktop/test/desktop-pet-host.test.ts`

**Step 1: Write the failing test**

Add component coverage for:

- a bubble anchored near the sidebar pet host
- notification-driven reminder selection from existing `NotificationRecord`
- TTL-based auto-dismiss
- click-through routing to `routeTo`
- bubble dismissal that preserves the underlying notification or inbox item
- no duplicate transient rendering of the same reminder in both the pet bubble and the global toast viewport
- no automatic execution side effects

**Step 2: Run test to verify it fails**

Run: `pnpm -C apps/desktop test -- test/desktop-pet-host.test.ts`

Expected: FAIL because the current pet host only toggles chat, does not render a dedicated reminder bubble, and the current notification flow does not reserve a reminder for the pet bubble instead of the global toast viewport.

**Step 3: Write minimal implementation**

Implement:

- a pet bubble component that follows the design system rather than inventing pet-only chrome
- selection logic that reuses `NotificationRecord.routeTo` and `toastVisibleUntil`
- derive bubble candidates from the notification plane only; runtime completion or inbox reminders must first exist as notifications
- a default bubble TTL from runtime config with a stable fallback
- current-workspace and current-user reminder filtering so the pet does not surface stale reminders from another workspace context
- a single transient-channel rule so the same notification ID is shown either in the pet bubble or in the default toast viewport, not both
- click behavior that routes first and only dismisses the bubble presentation, not the source record

**Step 4: Run test to verify it passes**

Run: `pnpm -C apps/desktop test -- test/desktop-pet-host.test.ts`

Expected: PASS

**Step 5: Commit**

```bash
git add apps/desktop/src/App.vue apps/desktop/src/components/pet/DesktopPetBubble.vue apps/desktop/src/components/pet/DesktopPetHost.vue apps/desktop/src/components/pet/DesktopPetChat.vue apps/desktop/src/components/layout/WorkbenchSidebar.vue apps/desktop/src/stores/notifications.ts apps/desktop/src/stores/pet.ts apps/desktop/src/locales/zh-CN.json apps/desktop/src/locales/en-US.json apps/desktop/test/desktop-pet-host.test.ts
git commit -m "feat: add proactive pet reminder bubbles"
```

## Task 6: Turn Personal Center into the canonical pet management dashboard

**Files:**
- Create: `apps/desktop/src/views/workspace/personal-center/PetStatsPanel.vue`
- Create: `apps/desktop/src/views/workspace/personal-center/PetPreferencesPanel.vue`
- Modify: `apps/desktop/src/views/workspace/PersonalCenterView.vue`
- Modify: `apps/desktop/src/views/workspace/personal-center/PersonalCenterPetView.vue`
- Modify: `apps/desktop/src/stores/pet.ts`
- Modify: `apps/desktop/src/stores/user-profile.ts`
- Modify: `apps/desktop/src/views/agents/useAgentCenter.ts`
- Modify: `apps/desktop/src/locales/zh-CN.json`
- Modify: `apps/desktop/src/locales/en-US.json`
- Modify: `crates/octopus-platform/src/workspace.rs`
- Modify: `crates/octopus-server/src/workspace_runtime.rs`
- Create: `apps/desktop/test/personal-center-pet-view.test.ts`

**Step 1: Write the failing test**

Add coverage for:

- pet identity and species summary
- model and permission preference display
- growth and activity metrics
- personal memory, knowledge, and resource counts
- reminder preference controls
- hiding pet agents from the default generic agent center list

**Step 2: Run test to verify it fails**

Run: `pnpm -C apps/desktop test -- test/personal-center-pet-view.test.ts`

Expected: FAIL because the current Personal Center pet tab only edits a few runtime config fields and exposes no metrics or reminder preferences.

**Step 3: Write minimal implementation**

Implement:

- a compact pet dashboard in Personal Center
- reminder TTL and quiet-hours preferences under user runtime config
- pet summary metrics from the new backend projection
- generic agent center filtering so personal pets do not clutter normal agent browsing

**Step 4: Run test to verify it passes**

Run: `pnpm -C apps/desktop test -- test/personal-center-pet-view.test.ts`

Expected: PASS

**Step 5: Commit**

```bash
git add apps/desktop/src/views/workspace/personal-center/PetStatsPanel.vue apps/desktop/src/views/workspace/personal-center/PetPreferencesPanel.vue apps/desktop/src/views/workspace/PersonalCenterView.vue apps/desktop/src/views/workspace/personal-center/PersonalCenterPetView.vue apps/desktop/src/stores/pet.ts apps/desktop/src/stores/user-profile.ts apps/desktop/src/views/agents/useAgentCenter.ts apps/desktop/src/locales/zh-CN.json apps/desktop/src/locales/en-US.json crates/octopus-platform/src/workspace.rs crates/octopus-server/src/workspace_runtime.rs apps/desktop/test/personal-center-pet-view.test.ts
git commit -m "feat: add personal center pet dashboard"
```

## Phase 1 Slice Checklist

- [ ] Bubble renders next to the left-bottom sidebar pet anchor
- [ ] Bubble payload comes from the existing notification plane
- [ ] Inbox or runtime events only reach the bubble after being bridged into `NotificationRecord`
- [ ] Bubble auto-dismisses after TTL
- [ ] Bubble click navigates through the existing router deep link target
- [ ] Bubble dismissal or expiry does not delete or mark the source event incorrectly
- [ ] The same reminder is not rendered simultaneously in both the pet bubble and the global toast viewport
- [ ] No bubble path starts tool execution, workflow execution, or approval submission
- [ ] Desktop component tests cover TTL and route behavior

## Phase 1 Exit Criteria

- A user can receive a pet reminder bubble without opening a project conversation first.
- Clicking the bubble always deep links to the intended workspace or project destination.
- Bubble expiry only removes the transient pet presentation, not the underlying notification or inbox record.
- The bubble is driven by notification records and does not depend on direct inbox polling.
- The same reminder is not displayed at the same time in both the pet bubble and the global toast viewport.
- No phase-1 pet reminder path performs automatic execution.
- The reminder bubble follows `docs/design/DESIGN.md` and does not introduce pet-only chrome.
- `pnpm -C apps/desktop test -- test/desktop-pet-host.test.ts` passes.

## Cross-Phase Checklist

- [ ] OpenAPI paths and schemas describe current-user pet semantics instead of shared pet scope
- [ ] Generated `@octopus/schema` matches the new transport contracts
- [ ] Personal pet identity is persisted through agent ownership plus a pet extension
- [ ] A database-level invariant enforces at most one pet per `(workspace, user)`
- [ ] New users receive one pet at bootstrap
- [ ] Existing users are backfilled safely and idempotently
- [ ] Generic agent lists and actor selectors exclude `asset_role=pet` by default
- [ ] The pet can open a home conversation with no selected project
- [ ] Project-context pet execution clamps to real user and project permission ceilings
- [ ] Personal knowledge uses the same scope semantics as personal resources
- [ ] Reminder bubbles are deep-link only and never auto-execute work
- [ ] Reminder bubbles auto-expire and preserve underlying notification state
- [ ] Personal Center exposes pet identity, metrics, and preferences
- [ ] Generic agent center excludes or hides `asset_role=pet` by default
- [ ] Localization strings exist in both `zh-CN` and `en-US`
- [ ] No generated file was hand-edited

## Verification Matrix

- Contract and adapter parity:
  - `pnpm openapi:bundle`
  - `pnpm schema:generate`
  - `pnpm schema:check`
- Desktop verification:
  - `pnpm -C apps/desktop typecheck`
  - `pnpm -C apps/desktop test`
- Rust verification:
  - `cargo test -p octopus-infra --locked`
  - `cargo test -p octopus-server --locked`
  - `cargo test -p octopus-runtime-adapter --locked`
- Full repo gates before merge:
  - `pnpm check:desktop`
  - `pnpm check:rust`

## Exit Criteria

- Each authenticated user sees exactly one stable personal pet in a workspace.
- The one-pet-per-user invariant is enforced by persistence, not only by client convention.
- The pet remains the same identity across home and project-context conversations.
- The pet can assist from workspace scope without requiring a selected project.
- When the pet enters project context, execution is limited by the real owner user and project permissions.
- No pet-triggered feature performs automatic execution in phase 1.
- The pet can surface a bubble reminder for runtime completion, bridged inbox items, or system notifications and deep link to the correct destination.
- Bubble reminders auto-dismiss after TTL without deleting the underlying notification or inbox record.
- Personal Center shows pet identity, species, current model, permission preference, growth metrics, and personal asset counts.
- Personal resources, personal knowledge, and personal memory all line up under one consistent user-scope model.
- All commands in the verification matrix pass.

## Stop Conditions

Pause implementation and re-review the design if any of the following occurs:

- the plan starts creating a second pet-only agent runtime
- project context begins to mutate pet ownership semantics
- permission derivation depends on client-sent owner data
- reminder bubbles require a brand-new notification persistence plane
- reminder bubbles depend on direct inbox polling instead of the notification plane in phase 1
- pet records start appearing in generic agent or project actor selectors by default
- personal knowledge starts duplicating runtime memory responsibilities
- the pet UI drifts away from `docs/design/DESIGN.md`
