# Documentation Overview

This repository uses a role-based documentation structure so each document has a clear job, a clear update trigger, and a clear owner.

## Current State Note

The tracked tree now includes the monorepo skeleton, refined GA shared contracts, first real Rust workspace members for the local SQLite-backed governed runtime plus `crates/access-auth`, the minimum `apps/desktop + apps/remote-hub + packages/schema-ts + packages/hub-client` surface foundation, the first doc/schema-only Model Center foundation contracts, and the first bounded model-governance persistence implementation slice. That tracked implementation proves the local `Automation(manual_event | cron | webhook | mcp_event) -> TriggerDelivery -> Task -> Policy / Budget / Approval -> Run -> Execution Adapter / MCP Gateway -> EnvironmentLease -> Artifact -> KnowledgeCandidate gate -> Shared Knowledge recall -> Audit / Trace` loop plus persistent approval inbox/notification, delivery, real MCP transport, registry/invocation/lease, knowledge-lineage records, the thin remote-hub webhook ingress / cron tick shell, the minimum remote auth/persistence shell, the minimum automation-management surface, Slice 11 approval-driven governance interaction for approval detail, inline resolution, and knowledge-promotion request / resolve flows, Slice 12 read-only governance explainability for project-bound capability execution state and run policy decisions, Slice 13 real desktop local-host foundation, Slice 14 desktop task workbench routing, Slice 15 project-scoped Shared Knowledge index parity across desktop/local/remote surfaces, Slice 16 desktop remote connection/session handling, Slice 17 workspace-scoped desktop project discovery / memory / entry, Slice 18 run retry / terminate control parity, Slice 19 secure session hardening, Slice 20 desktop degraded-state convergence, the post-GA remote session refresh / rotation / secure-restore closure, and Rust-side persistence truth for `ModelProvider` / `ModelCatalogItem` / `ModelProfile` / `TenantModelPolicy` plus one run-scoped `ModelSelectionDecision` record per run. The tracked desktop IA is now workspace-scoped `Projects / Inbox / Notifications / Connections` plus project-scoped `Tasks / Runs / Knowledge`, with secure restore, rotating refresh-token recovery, route-entry connection refresh orchestration, and shell-level degraded/read-only warnings. The Slice 20 GA acceptance matrix freezes post-GA backlog expansion by default; any follow-on slice must start with a new task package. [Remote Session Token Lifecycle Hardening](tasks/2026/2026-03-30-post-ga-session-token-lifecycle/README.md) is now completed and verified as the first post-GA auth hardening slice. [Post-GA: Model Center Foundation](tasks/2026/2026-03-30-post-ga-model-center-foundation/README.md) is now completed as a doc/schema-only post-GA architecture foundation. [Post-GA: Model Governance Persistence](tasks/2026/2026-03-30-post-ga-model-governance-persistence/README.md) is now implemented and verified as the first bounded model-governance consumer slice. The queued next design-only candidate is [Post-GA: Model Governance Read Transport](tasks/2026/2026-03-30-post-ga-model-governance-read-transport/README.md). It still does not prove full tenant / RBAC administration, external IdP integration, read-only transport consumers, provider connectivity or built-in tool modeling, vector retrieval, or Org Knowledge Graph promotion.

## Source Of Truth Order

1. [../README.md](../README.md)
2. [product/PRD.md](product/PRD.md)
3. [architecture/SAD.md](architecture/SAD.md)
4. [architecture/ga-implementation-blueprint.md](architecture/ga-implementation-blueprint.md)
5. [governance/README.md](governance/README.md) and owner docs under `governance/`
6. ADRs under [decisions/](decisions/README.md)
7. Task packages under [tasks/](tasks/README.md)
8. References under `references/` for inspiration only

## Directory Map

| Path | Role | Update when |
| --- | --- | --- |
| [`product/`](product/README.md) | Product semantics and release scope | Product meaning or release slicing changes |
| [`architecture/`](architecture/README.md) | Target-state architecture and current GA blueprint | Architecture boundaries or slice sequencing change |
| [`governance/`](governance/README.md) | Execution rules, templates, review, delivery, and repo governance | Process, structure, or engineering rules change |
| [`decisions/`](decisions/README.md) | Durable architecture decisions | A long-lived decision needs explicit record |
| [`tasks/`](tasks/README.md) | Task-specific design packages and delivery records | A meaningful task needs local design and delivery artifacts |
| [`references/`](references/README.md) | Non-normative external reference material | External inspiration or comparative material is worth preserving |

## Update Rules

- Product semantics belong in `product/PRD.md`.
- Architecture boundaries belong in `architecture/SAD.md`.
- Current GA sequencing belongs in `architecture/ga-implementation-blueprint.md`.
- Process and execution rules belong in the owner docs under `governance/`.
- Durable decisions belong in `decisions/`.
- Local task analysis and delivery artifacts belong in `tasks/`.
- References do not override any normative document.

## Adding Or Moving Documents

When you add or move a document:

1. Put it in the correct category directory.
2. Update this index if the new document becomes part of the normative flow.
3. Update the relevant category README.
4. Update inbound links and path references.
5. Keep links relative. Do not use machine-specific absolute filesystem paths inside repository docs.
6. If you add a new governance owner doc, register it in `governance/README.md` instead of creating a parallel summary rule elsewhere.
