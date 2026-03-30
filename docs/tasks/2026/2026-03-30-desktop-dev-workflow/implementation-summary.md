## Implementation Summary

- Goal:
  - Implement repo-managed desktop HMR and remote-hub联调 workflows without changing stable startup semantics.
- Files Added:
  - `apps/remote-hub/src/dev_seed.rs`
  - `apps/remote-hub/tests/dev_seed.rs`
  - `scripts/dev/workflows.mjs`
  - `scripts/dev/supervisor.mjs`
  - `scripts/dev/remote-hub-dev.mjs`
  - `scripts/dev/desktop-remote-dev.mjs`
  - `test/scripts/dev-workflows.test.mjs`
  - `docs/tasks/2026/2026-03-30-desktop-dev-workflow/*`
- Files Changed:
  - `package.json`
  - `README.md`
  - `pnpm-lock.yaml`
  - `apps/desktop/package.json`
  - `apps/desktop/tsconfig.json`
  - `apps/desktop/src-tauri/tauri.conf.json`
  - `apps/desktop/test/launch-config.test.ts`
  - `apps/remote-hub/src/lib.rs`
  - `apps/remote-hub/src/main.rs`
  - `docs/tasks/README.md`
- Files Removed:
  - None.
- Structure Decision:
  - Keep desktop/Tauri wiring in `apps/desktop`, keep remote-hub dev seed in `apps/remote-hub`, and keep orchestration plus command specs in repo-local `scripts/dev`.
- Why This Structure:
  - It keeps app behavior close to the owning surface while avoiding shared-contract or shared-runtime scope expansion.
  - The `scripts/dev/workflows.mjs` plus `scripts/dev/supervisor.mjs` split makes the user-facing scripts testable without GUI automation.
- Reused Patterns:
  - Existing launch-config regression tests and remote-hub runtime seeding helpers.
- New Dependencies:
  - Desktop-only `@tauri-apps/cli` for repo-managed Tauri v2 dev workflow.
  - Desktop-only `@types/node` so the config regression test can read the root manifest under `vue-tsc`.
- Error Handling Strategy:
  - Dev scripts prefix child logs, forward `SIGINT` / `SIGTERM`, kill sibling processes when one exits, and return non-zero when readiness or a child process fails.
  - Remote-hub dev seed remains env-gated and no-ops when the isolated dev DB already has project state for `workspace-alpha`.
- Deferred Items:
  - Native-window HMR behavior is not covered by automated GUI verification in this task package.
- Non-goals Preserved:
  - Stable startup commands, manual-login-only remote workflow, and unchanged shared contracts.

## Notes

- The approved plan proposed `beforeDevCommand` / `beforeBuildCommand` using `pnpm --dir .. ...`. In this repository, verified Tauri runtime behavior executes those commands from `apps/desktop`, so the working and verified configuration is `pnpm run ui:dev` / `pnpm run ui:build`.
