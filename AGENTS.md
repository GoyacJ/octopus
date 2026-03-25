# AGENTS.md

Repository-wide instructions for Codex and compatible coding agents.

## Repository Expectations

- This repository remains `doc-first` in governance. The current tracked truth is the repository root docs, `docs/`, `contracts/`, and `.github/`; do not assume `apps/`, `packages/`, `crates/`, or workspace manifests exist unless they are actually present in the tracked tree.
- Treat tracked files in the repository root, `docs/`, `contracts/`, and `.github/` as the source of truth for current implementation work.
- Read these files before product, architecture, governance, or implementation changes:
  - `README.md`
  - `docs/PRD.md`
  - `docs/SAD.md`
  - `docs/CONTRACTS.md`
  - `contracts/README.md`
  - `docs/DEVELOPMENT_PLAN.md`
  - `docs/DEVELOPMENT_CHANGELOG.md`
  - `docs/ENGINEERING_STANDARD.md`
  - `docs/VIBECODING.md`
  - `docs/adr/README.md`
- Keep this root file short and stable. If future subdirectories need different rules, add a nearby `AGENTS.override.md` or nested `AGENTS.md` instead of expanding the root file indefinitely.

## Working Agreements

- Stay inside the approved task boundary. If a request conflicts with the source docs, stop and call out the conflict before editing files.
- Do not restore deleted legacy doc trees or cite missing paths as if they still govern the repository.
- Do not restore deleted implementation skeletons or manifests unless the approved task explicitly requires rebuilding them.
- Never describe target-state architecture, repo layout, or runtime capabilities as implemented reality unless the tracked tree proves they exist.
- Code identifiers, schema fields, config keys, and code comments should be in English. Repository-level docs should remain in Chinese unless a file already uses another convention.
- For frontend UI polishing or page redesign requests, follow the AI frontend design prompt template in `docs/ENGINEERING_STANDARD.md`; treat Apple and Google as aesthetic references only, not direct copy targets.
- Never overwrite or revert unrelated user changes.

## Required Sync

- If a change affects repository entrypoints, architecture boundaries, engineering rules, or verification expectations, update the affected docs in the same change.
- Keep the formal doc list aligned across `README.md`, `AGENTS.md`, `docs/DEVELOPMENT_PLAN.md`, `docs/DEVELOPMENT_CHANGELOG.md`, `docs/ENGINEERING_STANDARD.md`, `docs/VIBECODING.md`, `.github/workflows/guardrails.yml`, and `.github/pull_request_template.md`, including `contracts/README.md` when contract sources are part of the change.
- Record architecture exceptions in `docs/adr/` rather than only in chat or commit messages.

## Verification

- Run only the checks supported by the current tracked repository state.
- The truthful minimum verification set is:
  - required-doc and contract-source existence checks
  - stale-reference searches for removed files, removed doc trees, and old project names
  - focused diff review for touched documents
  - `git diff --check`
- If the change touches `contracts/`, `packages/`, `apps/`, or `crates/`, also run the corresponding tracked workspace checks such as `pnpm run typecheck:web`, `pnpm run test:web`, `cargo test --workspace`, and `cargo build --workspace` when the required manifests and sources actually exist.
- Do not claim `pnpm`, `cargo`, `turbo`, `buf`, API lint, or UI test success unless the corresponding manifests, sources, and commands actually exist in the tracked tree.
