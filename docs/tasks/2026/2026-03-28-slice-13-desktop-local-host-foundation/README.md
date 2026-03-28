# Slice 13: Desktop Local Host Foundation

This task package records the GA desktop local-host foundation slice: replace the tracked `window.__OCTOPUS_LOCAL_HUB__` seam with a real Tauri/Rust local host while preserving the existing runtime, governance, and shared-client truth.

## Package Files

- [design-note.md](design-note.md)
- [contract-change.md](contract-change.md)
- [implementation-summary.md](implementation-summary.md)
- [verification.md](verification.md)
- [delivery-note.md](delivery-note.md)

## Task Definition

- Goal:
  - Implement the tracked local desktop host so `apps/desktop` can run against a real local SQLite-backed Tauri/Rust bridge instead of a hand-injected window seam.
- Scope:
  - Create this task package and keep local design, contract, verification, and delivery notes here.
  - Add a tracked Tauri 2 Rust host under `apps/desktop/src-tauri` as a Cargo workspace member.
  - Promote the local command directory and event channel to shared interop-owned constants so Rust and TypeScript consume one source.
  - Seed deterministic local demo context and governed defaults on first host start.
  - Wire the existing `HubClient` local adapter through real Tauri invoke/event transport.
  - Cover the currently consumed desktop surface operations in local mode.
  - Reject unsupported local trigger ingress (`webhook`, `mcp_event`) explicitly in both host behavior and desktop UX.
- Out of Scope:
  - Tenant, RBAC, IdP, JWT/session, or local auth expansion.
  - Workspace/project administration UI.
  - Grant/budget management UI.
  - Vector retrieval, Org Graph, A2A, deeper knowledge administration, or broader desktop redesign.
  - Turning `apps/desktop` into a second runtime/business-logic owner.
- Acceptance Criteria:
  - `apps/desktop` has a tracked `src-tauri` host that opens the local runtime, seeds deterministic demo state, and exposes the existing `HubClient` local surface through real Tauri commands/events.
  - `createWindowLocalHubClient()` still defines the desktop bootstrap semantics, but the window transport is registered by a tracked Tauri adapter rather than handwritten test setup.
  - Local host command/event ownership lives under `schemas/interop` and is consumed by both Rust and TypeScript.
  - Local mode reports fixed authenticated local connection state and does not introduce JWT/session.
  - Local mode allows `manual_event` and `cron`, while `webhook` and `mcp_event` are rejected with explicit user-visible guidance.
  - Workspace verification still passes through `cargo test --workspace`, `pnpm test:ts`, and `pnpm typecheck:ts`, plus a new desktop-host startup smoke check.
- Non-functional Constraints:
  - Keep `schemas/` as the only cross-language contract truth.
  - Keep shared business rules in existing `crates/`; the host may only assemble and map.
  - Preserve `HubClient` method signatures and existing JSON payload compatibility.
  - Prefer pull-based desktop store behavior; event support only needs to preserve the existing `HubEvent` contract.
- MVP Boundary:
  - Seed one deterministic `workspace=demo` / `project=demo` local environment.
  - Seed one governed capability path whose budget thresholds expose `executable`, `approval_required`, and `denied` explainability states by cost.
  - Support only the minimum current desktop surface already tracked in `apps/desktop`.
- Human Approval Points:
  - None.
- Source Of Truth Updates:
  - Update this task package.
  - Update `docs/tasks/README.md`.
  - Update repository entry docs and the GA blueprint only after implementation and verification land.
  - Add a new ADR only if local transport ownership exceeds ADR 0006.
- Affected Modules:
  - `docs/tasks`
  - `schemas/interop`
  - `packages/schema-ts`
  - `packages/hub-client`
  - `apps/desktop`
  - `crates/runtime` (thin surface helpers only if needed)
  - root workspace manifests
- Risks:
  - Letting the desktop host duplicate runtime orchestration or create app-local DTO truth.
  - Shipping a Tauri shell that still depends on untracked manual bridge injection.
  - Allowing unsupported local trigger types to look usable even though local ingress is absent.
  - Introducing transport constants in multiple sources again.
- Validation:
  - Add failing Rust host tests first for seed, command round-trip, approval resume, automation lifecycle/manual dispatch, and cron tick.
  - Add failing TypeScript contract tests first for shared transport constants and Tauri bridge registration.
  - Re-run full workspace verification and a desktop-host startup smoke check before claiming completion.
