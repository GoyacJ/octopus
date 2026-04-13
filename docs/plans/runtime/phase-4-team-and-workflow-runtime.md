# Phase 4 Team And Workflow Runtime

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Turn team delegation, worker lifecycle, workflow execution, mailbox handoff, artifact lineage, and background completion into native runtime behavior on top of the Phase 3 capability trunk.

**Architecture:** Phase 3 is assumed complete before this phase starts. That means `CapabilitySpec -> CapabilityExecutionPlan -> CapabilityExecutor` is already the only capability trunk for every run. Phase 4 builds the next layer on top of it: `Session -> Run -> Subrun` becomes the durable orchestration substrate for leader-orchestrated team execution and workflow execution, while mailbox state, artifact handoff, and background progress become runtime-native projections instead of tool-local side effects.

**Tech Stack:** OpenAPI runtime and workspace contracts under `contracts/openapi/src/**`, feature-based schema files under `packages/schema/src/*`, orchestration modules in `crates/octopus-runtime-adapter`, loop primitives in `crates/runtime`, capability and compat fences in `crates/tools`, SQLite projections, append-only JSONL runtime events, disk-backed checkpoint and mailbox artifacts under `runtime/`, and artifact bodies under `data/artifacts`.

---

## Fixed Decisions

- Phase 4 starts only after Phase 3 deletion gates pass. Team and workflow runtime must reuse the Phase 3 planner, executor, mediation, and checkpoint model rather than inventing a second execution trunk.
- Team runtime stays `leader-orchestrated` in this phase. The leader is a primary run or parent subrun; workers are durable subruns with explicit lineage.
- Workflow runtime is a first-class runtime substrate, not a plugin-only or cron-only feature.
- `RuntimeRunKind` remains rooted in `primary` and `subrun`; workflow and background semantics are added through explicit workflow and background projections instead of replacing the run lineage model.
- Worker state is isolated. No worker may share mutable prompt state, mutable capability state, or mutable checkpoint state directly with the leader or with sibling workers.
- Mailbox handoff and artifact handoff are first-class runtime contracts. String-only prompt stitching is not an acceptable delegation protocol.
- Existing task, worker, team, and cron helpers in `crates/tools` may survive only as internal primitives or compatibility surfaces. They must not become the primary runtime orchestration root.
- This phase does not pull durable memory writeback, full business authorization merge, or general automation scheduling into scope.

## Scope

This phase covers:

- runtime transport for team, worker, workflow, mailbox, handoff, and background state
- leader-orchestrated team execution inside the main runtime trunk
- worker spawn, suspend, resume, cancel, and background continuation
- mailbox and artifact handoff lineage
- workflow runtime state machines and workflow projections
- restart-safe persistence for subruns, workflows, and background completion
- desktop and server parity on the same runtime contract
- deletion or fencing of prompt-centric team and background side paths

This phase does not cover:

- durable memory selection or writeback changes from Phase 5
- final business authorization and approval merge from Phase 6
- workflow-template authoring UX or bundle translation redesign
- standalone cron or automation product surfaces beyond the runtime substrate needed to continue already-started background work
- a second orchestration substrate specialized for coding-only agents

## Current Baseline

The current repository is already partially prepared for Phase 4, but the execution substrate is not finished.

- Team assets already carry runtime-relevant fields in workspace contracts, including `teamTopology`, `mailboxPolicy`, `workflowAffordance`, `workerConcurrencyLimit`, `leaderRef`, and `memberRefs`.
- `WorkflowAffordance` already exposes `backgroundCapable` and `automationCapable`, which means the asset plane is already expressing runtime expectations for background work.
- Runtime session and run transport already expose `subrunCount`, `runKind`, `parentRunId`, `delegatedByToolCallId`, and `traceContext`, so lineage-safe projections are in place.
- Runtime event kinds already include `subrun.spawned`, `subrun.completed`, and `subrun.failed`, but there is still no public `workflow.*` event family.
- Team sessions can already be created and selected from product surfaces, including the desktop conversation actor picker, but `crates/octopus-runtime-adapter/src/agent_runtime_core.rs` still hard-blocks team execution with `team_runtime_not_enabled`.
- The Phase 2 and Phase 3 runtime path already persists SQLite projections plus `runtime/events/*.jsonl`, so Phase 4 can extend a durable runtime base instead of inventing a new persistence model.
- `crates/tools/src/workspace_runtime.rs` already contains global task, worker, team, and cron registries, and `crates/tools/src/builtin_exec.rs` still exposes `Agent`, `TeamCreate`, `TeamDelete`, `Worker*`, `Task*`, and `Cron*` entrypoints. Those are useful evidence and possible low-level primitives, but they are not yet runtime-native orchestration.
- `crates/tools/src/tool_registry.rs` and related tests already preserve handoff metadata at the tool layer, but that is not yet the same thing as mailbox-native runtime lineage and recovery.

Phase 4 must start from this actual baseline while also assuming the intended Phase 3 end state:

- planner and executor are already the only capability trunk
- direct plugin or compat execution side paths are already fenced from the primary runtime
- capability mediation, checkpoints, and provider state are already resumable and projection-friendly

If those Phase 3 assumptions are not true in code yet, Phase 4 must not paper over them with team-specific shims.

## Task 1: Expand Runtime Contracts For Team, Workflow, Mailbox, And Background State

**Files:**
- Modify: `contracts/openapi/src/components/schemas/runtime.yaml`
- Modify: `contracts/openapi/src/components/schemas/workspace.yaml`
- Modify: `contracts/openapi/src/paths/runtime.yaml`
- Create: `packages/schema/src/team-runtime.ts`
- Create: `packages/schema/src/workflow-runtime.ts`
- Modify: `packages/schema/src/agent-runtime.ts`
- Modify: `packages/schema/src/runtime.ts`
- Modify: `packages/schema/src/index.ts`
- Modify: `apps/desktop/test/openapi-transport.test.ts`
- Modify: `apps/desktop/test/runtime-store.test.ts`
- Modify: `apps/desktop/test/support/workspace-fixture-runtime.ts`

**Implement:**
- add first-class runtime transport for worker subruns, mailbox state, handoff state, workflow state, and background completion
- keep `/api/v1/runtime/*` as the only public runtime surface; do not create a side API just for workflow or worker orchestration
- expose workflow and background projections through the same adapter contract used by desktop and browser hosts
- keep existing session and run lineage stable while adding workflow-specific projections and refs

**Required contract groups:**
- `RuntimeSubrunSummary`
- `RuntimeMailboxSummary`
- `RuntimeHandoffSummary`
- `RuntimeWorkflowSummary`
- `RuntimeWorkflowRunDetail`
- `RuntimeBackgroundRunSummary`

**Required public fields:**
- on `RuntimeSessionDetail` or `RuntimeSessionSummary`:
  - workflow summary
  - pending mailbox summary
  - background run summary
- on `RuntimeRunSnapshot`:
  - worker role or dispatch summary
  - workflow run ref
  - mailbox ref or handoff ref
  - background state
- on runtime event taxonomy:
  - keep `subrun.*`
  - add `workflow.started`
  - add `workflow.step.started`
  - add `workflow.step.completed`
  - add `workflow.completed`
  - add `workflow.failed`

**Verification:**
```bash
pnpm openapi:bundle
pnpm schema:generate
pnpm schema:check
pnpm -C apps/desktop exec vitest run test/openapi-transport.test.ts test/runtime-store.test.ts
```

**Done when:**
- runtime transport can describe team, workflow, mailbox, and background state without opaque JSON fragments
- desktop fixtures and store tests accept only the new contract shape

## Task 2: Introduce Runtime-Native Team Orchestrator And Worker Subrun Modules

**Files:**
- Create: `crates/octopus-runtime-adapter/src/subrun_orchestrator.rs`
- Create: `crates/octopus-runtime-adapter/src/team_runtime.rs`
- Create: `crates/octopus-runtime-adapter/src/worker_runtime.rs`
- Modify: `crates/octopus-runtime-adapter/src/agent_runtime_core.rs`
- Modify: `crates/octopus-runtime-adapter/src/run_context.rs`
- Modify: `crates/octopus-runtime-adapter/src/session_service.rs`
- Modify: `crates/octopus-runtime-adapter/src/lib.rs`
- Modify: `crates/runtime/src/conversation.rs`
- Modify: `crates/runtime/src/conversation/turn_orchestrator.rs`

**Implement:**
- replace the hard `team_runtime_not_enabled` stop with a leader-orchestrated runtime path
- compile team execution from frozen `TeamManifest` inputs, including leader, members, delegation edges, mailbox policy, artifact handoff rules, workflow affordances, and worker concurrency ceiling
- let the leader use the same Phase 3 capability planner and executor surface that single-agent runs use
- spawn each worker as a durable subrun with:
  - its own `RunContext`
  - its own capability state and checkpoint
  - explicit `parentRunId`
  - explicit `delegatedByToolCallId`
  - explicit target actor ref
- support worker suspend, resume, cancel, and completion without replaying the leader's whole turn

**Required runtime rules:**
- worker creation is a runtime orchestration action, not a plain tool callback
- worker concurrency is bounded by team manifest policy, not by ad hoc caller loops
- worker restarts resume from subrun checkpoint state when possible; they do not silently rebuild from mutable session prompt history
- capability mediation, approval, auth, tracing, and usage accounting stay inside the shared runtime trunk for leader and worker runs alike

**Verification:**
```bash
cargo test -p octopus-runtime-adapter team
cargo test -p runtime
```

**Done when:**
- a team session can run through the main runtime trunk instead of failing early
- worker lifecycle is represented as real subrun state, not as prompt text or tool-local metadata

## Task 3: Make Mailbox, Handoff, And Artifact Lineage First-Class Runtime State

**Files:**
- Create: `crates/octopus-runtime-adapter/src/mailbox_runtime.rs`
- Create: `crates/octopus-runtime-adapter/src/handoff_runtime.rs`
- Modify: `crates/octopus-runtime-adapter/src/persistence.rs`
- Modify: `crates/octopus-runtime-adapter/src/execution_events.rs`
- Modify: `crates/octopus-runtime-adapter/src/event_bus.rs`
- Modify: `crates/tools/src/tool_registry.rs`
- Modify: `crates/tools/src/split_module_tests.rs`

**Implement:**
- replace tool-local handoff metadata with runtime-owned mailbox and handoff records
- represent delegation outputs as structured handoff envelopes with sender run, receiver run or actor, mailbox channel, artifact refs, and acknowledgment state
- keep artifact bodies in `data/artifacts` while mailbox and handoff runtime bodies live under `runtime/` with SQLite refs and hashes
- let the leader or workflow engine collect worker outputs through mailbox summaries and explicit artifact lineage instead of stitching raw text back into a prompt

**Required lineage fields:**
- `sessionId`
- `runId`
- `parentRunId`
- `delegatedByToolCallId`
- `senderActorRef`
- `receiverActorRef`
- `mailboxRef`
- `artifactRefs`
- `handoffState`

**Required runtime behavior:**
- mailbox delivery and acknowledgment are replayable from SQLite projection plus JSONL events plus runtime artifact refs
- artifact lineage survives adapter restart and session reload
- handoff rules come from manifest policy, not from tool-specific conventions

**Verification:**
```bash
cargo test -p octopus-runtime-adapter runtime
cargo test -p tools
```

**Done when:**
- mailbox and artifact handoff can be queried and explained as runtime state
- handoff metadata no longer lives only inside tool-layer records

## Task 4: Add Workflow Runtime And Background Continuation On The Same Subrun Substrate

**Files:**
- Create: `crates/octopus-runtime-adapter/src/workflow_runtime.rs`
- Create: `crates/octopus-runtime-adapter/src/background_runtime.rs`
- Modify: `crates/octopus-runtime-adapter/src/subrun_orchestrator.rs`
- Modify: `crates/octopus-runtime-adapter/src/approval_flow.rs`
- Modify: `crates/octopus-runtime-adapter/src/session_service.rs`
- Modify: `crates/runtime/src/conversation/turn_orchestrator.rs`
- Modify: `crates/tools/src/workspace_runtime.rs`
- Modify: `crates/tools/src/builtin_exec.rs`

**Implement:**
- treat workflow execution as a runtime state machine that can orchestrate subruns, mailbox transitions, approval pauses, and artifact outputs
- map `workflowAffordance` from team and workflow assets into runtime-owned workflow execution inputs
- support background continuation for eligible workflow or worker runs without requiring the initiating foreground UI connection to remain attached
- keep background completion signaling runtime-native by projecting completion markers and emitting declared runtime events
- if existing task, worker, or cron helpers remain useful, consume them only as low-level primitives behind runtime orchestration boundaries

**Required workflow semantics:**
- workflow steps and delegated workers share one lineage model
- workflow approval or auth pauses resume the specific suspended workflow or worker node, not the entire session turn
- background continuation preserves trace, approval, artifact, and mailbox lineage
- coding and non-coding actors can both run through the same workflow substrate

**Out of scope in this task:**
- general automation scheduling UX
- a separate product-only cron control plane
- bypassing the runtime to run workflows directly from plugin hooks or tool registries

**Verification:**
```bash
cargo test -p octopus-runtime-adapter workflow
cargo test -p runtime
```

**Done when:**
- workflow execution survives restart and can continue from persisted state
- background work is represented as runtime-owned workflow or subrun state instead of detached helper registry state

## Task 5: Extend SQLite Projection, JSONL Events, And Disk Artifacts For Recovery

**Files:**
- Modify: `crates/octopus-infra/src/infra_state.rs`
- Modify: `crates/octopus-runtime-adapter/src/persistence.rs`
- Modify: `crates/octopus-runtime-adapter/src/execution_events.rs`
- Modify: `crates/octopus-runtime-adapter/src/adapter_state.rs`
- Modify: `crates/octopus-runtime-adapter/src/session_service.rs`
- Modify: `apps/desktop/test/tauri-client-runtime.test.ts`

**Implement:**
- add queryable projection fields for subrun state, mailbox state, workflow state, and background completion state
- keep full-fidelity event history in `runtime/events/*.jsonl`
- store large workflow checkpoints, mailbox bodies, and resume artifacts under runtime disk paths with hashes and refs persisted in SQLite
- keep artifact bodies under `data/artifacts` with runtime lineage stored as metadata, not duplicated content
- make restart and reload recovery depend on SQLite projections, JSONL events, and checkpoint artifacts, not on debug session JSON files

**Required projection summaries:**
- session-level worker and workflow counts
- active background run summary
- pending mailbox or handoff count
- latest workflow state summary
- run-level worker dispatch summary
- workflow node ref and current background state

**Required event fields for workflow and subrun events:**
- `sessionId`
- `runId`
- `parentRunId`
- `iteration`
- `workflowRunId`
- `workflowStepId`
- `actorRef`
- `toolUseId` or equivalent dispatch key
- `outcome`

**Verification:**
```bash
cargo test -p octopus-infra
cargo test -p octopus-runtime-adapter
pnpm -C apps/desktop exec vitest run test/tauri-client-runtime.test.ts
```

**Done when:**
- session and run state can be recovered after restart without losing workflow, worker, mailbox, or background lineage
- SQLite stores summary and refs while JSONL and runtime artifact paths remain the audit and replay source

## Task 6: Cut Over Desktop And Server Consumers, Then Fence Legacy Team And Background Side Paths

**Files:**
- Modify: `crates/octopus-server/src/workspace_runtime.rs`
- Modify: `crates/octopus-platform/src/workspace.rs`
- Modify: `apps/desktop/src/tauri/workspace-client.ts`
- Modify: `apps/desktop/src/stores/runtime.ts`
- Modify: `apps/desktop/src/views/project/ConversationView.vue`
- Modify: `apps/desktop/test/openapi-transport.test.ts`
- Modify: `apps/desktop/test/runtime-store.test.ts`
- Modify: `apps/desktop/test/tauri-client-runtime.test.ts`
- Modify: `crates/tools/src/builtin_exec.rs`
- Modify: `crates/tools/src/workspace_runtime.rs`

**Implement:**
- keep desktop and browser hosts on the same runtime contract while exposing team and workflow runtime state through the shared adapter
- update runtime store and fixtures so team selection, workflow progress, mailbox summaries, and background completion consume the new contract shape without local patching
- keep workspace team CRUD and project assignment surfaces, but stop treating them as evidence that runtime orchestration is solved
- fence or remove direct runtime dependencies on tool-level `Agent`, `Worker*`, `Task*`, `Team*`, and `Cron*` orchestration paths once the native runtime path exists

**Deletion gate checks:**
```bash
rg -n "team_runtime_not_enabled" crates/octopus-runtime-adapter crates/runtime
rg -n "run_agent|WorkerCreate|WorkerSendPrompt|TaskCreate|CronCreate|TeamCreate|TeamDelete" crates/octopus-runtime-adapter crates/runtime crates/tools
rg -n "workspace_runtime::|global_task_registry|global_worker_registry|global_cron_registry|global_team_registry" crates/octopus-runtime-adapter crates/runtime crates/tools
```

**Done when:**
- desktop consumes team and workflow runtime state from the same typed transport surface as the server
- old tool-side orchestration helpers are no longer the primary runtime path

## Task 7: Phase-Level Validation And Mixed-Domain Acceptance Fence

**Files:**
- Verify only

**Run:**
```bash
pnpm openapi:bundle
pnpm schema:generate
pnpm schema:check
cargo test -p runtime
cargo test -p tools
cargo test -p octopus-infra
cargo test -p octopus-runtime-adapter
cargo test -p octopus-platform
cargo test -p octopus-server
pnpm -C apps/desktop exec vitest run test/openapi-transport.test.ts test/runtime-store.test.ts test/tauri-client-runtime.test.ts
git diff --stat -- \
  contracts/openapi/src/components/schemas/runtime.yaml \
  contracts/openapi/src/components/schemas/workspace.yaml \
  contracts/openapi/src/paths/runtime.yaml \
  packages/schema/src \
  crates/runtime/src \
  crates/tools/src \
  crates/octopus-infra/src \
  crates/octopus-runtime-adapter/src \
  crates/octopus-platform/src \
  crates/octopus-server/src \
  apps/desktop/src \
  apps/desktop/test
```

**Acceptance criteria:**
- a team session can spawn multiple workers, preserve lineage, and survive restart
- workflow execution and background continuation use the same durable subrun substrate as team workers
- mailbox and artifact handoff state is queryable, auditable, and replayable
- approval, auth, trace, capability mediation, and usage accounting remain on the shared runtime trunk for leader, worker, and workflow runs
- coding and non-coding domains both run on the same orchestration substrate
- no prompt-centric team execution path remains on the primary runtime route

## Handoff To Later Phases

Phase 4 is complete only when later phases can safely assume:

- team execution is a native runtime concern, not a disabled placeholder
- workflow and background work reuse the same durable subrun lineage model
- mailbox and artifact handoff are governed runtime contracts
- old helper registries and prompt-centric delegation paths are no longer the execution truth

The next major packages remain:

- `Phase 5: Memory Plane Rebuild`
- `Phase 6: Policy And Approval Merge`
- `Phase 7: Contract And Host Projection`
- `Phase 8: Legacy Deletion`
