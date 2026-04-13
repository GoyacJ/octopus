# Phase 7 Contract And Host Projection

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Expose the rebuilt runtime, memory, policy, team, and workflow platform through one consistent public contract across OpenAPI, generated TypeScript, server projection, browser host, Tauri host, and desktop product surfaces.

**Architecture:** Phases 1 through 6 are assumed complete before this phase starts. By this point the runtime trunk, subrun substrate, memory plane, and control plane already exist. Phase 7 does not invent another runtime feature layer. It is the public cutover and projection hardening phase. OpenAPI becomes the only human-maintained HTTP contract source, `packages/schema/src/*` becomes a clean feature-based export surface over generated transport types, server projection code exposes runtime truth without opaque JSON leaks, and desktop browser or Tauri hosts consume the same adapter contract without local shape repair.

**Tech Stack:** OpenAPI sources under `contracts/openapi/src/**`, generated transport under `packages/schema/src/generated.ts`, feature-based schema exports under `packages/schema/src/*`, server and platform transport code in `crates/octopus-server` and `crates/octopus-platform`, shared desktop adapters in `apps/desktop/src/tauri/*`, shared runtime store and views in `apps/desktop/src/stores` and `apps/desktop/src/views`, and parity tests under `apps/desktop/test/*`.

---

## Fixed Decisions

- `docs/api-openapi-governance.md` remains the canonical contract policy. Phase 7 implements that policy; it does not replace it.
- OpenAPI is the only human-maintained HTTP truth source. Generated transport types are the canonical TypeScript HTTP payload source.
- Feature-based files under `packages/schema/src/*` may alias or re-export generated transport types, but they must not grow a parallel handwritten HTTP truth source.
- Browser host and Tauri host must expose the same public adapter contract shapes.
- Stores and views consume runtime state through the shared adapter boundary. They do not assemble HTTP payloads or synthesize missing runtime fields locally.
- Fixtures and tests must reuse `@octopus/schema` transport types and final server shapes. They are not allowed to preserve stale compatibility fields once the public contract is cut over.
- Runtime projection fields must come from server or runtime persistence truth, not from store-local inference.
- This phase does not create a second host-specific projection API for browser versus Tauri.

## Scope

This phase covers:

- final OpenAPI and `@octopus/schema` alignment for runtime-facing features
- server projection and transport shaping for runtime, workflow, memory, approval, and trace data
- browser-host and Tauri-host parity on the same runtime contract
- removal of remaining transport-shape repair in adapters, stores, or fixtures
- end-to-end contract verification across server, adapter, fixture, and store layers

This phase does not cover:

- new runtime semantics that belong to earlier phases
- Tauri invoke contract redesign outside the shared adapter boundary
- product-surface redesign unrelated to contract parity
- resurrecting temporary compatibility shells after cutover

## Current Baseline

The current repository is already significantly closer to host parity than the early rebuild phases, but final convergence is not done yet.

- `apps/desktop/src/tauri/runtime_api.ts` already routes runtime requests through `fetchWorkspaceOpenApi(...)`, which means runtime traffic is already adapter-first and OpenAPI-backed.
- `apps/desktop/src/tauri/workspace-client.ts` already defines one shared runtime client surface consumed by product code.
- Host parity tests already exist, including `apps/desktop/test/tauri-client-host.test.ts`, `apps/desktop/test/tauri-client-runtime.test.ts`, `apps/desktop/test/openapi-transport.test.ts`, and `apps/desktop/test/runtime-store.test.ts`.
- `packages/schema/src/agent-runtime.ts` and `packages/schema/src/memory-runtime.ts` are already re-export or alias surfaces over generated transport types instead of large handwritten wrappers.
- Even with that progress, `apps/desktop/src/tauri/runtime_api.ts` still normalizes UI-facing permission input through `resolveRuntimePermissionMode(...)` before submit-turn transport, which means the adapter still performs transport-shape correction instead of acting as a narrow pass-through over final request types.
- As runtime transport grows to include capability, team, workflow, memory, and policy state, fixtures and stores remain the most likely place for drift if cutover is not forced explicitly.
- The runtime surface still needs one final pass where OpenAPI, generated types, server responses, adapter return types, fixtures, and store expectations all agree without extra shape repair.

Phase 7 starts from that real state: the adapter boundary and many tests already exist, but the remaining drift must be removed rather than tolerated.

## Task 1: Finish Feature-Based Schema Layout And Remove Remaining Handwritten HTTP Drift

**Files:**
- Modify: `packages/schema/src/index.ts`
- Modify: `packages/schema/src/runtime.ts`
- Modify: `packages/schema/src/agent-runtime.ts`
- Modify: `packages/schema/src/memory-runtime.ts`
- Modify: `packages/schema/src/runtime-policy.ts`
- Modify: `packages/schema/src/permissions.ts`
- Modify: `contracts/openapi/src/components/schemas/runtime.yaml`
- Modify: `contracts/openapi/src/paths/runtime.yaml`

**Implement:**
- keep generated transport types as the only HTTP payload truth for runtime-facing features
- keep feature-based schema files as re-export or alias surfaces and move any non-HTTP helpers into clearly non-transport modules
- remove remaining handwritten contract glue that preserves obsolete transport behavior
- make runtime-facing feature files line up with the runtime phase split instead of accumulating generic exports in `runtime.ts`

**Required feature surfaces:**
- `actor-manifest`
- `agent-runtime`
- `memory-runtime`
- `workflow-runtime`
- `runtime-policy`
- `capability-runtime`
- `team-runtime`

**Verification:**
```bash
pnpm openapi:bundle
pnpm schema:generate
pnpm schema:check
```

**Done when:**
- every HTTP payload already covered by OpenAPI is represented in TypeScript by generated transport plus thin feature-based exports only
- `packages/schema` no longer hides transport drift behind handwritten compatibility layers

## Task 2: Align Server Projection And Runtime Transport With The Final Public Contract

**Files:**
- Modify: `crates/octopus-server/src/workspace_runtime.rs`
- Modify: `crates/octopus-platform/src/workspace.rs`
- Modify: `crates/octopus-runtime-adapter/src/session_service.rs`
- Modify: `crates/octopus-runtime-adapter/src/persistence.rs`
- Modify: `crates/octopus-runtime-adapter/src/execution_events.rs`
- Modify: `crates/octopus-infra/src/infra_state.rs`

**Implement:**
- expose runtime session, run, subrun, workflow, mailbox, memory, approval, auth, trace, and background projection fields through typed transport instead of opaque JSON fragments
- keep SQLite summaries, JSONL events, and disk-backed refs as the backend truth while projecting only typed transport summaries outward
- remove server-side fallback shaping that depends on old runtime payload assumptions
- keep projection fields stable across reload, restart, and host choice

**Required projection families:**
- session summary and detail
- run and subrun snapshot
- workflow summary and detail
- memory selection and proposal summary
- pending mediation and auth summary
- trace and event projection

**Verification:**
```bash
cargo test -p octopus-platform
cargo test -p octopus-server
cargo test -p octopus-runtime-adapter
cargo test -p octopus-infra
```

**Done when:**
- server transport exposes the same typed runtime truth that SQLite and JSONL projections describe
- no public runtime surface depends on server-local opaque JSON passthroughs

## Task 3: Remove Remaining Adapter, Store, And Fixture Shape Repair

**Files:**
- Modify: `apps/desktop/src/tauri/runtime_api.ts`
- Modify: `apps/desktop/src/tauri/workspace-client.ts`
- Modify: `apps/desktop/src/stores/runtime.ts`
- Modify: `apps/desktop/src/views/project/ConversationView.vue`
- Modify: `apps/desktop/src/views/project/TraceView.vue`
- Modify: `apps/desktop/test/support/workspace-fixture-runtime.ts`
- Modify: `apps/desktop/test/openapi-transport.test.ts`
- Modify: `apps/desktop/test/runtime-store.test.ts`
- Modify: `apps/desktop/test/tauri-client-runtime.test.ts`

**Implement:**
- narrow `runtime_api.ts` so it forwards final generated request or response shapes instead of normalizing transport semantics on behalf of the server
- move UI-level enum translation or form normalization out of transport-specific adapters when that translation is not part of the runtime HTTP contract
- update fixtures to emit only final contract shapes
- remove store-local assumptions about missing fields, legacy aliases, or partial runtime payloads

**Required host rules:**
- browser host and Tauri host return the same runtime payload shapes
- runtime store accepts server responses without extra patching
- runtime fixtures are valid examples of the real API, not a parallel mock contract

**Verification:**
```bash
pnpm -C apps/desktop exec vitest run test/openapi-transport.test.ts test/runtime-store.test.ts test/tauri-client-runtime.test.ts
```

**Done when:**
- adapter, store, and fixture layers consume the final runtime contract directly
- host parity failures are caught by tests instead of patched locally

## Task 4: Prove Browser-Host And Tauri-Host Parity For Runtime Surfaces

**Files:**
- Modify: `apps/desktop/test/tauri-client-host.test.ts`
- Modify: `apps/desktop/test/tauri-client-runtime.test.ts`
- Modify: `apps/desktop/test/openapi-transport.test.ts`
- Modify: `apps/desktop/test/runtime-store.test.ts`

**Implement:**
- extend parity coverage so both hosts are tested against the same runtime contract for session load, submit turn, approval resolution, event polling or SSE, and projection reload
- include runtime memory, workflow, and mediation summaries in parity assertions instead of limiting parity to basic bootstrap and session CRUD
- keep parity enforcement at the adapter boundary, not only in view-level tests

**Verification:**
```bash
pnpm -C apps/desktop exec vitest run test/tauri-client-host.test.ts test/tauri-client-runtime.test.ts test/openapi-transport.test.ts test/runtime-store.test.ts
```

**Done when:**
- browser and Tauri hosts expose the same runtime semantics through the same adapter contract
- runtime parity no longer depends on host-specific fallback assumptions

## Task 5: Phase-Level Validation And Public Contract Acceptance Fence

**Files:**
- Verify only

**Run:**
```bash
pnpm openapi:bundle
pnpm schema:generate
pnpm schema:check
cargo test -p octopus-platform
cargo test -p octopus-server
cargo test -p octopus-runtime-adapter
cargo test -p octopus-infra
pnpm -C apps/desktop exec vitest run test/tauri-client-host.test.ts test/tauri-client-runtime.test.ts test/openapi-transport.test.ts test/runtime-store.test.ts
git diff --stat -- \
  contracts/openapi/src \
  packages/schema/src \
  crates/octopus-platform/src \
  crates/octopus-server/src \
  crates/octopus-runtime-adapter/src \
  crates/octopus-infra/src \
  apps/desktop/src \
  apps/desktop/test
```

**Acceptance criteria:**
- OpenAPI, generated types, feature-based schema exports, server responses, adapters, fixtures, and stores all agree
- browser host and Tauri host expose the same runtime payload shapes
- runtime projection fields are provided by the server and consumed directly by the store
- no local frontend or host layer remains an extra source of truth for runtime semantics

## Handoff To Later Phases

Phase 7 is complete only when Phase 8 can safely assume:

- every public runtime consumer is already on the final typed contract
- host parity is enforced at the adapter and store boundary
- `packages/schema` no longer preserves hidden transport compatibility logic
- deleting legacy runtime code will not strand a host or frontend on an obsolete shape

Phase 8 can then focus entirely on removing the remaining old execution trunk and compatibility infrastructure.
