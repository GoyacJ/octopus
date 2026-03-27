# Octopus

Octopus is currently a **doc-first rebuild** of a unified Agent Runtime Platform.

This repository now proves a **local, test-verified Slice 5 runtime** under `crates/`, including the first fake/test-double `Execution Adapter / MCP Gateway` and `EnvironmentLease` path on top of the existing `manual_event` Automation and Shared Knowledge slices, but it does **not** yet prove a runnable UI surface, remote-hub transport, real credentialed MCP transport, Org Knowledge Graph promotion, or a full GA implementation tree.  
The tracked truth starts with the repository entry docs, the `docs/` directory, and any tracked manifests, source files, schemas, and verification results that actually exist in the tree. If code, manifests, commands, or verification results are not present in the tracked tree, they must not be described as if they already exist.

## Where To Start

Read in this order:

1. [AGENTS.md](AGENTS.md)
2. [docs/README.md](docs/README.md)
3. [docs/product/PRD.md](docs/product/PRD.md)
4. [docs/architecture/SAD.md](docs/architecture/SAD.md)
5. [docs/architecture/ga-implementation-blueprint.md](docs/architecture/ga-implementation-blueprint.md)

## Documentation Map

- [docs/product/PRD.md](docs/product/PRD.md): product meaning, release slices, and formal scope
- [docs/architecture/SAD.md](docs/architecture/SAD.md): architecture boundaries, runtime model, and trust boundaries
- [docs/architecture/ga-implementation-blueprint.md](docs/architecture/ga-implementation-blueprint.md): current GA implementation direction and slice order
- [docs/governance/README.md](docs/governance/README.md): engineering process, schema-first, repo structure, review, and delivery rules
- [docs/decisions/README.md](docs/decisions/README.md): ADR index and durable decision rules
- [docs/tasks/README.md](docs/tasks/README.md): task-package placement and structure
- [docs/references/](docs/references/): non-normative reference material

## Current Tracked Tree

The repository currently contains:

- repository-level instructions in [AGENTS.md](AGENTS.md)
- product, architecture, governance, decision, and reference docs under [docs/](docs/README.md)
- monorepo root manifests in `Cargo.toml`, `package.json`, and `pnpm-workspace.yaml`
- refined Slice 1 through Slice 5 shared contracts for runtime, governance, context, and observation objects under `schemas/`
- first real Rust workspace members in `crates/domain-context`, `crates/execution`, `crates/governance`, `crates/interop-mcp`, `crates/knowledge`, `crates/observe-artifact`, and `crates/runtime`
- task packages for the GA foundation skeleton and Slice 1 through Slice 5 runtime deliveries under [docs/tasks/](docs/tasks/README.md)

The current tracked implementation proves a local SQLite-backed governed automation, Shared Knowledge, and fake/test-double MCP gateway path through automated tests: `Automation(manual_event) -> TriggerDelivery -> Task -> Policy / Budget / Approval -> Run -> Execution Adapter / MCP Gateway -> EnvironmentLease -> Artifact -> KnowledgeCandidate gate -> Shared Knowledge recall -> Audit / Trace`, including persistent `ApprovalRequest`, `InboxItem`, `Notification`, `PolicyDecisionLog`, TriggerDelivery dedupe/retry state, `McpServer`, `McpInvocation`, `EnvironmentLease`, project-scoped `KnowledgeSpace`, `KnowledgeCandidate`, `KnowledgeAsset`, knowledge-capture retry records, and knowledge lineage. Low-trust connector output can persist as an artifact but is gated before Shared Knowledge candidate creation. It does **not** yet prove UI surfaces, remote transport, `cron`/`webhook`/`MCP event` trigger types, real credentialed MCP transport, approval-driven knowledge promotion, vector retrieval, or Org Knowledge Graph slices.

## Working Rule

Blueprint first, local design second, contracts before implementation, and truthful verification over optimistic claims.
