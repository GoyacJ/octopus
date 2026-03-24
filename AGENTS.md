# AGENTS.md

## Repository expectations

- This repository is currently in a doc-first plus scaffold-baseline stage. Before proposing scaffolding, code, contract, or architecture changes, read `docs/PRD.md`, `docs/SAD.md`, and `docs/DEVELOPMENT_STANDARDS.md`.
- Before implementation, also read `docs/VIBECODING.md` for execution-process constraints and `docs/VISUAL_FRAMEWORK.md` for UI information architecture and visual boundaries. These two docs do not override the primary product, architecture, or engineering sources.
- Before starting implementation work for a phase or slice, also read the current execution docs in `docs/plans/` and the relevant records in `docs/changes/`. Treat them as the source of truth for current delivery order, phase status, exit criteria, and evidence expectations.
- Treat those three documents as the source of truth for product scope, architecture boundaries, and engineering standards. If code or requests conflict with them, call out the conflict before proceeding.
- Treat `docs/plans/` and `docs/changes/` as execution tracking documents only. They guide sequencing and status, but they do not override the product, architecture, or engineering source documents.
- The repository already contains Phase 0-2 baseline work: planning/change tracking, contract skeletons under `proto/`, and workspace scaffolding under `apps/`, `packages/`, and `crates/`. Do not describe these skeletons as full product capability.
- The working tree may already contain user changes. Never overwrite or revert unrelated edits. Prefer additive changes and explain conflicts early.
- Keep repository-wide guidance here in the root file. If a future subdirectory needs different rules, add a nearby `AGENTS.override.md` instead of expanding this file indefinitely.

## Project guardrails

- The project baseline is a single-repo monorepo. Node-side code must use `pnpm` workspaces plus `Turborepo`. Rust-side code must use a Cargo workspace.
- Do not introduce a second primary frontend stack, backend stack, package manager, or build system.
- Frontend and client work must follow the approved Vue baseline: Vue 3, TypeScript, Vite, Vue Router, Pinia, VueUse, UnoCSS, Vue I18n, self-built UI components, shared design tokens, and Tauri 2 / Tauri Mobile for desktop and mobile shells.
- Do not introduce React, Next.js, Nuxt, third-party themed UI kits, or ad hoc CSS systems as project defaults.
- Backend work must follow the approved Rust baseline: Rust stable, Tokio, Axum, Tonic, SQLx, Serde, tracing/OpenTelemetry, modular monolith boundaries, and adapter-based integrations.
- Database work must preserve the approved support model: SQLite is the default local/personal database, PostgreSQL is the recommended team/production database, and core paths must remain compatible with both.
- Redis and S3 are optional adapters only. Do not make them mandatory for the default local setup, and do not scatter their SDK usage through business logic.
- External HTTP contracts belong in OpenAPI. Internal RPC contracts belong in Protobuf/Buf. Plugin contracts belong in schema/manifest definitions. Do not hand-maintain drifting duplicate types.

## Frontend UX and design system rules

- All UI work must use the shared design system. Tokens are the only visual source of truth for color, spacing, typography, radius, shadows, motion, and icon sizing.
- Theme support is mandatory: `system`, `light`, and `dark`.
- Internationalization is mandatory for user-facing UI. At minimum, support `zh-CN` and `en-US`. Do not hardcode shipped UI copy directly in components.
- UI should follow the approved product design direction: modern minimalism, calm hierarchy, consistent spacing, smooth interactions, and an Apple/Google-like polish without copying either system directly.
- Self-built components are required. Low-level primitives may be wrapped internally, but business code should only consume the internal component library.
- Use the approved icon strategy: Iconify/UnoCSS icons with Lucide as the default functional icon family, `simple-icons` only for brands, and internal wrappers for product-specific icons.

## Coding and review rules

- Code identifiers, schema fields, config keys, commit types, and code comments should be in English. Repository docs and architecture docs should stay in Chinese unless a file already establishes a different convention.
- Prefer clear, explicit code over clever shortcuts. Keep domain logic out of handlers, views, and transport layers.
- For Vue, use Composition API and `<script setup lang="ts">` by default. Prefer Pinia setup stores, Vue Router route modules, and a clean separation between server state and UI state.
- For Rust, keep domain, application, infrastructure, and transport boundaries explicit. Domain code must not depend directly on Axum, Tonic, SQLx, Redis, S3, or filesystem SDKs.
- When adding Node tasks, define scripts in each package/app and register them in `turbo.json`. Root `package.json` should only delegate via `turbo run ...`.
- Architecture-level, contract-level, token-system, component-API, and database-strategy changes must be reflected in docs and, when material, in `docs/adr/`.
- If a change advances or completes a roadmap phase, update the corresponding phase status in `docs/plans/` and add or update the matching `docs/changes/` record in the same change.
- When repository entry points change, keep `README.md`, `.github/pull_request_template.md`, `lefthook.yml`, and `.github/workflows/guardrails.yml` aligned with the new baseline where relevant.

## VibeCoding rules

- Humans own scope boundaries, acceptance criteria, ADR approvals, and risk sign-off. AI executes within those boundaries; it must not expand scope on its own.
- Default delivery order is: governance baseline, contract skeleton, repository scaffold, one MVP vertical slice, then the next slice.
- AI must stay inside the approved directory, contract, and task boundary for the current change. If a request would cross architecture or policy boundaries, stop and surface the conflict.
- Safety controls are not deferrable. Approval, sandbox, audit, and resume/freshness checks must enter the design before broader capability expansion.
- When failures occur, use the repository taxonomy first, then isolate the boundary, then apply the smallest repair and re-verify.

## Verification and delivery

- Before claiming work is complete, run the relevant checks that actually exist in the repository at that point. Do not claim success based on expected future tooling.
- When Node workspace tooling exists, prefer repo-standard `pnpm` + `turbo run` commands. When Rust workspace tooling exists, prefer Cargo workspace commands.
- At the current repository stage, the default truthful verification set is: `pnpm install`, `pnpm lint`, `pnpm typecheck`, `pnpm test`, `pnpm build`, `cargo metadata --no-deps --format-version 1`, `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test`.
- Contract verification should include `buf lint`, OpenAPI lint, and generation-sync checks when those tools are available. If `buf`, `spectral`, or equivalent tooling is not installed yet, say so explicitly instead of implying contract lint passed.
- UI changes should be reviewed in both light and dark themes, and in both Chinese and English when copy is affected.
- Store UI evidence in `output/playwright/` unless a more specific repository convention is introduced later.
- Phase completion claims must include updated status in `docs/plans/`, an updated `docs/changes/` record, and verification evidence that matches what was actually run.
- If the required verification stack is missing because the repo is still being scaffolded, say so explicitly and verify at the highest truthful level available.
