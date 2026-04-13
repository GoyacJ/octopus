# Octopus Agent Platform Rebuild Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Rebuild Octopus into a unified agent platform with first-class agent and team assets, one runtime trunk, a governed capability system, workflow-native execution, and typed durable memory.

**Architecture:** Keep the current asset management shell, desktop shell, and host adapters as product surfaces, but replace the execution center with a compiled-manifest, capability-planned, multi-turn runtime core. Migration is phase-driven and deletion-driven: each new workstream must retire the old path it supersedes.

**Tech Stack:** Rust workspace crates, OpenAPI source under `contracts/openapi/src/**`, feature-based schema files under `packages/schema/src/*`, desktop stores and adapters, SQLite projections, JSONL runtime events, disk-backed bodies under `data/*`.

---

## Execution Rules

The implementation program follows these hard rules:

- no compatibility-first design
- no permanent legacy shim
- no new runtime behavior on the old executor path
- no dual execution trunk at the end of any completed phase
- no god modules or oversized orchestrator files
- no coding-only acceptance criteria

This plan is complete only when the old one-shot and prompt-centric paths are deleted or fenced from further growth and scheduled for immediate removal in the next phase.

## Module Decomposition Rules

Implementation work must stay split across explicit runtime modules. The canonical module families are:

- `asset compiler`
- `import or export translator`
- `manifest compiler`
- `session policy compiler`
- `turn engine`
- `capability planner`
- `model driver`
- `tool or skill or MCP executor`
- `approval or auth broker`
- `subrun and team orchestrator`
- `memory selector and memory writer`
- `event projector`
- `trace and telemetry projector`

The rebuild must not collapse these into one adapter service or one catch-all runtime module.

## Phase 0: Baseline Reset

### Goal

Freeze the architectural baseline and stop the old runtime path from absorbing new product behavior.

### Add Or Change

- record the new design baseline in `docs/plans/runtime/agent-runtime-rebuild-design.md`
- rewrite this implementation plan as the canonical workstream record
- mark the old adapter executor as legacy in code comments and internal docs
- establish an explicit ownership map for asset plane, runtime plane, and experience plane

### Remove Or Fence

- no new capability, team, memory, or workflow behavior may land directly on the one-shot `submit_turn` path except temporary bridging required for cutover

### Public Contract Impact

- no user-visible contract change in this phase
- internal architecture docs become normative

### Persistence Impact

- none

### Failure Modes To Prevent

- continuing to ship new behavior through legacy adapter hooks
- keeping docs vague enough that implementers invent incompatible local patterns

### Acceptance Gate

- both runtime docs are rewritten to the platform model
- internal implementation work is blocked from adding new features to the legacy executor

### Exit Criteria

- architecture baseline is published
- legacy growth is frozen

## Phase 1: Asset Contract Rebuild

### Goal

Turn agent, team, skill, MCP, plugin, and workflow packaging into a runtime-relevant asset system.

### Add Or Change

- design and implement `AssetBundleManifest v2`
- add native asset schemas for `agent`, `team`, `skill`, `mcp-server`, `plugin`, and `workflow-template`
- upgrade `workspace-plane` contracts so agent and team assets carry runtime-facing fields
- introduce bundle dependency, trust, compatibility, and translation diagnostics
- split asset compiler responsibilities from import or export translation responsibilities

### Remove Or Fence

- retire the assumption that `AgentRecord` and `UpsertAgentInput` are sufficient runtime inputs
- retire the assumption that bundle manifests are only static packaging descriptors

### Public Contract Impact

- add schema groups for `asset-bundle`, `runtime-policy`, and richer agent/team definitions
- extend import or export endpoints to return translation results, trust warnings, dependency resolution state, and unsupported feature reports

### Persistence Impact

- store asset trust, dependency, and translation metadata in SQLite
- keep large imported asset bodies on disk where appropriate

### Failure Modes To Prevent

- importing foreign bundles and silently executing foreign semantics
- treating team assets as membership lists only
- shipping assets without trust or compatibility state

### Acceptance Gate

- an imported bundle can be translated into native asset rows and diagnostics
- a native bundle can be exported with complete dependency and trust metadata
- agent/team assets are rich enough to compile into runtime manifests

### Exit Criteria

- bundle v2 is canonical
- thin asset contracts are retired from new flows

## Phase 2: Unified Runtime Core

### Goal

Replace one-shot turn submission with a compiled-manifest, stateful, resumable runtime core.

### Add Or Change

- add `session`, `run`, and `subrun` state models with explicit lineage
- add `ActorManifest` and `TeamManifest` compilers
- add a `SessionPolicy` compiler and immutable policy snapshot
- add a `TurnContext` builder
- add a runtime `turn engine`
- add a runtime `model driver` that supports streaming, partial events, budget tracking, compaction, and resume

### Remove Or Fence

- stop using `turn_submit` as the primary execution brain
- stop deriving runtime behavior directly from raw asset rows and prompt fields

### Public Contract Impact

- session contracts grow `selectedActorRef`, `manifestRevision`, `sessionPolicy`, `activeRunId`, `subrunCount`, `memorySummary`, and `capabilitySummary`
- run contracts grow `runKind`, `parentRunId`, `actorRef`, `delegatedByToolCallId`, `approvalState`, `usageSummary`, and `artifactRefs`

### Persistence Impact

- add manifest revision storage
- add session policy snapshots
- add run and subrun lineage projections

### Failure Modes To Prevent

- preserving hidden one-shot assumptions in the new loop
- mutating active sessions from config changes after session start
- letting primary and subrun state leak across trace contexts

### Acceptance Gate

- a single-agent session runs through the new multi-turn engine
- streaming, resume, budget guard, and max-turn guard work on the new path
- the runtime state can be reconstructed from projections and event logs

### Exit Criteria

- the unified turn engine is the only active path for new primary-run behavior

## Phase 3: Capability Runtime Consolidation

### Goal

Make builtin, skill, MCP, and plugin capabilities flow through one planning and execution trunk.

### Add Or Change

- promote `CapabilitySpec -> CapabilityExecutionPlan -> CapabilityExecutor` to the only canonical runtime path
- add or finish `capability planner`, `tool executor`, `skill executor`, and `MCP executor` boundaries
- support ordered concurrency for tool execution
- surface cancellation, interruption, degraded results, and retries as explicit runtime outcomes
- integrate provider health, auth state, and approval state into planning

### Remove Or Fence

- remove direct legacy tool registry dispatch from the main path
- remove any MCP side-path that bypasses capability planning
- remove prompt-skill behavior that relies on implicit shell injection

### Public Contract Impact

- expand runtime event taxonomy with `planner.*`, `model.*`, `tool.*`, `skill.*`, `mcp.*`, and `trace.*`
- expose capability summaries and provider state through runtime session and run projections

### Persistence Impact

- persist capability planning results needed for audit and replay
- persist provider health and degraded-state projections where required

### Failure Modes To Prevent

- exposing tools before policy and provider state are resolved
- flattening skills into plain function tools
- letting plugin or MCP integrations become sidecar executors

### Acceptance Gate

- all runtime-visible capabilities come from the planner
- ordered concurrency works for safe parallel calls without corrupting context mutations
- skill, MCP, and plugin capabilities are visible, hidden, deferred, or blocked through the same runtime rules

### Exit Criteria

- legacy registry dispatch is no longer part of the main runtime path

## Phase 4: Team And Workflow Runtime

### Goal

Turn delegation, worker lifecycle, workflow execution, and background tasks into native runtime behavior.

### Add Or Change

- add `subrun and team orchestrator`
- add `leader-orchestrated` team execution
- add worker spawn, resume, cancel, and background execution
- add mailbox and artifact handoff contracts
- add workflow lineage and workflow runtime projection
- add background task and long-run completion signaling

### Remove Or Fence

- retire prompt-centric team execution
- retire any design that treats delegation as a plain tool callback with no durable lineage

### Public Contract Impact

- add workflow and background-run contracts
- extend runtime events with `subrun.*` and `workflow.*`
- expose mailbox, handoff, and artifact lineage state

### Persistence Impact

- persist mailbox and artifact handoff metadata
- persist background run state and completion markers

### Failure Modes To Prevent

- workers sharing mutable prompt state directly
- losing lineage between team lead, worker, and delegated tool call
- background runs becoming detached from approval, trace, or artifact projection

### Acceptance Gate

- a team session can spawn multiple workers, collect results, and preserve lineage
- a workflow run can survive restarts and continue with its artifacts and state intact
- coding and non-coding domains both work on the same orchestration substrate

### Exit Criteria

- team execution and workflow execution use native subrun infrastructure

## Phase 5: Memory Plane Rebuild

### Goal

Build a typed durable memory system with bounded recall, freshness, and proposal-only writeback.

### Add Or Change

- add a `memory selector` with deterministic filtering and side-model relevance ranking
- add a `memory writer` that produces candidates and proposals rather than direct writes
- separate durable memory from conversation summaries and checkpoints
- add freshness metadata and revalidation hooks
- support `user`, `feedback`, `project`, and `reference` memory only

### Remove Or Fence

- stop mixing durable memory with conversation projection
- stop storing derivable repo state as long-term memory

### Public Contract Impact

- add `memory-runtime` schemas for selection, proposal, review, and projection
- extend runtime events with `memory.*`

### Persistence Impact

- store durable memory metadata and indexes in SQLite
- store durable memory bodies on disk under `data/knowledge`
- store append-only memory lifecycle events in `runtime/events/*.jsonl`

### Failure Modes To Prevent

- storing stale, derivable, or noisy data as durable memory
- silently writing workspace-shared memory by default
- using recalled memory without freshness context

### Acceptance Gate

- memory retrieval is bounded, typed, and freshness-aware
- proposal-only writeback works
- conversation summary and durable memory remain fully separate

### Exit Criteria

- durable memory is runtime-native and no longer coupled to conversation summary storage

## Phase 6: Policy And Approval Merge

### Goal

Merge business authorization, execution permission mode, approval brokering, and auth mediation into one deny-before-expose control plane.

### Add Or Change

- add a unified `approval or auth broker`
- feed business authorization into capability planning
- feed execution permission ceilings into planning and execution
- add approval handling for tool execution, memory write, team escalation, workflow escalation, and MCP auth

### Remove Or Fence

- retire approval as a narrow execution-time side flow
- retire any path where auth or approval can be bypassed after planning

### Public Contract Impact

- extend approval contracts with `approvalLayer`, `targetKind`, `targetRef`, and `escalationReason`
- expose pending approval and auth state in runtime session and run snapshots

### Persistence Impact

- persist approval state and auth mediation state as first-class projections

### Failure Modes To Prevent

- exposing blocked capabilities to the model
- silently widening execution permissions mid-session
- handling auth prompts outside the approval broker

### Acceptance Gate

- business deny removes capabilities from exposure
- execution permission mode narrows what can run
- approval and auth events are projected consistently across tool, memory, team, workflow, and MCP boundaries

### Exit Criteria

- deny-before-expose is real runtime behavior, not a doc-level aspiration

## Phase 7: Contract And Host Projection

### Goal

Expose the rebuilt platform through one consistent public contract across hosts and product surfaces.

### Add Or Change

- update OpenAPI source
- generate and split feature-based schema files under `packages/schema/src/*`
- update desktop stores and adapters
- update browser host and Tauri host projections
- expose asset, runtime, workflow, memory, approval, and trace projections through one contract surface

### Remove Or Fence

- remove or stop extending host-specific contract drift
- stop inventing local frontend-only runtime shapes

### Public Contract Impact

- add or extend schema groups for:
  - `actor-manifest`
  - `agent-runtime`
  - `memory-runtime`
  - `asset-bundle`
  - `workflow-runtime`
  - `runtime-policy`

### Persistence Impact

- none beyond projection alignment

### Failure Modes To Prevent

- browser host and Tauri host exposing different runtime semantics
- schema growth collapsing back into monolithic files
- UI state becoming an extra source of truth

### Acceptance Gate

- OpenAPI, generated types, handwritten schema files, adapters, and stores all agree
- browser host and Tauri host return the same public shapes

### Exit Criteria

- host parity is achieved and schema layout follows feature boundaries

## Phase 8: Legacy Deletion

### Goal

Delete the old execution trunk and redundant dispatch infrastructure.

### Add Or Change

- remove the one-shot executor path
- remove duplicate registry or dispatch entrypoints
- remove thin prompt-centric team execution paths
- remove runtime compatibility layers that are no longer used

### Remove Or Fence

- delete old adapter-owned execution logic
- delete duplicate MCP dispatch surfaces
- delete duplicated skill or tool registry surfaces that are no longer canonical

### Public Contract Impact

- none, except removal of deprecated internal behavior that is no longer surfaced

### Persistence Impact

- migrate or remove unused legacy projections

### Failure Modes To Prevent

- leaving dead code that still receives new features
- preserving two similar runtime paths because deletion feels risky

### Acceptance Gate

- only one execution trunk remains
- all targeted runtime tests pass on the new path only

### Exit Criteria

- legacy executor, duplicate dispatch, and prompt-centric team execution are deleted

## Public Contract Program

The rebuild must deliver these public contract additions:

- richer agent and team runtime policy models in `workspace-plane`
- asset-bundle and import or export translation schemas
- session, run, and subrun lineage schemas
- workflow and background-run schemas
- runtime policy snapshot schemas
- capability governance summaries
- expanded runtime event families

Contract rollout order stays fixed:

1. update OpenAPI source
2. generate transport types
3. add or update handwritten feature-based schema files
4. update adapters and stores
5. update runtime services and tests

## Non-Coding Rollout Track

The rebuild cannot validate only coding scenarios.

The first acceptance batch must include at least one non-coding domain such as:

- research or documentation
- browser task execution
- spreadsheet or data handling
- operations or workflow handling

Non-coding capability support must use the same session, policy, capability, workflow, and artifact contracts as coding.

## Acceptance And Test Matrix

The full implementation program must verify:

- single-agent coding sessions with multi-turn tool loops, streaming, retries, budget guards, and resume or compaction
- single-agent non-coding sessions on the same runtime trunk
- team execution with worker concurrency, mailbox, artifact handoff, and lineage tracking
- workflow and background execution with restart recovery
- deny-before-expose capability planning
- approval, auth, and escalation across tool, memory, team, workflow, and MCP boundaries
- typed durable memory recall, ignore-memory handling, freshness, revalidation, and anti-pollution rules
- native bundle roundtrip and external bundle translation diagnostics
- runtime state reconstruction from SQLite projections, JSONL event logs, and disk-backed bodies
- browser host and Tauri host contract parity

## Assumptions And Defaults

- compatibility with the old one-shot path is not a success criterion
- dual runtime trunks are not allowed after the migration completes
- imported foreign bundles are translated into native constructs instead of executed natively
- the UI design is not being redesigned in this program, but its data contracts are
- the first milestone may start with single-agent runtime core cutover, but the implementation plan remains scoped to the full platform rebuild
