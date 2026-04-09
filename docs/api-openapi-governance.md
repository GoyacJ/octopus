# API And OpenAPI Governance

This document is the canonical policy for AI agents and human contributors changing frontend/backend HTTP contracts in Octopus.

## Architecture Baseline

- Pages and feature views must call stores or view-model actions, not HTTP clients directly.
- Stores call the adapter layer. The adapter layer is the only HTTP transport boundary for desktop product flows.
- Backend `/api/v1/*` routes are the public HTTP transport surface.
- OpenAPI governs HTTP request/response contracts only.
- OpenAPI does not govern Tauri invoke payloads, UI-only view models, or internal domain-only data structures.

## Canonical Sources

- `contracts/openapi/src/**` is the only human-maintained HTTP contract source.
- `contracts/openapi/octopus.openapi.yaml` is the bundled OpenAPI artifact. Do not hand-edit it.
- `packages/schema/src/generated.ts` is the generated TypeScript transport artifact. Do not hand-edit it.
- For any payload already represented in OpenAPI, generated transport types are the canonical TypeScript source.
- Handwritten files under `packages/schema/src/*` may keep domain models, UI models, and Tauri-specific payloads, but they must not maintain a parallel handwritten HTTP truth source once an HTTP payload is covered by OpenAPI.
- Compatibility exports are allowed only as one-way alias or re-export layers pointing back to generated transport declarations.

## Directory And Ownership Rules

- `contracts/openapi/src/root.yaml` composes the contract tree and must remain minimal.
- `contracts/openapi/src/paths/*.yaml` only define path items and operations.
- `contracts/openapi/src/components/schemas/*.yaml` only define shared transport schemas.
- `contracts/openapi/src/components/parameters/*.yaml` only define reusable transport parameters and headers.
- `contracts/openapi/src/components/responses/*.yaml` only define reusable transport responses.
- Path files must not inline large object schemas. Reusable request/response bodies must live under `components`.
- Transport-only records should be modeled as dedicated transport schemas. Do not force richer UI or domain models into the HTTP contract just because they look similar.
- `misc.yaml` is for a small number of temporary or cross-domain top-level routes only. New feature work must not default to `misc`; when a family grows, promote it into its own domain file.

## How To Add Or Change An HTTP Endpoint

The required order is fixed:

1. Update `contracts/openapi/src/**`.
2. Run `pnpm openapi:bundle`.
3. Run `pnpm schema:generate`.
4. Update `@octopus/schema` aliases or exports if needed.
5. Update adapters, stores, server implementation, and tests.
6. Run `pnpm schema:check` and the relevant frontend or backend verification commands.

Forbidden workflow:

- do not add a new server route first and promise to document it later
- do not add a handwritten adapter URL first and patch generated typing later
- do not patch `octopus.openapi.yaml` or `generated.ts` directly to skip the source pipeline

## Frontend Transport Rules

- `apps/desktop` business code must not use bare `fetch` for workspace or host business APIs.
- Host requests must go through `apps/desktop/src/tauri/shell.ts`.
- Workspace and runtime requests must go through `apps/desktop/src/tauri/workspace-client.ts`.
- Shared request construction, auth headers, request IDs, idempotency handling, and error decoding must stay in `apps/desktop/src/tauri/shared.ts`.
- Generated OpenAPI path literals, path params, response types, and error types must be consumed from `@octopus/schema/generated`.
- Views and stores must not assemble bearer tokens, workspace headers, request IDs, idempotency keys, or SSE resume headers themselves.
- Browser host and Tauri host must expose the same public adapter contract shapes.

## Backend Transport Rules

- `crates/octopus-server` owns HTTP transport, authentication, request headers, idempotency behavior, CORS, and response shaping.
- `crates/octopus-platform` owns typed service and platform contracts below the transport boundary.
- Server handlers must validate request shape at the HTTP boundary before reaching service logic.
- Every `/api/v1/*` route must exist in OpenAPI.
- Service and transport layers must return typed results rather than ad-hoc JSON maps.

## Transport Conventions

- Every backend request carries `X-Request-Id`.
- Workspace-scoped requests carry `X-Workspace-Id`.
- Authenticated workspace requests use `Authorization: Bearer <token>`.
- Retryable mutations support `Idempotency-Key`.
- SSE resume uses `Last-Event-ID`.
- Error payloads use `ApiErrorEnvelope`.
- Polling and SSE transport for the same logical stream should share one OpenAPI path when they expose the same logical endpoint.
- File upload, void response, and pagination behavior must still be represented explicitly in the OpenAPI contract and adapter surface.

## Exception Policy

- The default is zero transport exceptions.
- If OpenAPI cannot accurately express a transport surface, the exception must be explicit, narrow, and temporary.
- Any exception must be reflected in parity policy and documented with:
  - why the exception exists
  - what exact transport surface is excluded
  - what behavior must still remain stable
  - what condition removes the exception
- Exception policy must not become a way to bypass OpenAPI-first development.

## Required Verification

- After changing OpenAPI source, always run:
  - `pnpm openapi:bundle`
  - `pnpm schema:generate`
  - `pnpm schema:check`
- When desktop adapters, stores, or generated transport types are touched, also run `pnpm check:frontend`.
- When server transport code changes, also run the relevant Rust or integration verification for the affected surface.
- If a change updates policy or governance rules, update the canonical policy doc first, then audit or release-oriented companion docs.

## Relationship To Other Docs

- `docs/openapi-audit.md` records current coverage and migration status. It is not the primary rule source.
- `docs/release-governance.md` records how bundled OpenAPI artifacts participate in release governance.
- `docs/runtime_config_api.md` records runtime-config-specific public API behavior and should complement this policy rather than redefine it.
