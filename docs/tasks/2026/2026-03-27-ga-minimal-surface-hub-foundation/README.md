# GA Minimal Surface And Hub Foundation

This task package records the first GA surface-facing slice for the rebuild: freeze the minimum visual framework, add the first cross-language Hub surface contracts, and build the initial client + app assembly baseline on top of the verified local Slice 5 runtime.

## Package Files

- [../../../../decisions/0006-hub-client-surface-contracts-and-transport-parity.md](../../../../decisions/0006-hub-client-surface-contracts-and-transport-parity.md)
- [design-note.md](design-note.md)
- [contract-change.md](contract-change.md)
- [implementation-summary.md](implementation-summary.md)
- [verification.md](verification.md)
- [delivery-note.md](delivery-note.md)

## Task Definition

- Goal:
  - Implement the first GA minimum surface and Hub consumer foundation so the verified runtime can be consumed through one shared client boundary by both the desktop shell and the remote-hub app.
- Scope:
  - Create this task package and keep local design, contract, verification, and delivery notes here.
  - Replace the `VISUAL_FRAMEWORK.md` placeholder with the first frozen GA minimum-surface specification.
  - Add ADR 0006 for Hub surface contract ownership, `HubClient` ownership, and local/remote transport parity rules.
  - Add the first shared surface DTO/command/query/event schemas needed by desktop and remote-hub consumers.
  - Turn `packages/schema-ts` into a real workspace package for schema import, validation, and TypeScript-side surface contract consumption.
  - Turn `packages/hub-client` into a real workspace package that hides local `invoke + event` transport and remote `HTTP + SSE` transport behind one interface.
  - Add the `apps/remote-hub` Rust app as a thin `axum` assembly layer over the existing runtime.
  - Add the `apps/desktop` Vue 3 + TypeScript + Pinia shell plus a minimal Tauri-facing local-client integration seam.
  - Add tests for schema consumption, client parity, remote-hub HTTP/SSE behavior, and one desktop happy-path render flow.
- Out of Scope:
  - `cron`, `webhook`, or `MCP event` trigger implementation.
  - Real MCP credential transport, connector management UI, secrets, or health probing.
  - PostgreSQL/JWT remote deployment semantics.
  - Vector retrieval, Org Knowledge Graph, `DiscussionSession`, `ResidentAgentSession`, A2A, or high-order Mesh work.
  - Full production visual design system or complex board/multi-view surfaces.
- Acceptance Criteria:
  - `VISUAL_FRAMEWORK.md` truthfully defines the first-round GA minimum pages, state presentation rules, and information architecture.
  - New schemas validate the minimum Task, Run, Approval, Inbox, Notification, Artifact, Knowledge, Capability-explanation, and Hub-connection payloads shared by hub/client consumers.
  - `packages/schema-ts` validates those payloads and exposes TypeScript-facing consumer bindings without becoming the contract source of truth.
  - `packages/hub-client` provides one `HubClient` interface with both local and remote adapters that pass the same contract suite.
  - `apps/remote-hub` exposes the minimum HTTP/SSE surface required by `HubClient` without duplicating runtime logic.
  - `apps/desktop` renders the minimum shell and can complete one create-task -> start-run -> view artifact/inbox/knowledge happy path through the shared client boundary in tests.
- Non-functional Constraints:
  - Keep `schemas/` as the only cross-language contract source of truth.
  - Keep `apps/` as assembly only and `packages/` as shared frontend consumption only.
  - Preserve the existing runtime truth; do not claim full production UI, remote deployment, or real connector transport exists.
  - Prefer the smallest set of DTOs and pages that prove the GA minimum surface.
- MVP Boundary:
  - Remote hub remains a thin local SQLite-backed shell over the existing runtime.
  - Desktop uses the shared client boundary and a Tauri-facing seam, but this slice does not prove final production desktop packaging or remote auth.
  - Shared Knowledge management remains minimal: view run-related assets/candidates and explicit promotion, not full knowledge administration.
- Human Approval Points: None.
- Source Of Truth Updates:
  - Update this task package.
  - Update `docs/architecture/VISUAL_FRAMEWORK.md`.
  - Add new shared schemas under `schemas/`.
  - Add ADR 0006 for surface contract and transport-parity rules.
  - Update repository entry docs if the tracked-state summary changes materially.
- Affected Modules:
  - `docs/architecture`
  - `docs/tasks`
  - `docs/decisions`
  - `schemas/runtime`
  - `schemas/governance`
  - `schemas/observe`
  - `packages/schema-ts`
  - `packages/hub-client`
  - `apps/remote-hub`
  - `apps/desktop`
- Affected Layers:
  - Documentation and owner docs
  - Cross-language contracts
  - Frontend shared consumer layer
  - Surface assembly layer
  - Remote transport assembly layer
- Risks:
  - Reintroducing parallel DTO truth in TypeScript or app-local code.
  - Letting the remote-hub app become a second runtime instead of a thin shell.
  - Freezing too much UI detail too early and silently pulling Beta surfaces into scope.
  - Mixing local and remote client behavior so parity becomes untestable.
- Validation:
  - Rust schema/contract tests.
  - TypeScript package tests for schema validation and client parity.
  - Remote-hub integration tests for HTTP/SSE routes.
  - Desktop component/integration tests for the minimum happy path.

