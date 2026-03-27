# Slice 5 MCP Gateway And Environment Lease

This task package records the fifth GA runtime slice for the rebuild: add the minimum MCP Gateway and `EnvironmentLease` path on top of the existing local SQLite-backed governed runtime.

## Package Files

- [../../../../decisions/0005-centralized-capability-invocation-and-gated-mcp-interop.md](../../../../decisions/0005-centralized-capability-invocation-and-gated-mcp-interop.md)
- [design-note.md](design-note.md)
- [contract-change.md](contract-change.md)
- [implementation-summary.md](implementation-summary.md)
- [verification.md](verification.md)
- [delivery-note.md](delivery-note.md)

## Task Definition

- Goal:
  - Implement the first verified MCP Gateway and `EnvironmentLease` slice so connector-backed capabilities can execute through a governed adapter path without bypassing approval, audit, artifact persistence, or Shared Knowledge gating.
- Scope:
  - Create the Slice 5 task package and keep local design, contract, verification, and delivery notes here.
  - Add ADR 0005 for centralized capability invocation, the dedicated `interop-mcp` crate boundary, and gate-before-knowledge handling for external results.
  - Refine shared schemas for `CapabilityDescriptor`, `EnvironmentLease`, `Artifact`, and `KnowledgeCandidate`, plus any task/automation action schemas needed to express connector-backed execution.
  - Convert `crates/interop-mcp` from placeholder directory into a real Rust workspace crate for MCP registry, invocation persistence, environment-lease lifecycle, connector health snapshotting, fake MCP execution, and trust/output gating.
  - Add SQLite migration `0005` for MCP servers, invocations, and environment leases while preserving forward migration from Slice 1 through Slice 4 databases.
  - Extend `crates/governance` with richer capability-descriptor metadata and runtime-facing capability visibility queries.
  - Extend `crates/observe-artifact` and `crates/knowledge` so connector-backed artifacts persist provenance / trust-gate metadata and low-trust external results stop before Shared Knowledge candidate creation.
  - Extend `crates/runtime` with MCP registry seeding/query APIs, capability visibility query APIs, environment-lease lifecycle/query APIs, and execution dispatch that routes deterministic built-ins through the existing engine and connector-backed actions through the MCP gateway.
  - Add integration tests for connector-backed success, approval-resume, retryable failure normalization, lease heartbeat/release, stale-lease expiry on reopen, and low-trust writeback blocking.
- Out of Scope:
  - Real MCP transport, credentials, remote secret storage, or connector management UI.
  - Desktop / Remote Hub surfaces, `cron` / `webhook` / `MCP event` triggers, or app-layer consumers.
  - Approval-driven knowledge promotion, vector retrieval, Org Knowledge Graph promotion, or later A2A / Mesh interop.
  - Rewriting Slice 1 through Slice 4 runtime semantics or changing the existing manual-task / `manual_event` happy path for built-in deterministic actions.
- Acceptance Criteria:
  - A connector-backed task can execute through the MCP gateway, persist one invocation record plus one execution artifact, and complete through the same governed run shell.
  - High-risk connector-backed execution still pauses for approval and resumes through the same run after approval without creating duplicate invocations or artifacts.
  - Retryable external failures normalize into the existing run retry semantics and can later succeed through explicit retry.
  - Connector-backed execution acquires a persisted `EnvironmentLease` with `requested -> granted -> active` transitions, supports heartbeat, can be explicitly released, and stale active leases expire when the runtime reopens.
  - Low-trust connector output can persist as an artifact and invocation record but does not create a `KnowledgeCandidate`.
  - Updated schemas validate example payloads for Slice 5 descriptor, lease, artifact, knowledge-candidate, and action shapes.
- Non-functional Constraints:
  - Keep `schemas/` as the only cross-language contract source of truth.
  - Preserve truthful current-state docs; do not imply real MCP network transport, UI surfaces, or remote-hub connectors already exist.
  - Keep the Slice 5 MVP local, SQLite-backed, Rust-only, and fake-server driven for tests.
  - Reuse the existing Task / Automation / Run / Governance / Artifact / Knowledge execution shell rather than introducing a parallel runtime path.
- MVP Boundary:
  - Slice 5 proves only the adapter boundary for manual tasks and `manual_event` automation tasks.
  - The first MCP registry is seed/query oriented for tests and local assembly, not a user-facing connector catalog.
  - Connector outputs enter Artifact storage but must not auto-promote or auto-bypass trust gates into Shared Knowledge.
- Human Approval Points: None.
- Source Of Truth Updates:
  - Update this task package.
  - Update shared schemas under `schemas/governance`, `schemas/runtime`, and `schemas/observe`.
  - Add ADR 0005 for the interop crate and centralized invocation boundary.
  - Update repository entry docs whose current-state summary would otherwise become inaccurate once Slice 5 is verified.
- Affected Modules:
  - `schemas/governance`
  - `schemas/runtime`
  - `schemas/observe`
  - `crates/execution`
  - `crates/governance`
  - `crates/interop-mcp`
  - `crates/observe-artifact`
  - `crates/knowledge`
  - `crates/runtime`
  - `docs/decisions`
- Affected Layers:
  - Cross-language contracts
  - Rust governance / interop persistence layer
  - Rust runtime / orchestration layer
  - Rust observation and knowledge persistence layer
  - Repository architecture decision layer
- Risks:
  - Accidentally treating fake MCP execution as proof of real transport integration or UI readiness.
  - Over-expanding Slice 5 into general connector management, trigger expansion, or target-state ToolSearch execution semantics.
  - Letting external outputs bypass provenance / trust gating and silently contaminate Shared Knowledge.
  - Coupling `interop-mcp` too tightly to runtime orchestration instead of keeping it as the interop authority boundary.
- Validation:
  - Schema parse and instance-validation tests.
  - Cargo integration tests for connector-backed success, approval-resume, retry, lease lifecycle, stale expiry, and trust-gate blocking.
  - Full `cargo test --workspace` regression verification.
