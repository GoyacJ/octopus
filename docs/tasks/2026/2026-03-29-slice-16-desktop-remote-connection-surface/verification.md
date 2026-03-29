# Verification

## Targeted Verification

- `pnpm --filter @octopus/hub-client test -- test/hub-client.contract.test.ts`
  - Passed
  - Covers the auth-only remote client login/session/logout flow, bearer-token injection, and auth-aware normalization including token-expired and workspace-mismatch failures.
- `pnpm --filter @octopus/desktop exec vitest run test/remote-connections.test.ts`
  - Passed
  - Covers remote unauthenticated bootstrap to `/connections`, remote login into the shared workbench, logout back to the read-only connection surface, token-expired UI handling, and regression-free switching back to local mode.

## Workspace Gates

- `cargo test --workspace`
  - Passed
- `pnpm test:ts`
  - Passed
- `pnpm typecheck:ts`
  - Passed

## Notes

- `crates/access-auth` still emits the previously known dead-code warnings for `display_name`, `last_seen_at`, `created_at`, and `updated_at` during `cargo test --workspace`; they remain non-blocking and out of Slice 16 scope.
- The root `pnpm test:ts` run includes the refreshed `schema-ts`, `hub-client`, and desktop suites after the Slice 16 implementation landed.
