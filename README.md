# Octopus

Octopus is currently a **doc-first rebuild** of a unified Agent Runtime Platform.

This repository now proves a **test-verified GA runtime and surface baseline through Slice 20 desktop degraded-state convergence**, on top of the local governed runtime, Slice 9 real MCP transport, Slice 10 remote-hub persistence / auth, the minimum `desktop + remote-hub + schema-ts + hub-client` plus automation-management surfaces, Slice 11 governance interaction, Slice 12 governance explainability, Slice 13 desktop local-host foundation, Slice 14 desktop task workbench, Slice 15 project knowledge index, Slice 16 desktop remote connection / session surface, Slice 17 desktop project scope entry, Slice 18 run control surface, and Slice 19 session hardening. The tracked implementation covers the local SQLite-backed governed loop for `Automation(manual_event | cron | webhook | mcp_event) -> TriggerDelivery -> Task -> Policy / Budget / Approval -> Run -> Execution Adapter / MCP Gateway -> EnvironmentLease -> Artifact -> KnowledgeCandidate gate -> Shared Knowledge recall -> Audit / Trace`, plus the minimum remote-hub auth shell for persisted remote users, workspace membership, JWT sessions, authenticated REST/SSE access, automation-management UI/API, approval detail + inbox handling, approval-driven knowledge-promotion requests and resolution, project-bound capability execution-state explanations, run policy-decision visibility, a real Tauri-backed local desktop host, auth-aware desktop/shared TypeScript consumers, and a route-split desktop shell with workspace-scoped `Projects / Inbox / Notifications / Connections` plus project-scoped `Tasks / Runs / Knowledge`, including secure app-local remote-session persistence, rotating refresh-token restore / replay-revocation closure, remembered remote project entry, run retry / terminate parity in `RunView`, and workbench-wide degraded/read-only remote-state semantics. The GA acceptance matrix under `docs/tasks/2026/2026-03-29-slice-20-desktop-degraded-state-convergence/ga-acceptance-matrix.md` aligns the PRD-scoped GA baseline to the tracked implementation and freezes post-GA backlog expansion by default. The post-GA [Remote Session Token Lifecycle Hardening](docs/tasks/2026/2026-03-30-post-ga-session-token-lifecycle/README.md) slice is implemented and verified. The post-GA [Model Center Foundation](docs/tasks/2026/2026-03-30-post-ga-model-center-foundation/README.md) slice is completed as a doc/schema-only foundation for `ModelProvider`, `ModelCatalogItem`, `ModelProfile`, `TenantModelPolicy`, and `ModelSelectionDecision`. The post-GA [Model Governance Persistence](docs/tasks/2026/2026-03-30-post-ga-model-governance-persistence/README.md) slice is now implemented and verified for Rust-side persistence truth plus one bounded run-scoped `ModelSelectionDecision` record per run. The next queued design-only candidate is [Post-GA: Model Governance Read Transport](docs/tasks/2026/2026-03-30-post-ga-model-governance-read-transport/README.md). It does **not** yet prove full tenant / RBAC administration, external IdP integration, read-only transport consumers, provider connectivity or built-in tool modeling, vector retrieval, Org Knowledge Graph promotion, or a full target-state implementation tree.
The tracked truth starts with the repository entry docs, the `docs/` directory, and any tracked manifests, source files, schemas, and verification results that actually exist in the tree. If code, manifests, commands, or verification results are not present in the tracked tree, they must not be described as if they already exist.

## Where To Start

Read in this order:

1. [AGENTS.md](AGENTS.md)
2. [docs/README.md](docs/README.md)
3. [docs/product/PRD.md](docs/product/PRD.md)
4. [docs/architecture/SAD.md](docs/architecture/SAD.md)
5. [docs/architecture/ga-implementation-blueprint.md](docs/architecture/ga-implementation-blueprint.md)

## Local Launch

### Stable Startup Commands

- Desktop shell: `pnpm desktop:open`
- Remote hub: `pnpm remote-hub:start`

Notes:

- `pnpm desktop:open` builds `apps/desktop/dist` first, then starts the tracked Tauri host `octopus-desktop-host`.
- `pnpm remote-hub:start` starts the tracked remote hub on `127.0.0.1:4000` by default, matching the desktop remote-profile default base URL.
- These stable paths do not enable Vite HMR and do not apply the dev-only remote-hub seed path.

### Desktop Dev Commands

- Local Tauri + Vite HMR: `pnpm desktop:dev:local`
- Remote hub + desktop联调: `pnpm desktop:dev:remote`
- Remote hub dev-only shell only: `pnpm remote-hub:dev`

Notes:

- `pnpm desktop:dev:local` uses repo-managed `@tauri-apps/cli` plus a Vite dev server on `http://127.0.0.1:5173`.
- `pnpm remote-hub:dev` keeps the existing remote-hub binary path, but forces an isolated dev database at `target/dev/remote-hub.sqlite` and applies the dev-only seed guarded by `OCTOPUS_REMOTE_HUB_DEV_SEED=1`.
- `pnpm desktop:dev:remote` starts both processes through repo-local Node orchestration, prefixes child logs, forwards shutdown signals, and prints the manual login instructions once the remote hub is ready.

### Remote Dev Login Defaults

For `pnpm remote-hub:dev` and `pnpm desktop:dev:remote`, the isolated dev database is seeded with:

- Workspace: `workspace-alpha`
- Project: `project-remote-demo`
- Base URL: `http://127.0.0.1:4000`
- Email: `admin@octopus.local`
- Password: `octopus-bootstrap-password`

Manual remote login remains the only tracked remote desktop dev flow. No auto-login, hidden profile mutation, or session injection is applied.

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
- refined shared contracts for the current GA runtime, governance, context, and observation slices under `schemas/`, including the full GA trigger set and the first post-GA model-governance foundation contracts
- first real Rust workspace members in `crates/access-auth`, `crates/domain-context`, `crates/execution`, `crates/governance`, `crates/interop-mcp`, `crates/knowledge`, `crates/observe-artifact`, and `crates/runtime`
- minimum surface and shared-client foundations under `apps/desktop`, `apps/remote-hub`, `packages/schema-ts`, and `packages/hub-client`
- task packages for the GA foundation skeleton, Slice 1 through Slice 20 planning / delivery, the minimum-surface / trigger-expansion programs, the completed post-GA token-lifecycle, model-center foundation, and model-governance persistence slices, plus the queued design-only post-GA model-governance read-transport package under [docs/tasks/](docs/tasks/README.md)

The current tracked implementation proves a local SQLite-backed governed automation, Shared Knowledge, and real credentialed MCP transport path through automated tests: `Automation(manual_event | cron | webhook | mcp_event) -> TriggerDelivery -> Task -> Policy / Budget / Approval -> Run -> Execution Adapter / MCP Gateway -> EnvironmentLease -> Artifact -> KnowledgeCandidate gate -> Shared Knowledge recall -> Audit / Trace`, including persistent `ApprovalRequest`, `InboxItem`, `Notification`, `PolicyDecisionLog`, TriggerDelivery dedupe/retry state, `McpServer`, `McpInvocation`, `EnvironmentLease`, project-scoped `KnowledgeSpace`, `KnowledgeCandidate`, `KnowledgeAsset`, knowledge-capture retry records, and knowledge lineage. The tracked shell layer also proves the minimum `remote-hub` webhook ingress / cron ticker path, persisted remote auth/session state, route-level workspace membership enforcement, auth-aware hub connection state, the minimum automation-management surface, approval-centric governance interaction flows for approval detail, inline approval resolution, and knowledge-promotion requests, read-only governance explainability for project-bound capability execution state and run policy decisions, a real desktop local host under `apps/desktop/src-tauri`, remote desktop connection/session handling, workspace-scoped project discovery and remembered entry, a route-split desktop workbench for workspace-scoped `Projects / Inbox / Notifications / Connections` and project-scoped `Tasks / Runs / Knowledge`, a project-scoped read-only knowledge index for `Knowledge`, the minimum run retry / terminate control surface, secure app-local remote-session persistence with startup restore, rotating refresh-token recovery / replay-revocation handling, and a workbench-global degraded-state banner plus route-entry refresh orchestration. The tracked contract layer also includes the post-GA Model Center foundation objects `ModelProvider`, `ModelCatalogItem`, `ModelProfile`, `TenantModelPolicy`, and `ModelSelectionDecision` with `packages/schema-ts` registration, plus implemented Rust-side persistence truth for provider/catalog/profile/policy records in `crates/governance` and one bounded run-scoped `ModelSelectionDecision` record per run in `crates/runtime`. Low-trust connector output can persist as an artifact but is gated before Shared Knowledge candidate creation. Post-Slice-20 backlog work must start with a new task package before implementation; the tracked post-GA closeouts are remote session token lifecycle hardening, the doc/schema-only Model Center foundation, and the bounded Model Governance persistence slice. The next queued design-only follow-on is [2026-03-30-post-ga-model-governance-read-transport](docs/tasks/2026/2026-03-30-post-ga-model-governance-read-transport/README.md). It does **not** yet prove full tenant administration or RBAC surfaces, external IdP integration, read-only transport consumers, provider connectivity or built-in tool modeling, vector retrieval, or Org Knowledge Graph slices.

## Working Rule

Blueprint first, local design second, contracts before implementation, and truthful verification over optimistic claims.
