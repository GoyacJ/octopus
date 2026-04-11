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

Every desktop release must pass:

- `pnpm check:desktop-release`
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --locked`
- `pnpm schema:check`
- `pnpm version:check`

`pnpm check:desktop-release` is the desktop packaging and publication gate.

`pnpm check:all` remains the full repository gate, and `pnpm check:website` stays independent from desktop bundle publication.

Because the Rust workspace includes the Tauri desktop shell, Ubuntu-based CI and release verification jobs must install the official Tauri Linux system dependencies before running `cargo clippy` and `cargo test`. The governance target is the full workspace gate, not a reduced Linux-only crate subset.

Because `apps/desktop/src-tauri/tauri.conf.json` bundles `bin/octopus-desktop-backend` as an external sidecar, Rust verification must prepare the sidecar binary before workspace compilation-driven checks. `pnpm check:rust` and the Ubuntu workflows both enforce this precondition explicitly.

## Formal Release Flow

- release publication is tag-driven only: pushing `vX.Y.Z` runs `.github/workflows/release.yml`
- the release workflow builds real desktop bundles on macOS, Linux, and Windows runners
- formal desktop release coverage is fixed to:
  - macOS `aarch64` and `x86_64` as separate installer artifacts
  - Linux `x86_64` as both `AppImage` and `DEB`
  - Windows `x64` and `ARM64` as `NSIS` installers
- hosted Windows release builds publish the NSIS installer path and do not rely on WiX/MSI, because GitHub-hosted runner environments do not provide a stable `light.exe` execution surface for formal releases
- `pnpm release:collect-artifacts` normalizes Tauri bundle output into `release-artifacts/publish/<platform>`
- `pnpm release:verify-artifacts` blocks publication unless release metadata plus every required desktop platform artifact variant is present
- GitHub Releases only upload the verified release directory:
  - `release-artifacts/publish/macos/*`
  - `release-artifacts/publish/linux/*`
  - `release-artifacts/publish/windows/*`

## Preview Release Flow

- preview publication is batch-driven: manually dispatching `.github/workflows/release-preview.yml` from `main` produces a preview release
- preview releases do not rewrite `VERSION`; they derive a unique prerelease tag as `vX.Y.Z-preview.<run_number>`
- preview releases reuse the same metadata normalization, artifact collection, checksum generation, and per-platform artifact verification gates as formal releases
- preview GitHub Releases are published as `prerelease=true` and are not marked as latest
- preview release notes default their change range to the previous preview tag in the same channel unless `--since-ref` is supplied explicitly
  - `release-artifacts/metadata/*`
  - `release-artifacts/SHA256SUMS.txt`

## Release Notes Governance

- release notes are generated from structured fragments plus Git metadata, not handwritten free-form Markdown dumps
- `docs/release-notes/README.md` defines the supported fragment categories and writing rules
- release generation produces:
  - the rendered Markdown notes file
  - `release-artifacts/metadata/release-notes.json`
  - `release-artifacts/metadata/change-log.json`
- release note正文 only comes from fragments under `docs/release-notes/fragments/*`
- Git history is used only to determine release range, compute diagnostics, and warn when releasable changes landed without matching fragments
- formal releases must include at least one `summary-*` fragment within the current formal release range as a manually reviewed version overview
- preview releases may use an auto-generated overview paragraph, but the default user-facing body still comes from fragments only
- release ranges are channel-local by default:
  - formal: previous formal tag -> current formal tag
  - preview: previous preview tag -> current preview tag
- `internal-*` and `governance-*` fragments do not enter the default user-facing body for either channel
- formal notes emphasize:
  - 版本概览
  - 用户可感知变化
  - 升级提示
  - 修复摘要
  - 技术附录
- preview notes emphasize:
  - 预览摘要
  - 本次变更
  - 验证状态
  - 构建元数据
- fragment lifecycle is fixed:
  - preview releases do not consume fragments
  - formal releases consume the fragments selected for the current formal range
  - consumed formal fragments are archived under `docs/release-notes/archive/<tag>/`
  - `pnpm release:archive-fragments -- --tag vX.Y.Z` is the supported archive command
- the repository does not maintain a hand-edited root `CHANGELOG.md`; versioned release notes and archives are the historical source of truth

## Shared Schema Governance

- `docs/api-openapi-governance.md` is the canonical development policy for HTTP contract work.
- `contracts/openapi/src/**` is the only human-maintained OpenAPI source tree.
- `contracts/openapi/octopus.openapi.yaml` is the bundled canonical protocol artifact consumed by release metadata, parity gates, and schema generation.
- `packages/schema/src/generated.ts` is the generated TypeScript transport artifact derived from the bundled OpenAPI spec.
- Generated contract drift is blocked by `pnpm schema:check`.
- `pnpm schema:check` now enforces three gates:
  - bundled artifact drift between `contracts/openapi/src/**` and `contracts/openapi/octopus.openapi.yaml`
  - generated schema drift
  - server `/api/v1/*` route parity against OpenAPI plus `contracts/openapi/route-parity-allowlist.json`
  - frontend adapter parity against OpenAPI plus `contracts/openapi/adapter-parity-allowlist.json`

OpenAPI maintenance order is now fixed:

1. edit `contracts/openapi/src/**`
2. run `pnpm openapi:bundle`
3. run `pnpm schema:generate`
4. wire implementation and tests against the generated surface

Current scope:

- host bootstrap, health, preferences, workspace connections, and notifications contracts
- workspace bootstrap, health, auth, summary, and overview contracts
- project, resource, knowledge, and artifact contracts for the primary workspace/project surfaces
- runtime effective config contract
- shared error envelope

Temporary migration rules:

- allowlists are transitional inventory, not a second source of truth
- new HTTP routes must be added to OpenAPI first or explicitly added to an allowlist with audit context
- transport types already represented in OpenAPI should resolve back to generated aliases instead of parallel handwritten declarations

This is the governance foundation for keeping the HTTP contract source modular without changing downstream release and verification behavior.
