# ADR 0002: JSON Schema Source Of Truth And Generation Boundary

- Status: Accepted
- Date: 2026-03-26
- Deciders: Repository maintainers
- Consulted: PRD, SAD, GA blueprint, governance owner docs
- Informed: Future slice task packages and implementation work

---

## Context

Octopus has already accepted a monorepo boundary model and a schema-first rule set, but the repository did not yet freeze which concrete contract format should serve as the cross-language source of truth for the initial GA rebuild.

That gap matters immediately because the first implementation step introduces:

- root workspace manifests
- top-level `apps/`, `crates/`, `packages/`, and `schemas/` directories
- first-priority GA object placeholders for `Workspace`, `Project`, `Run`, `Task`, `ApprovalRequest`, `Artifact`, `KnowledgeCandidate`, and related observation and governance objects

Without a fixed source-of-truth format, later Rust and TypeScript slices would still be free to drift into parallel DTO definitions or toolchain-specific pseudo-contracts.

## Decision

### 1. `schemas/` uses JSON Schema as the contract source format

The repository adopts JSON Schema as the initial cross-language contract source format under `schemas/`.

This decision applies to:

- first-priority GA shared objects
- shared state enums
- shared DTO-shaped contracts
- future cross-language command, query, and event payloads when they enter GA scope

### 2. `schemas/` remains the only cross-language contract fact source

Rust and TypeScript consumers may generate, wrap, or map from `schemas/`, but they must not redefine parallel shared contract truth in `crates/` or `packages/`.

### 3. Generation is downstream, not authoritative

Generated or hand-maintained consumer artifacts in Rust or TypeScript are derived outputs.

They may:

- adapt naming for local language ergonomics
- add validation helpers
- add mapping helpers

They may not:

- become the source of truth for shared field shape
- silently add or remove shared semantic fields
- redefine shared state enums without a corresponding schema change

### 4. GA foundation phase freezes naming and placement, not full field design

The current foundation task introduces placeholder JSON Schemas to freeze:

- object names
- group ownership
- `$id` conventions
- minimal identity and boundary references
- required state-enum files for strong state-machine objects

Full field design remains slice-specific work and must be refined in later task packages.

## Consequences

### Positive

- Shared contract truth is frozen before runtime code exists.
- Rust and TypeScript slices can converge on one contract source.
- Initial schema files can stay intentionally small while still being parseable and reviewable.
- Later code generation can be introduced without changing contract ownership.

### Negative

- Early contract work becomes more explicit and slower than ad hoc DTO authoring.
- Some JSON Schemas in the foundation phase are placeholders rather than production-complete contracts.
- Later slices still need additional task packages to refine fields, compatibility rules, and generation details.

### Trade-off

The repository accepts a small upfront ceremony cost in exchange for durable cross-language contract consistency and lower architectural drift during the GA rebuild.

## Rejected Alternatives

### 1. OpenAPI as the primary source of truth

Rejected because the current phase is freezing domain and runtime object boundaries, not public HTTP surface design. OpenAPI can still be derived later where API shape becomes relevant.

### 2. Protocol Buffers as the primary source of truth

Rejected for the foundation phase because it raises toolchain and generation complexity before the repository has even stabilized its first shared object set.

### 3. Per-language contracts with later manual alignment

Rejected because it recreates the exact schema-drift problem that the repo-structure ADR and schema-first guidelines were written to prevent.

## Follow-up

- Keep future shared object additions in `schemas/` first.
- Refine placeholder object schemas in `Slice 1` task packages before runtime implementation begins.
- Introduce Rust and TypeScript generation or consumption rules only after the first vertical slice freezes concrete field sets.
