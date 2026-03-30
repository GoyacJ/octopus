# Delivery Note

## What Changed

- Added a tracked post-Slice-20 task package for the desktop launch-path fix.
- Added `pnpm desktop:open` at the repo root and `pnpm open` inside `apps/desktop`.
- Updated the desktop Tauri config so the shell loads `apps/desktop/dist` instead of the raw source root.
- Added a regression test covering the Tauri asset root and desktop open script.
- Documented the current desktop and remote-hub launch commands in `README.md`.

## Why

- The repository had a real Tauri host and a buildable frontend, but no intentional startup command and a mismatched Tauri frontend asset root. That left the current project feeling “not launchable” even though the runtime pieces existed.

## User / System Impact

- Operators can now open the tracked desktop surface from the monorepo root with `pnpm desktop:open`.
- Operators can start the tracked remote hub from the monorepo root with `pnpm remote-hub:start`.
- The desktop shell now loads built frontend assets through the tracked Tauri host path.

## Risks

- `pnpm desktop:open` is a build-and-open workflow, not a hot-reload dev workflow.
- Future desktop tooling work must keep `tauri.conf.json` aligned with whichever asset root is actually generated and verified.

## Rollback Notes

- Revert the root/package script entries, the Tauri `frontendDist` change, the regression test, and the README launch section together.
- Do not keep the scripts while restoring `frontendDist` back to `..`; that would reintroduce the blank-window risk.

## Follow-ups

- If the project later needs hot reload or `tauri dev`, open a new task package and add the required tooling plus verification explicitly.

## Docs Updated

- Updated `README.md`.
- Updated `docs/tasks/README.md`.
- Added this task package.

## Tests Included

- `pnpm --filter @octopus/desktop exec vitest run test/launch-config.test.ts`
- `pnpm --filter @octopus/desktop test`
- `pnpm --filter @octopus/desktop typecheck`
- `pnpm --filter @octopus/desktop build`
- `pnpm desktop:open`
- `pnpm remote-hub:start`

## ADR Updated

- No new ADR was required.

## Temporary Workarounds

- None. The fix establishes the tracked local launch path rather than a temporary manual workaround.
