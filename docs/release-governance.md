# Release Governance

## Release Source Of Truth

- `main` is the only release branch.
- Git tags in the form `vX.Y.Z` trigger formal releases.
- `VERSION` is the single product version source.

Mirrored version fields are validated in:

- root `package.json`
- `apps/desktop/package.json`
- `packages/schema/package.json`
- `packages/ui/package.json`
- `apps/desktop/src-tauri/tauri.conf.json`
- root `Cargo.toml` workspace package version
- `contracts/openapi/octopus.openapi.yaml`

## Quality Gates

Every release must pass:

- `pnpm check:frontend`
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --locked`
- `pnpm schema:check`
- `pnpm version:check`

`pnpm check:all` is the local and CI entrypoint for the full repository gate.

Because the Rust workspace includes the Tauri desktop shell, Ubuntu-based CI and release verification jobs must install the official Tauri Linux system dependencies before running `cargo clippy` and `cargo test`. The governance target is the full workspace gate, not a reduced Linux-only crate subset.

## Formal Release Flow

- release publication is tag-driven only: pushing `vX.Y.Z` runs `.github/workflows/release.yml`
- the release workflow builds real desktop bundles on macOS and Windows runners
- `pnpm release:collect-artifacts` normalizes Tauri bundle output into `release-artifacts/publish/<platform>`
- `pnpm release:verify-artifacts` blocks publication unless release metadata plus formal macOS and Windows installers are present
- GitHub Releases only upload the verified release directory:
  - `release-artifacts/publish/macos/*`
  - `release-artifacts/publish/windows/*`
  - `release-artifacts/metadata/*`
  - `release-artifacts/SHA256SUMS.txt`

## Shared Schema Governance

- `contracts/openapi/octopus.openapi.yaml` is the canonical protocol spec.
- `packages/schema/src/generated.ts` is generated from the OpenAPI spec.
- Generated contract drift is blocked by `pnpm schema:check`.

Current scope:

- host bootstrap and health contracts
- workspace bootstrap and overview contracts
- runtime effective config contract
- shared error envelope

This is the governance foundation for migrating remaining transport contracts onto the same OpenAPI source.
