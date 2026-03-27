# ADR 0004: Knowledge Crate And Project Shared Knowledge Loop

- Status: Accepted
- Date: 2026-03-27
- Deciders: Repository maintainers
- Consulted: PRD, SAD, GA blueprint, Slice 4 task package
- Informed: Future knowledge, governance, and retrieval slices

---

## Context

Slice 4 introduces the first formal Shared Knowledge implementation step.

The repository already proves one governed execution chain:

- `Task / Automation -> Run -> Artifact -> Audit / Trace`

The open design questions for Slice 4 are:

- where knowledge persistence and local domain rules should live
- how successful runs should write candidate knowledge without bypassing the existing execution shell
- how promoted knowledge should be recalled by later runs without pretending the target-state knowledge system already exists

The relevant constraints are already fixed by source-of-truth docs:

- `Run` remains the only authority execution shell.
- Shared knowledge is formally owned by `KnowledgeSpace`, not by `Project` directly.
- Slice 4 must stay local, SQLite-backed, and Rust-only.
- Slice 4 must not silently pull in vector retrieval, Org Graph promotion, approval workflow, or UI surfaces.

## Decision

### 1. Shared Knowledge persistence gets a dedicated `octopus-knowledge` crate

Knowledge-space, candidate, asset, and capture-retry persistence move into a dedicated Rust workspace member under `crates/knowledge`.

That crate owns:

- `KnowledgeSpaceRecord`
- `KnowledgeCandidateRecord`
- `KnowledgeAssetRecord`
- `KnowledgeCaptureRetryRecord`
- SQLite persistence and idempotent upsert / fetch helpers for those records

Runtime orchestration and observation storage remain outside this crate.

### 2. Slice 4 proves one project-scoped Shared Knowledge loop

The MVP authority container is one active `KnowledgeSpace` per Workspace / Project pair.

That space:

- remains the owner of promoted shared knowledge
- is created or reused through explicit runtime API
- gives the repository a truthful Shared Knowledge owner without implementing target-state multi-space views yet

`Project` stays the execution scope and recall filter, but not the shared-knowledge owner.

### 3. Successful runs capture candidate knowledge from their primary execution artifact

After a successful run persists its `execution_output` artifact, runtime attempts to capture one `KnowledgeCandidate`.

That candidate:

- references the source run, task, and artifact
- uses an artifact-derived `dedupe_key` for idempotency
- carries the producing `capability_id`
- does not change the completed run result if capture prerequisites are missing

### 4. Promotion is explicit and recall is exact-match MVP logic

Promotion remains an explicit runtime API call that converts one candidate into one shared `KnowledgeAsset`.

Recall for Slice 4 is intentionally narrow:

- same Workspace / Project
- same project knowledge space
- exact `capability_id` match

This proves the closed loop without implying target-state vector retrieval or ranking already exists.

### 5. Knowledge writeback failure is non-blocking but must stay recoverable

If candidate capture cannot complete, the run still succeeds, but runtime must:

- persist a retry record
- write audit and trace evidence
- expose explicit retry entry through runtime API

This preserves execution truth while keeping knowledge writeback observable and recoverable.

## Consequences

### Positive

- Knowledge persistence rules stay out of the runtime orchestrator and do not get mixed into observation storage.
- Slice 4 reuses the already verified Task / Run / Artifact chain instead of creating a parallel knowledge execution model.
- Shared Knowledge now has a truthful owner object (`KnowledgeSpace`) and durable lineage records.
- Failure handling is visible and retryable instead of hidden behind optimistic success.

### Negative

- The workspace gains another crate and migration surface.
- Retrieval is intentionally simplistic and will need redesign for richer knowledge discovery.
- Promotion is still a developer-facing API operation with no approval or UI workflow.

### Trade-off

The repository accepts a narrow project-scoped Shared Knowledge MVP and explicit promotion flow in exchange for preserving architectural truth and keeping Slice 4 small enough to verify thoroughly.

## Rejected Alternatives

### 1. Keep knowledge persistence inside `crates/runtime`

Rejected because it would further overload runtime orchestration with a separate persistence/domain concern and weaken crate ownership clarity.

### 2. Store knowledge records inside `crates/observe-artifact`

Rejected because lineage, audit, and trace are observations about knowledge activity, not the authority store for shared knowledge objects themselves.

### 3. Attach shared knowledge directly to `Project`

Rejected because PRD and SAD already define `KnowledgeSpace` as the owner boundary for Shared Knowledge, with Project providing visibility scope rather than ownership.

### 4. Fail the run when knowledge capture cannot complete

Rejected because knowledge writeback is an adjacent post-run concern in Slice 4 and should not falsify the already completed execution result.

### 5. Auto-promote knowledge on capture

Rejected because Slice 4 needs a reviewable promotion step and must not collapse candidate capture into irreversible shared-knowledge publication.

## Follow-up

- Add `knowledge_promotion` approval and inbox workflows in later governance/UI slices.
- Extend beyond one project knowledge space when KnowledgeSpace view and attachment semantics are explicitly designed.
- Revisit retrieval semantics once embeddings, ranking, or richer scope rules are formally defined.
- Keep Org Knowledge Graph promotion deferred to Beta-oriented work.
