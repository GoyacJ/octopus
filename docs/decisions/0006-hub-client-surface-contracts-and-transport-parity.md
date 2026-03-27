# ADR 0006: Hub Client Surface Contracts And Transport Parity

- Status: Accepted
- Date: 2026-03-27
- Deciders: Repository maintainers
- Consulted: PRD, SAD, GA blueprint, GA minimum-surface task package
- Informed: Future desktop, remote-hub, schema-ts, and client-side surface slices

---

## Context

Slice 5 completed the first verified local runtime loop, but the repository still lacks any tracked client-facing transport boundary.

The next GA-facing step introduces:

- the first shared TypeScript schema-consumption package
- the first shared `HubClient`
- the first remote-hub transport shell
- the first desktop surface shell

Without a durable rule here, the repository is likely to drift into:

- local-only page code calling runtime-specific seams directly
- remote-only API DTOs that diverge from local payloads
- transport-specific contracts living in app code instead of `schemas/`
- duplicated business mappings in `apps/desktop` and `apps/remote-hub`

The governing source-of-truth docs already constrain the shape of the answer:

- `schemas/` remains the only cross-language contract source.
- `packages/` owns shared frontend consumer logic.
- `apps/` is assembly only.
- client surfaces must not become remote business truth.
- GA requires both `Desktop` and `Remote Hub`.

## Decision

### 1. One shared `HubClient` boundary owns all client-side runtime access

All tracked desktop and future web/mobile surface code must consume runtime behavior through one shared `HubClient` abstraction in `packages/hub-client`.

That abstraction owns:

- task creation/start entrypoints
- run detail/report reads
- approval resolution
- inbox/notification reads
- artifact and minimum knowledge reads
- minimum capability-visibility reads
- hub-status and SSE subscription entrypoints

Surface code must not call runtime internals, ad hoc fetch wrappers, or Tauri invoke bindings directly.

### 2. Local and remote transport must share one contract set and one behavior surface

The repository adopts transport parity as a durable rule:

- local mode uses `Tauri invoke + event`
- remote mode uses `HTTP + SSE`
- both must expose the same `HubClient` operations
- both must use the same schema-owned payload shapes

Transport differences are adapter details, not product-semantic differences.

### 3. Shared surface payloads live in `schemas/`, not in apps or packages

Any command, query, event, or DTO that crosses the Hub/Client boundary and is shared by Rust and TypeScript belongs in `schemas/`.

`packages/schema-ts` may validate, wrap, and type those contracts for TypeScript consumption, but it may not become the source of truth.

### 4. `apps/remote-hub` remains a thin assembly shell over the existing runtime

The remote-hub app may:

- open/configure runtime state
- map HTTP/SSE requests to runtime method calls
- shape responses using schema-owned DTOs

It may not:

- reimplement runtime orchestration
- redefine shared DTO truth
- introduce a parallel business object model

### 5. `apps/desktop` remains a surface shell and consumes the shared client only

The desktop app owns:

- routing
- view composition
- state management
- Tauri-facing adapter wiring

It may not become the shared truth for DTOs, client transport, or runtime behavior.

## Consequences

### Positive

- Desktop and remote-hub can evolve without creating two product semantics.
- Shared contracts remain reviewable and testable in one place.
- Surface code stays thinner and more replaceable.
- Later web/mobile work can reuse the same client boundary.

### Negative

- The first surface slice must create extra DTO and adapter layers before richer UI work.
- Some runtime records require explicit mapping into client-facing shapes instead of being exposed ad hoc.
- Parity tests become mandatory across two transport adapters.

### Trade-off

The repository accepts a small upfront abstraction and mapping cost to avoid long-term drift between local and remote surfaces.

## Rejected Alternatives

### 1. Let desktop and remote-hub each define their own client and DTO layer

Rejected because it would duplicate behavior, fragment transport semantics, and recreate parallel contract truth.

### 2. Let surface apps call runtime/private transport seams directly

Rejected because it would bypass the package ownership model and make later multi-surface reuse expensive.

### 3. Delay shared client work until after a first UI shell exists

Rejected because the first UI shell would then hard-code accidental transport choices and become difficult to unwind.

## Follow-up

- Add the first minimum-surface Hub/Client schemas.
- Implement `packages/schema-ts` and `packages/hub-client`.
- Implement a thin `apps/remote-hub` router and a minimal `apps/desktop` shell against the shared client.
- Keep remote auth/deployment and richer transport concerns out of this slice.

