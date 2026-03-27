# AGENTS.md

## 1. Purpose

This file defines the repository-level instructions that apply to all work in Octopus.

Keep this file concise and high signal.  
Detailed process, governance, and documentation rules belong in the owner documents under `docs/`.

---

## 2. Current Repository State

This repository is currently in **doc-first rebuild** mode.

Current tracked truth starts with the root entry docs, the `docs/` tree, and any tracked manifests or skeleton files that actually exist in the repository.  
Do **not** describe target-state architecture as if it were already implemented.  
Do **not** invent code structure, build commands, runtime behavior, or verification results that are not present in the tracked tree.

Current default allowed work:

- document analysis
- documentation governance
- schema refinement
- architecture refinement
- schema and module planning
- slice-scoped Rust implementation inside tracked crates
- slice-scoped TypeScript or thin assembly implementation inside tracked `packages/` and `apps/` when the current blueprint explicitly brings those modules into scope
- SQLite migration and automated verification work for tracked slices
- repo layout design
- slice planning
- skeleton planning
- repo-level review rules

If implementation work begins later, it must follow the rules in this file and the linked owner docs.

---

## 3. Instruction Layering

- This root `AGENTS.md` is the repository-wide default.
- When work primarily touches repository documentation, also read [docs/README.md](docs/README.md) and [docs/AGENTS.md](docs/AGENTS.md).
- Keep specialized rules close to the work. Nested `AGENTS.md` files may narrow or refine guidance for specific areas.
- Add nested `AGENTS.md` files only when a subtree needs materially more specific guidance; do not create them mechanically for every directory.
- Do not duplicate full policy text in multiple instruction files when a single owner doc already exists.

This follows the Codex guidance to keep repository-level instructions basic and place more specific guidance closer to specialized work.

---

## 4. Source Of Truth Order

When documents disagree or differ in level, use this order:

1. [README.md](README.md) — current repository state and document entry
2. [docs/product/PRD.md](docs/product/PRD.md) — product scope, release slices, and formal product semantics
3. [docs/architecture/SAD.md](docs/architecture/SAD.md) — architecture boundaries, runtime model, governance, recovery, and trust boundaries
4. [docs/architecture/ga-implementation-blueprint.md](docs/architecture/ga-implementation-blueprint.md) — current GA implementation direction and slice order
5. [docs/governance/README.md](docs/governance/README.md) and owner docs under `docs/governance/` — execution process, schema-first, repo structure, review, and delivery
6. ADRs under `docs/decisions/`
7. Task-specific design packages under `docs/tasks/`

Rules:

- PRD defines **what the product means**
- SAD defines **how the architecture is bounded**
- the blueprint defines **what current GA work should prioritize**
- governance docs define **how work must be executed**
- ADRs capture durable architectural decisions
- task packages refine a local slice, but may not silently expand PRD, SAD, or GA boundaries

---

## 5. Required Reading Path Before Substantive Work

For any non-trivial task, read in this order:

1. [README.md](README.md)
2. this [AGENTS.md](AGENTS.md)
3. [docs/README.md](docs/README.md)
4. [docs/product/PRD.md](docs/product/PRD.md)
5. [docs/architecture/SAD.md](docs/architecture/SAD.md)
6. [docs/architecture/ga-implementation-blueprint.md](docs/architecture/ga-implementation-blueprint.md)
7. the relevant owner docs under `docs/governance/`
8. relevant ADRs under `docs/decisions/`
9. relevant task packages under `docs/tasks/`

For difficult or ambiguous tasks, planning is required before coding.

---

## 6. Hard Stops Before Implementation

Implementation must not start if any of the following is true:

- the goal is unclear
- scope and out-of-scope are not defined
- the target layer or module boundary is unclear
- shared-contract impact is unclear
- schema-first impact is unclear
- compatibility impact is unclear
- required verification is not identified
- the change would cross GA/Beta boundaries without explicit approval in the docs
- the task silently expands product scope
- the task relies on fabricated repo structure or nonexistent commands

If blocked, produce or request the missing design artifact instead of forcing implementation.

---

## 7. Placement And Schema Rules

Follow these owner docs:

- [docs/governance/repo-structure-guidelines.md](docs/governance/repo-structure-guidelines.md)
- [docs/governance/schema-first-guidelines.md](docs/governance/schema-first-guidelines.md)

Current actual repository state note:

- the tracked tree now includes root workspace manifests, refined shared contracts for the current GA slices in `schemas/`, first real Rust workspace members under `crates/access-auth`, `crates/domain-context`, `crates/execution`, `crates/governance`, `crates/interop-mcp`, `crates/knowledge`, `crates/observe-artifact`, and `crates/runtime`, plus the minimum surface foundation under `apps/remote-hub`, `apps/desktop`, `packages/schema-ts`, and `packages/hub-client`
- the current verified implementation scope covers the local SQLite-backed governed runtime for `Automation(manual_event | cron | webhook | mcp_event) -> TriggerDelivery -> Task -> Policy / Budget / Approval -> Run -> Execution Adapter / MCP Gateway -> EnvironmentLease -> Artifact -> KnowledgeCandidate gate -> Shared Knowledge recall -> Audit / Trace`, with persistent `ApprovalRequest`, `InboxItem`, `Notification`, `PolicyDecisionLog`, TriggerDelivery dedupe/retry records, `McpServer`, `McpInvocation`, `EnvironmentLease`, project-scoped `KnowledgeSpace`, `KnowledgeCandidate`, `KnowledgeAsset`, capture-retry records, and lineage, plus real credentialed MCP transport, a thin remote-hub webhook ingress / cron tick shell, persisted remote users / workspace membership / JWT session state, route-level auth enforcement, and first cross-language client contracts; low-trust connector output is gated before Shared Knowledge candidate creation, while full tenant / RBAC administration, approval-driven knowledge promotion, vector retrieval, full automation-management surfaces, and Org Graph promotion remain out of scope unless later tracked files prove otherwise

---

## 8. Required Task Package

Before coding a meaningful slice or module, create the minimum applicable task package under `docs/tasks/` following [docs/tasks/README.md](docs/tasks/README.md).

Minimum package:

- `Task Definition`
- `Design Note`
- `Contract Change` when contracts, schemas, events, DTOs, states, or shared interfaces change
- ADR when a durable architecture or boundary decision is being made

Use the templates in:

- [docs/governance/ai-delivery-templates.md](docs/governance/ai-delivery-templates.md)
- [docs/governance/ai-phase-gates.md](docs/governance/ai-phase-gates.md)

---

## 9. ADR And Delivery Rules

- Store ADRs under `docs/decisions/`.
- Follow [docs/decisions/README.md](docs/decisions/README.md) for ADR scope and naming.
- Follow [docs/governance/change-delivery-guidelines.md](docs/governance/change-delivery-guidelines.md) and [docs/governance/code-review-checklist.md](docs/governance/code-review-checklist.md) for delivery and review expectations.
- Every substantial task must end with a concise delivery summary that states what changed, why, risks, verification status, temporary workarounds, follow-ups, and whether docs or ADRs were updated.

---

## 10. Keep AGENTS Small

Detailed rules belong in:

- [docs/README.md](docs/README.md)
- [docs/AGENTS.md](docs/AGENTS.md)
- [docs/governance/README.md](docs/governance/README.md)
- [docs/governance/ai-phase-gates.md](docs/governance/ai-phase-gates.md)
- [docs/governance/ai-delivery-templates.md](docs/governance/ai-delivery-templates.md)
- [docs/governance/repo-structure-guidelines.md](docs/governance/repo-structure-guidelines.md)
- [docs/governance/schema-first-guidelines.md](docs/governance/schema-first-guidelines.md)
- [docs/governance/code-review-checklist.md](docs/governance/code-review-checklist.md)
- [docs/governance/change-delivery-guidelines.md](docs/governance/change-delivery-guidelines.md)
- [docs/decisions/README.md](docs/decisions/README.md)
- [docs/tasks/README.md](docs/tasks/README.md)

If a rule is task-specific, put it in the task package, not here.

---

## 11. One-Sentence Operating Rule

Follow the blueprint globally, design each module locally before coding, keep contracts ahead of implementation, and never trade architectural truth for short-term convenience.
