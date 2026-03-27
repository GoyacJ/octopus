# GA Minimum Automation Surface

This task package records the next GA surface-facing slice after Slice 10: add the minimum automation-management surface on top of the verified runtime, trigger expansion, real MCP transport, and remote-hub auth baseline.

## Package Files

- [design-note.md](design-note.md)
- [contract-change.md](contract-change.md)
- [implementation-summary.md](implementation-summary.md)
- [verification.md](verification.md)
- [delivery-note.md](delivery-note.md)

## Task Definition

- Goal:
  - Implement the minimum automation-management surface so the existing governed Automation, Trigger, and TriggerDelivery runtime truth can be created, viewed, manually dispatched where allowed, retried after failure, and minimally lifecycle-managed through shared client and app surfaces.
- Scope:
  - Create this task package and keep local design, contract, verification, and delivery notes here.
  - Add the shared runtime surface contracts needed for automation create, list/detail, lifecycle commands, manual dispatch, trigger-delivery retry, and webhook secret reveal-at-create behavior.
  - Extend `crates/runtime` with the minimum Automation lifecycle subset and thin list/detail/recent-execution retrieval APIs without introducing a second execution shell.
  - Extend `packages/schema-ts`, `packages/hub-client`, `apps/remote-hub`, and `apps/desktop` so both local and remote modes expose the same minimum automation-management behavior.
  - Update tracked owner docs so this slice becomes the current completed next step once verification passes.
- Out of Scope:
  - Remote admin, tenant, RBAC, or external IdP management.
  - Full automation editing, bulk management, hard delete, secret rotation, or multi-trigger-per-automation design.
  - Resident, Team, Mesh, Org Graph, vector retrieval, or deeper knowledge-administration work.
  - A second automation read model, automation-specific SSE board, or new runtime orchestration semantics.
- Acceptance Criteria:
  - A project-scoped automation list and automation detail surface exist through one shared `HubClient` interface in both local and remote modes.
  - Automation creation supports all four GA trigger types: `manual_event`, `cron`, `webhook`, and `mcp_event`.
  - Automation lifecycle supports the minimum managed subset `active`, `paused`, and `archived`, with invalid transitions rejected explicitly.
  - Manual dispatch is only available for `manual_event` automations.
  - Failed trigger deliveries can be retried from the management surface and reuse the existing governed recovery path.
  - Automation detail shows trigger configuration plus recent deliveries and their associated recent run status without introducing a second source of truth.
- Non-functional Constraints:
  - Keep `schemas/` as the only cross-language contract truth.
  - Keep `apps/` as surface assembly only and `packages/` as shared consumer logic only.
  - Preserve the existing runtime main loop and reuse existing delivery/retry semantics.
  - Keep the slice local SQLite-backed and transport-parity-focused.
- MVP Boundary:
  - One automation list view, one automation create surface, and one automation detail view.
  - Lifecycle is limited to `activate`, `pause`, and `archive`.
  - Recent execution state is derived from existing `TriggerDelivery` and `Run` records only.
  - Webhook secret is only revealed once at create-time; later rotation or re-display is not included.
- Human Approval Points:
  - None.
- Source Of Truth Updates:
  - Update this task package.
  - Update `docs/tasks/README.md`.
  - Update repository entry docs and the GA blueprint once implementation and verification complete.
  - Add an ADR only if automation lifecycle ownership or recent-execution ownership becomes a durable repository-wide rule beyond this slice.
- Affected Modules:
  - `schemas/runtime`
  - `crates/runtime`
  - `packages/schema-ts`
  - `packages/hub-client`
  - `apps/remote-hub`
  - `apps/desktop`
  - `docs/tasks`
- Affected Layers:
  - Shared contracts
  - Governed runtime
  - Shared client transport boundary
  - Remote surface assembly
  - Desktop surface assembly
- Risks:
  - Pulling too much target-state automation administration into a GA-minimum slice.
  - Letting surface payloads diverge from the existing runtime truth.
  - Reimplementing recent execution state instead of deriving it from `TriggerDelivery` and `Run`.
  - Letting lifecycle semantics drift from PRD/SAD while only implementing a minimal subset.
- Validation:
  - Add failing Rust runtime and remote-hub integration tests first.
  - Add failing TypeScript schema/client tests first.
  - Add failing desktop automation-surface tests first.
  - Re-run targeted tests and then full `cargo test --workspace` plus `pnpm test:ts` before claiming completion.
