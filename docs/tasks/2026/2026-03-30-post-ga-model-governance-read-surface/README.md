# Post-GA: Model Governance Read Surface

This task package implements the first bounded read-only model-governance surface after the design-only [Post-GA: Model Governance Read Transport](../2026-03-30-post-ga-model-governance-read-transport/README.md) package froze the transport and consumer boundary.

## Package Files

- [design-note.md](design-note.md)
- [contract-change.md](contract-change.md)
- [implementation-summary.md](implementation-summary.md)
- [verification.md](verification.md)
- [delivery-note.md](delivery-note.md)

## Task Definition

- Goal:
  - Implement the minimum read-only model-governance slice so remote-hub, the shared `HubClient`, and desktop can consume provider/catalog/profile/policy truth plus run-scoped `ModelSelectionDecision` without widening into writes, provider connectivity, or tenant administration.
- Scope:
  - Create and maintain this implementation task package.
  - Extend `RunDetail` additively so run-scoped `ModelSelectionDecision` can surface through existing run detail reads.
  - Extend `schemas/interop`, `packages/schema-ts`, and `packages/hub-client` with additive read-only model-governance accessors.
  - Extend `apps/remote-hub` and `apps/desktop/src-tauri` so remote and local transports expose parity for the approved read-only surfaces.
  - Add one workspace-level read-only desktop `Models` page and render run-scoped `ModelSelectionDecision` in `RunView`.
- Out Of Scope:
  - Write commands, provider credentials, provider connectivity, `ProviderAdapter` SPI, built-in tool modeling, tenant administration, RBAC, IdP, vector retrieval, Org Graph promotion, or any new provider-connectivity DTOs.
  - Replacing the existing five model-governance contracts with new aggregate truth objects unless implementation proves a bounded additive contract change is unavoidable.
  - Creating desktop-only DTO truth or bypassing `HubClient` from `apps/desktop/src`.
- Acceptance Criteria:
  - `apps/remote-hub` exposes authenticated read-only routes for provider/catalog/profile/policy records and keeps workspace membership enforcement intact.
  - `packages/hub-client` exposes transport-neutral read-only accessors with local/remote parity.
  - `RunDetail` surfaces optional `model_selection_decision` without breaking existing consumers.
  - Desktop adds a workspace-level read-only `Models` route and `RunView` shows the run-scoped model-selection decision when present.
- Non-functional Constraints:
  - Keep `schemas/` as the cross-language contract source of truth.
  - Keep provider/catalog/profile/policy persistence truth in `crates/governance`.
  - Keep run-scoped decision truth in `crates/runtime`.
  - Keep desktop consumers read-only and explicit about degraded, offline, forbidden, and empty states.
- MVP Boundary:
  - Workspace-scoped read-only provider/catalog/profile/policy visibility.
  - Additive `RunDetail.model_selection_decision`.
  - One read-only `Models` page and one `RunView` decision panel.
  - No write UI and no admin enumeration surface.
- Human Approval Points:
  - None.
- Source Of Truth Updates:
  - Update this task package.
  - Update `README.md`, `docs/README.md`, `docs/architecture/ga-implementation-blueprint.md`, and `docs/tasks/README.md` so the active post-GA implementation slice is recorded truthfully.
- Affected Modules:
  - `docs/tasks`
  - `docs/architecture`
  - `schemas/runtime`
  - `schemas/interop`
  - `packages/schema-ts`
  - `packages/hub-client`
  - `crates/runtime`
  - `apps/remote-hub`
  - `apps/desktop`
  - `apps/desktop/src-tauri`
- Affected Layers:
  - Repository documentation / owner docs
  - Cross-language contracts
  - Local and remote transport assembly
  - Shared client surface
  - Desktop store, routing, and read-only views
- Risks:
  - Introducing a second app-local or transport-only truth object instead of reusing existing model-governance contracts.
  - Letting workspace-scoped policy reads widen into tenant admin listing behavior.
  - Diverging local and remote transport semantics for the same read-only surface.
- Validation:
  - `cargo test -p octopus-governance`
  - `cargo test -p octopus-runtime`
  - `cargo test -p octopus-remote-hub --test http_surface --test auth_surface`
  - `cargo test -p octopus-desktop-host --test local_host`
  - `pnpm test:ts`
  - `pnpm typecheck:ts`
  - `pnpm --filter @octopus/desktop smoke:local-host`
