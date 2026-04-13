# Phase 5 Memory Plane Rebuild

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Turn runtime memory from a manifest-summary placeholder into a typed durable memory plane with bounded recall, freshness, explicit ignore semantics, and proposal-only writeback, while keeping knowledge CRUD and conversation summaries separate.

**Architecture:** Phase 4 is assumed complete before this phase starts. The Phase 3 capability trunk and the Phase 4 subrun or workflow substrate already exist. Phase 5 adds two runtime-owned subsystems on top of that base: a `memory selector` and a `memory writer`. The selector compiles runtime memory inputs from frozen session policy, actor or team context, project scope, workflow or subrun lineage, and durable memory metadata. It performs deterministic filtering first, then bounded relevance ranking, and injects selected memory summaries plus freshness metadata into the runtime loop. The writer never writes durable memory directly from raw turn output. It produces typed memory proposals that move through review, approval, rejection, or revalidation before durable persistence.

**Tech Stack:** OpenAPI runtime contracts under `contracts/openapi/src/**`, feature-based schema files under `packages/schema/src/*`, runtime memory modules in `crates/octopus-runtime-adapter`, memory policy primitives in `crates/octopus-core`, knowledge storage integration under the existing workspace and platform stack, SQLite metadata and indexes in `data/main.db`, disk-backed durable memory bodies under `data/knowledge`, and append-only memory lifecycle events under `runtime/events/*.jsonl`.

---

## Fixed Decisions

- Runtime memory is not the same thing as knowledge CRUD. Knowledge CRUD stays in the workspace and knowledge planes; runtime only exposes selection, proposal, review, and projection state.
- Conversation summaries, checkpoints, and trace projections are not durable memory records.
- Supported durable memory kinds stay fixed in this phase: `user`, `feedback`, `project`, and `reference`.
- Memory recall is bounded and freshness-aware. A run may not inject unbounded or freshness-blind durable memory into the model context.
- Writeback is proposal-only in this phase. Runtime may generate candidates and proposals, but it may not silently persist durable memory as a side effect of normal execution.
- Workspace-shared or team-shared durable memory writes remain opt-in and policy-gated. They are never the default write target.
- The runtime must never persist repo-derivable or temporary noise as durable memory, including code structure, file paths, git history, temporary task state, or config-derivable facts.
- Memory review and revalidation become runtime-native state. They do not live only in UI prompts or ad hoc tool metadata.

## Scope

This phase covers:

- typed runtime transport for memory selection, proposal, review, freshness, and projection
- runtime-native bounded recall and ignore-memory handling
- proposal-only durable memory writeback
- durable memory metadata, index, body, and lifecycle persistence
- projection and recovery for memory state across session reload and adapter restart
- desktop and server consumption of the same typed memory runtime contract

This phase does not cover:

- a general redesign of workspace knowledge CRUD or document management
- implicit auto-write durable memory without proposal or review
- storing repo-derived architecture or source-control facts as durable memory
- full business authorization merge for memory write approval; that is finalized in Phase 6
- a separate memory-only side API outside `/api/v1/runtime/*`

## Current Baseline

The current repository already carries memory policy intent, but it does not yet have a runtime-native durable memory plane.

- `crates/octopus-runtime-adapter/src/actor_manifest.rs` currently builds `RuntimeMemorySummary` directly from manifest policy. It reports only a summary string, `durableMemoryCount`, and an always-empty `selectedMemoryIds`.
- `packages/schema/src/memory-runtime.ts` is currently only an alias export of the generated `RuntimeMemorySummary` transport type. There is no feature-level runtime memory selection, proposal, freshness, or review contract yet.
- `contracts/openapi/src/components/schemas/runtime.yaml` and generated `packages/schema/src/generated.ts` already expose `RuntimeMemorySummary`, but not typed selection items, proposal state, or memory review state.
- `crates/octopus-core/src/runtime_policy.rs` already defines real memory policy defaults such as `durable_scopes`, `write_requires_approval`, `allow_workspace_shared_write`, `max_selections`, and `freshness_required`. That means the policy plane is ahead of the runtime plane.
- Session and run transport already carry `memorySummary` and `sessionPolicy`, so there is a stable place to attach richer memory runtime projections.
- The design baseline in `docs/plans/runtime/agent-runtime-rebuild-design.md` already requires deterministic filtering, bounded relevance selection, freshness metadata, proposal-only write candidate generation, approval or review, and durable save or rejection. The codebase has not implemented that runtime loop yet.

Phase 5 starts from this real state: memory policy exists, memory summary transport exists, and durable memory storage responsibilities are already governed, but runtime memory recall and proposal-only writeback are still missing.

## Task 1: Expand Runtime Memory Contracts Beyond `RuntimeMemorySummary`

**Files:**
- Modify: `contracts/openapi/src/components/schemas/runtime.yaml`
- Modify: `contracts/openapi/src/paths/runtime.yaml`
- Modify: `packages/schema/src/memory-runtime.ts`
- Modify: `packages/schema/src/runtime.ts`
- Modify: `packages/schema/src/index.ts`
- Modify: `apps/desktop/test/openapi-transport.test.ts`
- Modify: `apps/desktop/test/runtime-store.test.ts`
- Modify: `apps/desktop/test/support/workspace-fixture-runtime.ts`

**Implement:**
- keep OpenAPI as the only HTTP contract source and add typed runtime memory transport instead of extending summary-only fields
- keep `packages/schema/src/memory-runtime.ts` as the memory feature surface, not a dumping ground for generic runtime wrappers
- expose runtime memory selection, freshness, proposal, and review state through the existing `/api/v1/runtime/*` surface
- keep knowledge CRUD payloads separate from runtime memory payloads

**Required contract groups:**
- `RuntimeMemorySelectionSummary`
- `RuntimeSelectedMemoryItem`
- `RuntimeMemoryProposal`
- `RuntimeMemoryProposalReview`
- `RuntimeMemoryFreshnessSummary`

**Required public fields:**
- on `RuntimeSessionDetail`:
  - `memorySelectionSummary`
  - `pendingMemoryProposalCount`
  - `memoryStateRef`
- on `RuntimeRunSnapshot`:
  - `selectedMemory`
  - `freshnessSummary`
  - `pendingMemoryProposal`
  - `memoryStateRef`
- on runtime event taxonomy:
  - `memory.selected`
  - `memory.proposed`
  - `memory.approved`
  - `memory.rejected`
  - `memory.revalidated`

**Verification:**
```bash
pnpm openapi:bundle
pnpm schema:generate
pnpm schema:check
pnpm -C apps/desktop exec vitest run test/openapi-transport.test.ts test/runtime-store.test.ts
```

**Done when:**
- runtime transport can describe actual memory selection and proposal state without opaque JSON or UI-local patching
- fixtures and store tests only accept the final typed memory runtime shape

## Task 2: Introduce A Runtime-Native Memory Selector And Bounded Recall Pipeline

**Files:**
- Create: `crates/octopus-runtime-adapter/src/memory_selector.rs`
- Create: `crates/octopus-runtime-adapter/src/memory_runtime.rs`
- Modify: `crates/octopus-runtime-adapter/src/run_context.rs`
- Modify: `crates/octopus-runtime-adapter/src/agent_runtime_core.rs`
- Modify: `crates/octopus-runtime-adapter/src/session_service.rs`
- Modify: `crates/octopus-runtime-adapter/src/lib.rs`
- Modify: `crates/octopus-core/src/runtime_policy.rs`

**Implement:**
- compile memory selection inputs from frozen session policy, actor or team policy, project scope, workflow or subrun lineage, and current durable memory metadata
- perform deterministic filtering first by scope, actor, project, freshness, and policy allowlist
- run bounded relevance selection second and cap the final injected set at policy-governed top-N
- inject memory summaries and freshness metadata into the runtime loop as runtime-owned context, not as ad hoc prompt stitching
- support explicit ignore-memory or skip-recall semantics so a user or actor can suppress selected memory for a run without deleting durable records

**Required selector inputs:**
- `sessionPolicy.memoryPolicy`
- selected actor or team ref
- project and workflow scope
- durable memory kind allowlist
- max selection count
- freshness requirement and freshness budget
- ignored memory refs

**Required runtime rules:**
- memory selection is reproducible from runtime state and policy; it is not a best-effort UI helper
- memory selection survives resume and restart through persisted refs and summaries
- selection may be recomputed only from explicit runtime triggers, not from arbitrary frontend refreshes
- recall never widens beyond the frozen policy captured for the run

**Verification:**
```bash
cargo test -p octopus-runtime-adapter memory
cargo test -p octopus-core runtime_policy
```

**Done when:**
- the runtime loop uses a real bounded memory selection instead of a summary-only placeholder
- memory injection includes freshness state and selected memory lineage

## Task 3: Add Proposal-Only Durable Memory Writeback And Review State

**Files:**
- Create: `crates/octopus-runtime-adapter/src/memory_writer.rs`
- Modify: `crates/octopus-runtime-adapter/src/agent_runtime_core.rs`
- Modify: `crates/octopus-runtime-adapter/src/execution_events.rs`
- Modify: `crates/octopus-runtime-adapter/src/approval_flow.rs`
- Modify: `crates/octopus-runtime-adapter/src/persistence.rs`

**Implement:**
- generate typed durable memory proposals from runtime outcomes, explicit user feedback, or validated workflow outputs
- never write durable memory bodies directly from the model loop without proposal state
- attach review metadata, proposal reason, freshness expectation, source run, and target scope to every candidate
- support approval, rejection, ignore, and revalidation as first-class runtime states
- keep the review path compatible with the broader approval broker that Phase 6 will merge

**Required anti-pollution rules:**
- reject candidates that are derivable from the repo or config
- reject candidates that only describe temporary task state
- reject candidates that are just conversation filler or coordination noise
- reject candidates that duplicate still-fresh durable memory without a revalidation reason
- require explicit review for workspace-shared or team-shared writes

**Verification:**
```bash
cargo test -p octopus-runtime-adapter proposal
```

**Done when:**
- writeback is proposal-only and reviewable
- durable memory writes are no longer conflated with session summary or checkpoint persistence

## Task 4: Persist Memory Metadata, Bodies, And Lifecycle Events According To Repository Governance

**Files:**
- Modify: `crates/octopus-infra/src/infra_state.rs`
- Modify: `crates/octopus-runtime-adapter/src/persistence.rs`
- Modify: `crates/octopus-runtime-adapter/src/execution_events.rs`
- Modify: `crates/octopus-platform/src/workspace.rs`
- Modify: `crates/octopus-server/src/workspace_runtime.rs`

**Implement:**
- store durable memory metadata and indexes in SQLite
- store durable memory bodies under `data/knowledge`
- store proposal and review artifacts under `runtime/` with hashes and refs in SQLite
- append memory lifecycle events to `runtime/events/*.jsonl`
- make recovery depend on SQLite projections, JSONL events, and disk-backed bodies instead of ephemeral session JSON

**Required memory metadata fields:**
- `memoryId`
- `kind`
- `scope`
- `ownerRef`
- `sourceRunId`
- `freshnessState`
- `lastValidatedAt`
- `proposalState`
- `storagePath`
- `contentHash`

**Verification:**
```bash
cargo test -p octopus-infra
cargo test -p octopus-runtime-adapter persistence
```

**Done when:**
- durable memory metadata is queryable from SQLite
- bodies, proposals, and lifecycle history live in the governed storage layers instead of mixed runtime JSON blobs

## Task 5: Cut Over Desktop And Store Consumers To The Final Memory Runtime Shape

**Files:**
- Modify: `apps/desktop/src/tauri/runtime_api.ts`
- Modify: `apps/desktop/src/stores/runtime.ts`
- Modify: `apps/desktop/src/views/project/ConversationView.vue`
- Modify: `apps/desktop/test/support/workspace-fixture-runtime.ts`
- Modify: `apps/desktop/test/openapi-transport.test.ts`
- Modify: `apps/desktop/test/runtime-store.test.ts`
- Modify: `apps/desktop/test/tauri-client-runtime.test.ts`

**Implement:**
- consume selected memory, freshness, and pending proposal state through the shared runtime adapter
- stop treating `memorySummary` as sufficient evidence that runtime memory is working
- keep browser host and Tauri host on the same public memory runtime contract
- prevent store-local shape repair or inferred memory review state

**Verification:**
```bash
pnpm -C apps/desktop exec vitest run test/openapi-transport.test.ts test/runtime-store.test.ts test/tauri-client-runtime.test.ts
```

**Done when:**
- desktop can consume runtime-selected durable memory state without local schema invention
- no fixture or store path depends on memory summary placeholders alone

## Task 6: Phase-Level Validation And Quality Fence

**Files:**
- Verify only

**Run:**
```bash
pnpm openapi:bundle
pnpm schema:generate
pnpm schema:check
cargo test -p octopus-core
cargo test -p octopus-infra
cargo test -p octopus-runtime-adapter
cargo test -p octopus-platform
cargo test -p octopus-server
pnpm -C apps/desktop exec vitest run test/openapi-transport.test.ts test/runtime-store.test.ts test/tauri-client-runtime.test.ts
git diff --stat -- \
  contracts/openapi/src/components/schemas/runtime.yaml \
  contracts/openapi/src/paths/runtime.yaml \
  packages/schema/src \
  crates/octopus-core/src \
  crates/octopus-infra/src \
  crates/octopus-runtime-adapter/src \
  crates/octopus-platform/src \
  crates/octopus-server/src \
  apps/desktop/src \
  apps/desktop/test
```

**Acceptance criteria:**
- memory retrieval is bounded, typed, and freshness-aware
- memory selection and durable memory storage remain separate from conversation summary and checkpoint state
- writeback is proposal-only and reviewable
- durable memory records are recoverable from SQLite projections, JSONL events, and disk-backed bodies
- coding and non-coding runs both use the same memory runtime behavior

## Handoff To Later Phases

Phase 5 is complete only when later phases can safely assume:

- runtime memory is a first-class plane rather than a summary placeholder
- memory selection, proposal, freshness, and review state are typed runtime facts
- durable memory writeback is proposal-only and auditable
- conversation summaries are no longer confused with durable memory

Phase 6 can then merge memory proposal review into the same deny-before-expose approval and auth control plane used by capability, team, workflow, and MCP mediation.
