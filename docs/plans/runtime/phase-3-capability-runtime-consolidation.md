# Phase 3 Capability Runtime Consolidation

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Consolidate builtin, skill, MCP, and plugin capabilities into one planned and executed runtime trunk so every model-visible tool surface, every prompt skill, and every provider-backed capability flows through `CapabilitySpec -> CapabilityExecutionPlan -> CapabilityExecutor`.

**Architecture:** Phase 2 made sessions manifest-driven and moved primary runs under `AgentRuntimeCore`, but capability planning and execution still live mostly inside `crates/tools` while the adapter still owns a thinner execution path. Phase 3 closes that gap. `crates/tools` becomes the canonical capability subsystem, `crates/octopus-runtime-adapter` consumes it through explicit planner and executor bridges, and runtime checkpoints, projections, and SSE events become capability-aware instead of prompt-centric.

**Tech Stack:** OpenAPI runtime contracts under `contracts/openapi/src/**`, feature-based schema files under `packages/schema/src/*`, capability runtime modules in `crates/tools`, runtime-core modules in `crates/octopus-runtime-adapter`, loop primitives in `crates/runtime`, SQLite runtime projections, append-only JSONL runtime events.

---

## Fixed Decisions

- `CapabilitySpec -> CapabilityExecutionPlan -> CapabilityExecutor` is the only canonical capability path.
- planning is deny-before-expose on every model iteration, not a one-time session bootstrap filter
- skill remains a prompt capability, not a plain function tool
- MCP and plugin integrations are provider families inside the same runtime trunk, not sidecar executors
- ordered concurrency is explicit runtime policy: read-safe capability calls may run in parallel, context-mutating or effectful calls stay serialized
- capability state is checkpointed and resumable; approval or auth must resume from persisted capability state, not by replaying a fresh turn
- no remaining frontend or adapter compatibility shim may preserve pre-Phase-2 turn actor or model override semantics

## Scope

This phase covers:

- capability transport and projection hardening
- capability runtime module decomposition in `crates/tools`
- planner and executor bridge integration into `AgentRuntimeCore`
- session capability state and checkpoint persistence
- ordered concurrency, mediation, and degraded outcome modeling
- deletion of remaining legacy tool, skill, MCP, plugin, and compatibility side paths from the primary runtime path

This phase does not cover:

- leader-orchestrated team runtime
- workflow or background-run orchestration
- durable memory writeback and relevance selection
- final business authorization merge across all runtime surfaces

## Current Baseline

The latest codebase already has the right core capability shapes, but they are not yet the only runtime trunk.

- `crates/tools/src/capability_runtime.rs` already owns `CapabilitySpec`, `CapabilitySurface`, `CapabilityExecutionPlan`, `CapabilityProvider`, `CapabilityExecutor`, `SessionCapabilityState`, and `SessionCapabilityStore`.
- `crates/tools/src/skill_runtime.rs` already returns structured `SkillExecutionResult` with injected messages, tool grants, overrides, and state updates.
- `crates/octopus-runtime-adapter/src/agent_runtime_core.rs` still owns the primary runtime path but does not yet consume a real per-iteration capability plan as the execution truth.
- `crates/tools/src/builtin_exec.rs` still contains live compatibility shims for `SkillDiscovery` and `SkillTool`.
- `crates/tools/src/capability_runtime.rs` still splits actual dispatch between `BuiltinOrPlugin` and `RuntimeCapability`, while `crates/plugins/src/hook_dispatch.rs` and `crates/plugins/src/lifecycle.rs` still spawn plugin commands directly.
- `crates/tools/src/tool_registry.rs` and `crates/compat-harness/src/lib.rs` are still static manifest or reference surfaces and must not grow back into runtime discovery or dispatch dependencies.

## Task 1: Harden Capability Runtime Contracts And Remove Remaining Turn Compatibility

**Files:**
- Modify: `contracts/openapi/src/components/schemas/runtime.yaml`
- Modify: `contracts/openapi/src/paths/runtime.yaml`
- Create: `packages/schema/src/capability-runtime.ts`
- Modify: `packages/schema/src/runtime.ts`
- Modify: `packages/schema/src/index.ts`
- Modify: `packages/schema/src/workbench.ts`
- Modify: `apps/desktop/test/openapi-transport.test.ts`
- Modify: `apps/desktop/test/runtime-store.test.ts`

**Implement:**
- move capability-runtime transport shapes out of the generic runtime hand-written wrapper surface
- remove the remaining TypeScript compatibility wrapper that still accepts legacy turn actor or model selection fields
- expose planner surface, provider state, mediation state, and execution outcome data as first-class runtime transport instead of opaque JSON fragments
- keep `/api/v1/runtime/*` as the single public runtime surface; capability details remain runtime projections, not a new side API

**Required contract groups:**
- `RuntimeCapabilityProviderState`
- `RuntimeCapabilityPlanSummary`
- `RuntimeCapabilitySurface`
- `RuntimeCapabilityStateSnapshot`
- `RuntimeCapabilityExecutionOutcome`

**Required capability-plan fields:**
- `visibleTools`
- `deferredTools`
- `discoverableSkills`
- `availableResources`
- `hiddenCapabilities`
- `activatedTools`
- `grantedTools`
- `pendingTools`
- `approvedTools`
- `authResolvedTools`
- `providerFallbacks`

**Verification:**
```bash
pnpm openapi:bundle
pnpm schema:generate
pnpm schema:check
pnpm -C apps/desktop exec vitest run test/openapi-transport.test.ts test/runtime-store.test.ts
```

**Done when:**
- runtime transport can describe the real capability surface and mediation state without ad hoc payload blobs
- submit-turn transport no longer accepts pre-Phase-2 actor or model override fields anywhere on the main path

## Task 2: Split `crates/tools` Capability Runtime Into Explicit Modules

**Files:**
- Create: `crates/tools/src/capability_runtime/mod.rs`
- Create: `crates/tools/src/capability_runtime/provider.rs`
- Create: `crates/tools/src/capability_runtime/planner.rs`
- Create: `crates/tools/src/capability_runtime/executor.rs`
- Create: `crates/tools/src/capability_runtime/state.rs`
- Create: `crates/tools/src/capability_runtime/events.rs`
- Modify: `crates/tools/src/skill_runtime.rs`
- Modify: `crates/tools/src/builtin_exec.rs`
- Modify: `crates/tools/src/tool_registry.rs`
- Modify: `crates/tools/src/lib.rs`
- Modify: `crates/tools/src/split_module_tests.rs`

**Implement:**
- break the current `crates/tools/src/capability_runtime.rs` monolith into provider, planner, executor, state, and event modules
- keep `ToolRegistry` as a static builtin manifest helper only, not a runtime discovery or dispatch hub
- keep prompt-skill execution under `skill_runtime.rs`, but make its runtime boundary explicit and capability-owned
- preserve provider-backed prompt skills and resources as executor-registered runtime capabilities, not implicit shell injection or bespoke MCP call sites
- keep the current `BuiltinOrPlugin` versus `RuntimeCapability` distinction internal to the executor boundary; callers must not branch on it

**Required module boundaries:**
- `CapabilityProvider`
- `CapabilityPlanner`
- `CapabilityExecutor`
- `SessionCapabilityState` and `SessionCapabilityStore`
- `CapabilityExecutionEvent`
- `McpCapabilityProvider`

**Required runtime rules:**
- prompt skills without registered runtime executors stay hidden from discovery
- MCP resources stay planned separately from visible tool exposure
- provider fallback metadata is planning output, not a public replacement API
- mediation and concurrency policy live in the executor, not in ad hoc tool wrappers

**Verification:**
```bash
cargo test -p tools
```

**Done when:**
- `capability_runtime.rs` stops being the de facto god module for capability behavior
- all runtime capability APIs still resolve through one canonical planner and executor surface after the split

## Task 3: Wire Real Capability Planning Into `AgentRuntimeCore`

**Files:**
- Create: `crates/octopus-runtime-adapter/src/capability_planner_bridge.rs`
- Create: `crates/octopus-runtime-adapter/src/capability_executor_bridge.rs`
- Create: `crates/octopus-runtime-adapter/src/capability_state.rs`
- Modify: `crates/octopus-runtime-adapter/src/run_context.rs`
- Modify: `crates/octopus-runtime-adapter/src/agent_runtime_core.rs`
- Modify: `crates/octopus-runtime-adapter/src/execution_target.rs`
- Modify: `crates/octopus-runtime-adapter/src/executor.rs`
- Modify: `crates/octopus-runtime-adapter/src/session_service.rs`
- Modify: `crates/octopus-runtime-adapter/src/lib.rs`

**Implement:**
- build a runtime-owned `CapabilityRuntime` from actor manifest policy, runtime config, plugin sources, skill packages, and MCP provider state
- build a fresh `CapabilityExecutionPlan` before each model request, not just once per submitted turn
- pass only planned tool definitions and discoverable prompt skills into the model loop
- persist and restore `SessionCapabilityStore` alongside the session checkpoint so tool grants, pending approvals, auth resolution, skill injections, and model overrides survive resume
- re-plan when capability state changes through `ToolSearch`, skill execution, approval resolution, auth resolution, or provider degradation

**Required runtime behavior:**
- `AgentRuntimeCore` does not synthesize planner events without a real planner result
- model-visible tool exposure always comes from `planned_tool_definitions`
- skill execution feeds back `messagesToInject`, `toolGrants`, `modelOverride`, `effortOverride`, and `stateUpdates`
- MCP pending or degraded state changes capability visibility before execution time
- plugin-provided tools and prompt skills use the same planner and executor bridges as builtin and MCP sources
- builtin execution, plugin command execution, and runtime capability dispatch all pass through the same mediation and tracing boundary before any local process runs

**Verification:**
```bash
cargo test -p octopus-runtime-adapter
cargo test -p runtime conversation
```

**Done when:**
- the primary runtime path uses a real capability plan as execution input
- capability state is part of the run context and checkpoint instead of being recomputed loosely around one provider call

## Task 4: Normalize Mediation, Ordered Concurrency, And Resume

**Files:**
- Modify: `crates/runtime/src/conversation.rs`
- Modify: `crates/runtime/src/conversation/turn_orchestrator.rs`
- Modify: `crates/tools/src/capability_runtime/executor.rs`
- Modify: `crates/octopus-runtime-adapter/src/approval_flow.rs`
- Modify: `crates/octopus-runtime-adapter/src/agent_runtime_core.rs`
- Modify: `crates/octopus-runtime-adapter/src/persistence.rs`
- Modify: `crates/octopus-runtime-adapter/src/adapter_tests.rs`

**Implement:**
- replace the remaining permission-prompt-centric tool mediation in the loop with capability mediation outcomes
- route approval-required, auth-required, denied, degraded, interrupted, and cancelled states through one structured runtime outcome model
- keep ordered concurrency explicit: `ParallelRead` may run concurrently, `Serialized` stays single-file and context-safe
- checkpoint pending capability requests with capability identity, provider key, dispatch kind, mediation reason, and concurrency policy so resume continues the blocked invocation instead of re-running the whole turn

**Required mediation outcomes:**
- `allow`
- `require_approval`
- `require_auth`
- `deny`
- `cancelled`
- `interrupted`
- `degraded`

**Required checkpoint fields:**
- `capabilityId`
- `toolName`
- `dispatchKind`
- `providerKey`
- `requiredPermission`
- `requiresApproval`
- `requiresAuth`
- `concurrencyPolicy`
- `input`
- `reason`

**Verification:**
```bash
cargo test -p runtime
cargo test -p tools
cargo test -p octopus-runtime-adapter
```

**Done when:**
- approval or auth resume continues from capability checkpoint state, not from a fresh turn replay
- concurrency policy is enforced by runtime-owned capability execution semantics rather than scattered caller assumptions

## Task 5: Expand Capability Event And Projection Persistence

**Files:**
- Modify: `crates/octopus-runtime-adapter/src/execution_events.rs`
- Modify: `crates/octopus-runtime-adapter/src/persistence.rs`
- Modify: `crates/octopus-runtime-adapter/src/event_bus.rs`
- Modify: `crates/octopus-runtime-adapter/src/session_service.rs`
- Modify: `crates/octopus-runtime-adapter/src/adapter_state.rs`
- Modify: `apps/desktop/test/tauri-client-runtime.test.ts`

**Implement:**
- project planner, tool, skill, MCP, approval, and trace events from real capability runtime hooks instead of adapter-synthesized placeholders
- persist queryable capability plan summaries in SQLite and append full-fidelity event history to `runtime/events/*.jsonl`
- keep large or replay-only capability plan bodies in runtime disk artifacts when needed; do not stuff large plan snapshots into SQLite as the primary body
- make session and run projections reflect current capability plan state, provider health, pending mediation, and capability-state deltas

**Required projection changes:**
- capability plan summary on session and run detail
- provider state summary and degraded-state markers
- pending capability mediation summary
- injected skill message count
- granted tool count
- hidden or deferred capability count

**Verification:**
```bash
cargo test -p octopus-runtime-adapter
pnpm -C apps/desktop exec vitest run test/tauri-client-runtime.test.ts
```

**Done when:**
- replay and audit can explain why a capability was visible, hidden, deferred, blocked, or degraded
- capability state is recoverable from SQLite projections plus JSONL event logs

## Task 6: Delete Remaining Legacy Capability Side Paths

**Files:**
- Modify: `crates/tools/src/builtin_exec.rs`
- Modify: `crates/tools/src/capability_runtime.rs`
- Modify: `crates/tools/src/tool_registry.rs`
- Modify: `crates/plugins/src/hook_dispatch.rs`
- Modify: `crates/plugins/src/lifecycle.rs`
- Modify: `crates/compat-harness/src/lib.rs`
- Modify: `crates/tools/src/lib.rs`
- Modify: `crates/octopus-runtime-adapter/src/turn_submit.rs`
- Modify: `crates/octopus-runtime-adapter/src/execution_service.rs`
- Verify only: `docs/capability_runtime.md`

**Implement:**
- remove any primary-runtime dependency on direct builtin skill compat shims
- fence `builtin_exec` helpers to non-runtime compatibility or test-only usage if they must survive temporarily, and schedule immediate deletion once no public call site remains
- fence direct plugin tool and lifecycle command spawning as provider-internal implementation only; no runtime caller may treat `crates/plugins` as a parallel execution trunk
- ensure `capability_runtime` dispatch branching remains internal and does not leak into adapter orchestration, plugin callers, or future team runtime
- ensure no primary runtime path discovers tools from `ToolRegistry` directly or dispatches capability calls without planner resolution
- ensure `compat-harness` remains import or translation reference code only and never becomes a live capability provider
- update `docs/capability_runtime.md` if the canonical capability entrypoints or retired entrypoints list changes

**Deletion gate checks:**
```bash
rg -n "run_skill_discovery_with_runtime|run_skill_tool_with_runtime|execute_runtime_skill_compat_tool" crates/tools crates/octopus-runtime-adapter crates/runtime
rg -n "BuiltinOrPlugin|RuntimeCapability|execute_local_tool|PluginTool::execute" crates/tools crates/octopus-runtime-adapter crates/plugins
rg -n "ToolRegistry|compat-harness|compat_harness" crates/tools crates/octopus-runtime-adapter crates/runtime crates/plugins crates/compat-harness
```

**Done when:**
- the primary runtime path has no direct legacy skill side path left
- plugin command spawning is mediated only through the capability executor and never through a parallel runtime path
- `ToolRegistry` and `compat-harness` are no longer runtime discovery or dispatch dependencies

## Task 7: Phase-Level Validation And Non-Coding Acceptance Fence

**Files:**
- Verify only

**Run:**
```bash
pnpm openapi:bundle
pnpm schema:generate
pnpm schema:check
cargo test -p runtime
cargo test -p tools
cargo test -p octopus-runtime-adapter
cargo test -p octopus-platform
cargo test -p octopus-server
pnpm -C apps/desktop exec vitest run test/openapi-transport.test.ts test/runtime-store.test.ts test/tauri-client-runtime.test.ts
git diff --stat -- \
  contracts/openapi/src/components/schemas/runtime.yaml \
  contracts/openapi/src/paths/runtime.yaml \
  packages/schema/src \
  crates/runtime/src/conversation.rs \
  crates/runtime/src/conversation/turn_orchestrator.rs \
  crates/tools/src \
  crates/plugins/src \
  crates/compat-harness/src \
  crates/octopus-runtime-adapter/src \
  apps/desktop/test
```

**Acceptance criteria:**
- all runtime-visible capabilities come from the planner
- the model sees only planned tool definitions and discoverable prompt skills from the active capability surface
- builtin, skill, MCP, and plugin execution flow through one runtime executor contract
- approval, auth, degraded, cancelled, and interrupted outcomes are explicit runtime states
- ordered concurrency works for parallel read-safe capability calls without corrupting runtime context
- a non-coding `research/docs` agent runs through the same capability trunk as a coding agent
- replay from SQLite projections plus JSONL events can explain capability visibility and execution outcomes

## Handoff To Later Phases

Phase 3 is complete only when later phases can assume:

- capability planning is the only way model-visible tools and skills appear
- session capability state is checkpointed, resumable, and projection-friendly
- plugin and MCP capability providers no longer create side execution trunks
- team and workflow runtime can reuse the same capability planner, executor, and mediation model instead of inventing another dispatch path

The next major execution packages are:

- `Phase 4: Team And Workflow Runtime`
- `Phase 5: Memory Plane Rebuild`
- `Phase 6: Policy And Approval Merge`
