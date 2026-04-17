# Workspace Automations Removal Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Remove the workspace automations feature from frontend, transport, backend, persistence, and test surfaces.

**Architecture:** Delete the feature end to end instead of hiding it. Keep the existing OpenAPI-first workflow: remove source contract definitions first, regenerate bundled artifacts, then remove adapter, store, UI, server, and infra code that depended on the feature.

**Tech Stack:** Vue 3 + Vite + Pinia + Vue Router + Vue I18n in `apps/desktop`, OpenAPI source in `contracts/openapi/src/**`, generated transport in `packages/schema/src/generated.ts`, handwritten schema helpers in `packages/schema/src/*`, and Rust service/server/infra code in `crates/octopus-*`.

---

## Read Before Starting

- `AGENTS.md`
- `docs/AGENTS.md`
- `docs/design/DESIGN.md`
- `docs/api-openapi-governance.md`
- `contracts/openapi/AGENTS.md`

## Task 1: Add failing removal tests

**Files:**
- Modify: `apps/desktop/test/openapi-parity-lib.test.ts`
- Modify: `apps/desktop/test/repo-governance.test.ts`
- Add: `apps/desktop/test/workspace-automations-removal.test.ts`
- Modify: `crates/octopus-infra/src/split_module_tests.rs`

**Steps:**
1. Update frontend governance/parity expectations so workspace automation paths and menu ids are expected to be absent.
2. Add a focused desktop test that asserts the menu registry and parity collectors no longer expose workspace automations.
3. Update the infra bootstrap test to expect no `automations` table after initialization.
4. Run the targeted tests and confirm they fail before production edits.

## Task 2: Remove transport and frontend integration

**Files:**
- Modify: `contracts/openapi/src/components/schemas/catalog.yaml`
- Modify: `contracts/openapi/src/paths/catalog.yaml`
- Modify: `packages/schema/src/catalog.ts`
- Modify: `packages/schema/src/shared.ts`
- Modify: `packages/schema/src/workspace-plane.ts`
- Modify: `apps/desktop/src/router/index.ts`
- Modify: `apps/desktop/src/components/layout/WorkbenchSidebar.vue`
- Modify: `apps/desktop/src/navigation/menuRegistry.ts`
- Modify: `apps/desktop/src/tauri/workspace-client.ts`
- Modify: `apps/desktop/src/tauri/workspace_api.ts`
- Modify: `apps/desktop/src/locales/zh-CN.json`
- Modify: `apps/desktop/src/locales/en-US.json`
- Delete: `apps/desktop/src/stores/automation.ts`
- Delete: `apps/desktop/src/views/workspace/AutomationsView.vue`
- Modify: `apps/desktop/test/support/workspace-fixture-state.ts`
- Modify: `apps/desktop/test/support/workspace-fixture-client.ts`

**Steps:**
1. Remove OpenAPI source definitions for the feature.
2. Remove handwritten schema aliases and shared automation-only types.
3. Remove route/menu/view/store/client wiring and fixture references.
4. Regenerate bundled OpenAPI and generated schema artifacts.

## Task 3: Remove backend and persistence support

**Files:**
- Modify: `crates/octopus-core/src/lib.rs`
- Modify: `crates/octopus-platform/src/workspace.rs`
- Modify: `crates/octopus-infra/src/lib.rs`
- Modify: `crates/octopus-infra/src/infra_state.rs`
- Modify: `crates/octopus-infra/src/projects_teams.rs`
- Modify: `crates/octopus-infra/src/access_control.rs`
- Modify: `crates/octopus-server/src/lib.rs`
- Modify: `crates/octopus-server/src/routes.rs`
- Modify: `crates/octopus-server/src/workspace_runtime.rs`
- Modify: `crates/octopus-server/src/handlers.rs`

**Steps:**
1. Remove the automation record domain type and workspace service trait methods.
2. Remove Axum routing, handlers, menu entries, and permission definitions for workspace automations.
3. Remove SQLite bootstrap, in-memory cache, CRUD helpers, and seed/test code for automations.
4. Compile and fix any remaining references.

## Task 4: Verify the removal

**Commands:**
```bash
pnpm openapi:bundle
pnpm schema:generate
pnpm schema:check
pnpm -C apps/desktop exec vitest run test/openapi-parity-lib.test.ts test/repo-governance.test.ts test/workspace-automations-removal.test.ts
cargo test -p octopus-infra split_module_tests
```

**Done when:**
- No desktop route, menu, locale section, store, or client surface remains for workspace automations.
- No OpenAPI path or transport schema remains for `/api/v1/workspace/automations`.
- No server route, permission code, menu definition, or infra persistence remains for workspace automations.
- Targeted frontend and Rust verification passes with regenerated artifacts.
