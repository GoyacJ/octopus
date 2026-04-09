# OpenAPI HTTP Contract Audit

## Current State

- Human-maintained OpenAPI source now lives under `contracts/openapi/src/**`.
- `contracts/openapi/octopus.openapi.yaml` remains the bundled canonical HTTP protocol artifact for parity checks, release metadata, and schema generation.
- Generated transport types live at `packages/schema/src/generated.ts`.
- OpenAPI now covers 91 HTTP paths across every `/api/v1/*` server route.
- Route parity and adapter parity allowlists are both empty: `route allowlist = 0`, `adapter allowlist = 0`.

## Coverage Snapshot

| Area | Covered in OpenAPI | Explicit exceptions | Risk |
| --- | --- | --- | --- |
| host | bootstrap, health, preferences, workspace connections, notifications | none | low |
| system/auth | system bootstrap, system health, auth login/register/logout/session | none | medium |
| workspace root | workspace summary, overview, workspace resources, workspace knowledge, pet, teams, RBAC, user-center profile/password, apps, inbox, audit, top-level knowledge | none | low |
| project/resource | projects, project update, dashboard, runtime config, resources, knowledge, artifacts, pet, team-links | none | medium |
| catalog/agent/team/rbac | catalog model snapshot, tool catalog, skills, MCP servers, provider credentials, tools, agents, agent-links, automations, teams, RBAC, user-center overview, user runtime config | none | low |
| runtime | runtime config, validate/probe, bootstrap, scoped config save, sessions, turns, approvals, JSON event polling, and SSE event streaming on the same path surface | none | medium |

## Governance Assets

- `docs/api-openapi-governance.md` is the canonical policy for AI and human contributors changing HTTP contracts, adapters, and OpenAPI source.
- `scripts/bundle-openapi.mjs` bundles `contracts/openapi/src/root.yaml` into the committed `contracts/openapi/octopus.openapi.yaml` artifact with stable ordering.
- `scripts/check-openapi-route-parity.mjs` compares server `/api/v1/*` routes against OpenAPI plus `contracts/openapi/route-parity-allowlist.json`.
- `scripts/check-openapi-adapter-parity.mjs` compares frontend adapter API paths against OpenAPI plus `contracts/openapi/adapter-parity-allowlist.json`.
- Both allowlist files remain in the repo as empty arrays so parity failures still have an explicit review point.
- `pnpm schema:check` runs bundled artifact drift, generated drift, server parity, and adapter parity in one gate.

## Canonical Transport Sources

The following transport-facing TypeScript declarations now resolve back to `packages/schema/src/generated.ts` instead of remaining parallel handwritten definitions:

- `ClientAppRecord` in `packages/schema/src/app.ts`
- `AuditRecord` in `packages/schema/src/observation.ts`
- `InboxItemRecord` in `packages/schema/src/transport-records.ts`
- `KnowledgeEntryRecord` in `packages/schema/src/transport-records.ts`
- previously migrated host, auth, workspace, catalog, and runtime HTTP payloads that already alias to generated transport types

Richer non-HTTP models remain handwritten by design:

- UI/domain inbox models such as `InboxItem` and `InboxApproval`
- richer knowledge models such as `KnowledgeEntry` and workspace/project `KnowledgeRecord`
- Tauri invoke payloads, local domain models, and UI-only shapes

## Runtime SSE Status

- Runtime SSE no longer bypasses the generated OpenAPI path surface.
- `apps/desktop/src/tauri/shared.ts` now exposes a stream-aware helper that resolves generated OpenAPI path literals, path params, and query params before opening the request.
- `apps/desktop/src/tauri/workspace-client.ts` binds `subscribeEvents` to `/api/v1/runtime/sessions/{sessionId}/events` while preserving `Accept: text/event-stream`, `Last-Event-ID`, `after`, and resume semantics.
- The same OpenAPI path now governs both polling (`application/json`) and streaming (`text/event-stream`) transport on the frontend adapter surface.

## Exit Criteria Status

- Every `/api/v1/*` backend route appears in OpenAPI.
- Every frontend adapter request uses an OpenAPI path from the generated surface.
- Route parity and adapter parity allowlists are empty.
- Covered transport types resolve to generated definitions instead of parallel handwritten interfaces.

## Ongoing Rule

Any new `/api/v1/*` route or adapter request must follow the canonical policy in `docs/api-openapi-governance.md`. The required order remains:

1. Update `contracts/openapi/src/**`.
2. Run `pnpm openapi:bundle`.
3. Run `pnpm schema:generate`.
4. Consume the generated path/type surface from adapters and schema exports.
5. Implement or update the server route and tests.

Changes that skip this order should fail parity or generated drift checks.
