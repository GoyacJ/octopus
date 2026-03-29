# Octopus

Octopus is currently a **doc-first rebuild** of a unified Agent Runtime Platform.

This repository now proves a **test-verified GA runtime and surface baseline through Slice 17 desktop project scope entry**, on top of the local governed runtime, Slice 9 real MCP transport, Slice 10 remote-hub persistence / auth, the minimum `desktop + remote-hub + schema-ts + hub-client` plus automation-management surfaces, Slice 11 governance interaction, Slice 12 governance explainability, Slice 13 desktop local-host foundation, Slice 14 desktop task workbench, Slice 15 project knowledge index, and Slice 16 desktop remote connection / session surface. The tracked implementation covers the local SQLite-backed governed loop for `Automation(manual_event | cron | webhook | mcp_event) -> TriggerDelivery -> Task -> Policy / Budget / Approval -> Run -> Execution Adapter / MCP Gateway -> EnvironmentLease -> Artifact -> KnowledgeCandidate gate -> Shared Knowledge recall -> Audit / Trace`, plus the minimum remote-hub auth shell for persisted remote users, workspace membership, JWT sessions, authenticated REST/SSE access, automation-management UI/API, approval detail + inbox handling, approval-driven knowledge-promotion requests and resolution, project-bound capability execution-state explanations, run policy-decision visibility, a real Tauri-backed local desktop host, auth-aware desktop/shared TypeScript consumers, and a route-split desktop shell with workspace-scoped `Projects / Inbox / Notifications / Connections` plus project-scoped `Tasks / Runs / Knowledge`, including remembered remote project entry. **Slice 18 is now frozen as run control surface**, with `retry / terminate` parity as the next tracked delivery and session hardening explicitly deferred to the following candidate slice. It does **not** yet prove full tenant / RBAC administration, external IdP integration, vector retrieval, Org Knowledge Graph promotion, or a full GA implementation tree.
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
- refined shared contracts for the current GA runtime, governance, context, and observation slices under `schemas/`, including the full GA trigger set
- first real Rust workspace members in `crates/access-auth`, `crates/domain-context`, `crates/execution`, `crates/governance`, `crates/interop-mcp`, `crates/knowledge`, `crates/observe-artifact`, and `crates/runtime`
- minimum surface and shared-client foundations under `apps/desktop`, `apps/remote-hub`, `packages/schema-ts`, and `packages/hub-client`
- task packages for the GA foundation skeleton, Slice 1 through Slice 18 planning / delivery, and the minimum-surface / trigger-expansion programs under [docs/tasks/](docs/tasks/README.md)

The current tracked implementation proves a local SQLite-backed governed automation, Shared Knowledge, and real credentialed MCP transport path through automated tests: `Automation(manual_event | cron | webhook | mcp_event) -> TriggerDelivery -> Task -> Policy / Budget / Approval -> Run -> Execution Adapter / MCP Gateway -> EnvironmentLease -> Artifact -> KnowledgeCandidate gate -> Shared Knowledge recall -> Audit / Trace`, including persistent `ApprovalRequest`, `InboxItem`, `Notification`, `PolicyDecisionLog`, TriggerDelivery dedupe/retry state, `McpServer`, `McpInvocation`, `EnvironmentLease`, project-scoped `KnowledgeSpace`, `KnowledgeCandidate`, `KnowledgeAsset`, knowledge-capture retry records, and knowledge lineage. The tracked shell layer also proves the minimum `remote-hub` webhook ingress / cron ticker path, persisted remote auth/session state, route-level workspace membership enforcement, auth-aware hub connection state, the minimum automation-management surface, approval-centric governance interaction flows for approval detail, inline approval resolution, and knowledge-promotion requests, read-only governance explainability for project-bound capability execution state and run policy decisions, a real desktop local host under `apps/desktop/src-tauri`, remote desktop connection/session handling, workspace-scoped project discovery and remembered entry, a route-split desktop workbench for workspace-scoped `Projects / Inbox / Notifications / Connections` and project-scoped `Tasks / Runs / Knowledge`, and a project-scoped read-only knowledge index for `Knowledge`. Low-trust connector output can persist as an artifact but is gated before Shared Knowledge candidate creation. Slice 18 is now frozen as the run control surface slice, with session hardening deferred until after that tracked work lands. It does **not** yet prove full tenant administration or RBAC surfaces, external IdP integration, vector retrieval, or Org Knowledge Graph slices.

## Working Rule

Blueprint first, local design second, contracts before implementation, and truthful verification over optimistic claims.
