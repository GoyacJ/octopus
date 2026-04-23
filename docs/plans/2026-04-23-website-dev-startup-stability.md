# Website Dev Startup Stability Plan

## Goal

Make `pnpm dev:website` start reliably without the intermittent `Failed to resolve import "#app-manifest"` pre-transform error.

## Architecture

This repair belongs in `apps/website` startup tooling, not in page code. The failure comes from stale Vite transform caches around Nuxt's internal `#app-manifest` resolution, so the fix must reset generated caches before each dev boot and then regenerate `.nuxt` metadata in a controlled way.

## Scope

- In scope:
  - `apps/website/package.json` dev entrypoint behavior
  - Verification of `pnpm dev:website` from the repository root
- Out of scope:
  - Changing page/runtime behavior
  - Nuxt module upgrades or dependency churn

## Risks Or Open Questions

- `nuxt dev` currently starts successfully on this machine after a manual `nuxi prepare`, so the remaining issue is startup race tolerance rather than a persistent hard failure.
- If `predev` does not suppress the alias error after deleting `.nuxt`, the fix belongs deeper in Nuxt config and execution must stop there.

## Execution Rules

- Do not start implementation until each task has exact files, acceptance, verification, and stop conditions.
- Do not collapse shared-layer work into a page-local workaround.
- Stop when source of truth, ownership, or verification output is unclear.
- Execute in small batches and update status in place after each batch.

## Task Ledger

### Task 1: Stabilize Nuxt manifest startup path

Status: `done`

Files:
- Modify: `apps/website/package.json`
- Create: `apps/website/scripts/prepare-dev.mjs`

Preconditions:
- `apps/website/.nuxt/tsconfig.json` maps `#app-manifest` to generated manifest metadata under `.nuxt/manifest/meta/dev.json`.

Step 1:
- Action: Add a `predev` hook that runs a local script to clear app/root Vite caches plus generated `.nuxt/.output`, then execute `nuxt prepare`.
- Done when: invoking `pnpm -C apps/website dev` always starts from a clean generated state and materializes `.nuxt/manifest/meta/dev.json` before the long-running dev process relies on it.
- Verify: `rm -rf apps/website/.nuxt apps/website/.output && pnpm dev:website`
- Stop if: `nuxt prepare` still does not emit the manifest metadata or introduces a different startup failure.

### Task 2: Re-verify desktop startup regression is absent

Status: `done`

Files:
- Modify: `docs/plans/runtime/2026-04-23-session-jsonl-compat.md`

Preconditions:
- Existing session-store compatibility changes remain in the current worktree.

Step 1:
- Action: Re-run `pnpm dev:desktop` far enough to confirm the previous `missing field \`event_id\`` startup crash does not recur in the current workspace.
- Done when: desktop startup reaches the Tauri shell launch path without printing the old session serde error.
- Verify: `pnpm dev:desktop`
- Stop if: the old serde failure reappears and requires new session-store code changes.

## Checkpoint 2026-04-23 10:18

- Batch: Task 1 discovery
- Completed:
  - Confirmed `apps/website/.nuxt/tsconfig.json` maps `#app-manifest` to `./manifest/meta/dev.json`.
  - Confirmed `predev` with only `nuxt prepare` is not sufficient; the `#app-manifest` error still reproduces until Vite cache directories are cleared.
  - Confirmed removing `apps/website/node_modules/.cache/vite`, `apps/website/node_modules/.vite`, and workspace-root `node_modules/.vite` makes `pnpm dev:website` start cleanly again.
- Verification:
  - `pnpm -C apps/website exec nuxi prepare` -> pass
  - `rm -rf apps/website/.nuxt apps/website/.output && pnpm dev:website` -> fail (`#app-manifest` still unresolved)
  - `rm -rf apps/website/.nuxt apps/website/.output apps/website/node_modules/.cache/vite apps/website/node_modules/.vite node_modules/.vite && pnpm dev:website` -> pass
- Blockers:
  - none
- Next:
  - Task 1 Step 1 implementation

## Checkpoint 2026-04-23 10:46

- Batch: Task 1 Step 1 -> Task 2 Step 1
- Completed:
  - Added `apps/website/scripts/prepare-dev.mjs` to clear app/root Vite caches plus generated `.nuxt/.output` before running `nuxt prepare`.
  - Switched `apps/website/package.json` `predev` to the new script so `pnpm dev:website` self-heals the stale `#app-manifest` cache state.
  - Re-ran desktop startup and confirmed the previous `session serde error: missing field \`event_id\`` regression still does not recur in the current workspace.
- Verification:
  - `pnpm -C apps/website exec node scripts/prepare-dev.mjs` -> pass
  - `pnpm dev:website` -> pass, no `#app-manifest` pre-transform errors after cache reset
  - `pnpm dev:desktop` -> pass far enough to reach `Running /Users/goya/Work/weilaizhihuigu/super-agent/octopus/target/debug/octopus-desktop-shell` without the old serde crash
- Blockers:
  - none
- Next:
  - none
