# Octopus

Octopus is currently a **doc-first rebuild** of a unified Agent Runtime Platform.

This repository does **not** currently prove a working implementation tree.  
The tracked truth is the repository entry docs and the `docs/` directory. If code, manifests, commands, or verification results are not present in the tracked tree, they must not be described as if they already exist.

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

The repository does **not** yet prove that target-state implementation directories such as `apps/`, `crates/`, `packages/`, or `schemas/` are present.

## Working Rule

Blueprint first, local design second, contracts before implementation, and truthful verification over optimistic claims.
