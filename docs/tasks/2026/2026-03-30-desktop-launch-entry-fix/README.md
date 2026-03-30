# Desktop Launch Entry Fix

This task package records the post-Slice-20 desktop launch-path fix so the tracked desktop surface has a truthful, reproducible local open command.

## Package Files

- [design-note.md](design-note.md)
- [contract-change.md](contract-change.md)
- [implementation-summary.md](implementation-summary.md)
- [verification.md](verification.md)
- [delivery-note.md](delivery-note.md)

## Task Definition

- Goal:
  - Restore a truthful local desktop launch path by pointing the Tauri shell at built frontend assets and adding a documented monorepo entry command.
- Scope:
  - Create and maintain this task package.
  - Add a minimal regression test for the desktop launch configuration.
  - Update the desktop Tauri build config so the shell loads `apps/desktop/dist`.
  - Add a root-level desktop open command and a package-local desktop open command.
  - Document the current local desktop and remote-hub startup commands in repository entry docs.
- Out of Scope:
  - Any `schemas/`, shared DTO, runtime, governance, or remote-hub API change.
  - Hot-reload dev-server integration, Tauri CLI adoption, or packaging/distribution work.
  - UI redesign, auth-flow changes, or desktop runtime behavior changes beyond startup wiring.
- Acceptance Criteria:
  - A targeted regression test proves the desktop launch config points at built assets and exposes an intentional open script.
  - `pnpm desktop:open` builds the desktop frontend and hands off to the tracked Tauri host.
  - Repository docs state the current desktop and remote-hub launch commands without inventing untracked workflows.
  - The change remains app-local and contract-compatible.
- Non-functional Constraints:
  - No new dependencies.
  - Keep the fix limited to launch wiring and documentation.
  - Preserve current desktop runtime/bootstrap semantics after the frontend loads.
- MVP Boundary:
  - One built-asset Tauri entry path.
  - One root launch command.
  - One package-local launch command.
  - One short documentation update.
- Human Approval Points:
  - None.
- Source Of Truth Updates:
  - Update this task package.
  - Update `docs/tasks/README.md`.
  - Update `README.md` with truthful launch commands.
- Affected Modules:
  - `docs/tasks`
  - `apps/desktop`
  - root workspace manifests
- Affected Layers:
  - Repository entry docs
  - Desktop surface assembly
  - Tauri shell configuration
- Risks:
  - Pointing Tauri at the wrong asset root and preserving the blank-window launch failure.
  - Adding a script that bypasses the required frontend build step.
  - Accidentally describing a richer dev workflow than the repository currently tracks.
- Validation:
  - `pnpm --filter @octopus/desktop exec vitest run test/launch-config.test.ts`
  - `pnpm --filter @octopus/desktop build`
  - `pnpm desktop:open`
