# Workspace Automations Removal Design

**Goal:** Remove the workspace-level automations feature from Octopus so the product no longer exposes, transports, persists, or authorizes this business capability.

## Scope

- Remove the desktop route, sidebar entry, navigation registry entry, view, store, locale copy, and workspace client methods for workspace automations.
- Remove the OpenAPI path items and transport schema for workspace automations, then regenerate bundled OpenAPI and generated TypeScript transport output.
- Remove the server routes, handlers, service trait methods, infra persistence, permission definitions, and menu definitions that only exist for workspace automations.
- Remove related fixture state and repository governance assertions that still treat workspace automations as a supported workspace surface.

## Non-Goals

- Do not remove unrelated runtime workflow affordance fields or generic “automation” wording that is not the workspace automations feature itself.
- Do not refactor adjacent workspace console, access-control, pet, agent, or team features beyond what is required to compile and test after removal.

## Design

The removal should be complete rather than cosmetic. The desktop shell must stop linking to `workspace-automations`, the adapter surface must stop exposing `/api/v1/workspace/automations`, and the backend must stop declaring that route family in OpenAPI, Axum routing, permissions, menus, and SQLite bootstrap state.

The transport-first order stays intact:

1. Remove workspace automation definitions from `contracts/openapi/src/**`.
2. Regenerate `contracts/openapi/octopus.openapi.yaml` and `packages/schema/src/generated.ts`.
3. Remove TypeScript adapter/store/view usage and handwritten schema aliases.
4. Remove Rust route/service/persistence code and related tests.

## Verification

- Frontend parity and governance tests should stop expecting workspace automation routes.
- The OpenAPI parity collectors should no longer report `/api/v1/workspace/automations`.
- Workspace bootstrap tests should confirm the `automations` table is no longer created.
- Targeted desktop Vitest, schema checks, and targeted Rust tests should pass after regeneration.
