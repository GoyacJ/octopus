# OpenAPI AGENTS

- Only hand-edit `contracts/openapi/src/**`.
- Do not hand-edit `contracts/openapi/octopus.openapi.yaml`.
- Do not hand-edit `packages/schema/src/generated.ts`.
- `contracts/openapi/src/paths/*.yaml` only define path items and operations.
- `contracts/openapi/src/components/schemas/*.yaml` only define shared transport schemas.
- `contracts/openapi/src/components/parameters/*.yaml` only define reusable transport parameters and headers.
- `contracts/openapi/src/components/responses/*.yaml` only define reusable transport responses.
- Do not place large inline object schemas in path files; promote reusable transport shapes into `components`.
- New `/api/v1/*` transport work must follow this order: update `src/**`, run `pnpm openapi:bundle`, run `pnpm schema:generate`, then update adapters/server/tests.
- `misc.yaml` is a narrow temporary bucket for a small number of cross-domain routes. Do not default new feature work into `misc`.
- If an HTTP payload is already represented in OpenAPI, TypeScript transport code must resolve back to generated declarations rather than handwritten duplicates.
