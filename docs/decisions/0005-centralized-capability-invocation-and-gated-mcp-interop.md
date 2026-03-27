# ADR 0005: Centralized Capability Invocation And Gated MCP Interop

- Status: Accepted
- Date: 2026-03-27
- Deciders: Repository maintainers
- Consulted: PRD, SAD, GA blueprint, Slice 5 task package
- Informed: Future connector, ToolSearch, runtime, and knowledge slices

---

## Context

Slice 5 introduces the first formal MCP / connector-backed execution step.

The repository already proves one governed local execution chain:

- `Task / Automation -> Run -> Policy / Budget / Approval -> Execution -> Artifact -> KnowledgeCandidate -> Shared Knowledge`

The open design questions for Slice 5 are:

- where connector-backed invocation state should live
- how runtime should choose between built-in deterministic actions and connector-backed actions
- how external connector output should enter Artifact storage without being treated as trusted Shared Knowledge by default

The relevant constraints are already fixed by source-of-truth docs:

- `Run` remains the only authority execution shell.
- Capability visibility and execution must still be resolved through the formal capability catalog plus governance context.
- `EnvironmentLease` is a first-class recovery object with heartbeat, expiry, and resume semantics.
- External protocol output must be gated before entering Artifact or Knowledge flows, and default external trust is low.
- Slice 5 must stay local, SQLite-backed, Rust-only, and fake-server driven for verification.

## Decision

### 1. Capability invocation stays centralized in the runtime execution shell

`RunOrchestrator` remains the only authority that turns a `Task` into actual execution.

Built-in deterministic actions continue to use `octopus-execution` directly.

Connector-backed actions must still enter the same shell:

- capability resolution and governance happen first
- runtime then chooses the execution adapter
- artifacts, audits, traces, retries, approvals, and knowledge hooks remain on the same formal run path

This prevents connector-backed work from becoming an out-of-band side channel.

### 2. MCP interop state gets a dedicated `octopus-interop-mcp` crate

Slice 5 creates a dedicated Rust workspace member under `crates/interop-mcp`.

That crate owns:

- MCP server registry records
- environment-lease records and lifecycle helpers
- invocation persistence
- connector health snapshot fields
- fake/test-double gateway execution
- trust/output-gate evaluation helpers

Runtime orchestration, governance policy evaluation, artifact persistence, and knowledge persistence remain outside this crate.

### 3. External connector output may enter Artifact, but not Shared Knowledge without passing a trust gate

Connector-backed output is allowed to persist as an `Artifact` so runs remain truthful and auditable.

However, Slice 5 requires a separate trust/provenance gate before knowledge capture:

- trusted local or explicitly trusted outputs may continue into `KnowledgeCandidate`
- low-trust external outputs stop at Artifact persistence and observation evidence

This keeps the execution record complete without silently turning external output into shared knowledge.

## Consequences

### Positive

- Connector-backed execution reuses the already verified Run / Approval / Artifact / Audit path instead of creating a parallel runtime.
- Interop persistence and fake gateway logic stay out of runtime orchestration and out of the observation / knowledge stores.
- External output becomes auditable without forcing premature trust or promotion.
- Slice 5 creates a clean boundary for later real MCP transport work.

### Negative

- The workspace gains another crate and more migration surface.
- Action schemas and capability descriptors become richer earlier than a UI consumer exists.
- The first visibility query remains intentionally narrow compared with the future ToolSearch surface.

### Trade-off

The repository accepts a narrow fake-gateway MVP in exchange for proving the real architecture boundary now and deferring actual connector transport, secrets, and UI management to later slices.

## Rejected Alternatives

### 1. Call MCP connectors directly from application/UI code

Rejected because it would bypass the formal Run shell, weaken governance/audit guarantees, and create a second execution path.

### 2. Store lease and invocation records inside `crates/runtime`

Rejected because interop persistence and gateway behavior are a distinct boundary and would further overload runtime orchestration.

### 3. Store MCP invocation output directly as Shared Knowledge when the run succeeds

Rejected because external protocol output is low-trust by default and must not bypass provenance / trust gating.

### 4. Postpone `EnvironmentLease` until real MCP transport exists

Rejected because lease lifecycle and recovery semantics are already formal GA objects and must be proven on the first connector-backed execution slice.

## Follow-up

- Implement SQLite migration `0005` plus the new `octopus-interop-mcp` crate.
- Extend shared schemas and runtime contract tests for connector-backed actions, leases, artifact provenance, and knowledge gating.
- Update repository entry docs once Slice 5 is fully verified.
- Revisit connector health probing, secrets, and ToolSearch/UI surfaces in later slices.
