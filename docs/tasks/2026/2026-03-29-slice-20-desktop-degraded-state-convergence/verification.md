# Verification

## Unit Tests

- `pnpm --filter @octopus/desktop exec vitest run test/remote-connections.test.ts`
  - Passed
  - Covers the new Slice 20 workbench-global degraded banner, memory-only shell warning, reconnect-clears-banner path, and remote-workspace switch isolation, alongside the existing Slice 19 restore/auth/logout regressions.

## Integration Tests

- `cargo test --workspace`
  - Passed
- `pnpm test:ts`
  - Passed

## Contract Tests

- No shared-contract changes in this slice.

## Failure Cases

- Cached restore with transport failure still lands on the remembered workbench route and now shows a shell-level degraded banner.
- Secure-storage-unavailable login still succeeds and now surfaces a shell-level memory-only warning.
- Route-entry refresh after connectivity recovery clears the degraded banner.
- Switching to a different remote workspace clears the prior degraded-session context and remembered project.
- Auth-invalid restore continues to fall back to `/connections`.

## Boundary Cases

- Remote-mode authenticated state with warning remains writable and shows warning-only shell treatment.
- Remote-mode unauthenticated workbench entry shows `auth_required` rather than leaking `restored_but_disconnected`.
- Local mode remains outside the degraded-state banner model.

## Manual Verification

- None beyond automated route and shell coverage in this slice.

## Static Checks

- `pnpm typecheck:ts`
  - Passed
- `pnpm build:ts`
  - Passed

## Remaining Gaps

- No slice-local blockers remain after the required workspace gates.

## Confidence Level

- High.
