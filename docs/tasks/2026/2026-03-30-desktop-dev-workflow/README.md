# Desktop Dev Workflow

This task package records the post-Slice-20 desktop developer-workflow expansion for repo-managed Tauri CLI usage, Vite HMR, and an explicit remote-hub联调 path.

## Package Files

- [design-note.md](design-note.md)
- [contract-change.md](contract-change.md)
- [implementation-summary.md](implementation-summary.md)
- [verification.md](verification.md)
- [delivery-note.md](delivery-note.md)

## Task Definition

- Goal:
  - Add a truthful desktop developer workflow with repo-managed Tauri CLI, local Vite HMR, and a dev-only remote-hub联调 path while preserving the existing stable startup commands.
- Scope:
  - Create and maintain this task package.
  - Add desktop package scripts for Vite HMR and repo-managed Tauri CLI usage.
  - Extend Tauri config and desktop regression coverage for dev/build wiring.
  - Add root-level `desktop:dev:local`, `remote-hub:dev`, and `desktop:dev:remote` entry points.
  - Add repo-local Node orchestration scripts under `scripts/dev/`.
  - Add a remote-hub dev-only seed path guarded by `OCTOPUS_REMOTE_HUB_DEV_SEED=1`.
  - Document the new dev workflows, seeded defaults, and manual remote login steps.
- Out of Scope:
  - Any `schemas/`, shared DTO, remote auth contract, or REST surface change.
  - Auto-login, profile mutation, refresh-token work, or hidden session injection.
  - Packaging / distribution changes, production deployment changes, or desktop UI redesign.
- Acceptance Criteria:
  - `pnpm desktop:dev:local` runs the desktop app through repo-managed Tauri CLI against a Vite dev server on `127.0.0.1:5173`.
  - `pnpm desktop:dev:remote` starts remote-hub dev mode and local desktop dev mode together, forwards signals, and prints manual login guidance.
  - `pnpm desktop:open` and `pnpm remote-hub:start` remain unchanged stable paths.
  - A remote-hub dev-only seed makes an empty isolated dev database loginable and project-listable for `workspace-alpha`.
  - Automated tests cover desktop config wiring, dev script orchestration specs, and remote-hub dev seed behavior.
- Non-functional Constraints:
  - Keep stable launch/start semantics unchanged.
  - Keep new seed behavior limited to dev-only env-gated startup in `apps/remote-hub`.
  - Prefer app-local and repo-local assembly changes over shared runtime or schema changes.
  - Keep orchestration cross-platform and repo-owned, without global CLI requirements or new process-runner dependencies.
- MVP Boundary:
  - One local HMR desktop path.
  - One remote-hub联调 path.
  - One dev-only remote seed path for deterministic manual login.
  - One documentation update covering stable versus dev workflows.
- Human Approval Points:
  - None. The implementation follows the approved plan.
- Source Of Truth Updates:
  - Update this task package.
  - Update `docs/tasks/README.md`.
  - Update `README.md` with truthful stable and dev workflow guidance.
- Affected Modules:
  - `apps/desktop`
  - `apps/remote-hub`
  - root workspace manifests
  - `scripts/dev`
  - `test/scripts`
  - `docs/tasks`
- Affected Layers:
  - Repository entry docs
  - Desktop surface assembly
  - Remote-hub app assembly
  - Tauri shell configuration
  - Dev-tooling orchestration
- Risks:
  - Regressing the current stable open/start commands while adding dev-only flows.
  - Starting desktop dev mode without deterministic Vite or remote-hub readiness behavior.
  - Accidentally widening remote-hub seed behavior beyond the dev-only path.
  - Adding dev tooling that depends on global binaries or shell-specific glue.
- Validation:
  - `pnpm --filter @octopus/desktop exec vitest run test/launch-config.test.ts`
  - `node --test test/scripts/dev-workflows.test.mjs`
  - `cargo test -p octopus-remote-hub dev_seed`
  - `pnpm --filter @octopus/desktop test`
  - `pnpm --filter @octopus/desktop typecheck`
