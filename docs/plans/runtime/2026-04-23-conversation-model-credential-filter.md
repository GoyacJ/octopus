# Conversation Runtime Model Credential Filter Plan

## Goal

Make desktop conversation and pet runtime entry points exclude configured models whose provider credentials are not runnable, so starting a turn no longer fails with `missing auth secret for provider minimax`.

## Architecture

This repair belongs in the shared desktop catalog filter layer, because the catalog snapshot already carries model execution capability plus credential health for each configured model. The fix should define a reusable runtime-ready predicate in `apps/desktop/src/stores/catalog_filters.ts`, expose runtime-only option lists, and have conversation and pet execution paths consume those lists while leaving workspace/project management views free to show non-runnable configured models.

## Scope

- In scope:
  - Runtime-ready configured-model filtering in `apps/desktop/src/stores/catalog_filters.ts`
  - Conversation and pet runtime selection updates in `apps/desktop/src/views/project/ConversationView.vue`, `apps/desktop/src/stores/pet.ts`, and pet runtime settings UI if it consumes the same execution list
  - Regression coverage in `apps/desktop/test/catalog-store.test.ts`, `apps/desktop/test/conversation-surface.test.ts`, and any directly affected pet UI test
- Out of scope:
  - Backend auth-secret lookup or provider adapter behavior
  - Reworking workspace/project model management UX
  - Changing persisted model configuration data

## Risks Or Open Questions

- Catalog credential status values are not fully normalized across sources; the predicate must tolerate values such as `configured`, `healthy`, `missing`, `unconfigured`, `error`, `missing_credentials`, and `disabled`.
- Project settings and model-management surfaces still need visibility into configured but non-runnable models, so the repair should avoid tightening `configuredModelOptions` globally unless verification shows no consumer depends on the broader list.
- If a runtime consumer outside conversation or pet still uses the old option list to start sessions, that follow-up should stop here and be handled as a separate execution batch instead of guessed silently.

## Execution Rules

- Do not start implementation until each task has exact files, acceptance, verification, and stop conditions.
- Do not collapse shared-layer work into a page-local workaround.
- Stop when contract ownership, source of truth, or verification output is unclear.
- Execute in small batches and update status in place after each batch.

## Task Ledger

### Task 1: Define runtime-ready configured model options

Status: `done`

Files:
- Create: `docs/plans/runtime/2026-04-23-conversation-model-credential-filter.md`
- Modify: `apps/desktop/src/stores/catalog_filters.ts`
- Modify: `apps/desktop/src/stores/catalog.ts`

Preconditions:
- Catalog snapshot rows already expose `enabled`, `supportsConversationExecution`, `credentialStatus`, and `credentialConfigured`.
- `configuredModelOptions` is currently used by both runtime entry points and management surfaces.

Step 1:
- Action: Document the runtime-ready filtering boundary and execution tasks in this plan.
- Done when: this plan states that runtime consumers must use a shared credential-aware predicate instead of the broad configured-model list.
- Verify: `test -f docs/plans/runtime/2026-04-23-conversation-model-credential-filter.md`
- Stop if: an existing active runtime plan already owns the same frontend credential-filter regression.

Step 2:
- Action: Add a shared predicate plus runtime-only option lists that include only configured models that are enabled, conversation-executable, and credential-runnable.
- Done when: the catalog store exposes runtime-ready configured-model options without changing the broader management-facing configured-model lists.
- Verify: `pnpm -C apps/desktop test -- --runInBand catalog-store.test.ts`
- Stop if: catalog consumers require a different source of truth than `CatalogConfiguredModelRow`.

### Task 2: Switch runtime entry points to the runtime-ready list

Status: `done`

Files:
- Modify: `apps/desktop/src/views/project/ConversationView.vue`
- Modify: `apps/desktop/src/stores/pet.ts`
- Modify: `apps/desktop/src/views/workspace/personal-center/PersonalCenterPetView.vue`

Preconditions:
- Task 1 Step 2 is implemented locally.

Step 1:
- Action: Update conversation and pet runtime selection flows to resolve defaults and availability from the runtime-ready model options.
- Done when: missing-credential models no longer appear as selectable defaults for conversation or pet runtime flows, and conversation setup falls back to the existing no-model guidance when no runnable model remains.
- Verify: `pnpm -C apps/desktop test -- --runInBand conversation-surface.test.ts personal-center-pet-view.test.ts layout-shell.test.ts`
- Stop if: a consumer needs to keep showing non-runnable models for editing while still using the same list for execution.

### Task 3: Add regression coverage for missing-credential models

Status: `done`

Files:
- Modify: `apps/desktop/test/catalog-store.test.ts`
- Modify: `apps/desktop/test/conversation-surface.test.ts`
- Modify: `apps/desktop/test/personal-center-pet-view.test.ts`

Preconditions:
- Task 1 and Task 2 are implemented locally.

Step 1:
- Action: Add fixture-driven tests covering a configured model with missing provider credentials and assert it is excluded from runtime-ready options and setup/selection flows.
- Done when: tests fail on the old behavior and pass with the credential-aware filtering, without depending on live provider secrets.
- Verify: `pnpm -C apps/desktop test -- --runInBand catalog-store.test.ts conversation-surface.test.ts personal-center-pet-view.test.ts`
- Stop if: the existing fixture shape cannot express missing-credential configured models without schema changes.

### Task 4: Re-verify the desktop runtime path

Status: `blocked`

Files:
- Modify: `docs/plans/runtime/2026-04-23-conversation-model-credential-filter.md`

Preconditions:
- Task 1 through Task 3 are `done`.

Step 1:
- Action: Re-run targeted tests and retry desktop startup far enough to confirm the runtime no longer defaults into a missing-secret model on first conversation submit.
- Done when: targeted desktop tests pass and manual startup no longer reproduces the missing-auth-secret failure for the missing-credential fixture/setup.
- Verify: `pnpm -C apps/desktop test -- --runInBand catalog-store.test.ts conversation-surface.test.ts personal-center-pet-view.test.ts && pnpm dev:desktop`
- Stop if: startup now fails on a different runtime error unrelated to model credential selection.

### Task 5: Add runtime session model-rebind support for stale sessions

Status: `done`

Files:
- Modify: `docs/plans/runtime/2026-04-23-conversation-model-credential-filter.md`
- Modify: `contracts/openapi/src/components/schemas/runtime.yaml`
- Modify: `contracts/openapi/src/paths/runtime.yaml`
- Modify: `crates/octopus-core/src/runtime_config.rs`
- Modify: `crates/octopus-platform/src/runtime.rs`
- Modify: `crates/octopus-platform/src/runtime_sdk/mod.rs`
- Modify: `crates/octopus-platform/src/runtime_sdk/session_bridge.rs`
- Modify: `crates/octopus-server/src/routes.rs`
- Modify: `crates/octopus-server/src/workspace_runtime/runtime_sessions.rs`
- Modify: `crates/octopus-platform/tests/runtime_sdk_bridge.rs`
- Modify: `crates/octopus-sdk-session/src/sqlite/stream.rs`
- Modify: `crates/octopus-sdk-session/tests/sqlite_jsonl.rs`

Preconditions:
- Task 1 through Task 3 stay intact.
- Runtime session model selection is still fixed at session start in the SDK runtime layer.

Step 1:
- Action: Add a transport and platform service path that rebinds an existing runtime session to a replacement configured model without discarding its transcript.
- Done when: desktop and server code can request a session rebind for a known `sessionId`, and the returned session detail reflects the replacement configured model for future turns.
- Verify: `pnpm openapi:bundle && pnpm schema:generate && cargo test -p octopus-platform runtime_sdk_bridge`
- Stop if: the runtime/session ownership boundary forces destructive session recreation instead of an in-place rebind.

Step 2:
- Action: Make SDK session snapshot repair treat the latest `SessionStarted` event as the active session binding so the rebind survives restart/reopen.
- Done when: reopening the session store preserves the re-bound model instead of repairing back to the original session-start metadata.
- Verify: `cargo test -p octopus-sdk-session sqlite_jsonl`
- Stop if: the session event contract requires a dedicated migration event instead of reusing `SessionStarted`.

### Task 6: Repair desktop project conversation reuse of stale sessions

Status: `done`

Files:
- Modify: `apps/desktop/src/tauri/runtime_api.ts`
- Modify: `apps/desktop/src/tauri/workspace-client.ts`
- Modify: `apps/desktop/src/stores/runtime_sessions.ts`
- Modify: `apps/desktop/src/views/project/ConversationView.vue`

Preconditions:
- Task 5 Step 1 is implemented locally.

Step 1:
- Action: Detect when a loaded project conversation session is bound to a configured model that is no longer runnable in the current project scope, then rebind it to the current runnable selection before submit continues.
- Done when: opening an old project conversation no longer silently keeps a missing-secret configured model bound for subsequent turns, and the composer stays aligned with the repaired session model.
- Verify: `pnpm -C apps/desktop exec vitest run test/conversation-surface.test.ts`
- Stop if: preserving transcript history requires a deeper conversation-domain migration outside runtime session ownership.

### Task 7: Add stale-session regression coverage and rerun targeted checks

Status: `done`

Files:
- Modify: `apps/desktop/test/conversation-surface.test.ts`
- Modify: `apps/desktop/test/support/workspace-fixture-client.ts`

Preconditions:
- Task 5 and Task 6 are implemented locally.

Step 1:
- Action: Add a regression where an existing project conversation session is still bound to a missing-credential configured model, assert the desktop surface repairs the session, and then confirm submit no longer fails with the stale provider secret error.
- Done when: the regression fails on stale-session reuse and passes once rebind support is wired through.
- Verify: `pnpm -C apps/desktop exec vitest run test/conversation-surface.test.ts`
- Stop if: the existing fixture runtime surface cannot represent session rebind semantics without broader harness changes.

## Execution Checkpoint

- 2026-04-23:
  - Task 1 done. Added runtime-only credential-aware model filtering in `apps/desktop/src/stores/catalog_filters.ts` and exposed conversation-default runnable model helpers without tightening management-facing option lists.
  - Task 2 done. Updated `apps/desktop/src/stores/pet.ts` to fall back to the catalog conversation default runnable model before taking the first runnable entry, which fixes personal-center pet preferences when the saved model is missing credentials and there is no active project scope.
  - Task 3 done. Added regression coverage in `apps/desktop/test/catalog-store.test.ts` and `apps/desktop/test/personal-center-pet-view.test.ts`, alongside the existing conversation regression.
  - Verification passed: `pnpm -C apps/desktop exec vitest run test/catalog-store.test.ts test/conversation-surface.test.ts test/personal-center-pet-view.test.ts`
  - Task 4 blocked on local environment state: `pnpm dev:desktop` stopped in `BeforeDevCommand` because Vite port `15420` was already in use, so desktop startup could not be re-verified in this turn.
- 2026-04-23:
  - Task 5 started. Root cause narrowed to stale project runtime sessions that still carry a non-runnable `selectedConfiguredModelId`; conversation submit does not send a replacement model, so the backend keeps executing the old binding.
  - Task 5 done. Added `POST /api/v1/runtime/sessions/{sessionId}/configured-model`, wired desktop/server/platform rebind support, and kept the same runtime session id while refreshing configured-model metadata and config snapshot fields.
  - Task 5 done. Updated SQLite JSONL snapshot repair to treat the latest `SessionStarted` event as authoritative, so rebind survives reopen and repair.
  - Task 6 done. Project conversation load now inspects the loaded runtime session binding, auto-rebinds stale non-runnable configured models to the current runnable selection, and syncs the composer selectors back from the repaired session.
  - Task 7 done. Added a stale-session regression in `apps/desktop/test/conversation-surface.test.ts` plus fixture support in `apps/desktop/test/support/workspace-fixture-client.ts`.
  - Verification passed: `pnpm openapi:bundle`
  - Verification passed: `pnpm schema:generate`
  - Verification passed: `cargo test -p octopus-platform runtime_sdk_bridge`
  - Verification passed: `cargo test -p octopus-sdk-session --test sqlite_jsonl`
  - Verification passed: `cargo check -p octopus-server`
  - Verification passed: `pnpm -C apps/desktop exec vitest run test/conversation-surface.test.ts`
