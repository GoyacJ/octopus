# Octopus

Octopus is currently a **doc-first rebuild** of a unified Agent Runtime Platform.

This repository now proves a **local, test-verified Slice 3 runtime** under `crates/`, including the first `manual_event` Automation path, but it does **not** yet prove a runnable UI surface, remote-hub transport, MCP, shared knowledge, or a full GA implementation tree.  
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
- refined Slice 1, Slice 2, and Slice 3 shared contracts for runtime, governance, and observation objects under `schemas/`
- first real Rust workspace members in `crates/domain-context`, `crates/execution`, `crates/governance`, `crates/observe-artifact`, and `crates/runtime`
- task packages for the GA foundation skeleton, Slice 1 runtime startup, Slice 2 governance runtime, and Slice 3 automation runtime under [docs/tasks/](docs/tasks/README.md)

The current tracked implementation proves a local SQLite-backed governed automation path through automated tests: `Automation(manual_event) -> TriggerDelivery -> Task -> Policy / Budget / Approval -> Run -> Artifact -> Audit / Trace`, including persistent `ApprovalRequest`, `InboxItem`, `Notification`, and `PolicyDecisionLog` records plus delivery dedupe/retry state. It does **not** yet prove UI surfaces, remote transport, `cron`/`webhook`/`MCP event` trigger types, MCP execution, or shared knowledge slices.

## Working Rule

Blueprint first, local design second, contracts before implementation, and truthful verification over optimistic claims.
