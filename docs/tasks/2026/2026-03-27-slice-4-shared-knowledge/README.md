# Slice 4 Shared Knowledge

This task package records the fourth verified GA runtime slice for the rebuild: add the minimum Shared Knowledge loop on top of the existing local SQLite-backed governed automation runtime.

## Package Files

- [../../../../decisions/0004-knowledge-crate-and-project-shared-knowledge-loop.md](../../../../decisions/0004-knowledge-crate-and-project-shared-knowledge-loop.md)
- [design-note.md](design-note.md)
- [contract-change.md](contract-change.md)
- [implementation-summary.md](implementation-summary.md)
- [verification.md](verification.md)
- [delivery-note.md](delivery-note.md)

## Task Definition

- Goal:
  - Implement the first verified Shared Knowledge MVP so successful manual-task and `manual_event` automation runs can capture `KnowledgeCandidate`, explicitly promote it into shared knowledge, and recall it on later runs.
- Scope:
  - Create the Slice 4 task package and keep local design, contract, verification, and delivery notes here.
  - Add ADR 0004 for the dedicated knowledge crate and the project-scoped Shared Knowledge closed loop.
  - Refine shared schemas for `KnowledgeSpace`, `KnowledgeCandidate`, `KnowledgeAsset`, and `KnowledgeLineageRecord`, and add strong status-enum schemas for candidate and asset states.
  - Add the new Rust workspace member `crates/knowledge` for SQLite-backed knowledge-space, candidate, asset, and capture-retry persistence.
  - Extend `crates/observe-artifact` with knowledge-lineage persistence and Slice 4 audit / trace constants.
  - Extend `crates/runtime` with project knowledge-space upsert, pre-run recall, post-run candidate capture, explicit promotion, explicit capture retry, and report/query APIs.
  - Add SQLite migration `0004` and integration tests for capture, promotion, recall, failure, retry, and idempotency.
- Out of Scope:
  - Org Knowledge Graph promotion, graph projection, or conflict propagation.
  - `knowledge_promotion` approval workflows, inbox surfaces, or UI management pages.
  - Background workers, autonomous writeback retry, vector retrieval, embedding, or full-text ranking.
  - Remote Hub transport, MCP execution, `cron` / `webhook` / `MCP event` triggers, and TypeScript or app-surface consumers.
- Acceptance Criteria:
  - A completed run with a project knowledge space captures exactly one `KnowledgeCandidate` from its primary execution artifact.
  - `promote_knowledge_candidate(...)` explicitly promotes a captured candidate into one shared `KnowledgeAsset` and records promotion lineage.
  - Later manual-task and `manual_event` automation runs in the same Workspace / Project and exact `capability_id` scope recall the promoted shared asset.
  - Missing knowledge-space writeback does not block run success; the runtime records audit / trace evidence plus a retry record recoverable through `retry_knowledge_capture(...)`.
  - Repeated promotion and repeated capture retry remain idempotent.
  - Updated schemas validate example payloads for the Slice 4 objects and statuses.
- Non-functional Constraints:
  - Keep `schemas/` as the only cross-language contract source of truth.
  - Preserve truthful current-state docs; do not imply UI, remote transport, vector retrieval, or Org Graph behavior exists.
  - Keep the Shared Knowledge MVP project-scoped: one active project knowledge space, exact `capability_id` recall, and explicit API-driven promotion / retry.
  - Reuse the existing Task / Run / Governance / Observation execution path instead of introducing a second runtime model.
- MVP Boundary:
  - Shared knowledge remains owned by `KnowledgeSpace`, but Slice 4 runtime only proves one project-scoped space per Workspace / Project pair.
  - Candidate capture uses the successful run's `execution_output` artifact only.
  - Promotion is explicit runtime API work; no approval workflow or background promotion worker is introduced.
- Human Approval Points: None.
- Source Of Truth Updates:
  - Update this task package.
  - Update shared schemas under `schemas/context` and `schemas/observe`.
  - Add ADR 0004 for the knowledge crate and project-scoped Shared Knowledge loop.
  - Update repository entry docs whose current-state summary would otherwise become inaccurate.
- Affected Modules:
  - `schemas/context`
  - `schemas/observe`
  - `crates/knowledge`
  - `crates/observe-artifact`
  - `crates/runtime`
  - `docs/decisions`
- Affected Layers:
  - Cross-language contracts
  - Rust knowledge persistence/domain layer
  - Rust runtime/orchestration layer
  - Rust observation/persistence layer
  - Repository architecture decision layer
- Risks:
  - Over-expanding Slice 4 into target-state Knowledge Plane work such as multi-space views, vector retrieval, or Org Graph promotion.
  - Accidentally treating Project as the owner of shared knowledge instead of `KnowledgeSpace`.
  - Letting knowledge capture failure block run success or become non-recoverable.
- Validation:
  - Schema parse and instance-validation tests.
  - Cargo integration tests for capture, promotion, recall, failure handling, retry, and idempotency.
