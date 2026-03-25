# AGENTS.md

Repository-wide instructions for Codex and compatible coding agents.

## Repository Expectations

- This repository remains `doc-first` in governance, but the tracked tree now includes a Phase 1 workspace skeleton (`contracts/`, `apps/`, `packages/`, `crates/`, `package.json`, `pnpm-workspace.yaml`, `Cargo.toml`).
- Treat tracked files in the repository root, `docs/`, `contracts/`, and `.github/` as the source of truth for current implementation work.
- Read these files before product, architecture, governance, or implementation changes:
  - `README.md`
  - `docs/PRD.md`
  - `docs/SAD.md`
  - `docs/CONTRACTS.md`
  - `docs/ENGINEERING_STANDARD.md`
  - `docs/VIBECODING.md`
  - `docs/adr/README.md`
- Keep this root file short and stable. If future subdirectories need different rules, add a nearby `AGENTS.override.md` or nested `AGENTS.md` instead of expanding the root file indefinitely.

## Working Agreements

- Stay inside the approved task boundary. If a request conflicts with the source docs, stop and call out the conflict before editing files.
- Do not restore deleted legacy doc trees or cite missing paths as if they still govern the repository.
- Never describe target-state architecture, repo layout, or runtime capabilities as implemented reality unless the tracked tree proves they exist.
- Code identifiers, schema fields, config keys, and code comments should be in English. Repository-level docs should remain in Chinese unless a file already uses another convention.
- Never overwrite or revert unrelated user changes.

## Required Sync

- If a change affects repository entrypoints, architecture boundaries, engineering rules, or verification expectations, update the affected docs in the same change.
- Keep the formal doc list aligned across `README.md`, `AGENTS.md`, `docs/ENGINEERING_STANDARD.md`, `docs/VIBECODING.md`, `.github/workflows/guardrails.yml`, and `.github/pull_request_template.md`.
- Record architecture exceptions in `docs/adr/` rather than only in chat or commit messages.

## Verification

- Run only the checks supported by the current tracked repository state.
- The truthful minimum verification set is:
  - required-doc existence checks
  - stale-reference searches for removed files, removed doc trees, and old project names
  - focused diff review for touched documents
  - `git diff --check`
- If the change touches `contracts/`, `packages/`, `apps/`, or `crates/`, also run the corresponding tracked workspace checks such as `pnpm run typecheck:web`, `pnpm run test:web`, `cargo test --workspace`, and `cargo build --workspace`.
- Do not claim `pnpm`, `cargo`, `turbo`, `buf`, API lint, or UI test success unless the corresponding manifests, sources, and commands actually exist in the tracked tree.
