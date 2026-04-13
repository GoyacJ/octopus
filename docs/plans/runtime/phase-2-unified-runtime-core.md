# Phase 2 Unified Runtime Core

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the current one-shot runtime session path with a stateful, resumable, multi-turn runtime core that consumes compiled actor or team manifests, freezes session policy at start, emits lineage-safe runs and subruns, and prepares the platform for unified capability planning.

**Architecture:** Phase 2 is runtime-core-first. It upgrades the session, run, and event model, introduces manifest and policy compilation, and routes single-agent primary execution through a turn engine instead of direct `submit_turn` state mutation. Team orchestration remains phase-separated, but the data model must be subrun-ready now.

**Tech Stack:** OpenAPI runtime contracts, feature-based schema files in `packages/schema`, Rust execution modules in `crates/octopus-runtime-adapter`, conversation loop primitives in `crates/runtime`, capability primitives in `crates/tools`, SQLite projections, JSONL runtime events.

---

## Fixed Decisions

- `crates/octopus-runtime-adapter/src/turn_submit.rs` stops being the execution brain
- runtime execution always starts from compiled `ActorManifest` or `TeamManifest`
- session creation freezes `SessionPolicy` and `ConfigSnapshot`
- primary run execution moves to a turn engine with streaming, retry, budget, and max-turn guards
- run lineage must be subrun-ready in this phase even if full team orchestration lands later
- no new runtime behavior may be added directly to the legacy one-shot path during this phase

## Scope

This phase covers:

- runtime session and run contract expansion
- actor and team manifest compilation
- session policy freeze
- turn context and turn engine introduction
- event taxonomy expansion
- primary-run cutover to runtime core

This phase does not cover:

- full team worker mailbox runtime
- full workflow runtime
- durable memory writeback
- final legacy deletion

## Task 1: Expand Runtime Contracts And Split Feature Files

**Files:**
- Modify: `contracts/openapi/src/components/schemas/runtime.yaml`
- Modify: `contracts/openapi/src/paths/runtime.yaml`
- Create: `packages/schema/src/actor-manifest.ts`
- Create: `packages/schema/src/agent-runtime.ts`
- Create: `packages/schema/src/memory-runtime.ts`
- Modify: `packages/schema/src/runtime.ts`
- Modify: `packages/schema/src/knowledge.ts`
- Modify: `packages/schema/src/index.ts`

**Implement:**
- split runtime handwritten types into feature files instead of growing `packages/schema/src/runtime.ts`
- add manifest, session policy, lineage, and event family fields to runtime transport
- keep `/api/v1/runtime/*` as the single public runtime surface

**Required session fields:**
- `selectedActorRef`
- `manifestRevision`
- `sessionPolicy`
- `activeRunId`
- `subrunCount`
- `memorySummary`
- `capabilitySummary`

**Required run fields:**
- `runKind`
- `parentRunId`
- `actorRef`
- `delegatedByToolCallId`
- `approvalState`
- `usageSummary`
- `artifactRefs`
- `traceContext`

**Required event families:**
- `planner.*`
- `model.*`
- `tool.*`
- `skill.*`
- `mcp.*`
- `subrun.*`
- `memory.*`
- `approval.*`
- `trace.*`

**Verification:**
```bash
pnpm openapi:bundle
pnpm schema:generate
pnpm schema:check
```

**Done when:**
- runtime contracts represent manifest-aware sessions and lineage-aware runs
- schema layout follows feature boundaries

## Task 2: Introduce Manifest And Session Policy Compilation

**Files:**
- Create: `crates/octopus-runtime-adapter/src/actor_manifest.rs`
- Create: `crates/octopus-runtime-adapter/src/session_policy.rs`
- Modify: `crates/octopus-runtime-adapter/src/actor_context.rs`
- Modify: `crates/octopus-runtime-adapter/src/session_service.rs`
- Modify: `crates/octopus-runtime-adapter/src/adapter_state.rs`
- Modify: `crates/octopus-runtime-adapter/src/lib.rs`

**Implement:**
- compile agent and team assets into immutable runtime manifests at session start
- derive and freeze `SessionPolicy` from:
  - selected actor or team
  - selected configured model
  - execution permission mode
  - config snapshot
  - workspace and project authorization state
- persist manifest revision and session policy revision into runtime projections

**Required manifest fields:**
- identity or profile
- task-domain profile
- default model strategy
- capability policy
- permission envelope
- memory policy
- delegation policy
- approval preference
- output contract
- shared capability or shared memory policy when applicable

**Verification:**
```bash
cargo test -p octopus-runtime-adapter actor
```

**Done when:**
- session creation persists compiled manifest metadata
- runtime no longer relies on raw prompt text as the authoring truth

## Task 3: Add Run Context And Turn Engine Modules

**Files:**
- Create: `crates/octopus-runtime-adapter/src/run_context.rs`
- Create: `crates/octopus-runtime-adapter/src/agent_runtime_core.rs`
- Create: `crates/octopus-runtime-adapter/src/trace_context.rs`
- Modify: `crates/octopus-runtime-adapter/src/lib.rs`
- Modify: `crates/runtime/src/conversation/turn_orchestrator.rs`

**Implement:**
- create a runtime-owned `RunContext` that contains frozen session, manifest, policy, trace, and capability inputs
- create an `AgentRuntimeCore` or equivalent turn engine entrypoint
- thread trace context through session, run, event, and tool boundaries
- reuse `crates/runtime` loop primitives where they are correct, but keep adapter projection logic outside the loop

**Turn engine responsibilities:**
- build turn context
- invoke capability planning
- invoke model loop
- collect tool or planning outcomes
- return structured run results for projection

**Verification:**
```bash
cargo test -p runtime conversation
cargo test -p octopus-runtime-adapter submit_turn
```

**Done when:**
- the runtime has a dedicated turn engine module
- `turn_submit` can delegate instead of orchestrating directly

## Task 4: Expand Event Taxonomy And Projection Pipeline

**Files:**
- Modify: `crates/octopus-runtime-adapter/src/execution_events.rs`
- Modify: `crates/octopus-runtime-adapter/src/persistence.rs`
- Modify: `crates/octopus-runtime-adapter/src/event_bus.rs`
- Modify: `crates/octopus-runtime-adapter/src/adapter_tests.rs`
- Modify: `apps/desktop/test/openapi-transport.test.ts`

**Implement:**
- project new planner, model, tool, skill, subrun-ready, approval, and trace event families
- keep SSE and polling contracts stable while expanding the payload model
- ensure event and trace projections are reconstructable from JSONL plus SQLite state

**Required projection changes:**
- explicit run lineage references
- trace context fields
- approval-layer metadata
- capability summary deltas
- usage summary deltas

**Verification:**
```bash
cargo test -p octopus-runtime-adapter runtime
pnpm -C apps/desktop test openapi-transport
```

**Done when:**
- runtime projections are rich enough to debug the new engine
- event taxonomy no longer collapses everything into generic message or trace updates

## Task 5: Cut Single-Agent Primary Runs Over To Runtime Core

**Files:**
- Modify: `crates/octopus-runtime-adapter/src/turn_submit.rs`
- Modify: `crates/octopus-runtime-adapter/src/execution_service.rs`
- Modify: `crates/octopus-runtime-adapter/src/executor.rs`
- Modify: `crates/tools/src/capability_runtime.rs`
- Modify: `crates/tools/src/skill_runtime.rs`
- Modify: `crates/octopus-runtime-adapter/src/adapter_tests.rs`

**Implement:**
- change `submit_turn` from direct state mutation around a single provider call into:
  - session lookup
  - run context creation
  - capability planning
  - turn engine execution
  - projection and persistence
- keep the first cut limited to single-agent primary runs
- ensure the result is subrun-ready but do not implement full team worker execution yet

**Required runtime behavior in this phase:**
- manifest-aware prompt assembly
- turn-scoped model or effort overrides
- max-turn guard
- budget or usage tracking
- streaming-capable execution path
- structured failure reporting

**Verification:**
```bash
cargo test -p octopus-runtime-adapter submit_turn
cargo test -p octopus-runtime-adapter
```

**Done when:**
- the active single-agent path no longer depends on one-shot `ExecutionResponse` semantics
- run state comes back from runtime core, not from manual adapter mutation

## Task 6: Make The Runtime Data Model Subrun-Ready

**Files:**
- Create: `crates/octopus-runtime-adapter/src/subrun_runtime.rs`
- Modify: `crates/octopus-runtime-adapter/src/session_service.rs`
- Modify: `crates/octopus-runtime-adapter/src/adapter_state.rs`
- Modify: `crates/octopus-runtime-adapter/src/execution_events.rs`
- Modify: `packages/schema/src/workbench.ts`

**Implement:**
- add lineage-safe data structures for `subrunCount`, `parentRunId`, and `delegatedByToolCallId`
- do not implement full team orchestration yet
- make session and event projections capable of hosting delegated runs without schema churn later

**Verification:**
```bash
cargo test -p octopus-runtime-adapter team
```

**Done when:**
- runtime projections can represent delegated runs cleanly
- Phase 4 can add actual team orchestration without breaking contracts from Phase 2

## Task 7: Phase-Level Validation And Deletion Fence

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
git diff --stat -- \
  contracts/openapi/src/components/schemas/runtime.yaml \
  contracts/openapi/src/paths/runtime.yaml \
  packages/schema/src \
  crates/octopus-runtime-adapter/src \
  crates/runtime/src/conversation/turn_orchestrator.rs \
  crates/tools/src/capability_runtime.rs \
  crates/tools/src/skill_runtime.rs \
  apps/desktop/test/openapi-transport.test.ts
```

**Acceptance criteria:**
- runtime sessions start with compiled manifest and frozen session policy
- the single-agent primary execution path uses the new turn engine
- runtime contracts and events are lineage-aware and subrun-ready
- no new behavior was added to the legacy one-shot path except the minimum bridge needed to reach the new engine

## Handoff To Later Phases

Phase 2 is complete only when later phases can assume:

- actor or team selection is manifest-driven
- session and run models are immutable-snapshot-friendly
- the primary run loop is stateful and resumable
- event and trace projections are rich enough to support team, workflow, memory, and approval phases

The next major execution packages are:

- `Phase 3: Capability Runtime Consolidation`
- `Phase 4: Team And Workflow Runtime`
- `Phase 5: Memory Plane Rebuild`
