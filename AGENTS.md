# AGENTS.md

Repository-wide instructions for Codex. Keep this file short and stable. If a subdirectory needs different rules, add a nearby `AGENTS.override.md` instead of expanding this file.

## Repository Expectations

- This repository is currently `doc-first`. Treat `docs/` and `.github/` as the tracked source of truth, and do not describe the repo as a fully scaffolded monorepo unless the tracked tree actually shows that state.
- Read these documents before product, contract, architecture, or implementation changes:
  - `docs/PRD.md`
  - `docs/SAD.md`
  - `docs/ARCHITECTURE.md`
  - `docs/DOMAIN.md`
  - `docs/DATA_MODEL.md`
  - `docs/ENGINEERING_STANDARD.md`
- For API work, also read `docs/API/README.md` and the resource-specific file under `docs/API/`.
- For UI, interaction, or navigation work, also read `docs/VISUAL_FRAMEWORK.md`.
- For AI execution-process constraints, also read `docs/VIBECODING.md`.
- For tracked execution, also read the relevant files under `docs/plans/` and `docs/changes/`. These guide sequencing and status, but they do not override the primary source documents above.
- When executing from a tracked plan, update task-level checklist state in `docs/plans/` as each item completes, and update the related `docs/changes/` record when the corresponding milestone or workstream enters `In Progress`, `Blocked`, or `Done`.

## Working Agreements

- Stay inside the approved task boundary. If a request conflicts with the source docs, stop and call out the conflict before changing files.
- Never overwrite or revert unrelated user changes.
- Prefer the smallest coherent change that keeps the source docs aligned.
- Do not assume `apps/`, `packages/`, `crates/`, `proto/`, `package.json`, `Cargo.toml`, `turbo.json`, or `lefthook.yml` exist unless they are present in the tracked tree you inspected.
- Code identifiers, schema fields, config keys, and code comments should be in English. Repository docs should remain in Chinese unless a file already uses another convention.

## Project Guardrails

- Intended frontend baseline: Vue 3, TypeScript, Vite, Vue Router, Pinia, VueUse, UnoCSS, Vue I18n, self-built UI components, shared design tokens, and Tauri 2 / Tauri Mobile.
- Intended backend baseline: Rust stable, Tokio, Axum, Tonic, SQLx, Serde, tracing / OpenTelemetry, modular-monolith boundaries, and adapter-based integrations.
- Default database support model: SQLite for local/personal use, PostgreSQL for team/production use. Keep core paths compatible with both.
- Do not introduce a second primary frontend stack, backend stack, package manager, or build system.
- Keep domain logic out of handlers, views, and transport layers.
- For Vue, use Composition API with `<script setup lang="ts">` by default.
- For Rust, preserve explicit domain / application / infrastructure / transport boundaries.
- External HTTP contracts belong in OpenAPI or the current `docs/API/` source docs until formal contract files exist. Internal RPC contracts belong in Protobuf / Buf once those sources exist. Plugin contracts belong in schema / manifest definitions.

## Required Sync

- If a change affects architecture, contracts, model management, data model, component API, repo entrypoints, or verification expectations, update the affected docs in the same change.
- Keep `README.md`, `.github/pull_request_template.md`, `.github/workflows/guardrails.yml`, and relevant `docs/changes/` or `docs/plans/` files aligned when the change touches their scope.
- If user-facing UI copy changes, keep `zh-CN` and `en-US` in scope.

## Verification

- Run the checks that actually exist in the tracked repository state. Do not claim success based on expected future tooling.
- In the current `doc-first` state, the default truthful verification set is:
  - required-doc existence checks
  - stale-reference searches for removed files or renamed docs
  - focused diff review for touched documents
  - targeted consistency grep for renamed fields or contracts when relevant
- Only use `pnpm`, `turbo`, `cargo`, `buf`, OpenAPI lint, or Playwright when the corresponding manifests, sources, and tools are actually present.
- If the verification stack is missing because the repo is still in documentation or scaffold transition, say so explicitly.
