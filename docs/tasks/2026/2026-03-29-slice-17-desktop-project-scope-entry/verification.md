# Verification

## Targeted Verification

- `cargo test -p octopus-domain-context --test project_list`
  - Passed
  - Covers workspace-scoped ordering and empty-result behavior for project listing at the context-store boundary.
- `cargo test -p octopus-runtime --test project_list`
  - Passed
  - Covers runtime project list ordering and missing-workspace empty-result behavior.
- `cargo test -p octopus-remote-hub --test auth_surface project_list_route_requires_authentication_and_enforces_workspace_membership`
  - Passed
  - Covers authenticated access, missing-token rejection, and workspace-membership enforcement for the new project list route.
- `cargo test -p octopus-desktop-host --test local_host list_projects_is_workspace_scoped_and_sorted_latest_first`
  - Passed
  - Covers the new local-host transport command and runtime parity from the desktop Tauri surface.
- `pnpm --filter @octopus/hub-client test -- test/hub-client.contract.test.ts`
  - Passed
  - Covers local and remote `listProjects` parity, shared `Project` normalization, and transport command / route expectations.
- `pnpm --filter @octopus/desktop exec vitest run test/remote-connections.test.ts`
  - Passed
  - Covers remote authenticated entry without a remembered project, project selection and persistence, remembered project reuse, stale-project fallback, and local-mode regression.

## Workspace Gates

- `cargo test --workspace`
  - Passed
- `pnpm test:ts`
  - Passed
- `pnpm typecheck:ts`
  - Passed

## Notes

- `crates/access-auth` still emits the previously known dead-code warnings for `display_name`, `last_seen_at`, `created_at`, and `updated_at` during `cargo test --workspace`; they remain non-blocking and out of Slice 17 scope.
- The root `pnpm test:ts` run now includes the updated `schema-ts` owner-contract assertion for the additive `hub:list_projects` command.
