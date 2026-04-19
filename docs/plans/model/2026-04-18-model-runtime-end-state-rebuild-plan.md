# Model Runtime End-State Rebuild Plan

> **For Claude:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task.

**Goal:** Rebuild Octopus model execution into an end-state architecture where agent session runtime only uses natively streamed, tool-capable conversation drivers, prompt-style generation runs through a separate execution path, and model budget governance uses reservation plus settlement instead of post-hoc quota blocking.

**Architecture:** Keep the existing `catalog -> configured model -> resolved execution target -> runtime loop` outer backbone, but break the inner execution stack into two explicit runtimes: `agent_conversation` and `single_shot_generation`. Catalog/runtime contracts must expose true execution class and budget accounting semantics; agent conversation turns must run on upstream streaming drivers only; budget enforcement must reserve before execution and settle after provider usage is known.

**Tech Stack:** Rust 2021, `octopus-core`, `octopus-runtime-adapter`, `runtime`, `api`, `octopus-server`, OpenAPI under `contracts/openapi`, generated TS schema in `@octopus/schema`, Vue 3 desktop stores/views, SQLite projections in `octopus-infra`.

---

## Supersession

- This is the canonical implementation plan for model execution architecture, upstream streaming, tool-loop symmetry, and budget governance.
- It supersedes the model-execution portions of `docs/plans/model/2026-04-18-model-runtime-refactor-implementation-plan.md`.
- The older plan remains relevant only for unrelated completed or still-active work such as managed secret storage.

## Scope

- In scope:
  - replace `RuntimeExecutionSupport` with explicit execution-profile contracts
  - split agent session runtime from prompt-style generation runtime
  - remove conversation fallback through `simple_completion`
  - make upstream provider streaming mandatory for agent-conversation drivers
  - make tool-loop support symmetrical for all models exposed to agent sessions
  - replace `tokenQuota` with reservation-based `budgetPolicy`
  - update server, OpenAPI, schema, and desktop selection logic to match the new model runtime truth
- Out of scope:
  - `realtime` runtime
  - `vendor_native` protocol support
  - adding native agent-conversation drivers for `openai_responses` or `gemini_native` in this batch
  - backward-compatibility aliases for old field names, old DB tables, or old execution fallbacks

## Non-Negotiable Design Rules

- Octopus is not launched yet. Breaking refactors are allowed and preferred over compatibility shims.
- No dual execution trunk may remain after this refactor.
- No model may appear as selectable for agent sessions unless it has native upstream streaming and native tool-loop support.
- Prompt-style generation and agent-conversation runtime must not share the same driver trait or endpoint path.
- `budgetPolicy` is runtime governance, not a UI-only warning.
- Validation probe traffic must be a first-class traffic class, not an accidental side effect of normal usage accounting.

## Risks Or Open Questions

- Recommended decision: keep product-facing `surface` names such as `conversation`, but add a new explicit runtime execution class instead of overloading booleans.
- Recommended decision: keep `openai_responses` and `gemini_native` available only for generation flows until they gain native agent drivers; do not show them in session model pickers.
- Recommended decision: persist incremental output in the event log, project partial assistant state in memory, and persist only the final assistant message body as the durable message projection.
- Recommended decision: default probe traffic to `non_billable`, but still record it in the budget ledger for diagnostics.

## Execution Rules

- Do not begin implementation until every task below has exact files, verification commands, and stop conditions.
- Follow OpenAPI-first order for any `/api/v1/*` contract change:
  `contracts/openapi/src/** -> pnpm openapi:bundle -> pnpm schema:generate -> adapters/stores/server/tests`.
- Delete replaced code paths after the new tests pass; do not leave permanent aliases or parallel fallback logic.
- Execute in small batches and update status markers inside this plan after each batch.
- Stop if a task requires preserving legacy contract names, preserving old DB tables, or keeping prompt-only models selectable in agent sessions.

## Recommended Execution Batches

- Batch 1: Task 1 -> Task 2
- Batch 2: Task 3 -> Task 4
- Batch 3: Task 5
- Batch 4: Task 6 -> Task 7
- Batch 5: Task 8

## Task Ledger

### Task 1: Freeze Public Contracts Around Execution Profile And Budget Policy

Status: `in_progress`

Files:
- Modify: `crates/octopus-core/src/lib.rs`
- Modify: `contracts/openapi/src/components/schemas/catalog.yaml`
- Modify: `contracts/openapi/src/components/schemas/runtime.yaml`
- Modify: `contracts/openapi/src/paths/catalog.yaml`
- Modify: `contracts/openapi/src/paths/runtime.yaml`
- Modify: `packages/schema/src/catalog.ts`
- Regenerate: `packages/schema/src/generated.ts`

Preconditions:
- User approved breaking contract changes and no compatibility aliases.

Step 1:
- Action: Replace `RuntimeExecutionSupport` with explicit transport contracts in `octopus-core`, introducing `RuntimeExecutionProfile`, `RuntimeExecutionClass`, and `BudgetAccountingMode`; replace `ConfiguredModelTokenQuota` with `ConfiguredModelBudgetPolicy`.
- Done when: the public Rust contract no longer describes runtime truth using only `prompt/conversation/tool_loop/streaming` booleans or `tokenQuota.totalTokens`.
- Verify: `cargo test -p octopus-core --lib`
- Stop if: a required downstream runtime still cannot distinguish `agent_conversation` from `single_shot_generation` without an additional contract field.

Step 2:
- Action: Update the OpenAPI source for catalog and runtime payloads so the new execution-profile and budget-policy shapes are the only HTTP contract source.
- Done when: OpenAPI source has no remaining `RuntimeExecutionSupport` or `tokenQuota` schema definitions.
- Verify: `pnpm openapi:bundle`
- Stop if: the new execution class requires splitting an endpoint that is currently carrying two incompatible semantics.

Step 3:
- Action: Regenerate TypeScript transport artifacts and align handwritten schema exports to the new contract names.
- Done when: generated TypeScript matches the new OpenAPI source and no handwritten schema keeps a parallel old truth source.
- Verify: `pnpm schema:generate && pnpm schema:check`
- Stop if: any handwritten TS domain model still has to preserve the deleted contract names for UI compatibility.

### Task 2: Rebuild Catalog Resolution Around True Runtime Executability

Status: `done`

Files:
- Modify: `crates/octopus-runtime-adapter/src/registry.rs`
- Modify: `crates/octopus-runtime-adapter/src/registry_baseline.rs`
- Modify: `crates/octopus-runtime-adapter/src/registry_overrides.rs`
- Modify: `crates/octopus-runtime-adapter/src/registry_resolution.rs`
- Modify: `crates/octopus-runtime-adapter/src/execution_target.rs`
- Modify: `crates/octopus-runtime-adapter/src/model_runtime/canonical_model_policy.rs`
- Modify: `crates/octopus-runtime-adapter/src/model_runtime/driver_registry.rs`
- Test: `crates/octopus-runtime-adapter/tests/registry_execution_support.rs`
- Test: `crates/octopus-runtime-adapter/tests/canonical_model_policy.rs`

Preconditions:
- Task 1 is done.

Step 1:
- Action: Compute `RuntimeExecutionProfile` from protocol-family driver truth and make registry projection fail closed for unsupported or partial protocol families.
- Done when: catalog/runtime metadata reports executable agent-session support only for protocol families that really implement the full agent-conversation contract.
- Verify: `cargo test -p octopus-runtime-adapter --test registry_execution_support`
- Stop if: runtime truth requires a protocol-level distinction that cannot be represented in the new execution-profile contract.

Step 2:
- Action: Reclassify `openai_responses` and `gemini_native` bindings as `single_shot_generation` only, and remove them from agent-session target resolution.
- Done when: session target resolution cannot return `openai_responses` or `gemini_native` for agent sessions, even when the configured model exists.
- Verify: `cargo test -p octopus-runtime-adapter --test canonical_model_policy`
- Stop if: product insists on keeping prompt-only models visible as normal agent-session choices before native drivers exist.

Step 3:
- Action: Make `resolve_target()` require runtime execution class for runtime session calls instead of fuzzy surface fallback.
- Done when: no runtime session path can silently fall back from conversation runtime to prompt completion.
- Verify: `cargo test -p octopus-runtime-adapter --test registry_execution_support`
- Stop if: another subsystem still depends on the current surface-only fallback semantics.

### Task 3: Split Driver Abstractions Into Agent Conversation And Generation Runtimes

Status: `done`

Files:
- Create: `crates/octopus-runtime-adapter/src/model_runtime/conversation_driver.rs`
- Create: `crates/octopus-runtime-adapter/src/model_runtime/generation_driver.rs`
- Create: `crates/octopus-runtime-adapter/src/model_runtime/stream_bridge.rs`
- Modify: `crates/octopus-runtime-adapter/src/model_runtime/mod.rs`
- Modify: `crates/octopus-runtime-adapter/src/model_runtime/driver.rs`
- Modify: `crates/octopus-runtime-adapter/src/model_runtime/driver_registry.rs`
- Modify: `crates/octopus-runtime-adapter/src/model_runtime/simple_completion.rs`
- Test: `crates/octopus-runtime-adapter/tests/protocol_drivers.rs`
- Test: `crates/octopus-runtime-adapter/tests/simple_completion.rs`

Preconditions:
- Task 2 is done.

Step 1:
- Action: Replace the current multi-mode `ProtocolDriver` abstraction with separate `ConversationModelDriver` and `GenerationModelDriver` contracts.
- Done when: agent-conversation execution and prompt-style generation are no longer dispatched through one catch-all trait with capability flags.
- Verify: `cargo test -p octopus-runtime-adapter --test protocol_drivers`
- Stop if: a third runtime class appears that does not fit either contract and needs separate treatment before implementation continues.

Step 2:
- Action: Introduce a normalized turn-event stream contract for conversation drivers, including text delta, completed tool use, usage report, request metadata, and stop reason.
- Done when: conversation drivers can produce incremental normalized turn events without depending on provider-specific payload types outside the driver layer.
- Verify: `cargo test -p octopus-runtime-adapter --test protocol_drivers`
- Stop if: provider-specific stream normalization requires moving transport-only code back out of the driver layer.

Step 3:
- Action: Remove the current conversation fallback through `simple_completion` so tool-free conversation turns still use the conversation runtime contract.
- Done when: `simple_completion` is reachable only through generation runtime code paths.
- Verify: `cargo test -p octopus-runtime-adapter --test simple_completion`
- Stop if: any agent-session runtime code still depends on prompt-only driver responses.

### Task 4: Implement Native Upstream Streaming For Anthropic And OpenAI Chat

Status: `done`

Files:
- Modify: `crates/api/src/types.rs`
- Modify: `crates/api/src/providers/anthropic.rs`
- Modify: `crates/api/src/providers/anthropic_stream.rs`
- Modify: `crates/api/src/providers/openai_compat.rs`
- Modify: `crates/api/src/providers/stream_parsing.rs`
- Modify: `crates/octopus-runtime-adapter/src/model_runtime/drivers/mod.rs`
- Modify: `crates/octopus-runtime-adapter/src/model_runtime/drivers/anthropic_messages.rs`
- Modify: `crates/octopus-runtime-adapter/src/model_runtime/drivers/openai_chat.rs`
- Test: `crates/api/src/providers/anthropic_tests.rs`
- Test: `crates/octopus-runtime-adapter/tests/protocol_drivers.rs`

Preconditions:
- Task 3 is done.

Step 1:
- Action: Extend provider stream normalization so Anthropic and OpenAI-compatible streams can yield final request metadata, usage, and fully assembled tool-use payloads through the driver stream bridge.
- Done when: the driver layer can consume provider streams without waiting for a terminal request-response body to reconstruct assistant output.
- Verify: `cargo test -p api --lib`
- Stop if: the provider SDK layer cannot expose enough stream detail to assemble completed tool use events.

Step 2:
- Action: Switch `anthropic_messages` agent-conversation execution to `stream_message()` and remove request-response conversation execution from that path.
- Done when: Anthropic agent turns are driven by upstream stream events from request start through message stop.
- Verify: `cargo test -p octopus-runtime-adapter --test protocol_drivers`
- Stop if: any Anthropic tool-loop behavior still depends on the old post-hoc `response_to_events()` conversion.

Step 3:
- Action: Switch `openai_chat` agent-conversation execution to `stream_message()` with the same normalized turn-event bridge.
- Done when: OpenAI-compatible agent turns emit text and tool events incrementally from upstream stream data.
- Verify: `cargo test -p octopus-runtime-adapter --test protocol_drivers`
- Stop if: OpenAI-compatible stream parsing still requires a second request-response call to recover final tool payloads.

### Task 5: Rebuild Agent Runtime Loop And Event Projection Around Incremental Model Events

Status: `done`

Files:
- Modify: `crates/octopus-runtime-adapter/src/agent_runtime_core.rs`
- Modify: `crates/octopus-runtime-adapter/src/execution_events.rs`
- Modify: `crates/octopus-runtime-adapter/src/adapter_test_support.rs`
- Modify: `crates/octopus-runtime-adapter/src/approval_runtime_tests.rs`
- Modify: `crates/octopus-runtime-adapter/src/capability_runtime_tests.rs`
- Modify: `crates/octopus-runtime-adapter/src/runtime_contract_tests.rs`
- Modify: `crates/octopus-runtime-adapter/src/runtime_compatibility_tests.rs`
- Modify: `contracts/openapi/src/components/schemas/runtime.yaml`
- Regenerate: `contracts/openapi/octopus.openapi.yaml`
- Regenerate: `packages/schema/src/generated.ts`
- Test: `crates/octopus-runtime-adapter/tests/runtime_turn_loop.rs`

Preconditions:
- Task 4 is done.

Step 1:
- Action: Change the model execution loop to consume normalized turn events incrementally, append partial assistant content to the active run, and finalize the assistant message only after explicit model stop.
- Done when: the runtime can forward model deltas to SSE/event projection while the provider stream is still open.
- Verify: `cargo test -p octopus-runtime-adapter --test runtime_turn_loop`
- Stop if: the runtime loop still assumes the whole assistant message is available before tool planning or event emission begins.

Step 2:
- Action: Replace synthetic `model.streaming` semantics in `execution_events.rs` with real `model.started`, `model.delta`, `model.tool_use`, `model.usage`, and `model.completed` progression derived from the streamed turn.
- Done when: execution events represent true upstream progression instead of post-completion runtime narration.
- Verify: `cargo test -p octopus-runtime-adapter --test runtime_turn_loop && pnpm openapi:bundle && pnpm schema:generate`
- Stop if: event-envelope schema needs a separate explicit contract update that is not yet reflected in Task 1.

Step 3:
- Action: Make mid-stream failures or disconnects stop before final assistant-message commit and leave a resumable checkpoint with partial output metadata only.
- Done when: failed turns do not persist a completed assistant message after an interrupted stream.
- Verify: `cargo test -p octopus-runtime-adapter approval_runtime_tests capability_runtime_tests runtime_turn_loop -- --nocapture`
- Stop if: recovery semantics require broader session-resume changes outside the model runtime boundary.

Step 4:
- Action: Update scripted runtime driver support and runtime tests to model incremental event sequences instead of one-shot completed responses.
- Done when: approval/capability/runtime-loop tests exercise streaming semantics directly.
- Verify: `cargo test -p octopus-runtime-adapter --lib`
- Stop if: test infrastructure still hardcodes request-response model assumptions across multiple subsystems.

### Task 6: Separate Agent Session APIs From Generation APIs

Status: `done`

Files:
- Modify: `crates/octopus-core/src/lib.rs`
- Modify: `crates/octopus-platform/src/runtime.rs`
- Modify: `crates/octopus-server/src/workspace_runtime.rs`
- Modify: `crates/octopus-server/src/routes.rs`
- Modify: `crates/octopus-runtime-adapter/src/lib.rs`
- Modify: `crates/octopus-runtime-adapter/src/execution_service.rs`
- Modify: `crates/octopus-runtime-adapter/src/execution_target.rs`
- Modify: `contracts/openapi/src/components/schemas/runtime.yaml`
- Modify: `contracts/openapi/src/paths/runtime.yaml`
- Modify: `packages/schema/src/generated.ts`
- Modify: `apps/desktop/src/tauri/runtime_api.ts`
- Modify: `apps/desktop/src/tauri/workspace-client.ts`
- Modify: `apps/desktop/src/stores/runtime_sessions.ts`
- Modify: `apps/desktop/src/stores/runtime_actions.ts`
- Modify: `apps/desktop/src/stores/catalog.ts`
- Modify: `apps/desktop/src/stores/catalog_filters.ts`
- Modify: `apps/desktop/src/stores/catalog_normalizers.ts`
- Modify: `apps/desktop/src/views/workspace/ModelDetailsPanel.vue`
- Modify: `apps/desktop/src/views/workspace/models-runtime-helpers.ts`
- Modify: `apps/desktop/src/views/workspace/useModelsDraft.ts`
- Modify: `apps/desktop/test/catalog-store.test.ts`
- Modify: `apps/desktop/test/models-view.test.ts`
- Modify: `apps/desktop/test/openapi-transport.test.ts`
- Modify: `apps/desktop/test/support/workspace-fixture-state.ts`
- Modify: `apps/desktop/test/tauri-client-runtime.test.ts`
- Modify: `packages/schema/src/runtime.ts`

Preconditions:
- Tasks 1 through 5 are done.

Step 1:
- Action: Restrict runtime session create and turn-submit flows to configured models whose execution class is `agent_conversation`.
- Done when: session APIs reject prompt-only configured models before a run starts.
- Verify: `cargo test -p octopus-server --lib`
- Stop if: any existing session workflow still intentionally depends on prompt-only runtime semantics.

Step 2:
- Action: Define a separate generation execution path for `single_shot_generation` models rather than routing them through session turns.
- Done when: prompt-style model execution has a distinct adapter/server path and does not reuse agent session endpoints.
- Verify: `pnpm openapi:bundle && pnpm schema:generate && cargo test -p octopus-server --lib`
- Stop if: generation UX and API ownership are unresolved enough to require a separate product decision before implementation.

Step 3:
- Action: Update desktop adapters and stores so conversation model pickers filter on execution class and no longer rely on legacy runtime-support booleans.
- Done when: desktop state and validation logic consume the new execution-profile contract end to end.
- Verify: `pnpm schema:check && pnpm check:frontend`
- Stop if: any desktop flow still reads deleted `RuntimeExecutionSupport` or `tokenQuota` fields.

### Task 7: Replace Token Quota With Reservation-Based Budget Governance

Status: `done`
Current step: `complete`

Files:
- Create: `crates/octopus-runtime-adapter/src/model_budget.rs`
- Delete: `crates/octopus-runtime-adapter/src/model_usage.rs`
- Modify: `crates/octopus-runtime-adapter/src/config_service.rs`
- Modify: `crates/octopus-runtime-adapter/src/execution_target.rs`
- Modify: `crates/octopus-runtime-adapter/src/agent_runtime_core.rs`
- Modify: `crates/octopus-runtime-adapter/src/execution_events.rs`
- Modify: `crates/octopus-runtime-adapter/src/lib.rs`
- Modify: `crates/octopus-infra/src/infra_state.rs`
- Modify: `crates/octopus-runtime-adapter/src/token_usage_tests.rs`
- Modify: `crates/octopus-runtime-adapter/src/runtime_compatibility_tests.rs`
- Modify: `apps/desktop/src/views/workspace/ModelDetailsPanel.vue`
- Modify: `apps/desktop/src/views/workspace/ModelsListPane.vue`
- Modify: `apps/desktop/src/views/workspace/useModelsDraft.ts`

Preconditions:
- Tasks 1 and 6 are done.

Step 1:
- Action: Replace configured-model `tokenQuota` settings with `budgetPolicy`, including traffic classes, accounting mode, warning thresholds, and reservation strategy.
- Done when: runtime config, public contracts, and desktop editing flows refer only to budget policy semantics.
- Verify: `cargo test -p octopus-core --lib && pnpm schema:check`
- Stop if: budget policy needs org-level or billing-ledger ownership that is not part of the model-runtime boundary.

Step 2:
- Action: Replace `configured_model_usage_projections`-only enforcement with reservation and settlement tables plus projections in SQLite.
- Done when: the runtime can reserve before run start, settle on completion, and release unused reservation on failure or interruption.
- Verify: `cargo test -p octopus-infra --lib`
- Stop if: persistence rules require a different canonical place for budget reservation state than SQLite.

Step 3:
- Action: Apply budget reservation at probe start and run start, with separate traffic-class handling for `probe`, `interactive_turn`, and future background work.
- Done when: validation probes are no longer silently charged as normal conversation usage and the runtime blocks only when reservation fails.
- Verify: `cargo test -p octopus-runtime-adapter token_usage_tests runtime_compatibility_tests -- --nocapture`
- Stop if: a stricter product decision is required on whether probe traffic is billable for some model classes.

Step 4:
- Action: Make budget-enforced models reject unsupported accounting modes at configuration or validation time instead of failing after execution when provider usage is missing.
- Done when: the runtime no longer discovers unusable budget semantics only after a model call has completed.
- Verify: `cargo test -p octopus-runtime-adapter --lib`
- Stop if: a provider requires estimated usage math that cannot be made stable enough in this batch.

### Task 8: Update Workspace Model Console, Runtime Docs, And Verification Matrix

Status: `done`
Current step: `complete`

Files:
- Modify: `apps/desktop/src/views/workspace/ModelsView.vue`
- Modify: `apps/desktop/src/views/workspace/ModelsListPane.vue`
- Modify: `apps/desktop/src/views/workspace/ModelDetailsPanel.vue`
- Modify: `apps/desktop/src/views/workspace/useModelsDraft.ts`
- Modify: `apps/desktop/src/views/workspace/models-runtime-helpers.ts`
- Modify: `apps/desktop/src/stores/catalog_management.ts`
- Modify: `docs/plans/model/2026-04-18-model-module-architecture.md`
- Modify: `docs/runtime_config_api.md`

Preconditions:
- Tasks 1, 6, and 7 are done.

Step 1:
- Action: Replace UI labels and editing affordances based on `streaming/toolLoop` booleans with explicit execution class, upstream streaming truth, tool-loop truth, credential health, and budget-policy sections.
- Done when: the workspace model console exposes runtime truth directly and no longer suggests that prompt-only models are normal session-runtime options.
- Verify: `pnpm check:frontend`
- Stop if: the workspace model page needs a larger IA redesign that cannot be scoped as a follow-up to the current list/detail model console.

Step 2:
- Action: Update the model architecture and runtime-config docs so they describe the new execution-profile and budget-policy system as the canonical runtime contract.
- Done when: internal docs no longer describe `RuntimeExecutionSupport` or `tokenQuota` as current truth.
- Verify: `rg -n "RuntimeExecutionSupport|tokenQuota" docs apps/desktop/src crates/octopus-runtime-adapter/src packages/schema/src contracts/openapi/src`
- Stop if: another canonical governance document must be updated first to own one of the new rules.

Step 3:
- Action: Run the full backend, contract, and frontend verification matrix after all refactor tasks are complete.
- Done when: all commands below pass on the new end-state architecture with deleted compatibility paths.
- Verify:
  - `cargo test -p octopus-core --lib`
  - `cargo test -p api --lib`
  - `cargo test -p octopus-runtime-adapter --lib`
  - `cargo test -p octopus-runtime-adapter --test registry_execution_support`
  - `cargo test -p octopus-runtime-adapter --test canonical_model_policy`
  - `cargo test -p octopus-runtime-adapter --test protocol_drivers`
  - `cargo test -p octopus-runtime-adapter --test runtime_turn_loop`
  - `cargo test -p octopus-runtime-adapter --test simple_completion`
  - `cargo test -p octopus-server --lib`
  - `cargo test -p octopus-infra --lib`
  - `pnpm openapi:bundle`
  - `pnpm schema:generate`
  - `pnpm schema:check`
  - `pnpm check:frontend`
- Stop if: any verification still depends on a deleted fallback path and needs an explicit replacement test before proceeding.

## Batch Checkpoint Format

After each execution batch, append:

```md
## Checkpoint YYYY-MM-DD HH:MM

- Batch: Task N Step X -> Task M Step Y
- Completed:
  - concise list
- Verification:
  - `command` -> pass or fail
- Blockers:
  - none
- Next:
  - next exact task and step
```

## Checkpoint 2026-04-19 00:20

- Batch: Task 1 Step 1 -> Task 2 Step 3
- Completed:
  - replaced `RuntimeExecutionSupport` with `RuntimeExecutionProfile` and `RuntimeExecutionClass` across Rust core contracts, OpenAPI schemas, and TypeScript schema surfaces
  - replaced public `tokenQuota` transport contract with `budgetPolicy` and aligned runtime-adapter parsing/usage helpers to the new field
  - rebuilt registry execution projection and session target resolution around explicit execution class requirements instead of runtime-support booleans
  - reclassified partial protocol-family runtime support so only matching execution classes can be resolved through session paths
- Verification:
  - `cargo test -p octopus-core --lib` -> pass
  - `pnpm openapi:bundle` -> pass
  - `pnpm schema:generate` -> pass
  - `pnpm schema:check` -> pass
  - `cargo test -p octopus-runtime-adapter --test registry_execution_support` -> pass
  - `cargo test -p octopus-runtime-adapter --test canonical_model_policy` -> pass
- Blockers:
  - none
- Next:
  - Task 3 Step 1: split the driver abstraction into explicit conversation and generation runtime contracts

## Checkpoint 2026-04-19 00:40

- Batch: Task 3 Step 1 -> Task 4 Step 3
- Completed:
  - split the monolithic protocol-driver path into explicit `ConversationModelDriver` and `GenerationModelDriver` contracts while keeping the external `RuntimeModelDriver` surface stable for the rest of the adapter
  - introduced a normalized conversation stream bridge that assembles text deltas, completed tool calls, usage, request metadata, and stop semantics from provider `StreamEvent` payloads
  - removed conversation fallback through `simple_completion` so generation-only drivers remain prompt-only and agent turns require a real conversation driver
  - switched Anthropic and OpenAI Chat conversation execution to native upstream `stream_message()` flows and updated registry execution truth so those protocol families project as `agent_conversation`
  - updated runtime-driver and registry tests to assert separate conversation/generation lookup, generation-only rejection for conversation execution, and incremental streaming tool-use behavior
- Verification:
  - `cargo test -p octopus-runtime-adapter --test protocol_drivers --test simple_completion --test registry_execution_support --test canonical_model_policy` -> pass
  - `cargo test -p api --lib` -> pass
  - `cargo fmt` -> pass
- Blockers:
  - none
- Next:
  - Task 5 Step 1: rebuild the agent runtime loop to consume normalized incremental model-turn events instead of completed one-shot assistant responses

## Checkpoint 2026-04-19 01:05

- Batch: Task 5 Step 1 -> Task 5 Step 4 (partial)
- Completed:
  - rebuilt runtime turn-loop projection so streamed assistant events are consumed incrementally and persisted as per-iteration `model.delta`, `model.tool_use`, and `model.usage` records instead of a synthetic `model.streaming` flag
  - updated runtime event emission, runtime event contract tests, and runtime turn-loop tests to assert the new event progression and ordering
  - upgraded prompt-oriented runtime test doubles to also provide conversation execution so runtime, approval, capability, actor, MCP, and persistence tests continue to exercise the agent-conversation path after the fallback removal
  - aligned runtime event OpenAPI/schema surfaces from `model.streaming` to `model.delta`, `model.tool_use`, and `model.usage`
- Verification:
  - `cargo test -p octopus-runtime-adapter --test runtime_turn_loop` -> pass
  - `cargo test -p octopus-runtime-adapter runtime_events_only_emit_declared_runtime_event_kinds -- --nocapture` -> pass
  - `cargo test -p octopus-runtime-adapter runtime_session_public_contract_and_projection_fields_match_phase_two_shape -- --nocapture` -> pass
  - `pnpm openapi:bundle` -> pass
  - `pnpm schema:generate` -> pass
  - `cargo test -p octopus-runtime-adapter --lib` -> pass
- Blockers:
  - Task 5 Step 3 remains open: interrupted model streams currently avoid final assistant-message commit, but they do not yet persist a dedicated resumable checkpoint artifact with partial output metadata for non-mediation failures
- Next:
  - Task 5 Step 3: persist interrupted-stream runtime checkpoints with partial assistant-output metadata and emit matching failure-state projection

## Checkpoint 2026-04-19 01:40

- Batch: Task 5 Step 3
- Completed:
  - changed the runtime turn loop to treat interrupted assistant streams as a structured failed-turn exit instead of a hard submit error on the direct submit path
  - persisted dedicated failed-run checkpoint artifacts under the generic runtime checkpoint area, with serialized session state plus `partialOutput` metadata and no committed assistant message
  - projected interrupted turns as failed runs with `runtime.error` plus streamed `model.delta` and `model.usage` events, while suppressing `model.completed`
  - added a runtime turn-loop test that asserts failed-run checkpoint persistence, partial-output metadata, missing assistant-message commit, and failure event projection for disconnected streams
- Verification:
  - `cargo test -p octopus-runtime-adapter approval_runtime_tests -- --nocapture` -> pass
  - `cargo test -p octopus-runtime-adapter capability_runtime_tests -- --nocapture` -> pass
  - `cargo test -p octopus-runtime-adapter --test runtime_turn_loop -- --nocapture` -> pass
- Blockers:
  - Task 5 Step 3 is still partially open for approval/auth resume flows: resumed runtime loops currently surface interrupted streams as hard errors instead of projecting the same failed-run checkpoint semantics
- Next:
  - Task 5 Step 3: extend interrupted-stream failed-checkpoint projection to approval/auth resume call sites or explicitly narrow the recovery boundary if that is not required

## Checkpoint 2026-04-19 01:50

- Batch: Task 5 Step 3
- Completed:
  - extended interrupted-stream failed-run projection to main-session approval resume so disconnected replay turns now persist failed checkpoint artifacts, retain `partialOutput` metadata, suppress committed assistant-message creation for the interrupted replay, and keep the run in a failed/idle state instead of returning a hard resolution error
  - extended the same failed-run checkpoint semantics to main-session auth resume, including failed checkpoint persistence, `runtime.error` projection, preserved partial output metadata, and no committed assistant message for the interrupted replay content
  - tightened approval/auth interruption tests to match the real projection boundary: already-completed prior assistant messages remain durable, but interrupted replay output is kept only in failed checkpoint artifacts and failure events
- Verification:
  - `cargo test -p octopus-runtime-adapter --test runtime_turn_loop approval_resume_persists_failed_checkpoint_when_assistant_stream_disconnects -- --nocapture` -> pass
  - `cargo test -p octopus-runtime-adapter auth_resume_persists_failed_checkpoint_on_disconnect -- --nocapture` -> pass
  - `cargo test -p octopus-runtime-adapter approval_runtime_tests -- --nocapture` -> pass
  - `cargo test -p octopus-runtime-adapter capability_runtime_tests -- --nocapture` -> pass
  - `cargo test -p octopus-runtime-adapter --test runtime_turn_loop -- --nocapture` -> pass
- Blockers:
  - Task 5 Step 3 remains open for team-subrun approval/auth resume interruption semantics; those paths still need either equivalent failed-checkpoint projection or an explicit architecture decision that main-session coverage is the intended boundary
- Next:
  - Task 5 Step 3: decide and implement the team-subrun resume interruption boundary before advancing to Task 6

## Checkpoint 2026-04-19 02:00

- Batch: Task 5 Step 3
- Completed:
  - extended interrupted-stream failed-run projection to team-subrun approval resume so blocked worker replays no longer hard-error on disconnect; the failed worker subrun now persists a failed checkpoint artifact with `partialOutput` metadata, emits replay `runtime.error` and subrun/workflow/background failure events, and releases the scheduler queue for later workers
  - extended the same failed-run projection to team-subrun auth resume, including failed worker checkpoint persistence, replay failure event emission, and queued-worker continuation after the failed subrun frees its slot
  - changed `execute_team_subrun()` to return projected failed subrun state plus replay event metadata instead of aborting the whole resolution call on `RuntimeLoopExit::Failed`, and threaded the subrun replay planner/model/capability event stream into approval/auth resolution event emission
  - added regression tests that cover disconnected team-subrun approval/auth replays, assert failed checkpoint persistence and `partialOutput` content on the failed worker subrun, and verify queued-worker continuation plus workflow/background failure projection
- Verification:
  - `cargo fmt` -> pass
  - `cargo test -p octopus-runtime-adapter team_worker_subrun_approval_resume_persists_failed_checkpoint_on_disconnect_and_releases_scheduler_queue -- --nocapture` -> pass
  - `cargo test -p octopus-runtime-adapter team_worker_subrun_auth_resume_persists_failed_checkpoint_on_disconnect_and_releases_scheduler_queue -- --nocapture` -> pass
  - `cargo test -p octopus-runtime-adapter approval_runtime_tests -- --nocapture` -> pass
  - `cargo test -p octopus-runtime-adapter capability_runtime_tests -- --nocapture` -> pass
  - `cargo test -p octopus-runtime-adapter --test runtime_turn_loop -- --nocapture` -> pass
- Blockers:
  - none
- Next:
  - Task 6 Step 1: reject prompt-only configured models from runtime session create and turn-submit flows before execution starts

## Checkpoint 2026-04-19 02:11

- Batch: Task 6 Step 1
- Completed:
  - added runtime-session creation validation so any selected or snapshotted default configured model must resolve to `agent_conversation` before the session is persisted
  - added adapter regression coverage for both create-session rejection and submit-time resolution rejection using a `single_shot_generation` Gemini configured model fixture
  - added a server-layer regression test proving `POST /api/v1/runtime/sessions` rejects prompt-only configured models at the public API boundary
- Verification:
  - `cargo test -p octopus-runtime-adapter create_session_rejects_single_shot_generation_model_selection -- --nocapture` -> pass
  - `cargo test -p octopus-runtime-adapter resolve_submit_execution_rejects_single_shot_generation_model_selection -- --nocapture` -> pass
  - `cargo fmt` -> pass
  - `cargo test -p octopus-server --lib` -> pass
- Blockers:
  - none
- Next:
  - Task 6 Step 2: define the dedicated generation execution path for `single_shot_generation` models instead of reusing runtime session turns

## Checkpoint 2026-04-19 02:35

- Batch: Task 6 Step 2
- Completed:
  - added a dedicated `POST /api/v1/runtime/generations` API path with new `RunRuntimeGenerationInput` and `RuntimeGenerationResult` transport contracts in Rust core, OpenAPI, and generated TypeScript schema output
  - extended the runtime execution service with a separate single-shot generation method that resolves only `single_shot_generation` targets, executes prompt-style requests outside session turns, and records request cost plus configured-model usage
  - wired the server runtime facade and desktop runtime adapter to the new generation path, keeping prompt execution out of `/runtime/sessions/*` and out of configured-model probe endpoints
  - added regression coverage for successful generation execution, rejection of `agent_conversation` models on the generation route, generated transport contract presence, and desktop adapter request routing
- Verification:
  - `cargo test -p octopus-server runtime_generation_route_ -- --nocapture` -> pass
  - `pnpm openapi:bundle` -> pass
  - `pnpm schema:generate` -> pass
  - `cargo fmt` -> pass
  - `cargo test -p octopus-server --lib` -> pass
  - `pnpm -C apps/desktop test -- test/openapi-transport.test.ts -t "dedicated runtime generation schemas and route"` -> relevant targeted assertion passed, command also loaded unrelated suites and failed on pre-existing `lucide-vue-next` resolution outside this batch
  - `pnpm -C apps/desktop test -- test/tauri-client-runtime.test.ts -t "dedicated runtime generation requests"` -> relevant targeted assertion passed, command also loaded unrelated suites and failed on the same pre-existing `lucide-vue-next` resolution outside this batch
- Blockers:
  - none for Task 6 Step 2
  - existing frontend test environment issue: Vitest loads unrelated suites in this worktree and they currently fail on `lucide-vue-next` module resolution unrelated to the generation-path changes
- Next:
  - Task 6 Step 3: update desktop model-picking and validation state to filter on execution class instead of legacy runtime-support booleans

## Checkpoint 2026-04-19 02:53

- Batch: Task 6 Step 3
- Completed:
  - replaced desktop catalog normalization and draft/runtime helper parsing so provider surfaces, model bindings, and configured-model persistence consume `executionProfile` and `budgetPolicy` instead of deleted `runtimeSupport` and `tokenQuota` fields
  - restricted conversation-facing configured-model options to models whose normalized surfaces resolve to `agent_conversation`, preventing `single_shot_generation` entries from appearing in desktop conversation pickers while keeping row/detail surfaces consistent with the new execution-class contract
  - updated the models workspace fixtures and regression coverage to use `executionProfile` / `budgetPolicy`, and exported the new generation transport types through `@octopus/schema` so desktop adapters typecheck against the Task 6 Step 2 API additions
- Verification:
  - `pnpm -C apps/desktop exec vitest run test/catalog-store.test.ts --testNamePattern "filters generation-only configured models out of conversation-facing catalog options and surfaces"` -> pass
  - `pnpm -C apps/desktop exec vitest run test/models-view.test.ts --testNamePattern "renders explicit workspace models with pagination and structured filters|switches create dialog fields for standard, custom, and ollama providers|creates custom models by auto-saving configuredModels and modelRegistry patches"` -> pass
  - `pnpm -C apps/desktop typecheck` -> pass
  - `pnpm schema:check` -> pass
  - `pnpm check:frontend` -> pass
- Blockers:
  - none
- Next:
  - Task 7 Step 1: replace desktop/runtime configured-model quota editing and persistence semantics with `budgetPolicy` as the public source of truth before moving reservation enforcement into the runtime adapter and SQLite projections

## Checkpoint 2026-04-19 03:06

- Batch: Task 7 Step 1
- Completed:
  - replaced workspace model console create/detail flows so configured-model budget editing now persists full `budgetPolicy` fields instead of only `totalBudgetTokens`
  - added explicit desktop inputs for accounting mode, traffic classes, warning thresholds, reservation strategy, and budget total in both create and detail panes
  - updated workspace model list/detail copy from quota wording to budget-policy wording and added regression coverage for create/save round-tripping the full `budgetPolicy`
- Verification:
  - `pnpm -C apps/desktop exec vitest run test/models-view.test.ts --testNamePattern "creates custom models by auto-saving configuredModels and modelRegistry patches|saves detail edits as a full configured-model budget policy patch|renders explicit workspace models with pagination and structured filters"` -> pass
  - `pnpm -C apps/desktop typecheck` -> pass
  - `cargo test -p octopus-core --lib` -> pass
  - `pnpm schema:check` -> pass
- Blockers:
  - none
- Next:
  - Task 7 Step 2: replace projection-only token-usage enforcement with reservation/settlement persistence in the runtime adapter and SQLite projections

## Checkpoint 2026-04-19 03:23

- Batch: Task 7 Step 2
- Completed:
  - added SQLite reservation, settlement, and projection tables for configured-model budgets, plus bootstrap/backfill from legacy `configured_model_usage_projections`
  - replaced runtime-adapter post-hoc token-usage increments with reservation lifecycle helpers in new `model_budget.rs`, using settled-token projections for catalog usage and exhaustion checks
  - wrapped conversation run execution and dedicated generation execution with reserve/settle/release behavior so successful runs settle usage and failed/interrupted runs release held budget
  - kept probe traffic on direct settlement for now while removing legacy usage increments from runtime event writers
  - fixed and extended token-usage regression coverage to assert projection rows, reservation status, settlement rows, and release-on-generation-failure behavior
- Verification:
  - `cargo test -p octopus-runtime-adapter token_usage_tests -- --nocapture` -> pass
  - `cargo test -p octopus-infra --lib` -> pass
- Blockers:
  - none
- Next:
  - Task 7 Step 3: apply reservation semantics at probe start and separate traffic-class handling for `probe` vs `interactive_turn`

## Checkpoint 2026-04-19 03:31

- Batch: Task 7 Step 3
- Completed:
  - moved configured-model probe execution onto the same reservation lifecycle as runtime turns and dedicated generations, so probes now reserve before execution, settle on success, and release on probe failure or missing provider usage
  - added `traffic_class` persistence on budget reservations and settlements, with SQLite bootstrap/migration support for existing workspaces
  - scoped budget charging by `budgetPolicy.trafficClasses`, so `probe` traffic can be budgeted independently from `interactive_turn` instead of silently charging conversation budget
  - removed pre-execution budget blocking from target resolution paths so budgeted execution now fails only when reservation creation fails, keeping enforcement inside the reservation boundary
  - updated runtime compatibility assertions to check released reservations and zeroed projections on failed budget-accounted runs without provider token metadata
- Verification:
  - `cargo test -p octopus-runtime-adapter token_usage_tests -- --nocapture` -> pass
  - `cargo test -p octopus-runtime-adapter runtime_compatibility_tests -- --nocapture` -> pass
- Blockers:
  - none
- Next:
  - Task 7 Step 4: reject unsupported budget accounting modes before execution instead of failing after the model call returns

## Checkpoint 2026-04-19 03:49

- Batch: Task 7 Step 4
- Completed:
  - added a shared budget accounting-mode guard so budget-enforced configured models reject `estimated` and `non_billable` accounting before probe or runtime execution starts
  - wired the same guard into registry validation so `validate_config`, `save_config`, and probe validation now surface `budgetPolicy.accountingMode` errors that point callers to `provider_reported`
  - normalized actor-template default model refs like `opus` into concrete configured model ids when freezing session policy, keeping actor defaults compatible with the rebuilt execution-target contract
  - narrowed session-creation execution validation so explicitly disabled configured models still produce deny-hidden target decisions instead of aborting session creation, while unsupported runtime classes continue to fail fast
  - repaired the owner-ceiling runtime-config test fixture so it seeds the `quota-model` configured model expected by the test
- Verification:
  - `cargo test -p octopus-runtime-adapter runtime_config_validation_rejects_unsupported_budget_accounting_modes -- --nocapture` -> pass
  - `cargo test -p octopus-runtime-adapter unsupported_budget_accounting_modes_fail_before_runtime_execution_starts -- --nocapture` -> pass
  - `cargo test -p octopus-runtime-adapter --lib` -> pass
- Blockers:
  - none
- Next:
  - Task 8 Step 1: update workspace model console labels and affordances to reflect execution class, upstream streaming truth, tool-loop truth, credential health, and budget-policy sections

## Checkpoint 2026-04-19 04:06

- Batch: Task 8 Step 1 -> Task 8 Step 2
- Completed:
  - updated the workspace model console list/detail surfaces so configured-model rows and detail panes expose execution class, upstream streaming truth, tool-loop truth, conversation eligibility, credential health, and budget-policy summaries directly from runtime truth
  - narrowed conversation-facing configured-model pickers to `agent_conversation` models while keeping generation-only models visible in the workspace model console with their real execution classification
  - rewrote the model module architecture and runtime-config contract docs so `executionProfile` and `budgetPolicy` are the canonical runtime terms for model executability and budget governance
- Verification:
  - `pnpm check:frontend` -> pass
  - `rg -n "RuntimeExecutionSupport|tokenQuota|quota" docs/plans/model/2026-04-18-model-module-architecture.md docs/runtime_config_api.md` -> pass
  - `rg -n "RuntimeExecutionSupport|tokenQuota" docs apps/desktop/src crates/octopus-runtime-adapter/src packages/schema/src contracts/openapi/src` -> remaining hits only in historical/active plan documents; no runtime code, schema, OpenAPI source, desktop source, or current canonical docs describe them as current truth
- Blockers:
  - none
- Next:
  - Task 8 Step 3: run the full backend, contract, and frontend verification matrix for the rebuilt end-state architecture

## Checkpoint 2026-04-19 04:10

- Batch: Task 8 Step 3
- Completed:
  - ran the full end-state verification matrix across Rust core/provider/runtime/server/infra crates, OpenAPI bundling, schema generation/parity, and desktop/website frontend checks
  - confirmed the rebuilt execution-profile and budget-policy architecture passes without restoring any legacy conversation fallback or quota-based transport contract
- Verification:
  - `cargo test -p octopus-core --lib` -> pass
  - `cargo test -p api --lib` -> pass
  - `cargo test -p octopus-runtime-adapter --lib` -> pass
  - `cargo test -p octopus-runtime-adapter --test registry_execution_support` -> pass
  - `cargo test -p octopus-runtime-adapter --test canonical_model_policy` -> pass
  - `cargo test -p octopus-runtime-adapter --test protocol_drivers` -> pass
  - `cargo test -p octopus-runtime-adapter --test runtime_turn_loop` -> pass
  - `cargo test -p octopus-runtime-adapter --test simple_completion` -> pass
  - `cargo test -p octopus-server --lib` -> pass
  - `cargo test -p octopus-infra --lib` -> pass
  - `pnpm openapi:bundle` -> pass
  - `pnpm schema:generate` -> pass
  - `pnpm schema:check` -> pass
  - `pnpm check:frontend` -> pass
- Blockers:
  - none
- Next:
  - plan complete
