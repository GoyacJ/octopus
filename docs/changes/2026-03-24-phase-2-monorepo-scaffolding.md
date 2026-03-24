# Stage Change Record: Phase 2 Monorepo Scaffolding

- Stage: `phase-2`
- Status: `Done`
- Last Updated: `2026-03-24`
- Related Plan: `docs/plans/2026-03-24-v1-development-roadmap.md`

## Summary

- Initialized the first `apps/web` control plane shell with Vue 3, Vite, Vue Router, Pinia, Vue I18n, UnoCSS, theme switching, and language switching.
- Initialized workspace packages for design tokens, UI, icons, i18n, composables, shared navigation metadata, API client placeholder, tsconfig, and eslint config.
- Converted the Rust side from an empty workspace into a real multi-crate workspace with explicit crate boundaries and minimal runtime/domain placeholders.

## Scope

- In scope:
  - Root `package.json`, `pnpm-workspace.yaml`, `eslint.config.mjs`, and workspace dependency installation
  - `apps/web` Vite shell and first control plane surfaces
  - `packages/design-tokens`, `ui`, `icons`, `i18n`, `composables`, `shared`, `api-client`, `tsconfig`, `eslint-config`
  - Cargo workspace members and per-crate `Cargo.toml` / `src/lib.rs`
  - README/guardrail sync for the new planning and scaffolding baseline
- Out of scope:
  - Real backend transport implementations
  - Real API client generation
  - Phase 3 runtime event model and end-to-end MVP behavior

## Risks

- Main risk:
  - Several packages are intentionally source-first and minimal; future phases must avoid letting placeholder code drift from the formal contracts.
- Rollback or mitigation:
  - Keep the workspace package boundaries stable and replace placeholder internals behind the same package/crate boundaries as implementation matures.

## Verification

- Commands run:
  - `pnpm install`
  - `pnpm lint`
  - `pnpm typecheck`
  - `pnpm test`
  - `pnpm build`
  - `cargo metadata --no-deps --format-version 1`
  - `cargo fmt --check`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - `cargo test`
- Manual checks:
  - Reviewed the Web shell in light and dark themes and in both `zh-CN` and `en-US`.
  - Added `favicon.svg` after browser verification surfaced a missing favicon 404.

## Docs Sync

- [ ] `docs/PRD.md`
- [ ] `docs/SAD.md`
- [ ] `docs/DEVELOPMENT_STANDARDS.md`
- [ ] `docs/VIBECODING.md`
- [ ] `docs/VISUAL_FRAMEWORK.md`
- [ ] `docs/adr/`
- [x] `docs/plans/`
- [x] `docs/changes/`
- [ ] No doc update needed

## UI Evidence

- [ ] Not applicable
- [x] Light theme screenshot attached
- [x] Dark theme screenshot attached
- [x] zh-CN screenshot attached
- [x] en-US screenshot attached

Attached screenshot paths:

1. `output/playwright/octopus-zh-light.png`
2. `output/playwright/octopus-en-light.png`
3. `output/playwright/octopus-en-dark.png`
4. `output/playwright/octopus-zh-dark.png`

## Review Notes

- ADR or architecture impact:
  - None. This phase implements the already approved Vue and Rust workspace baseline without changing primary stack or directory boundaries.
- Security or policy impact:
  - None at this stage; shell and crate internals remain non-production placeholders.
- Contract or schema impact:
  - The Web shell and Rust workspace now depend on the newly introduced contract-source baseline and source-first workspace packages.
- Blocking reason:
  - None.
- Next action:
  - Start Phase 3 by binding the Phase 1 contracts to the first `run -> interaction/approval -> resume -> timeline/audit` MVP slice.
