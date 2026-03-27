# Documentation Overview

This repository uses a role-based documentation structure so each document has a clear job, a clear update trigger, and a clear owner.

## Current State Note

The tracked tree now includes the monorepo skeleton, refined GA shared contracts, first real Rust workspace members for the local SQLite-backed governed runtime plus `crates/access-auth`, and the minimum `apps/desktop + apps/remote-hub + packages/schema-ts + packages/hub-client` surface foundation. That tracked implementation proves the local `Automation(manual_event | cron | webhook | mcp_event) -> TriggerDelivery -> Task -> Policy / Budget / Approval -> Run -> Execution Adapter / MCP Gateway -> EnvironmentLease -> Artifact -> KnowledgeCandidate gate -> Shared Knowledge recall -> Audit / Trace` loop plus persistent approval inbox/notification, delivery, real MCP transport, registry/invocation/lease, knowledge-lineage records, the thin remote-hub webhook ingress / cron tick shell, the minimum remote auth/persistence shell, and first TypeScript surface consumers through tests. It still does not prove a minimum automation-management surface, full tenant / RBAC administration, external IdP integration, approval-driven knowledge promotion, or vector retrieval.

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
