# Desktop Build Unblock Plan

## Goal

Unblock `pnpm build:desktop` by fixing the current TypeScript compile failures in the desktop UI build step without changing unrelated product behavior.

## Task 1: Repair runtime schema export surface

Status: `done`

Files:
- Modify: `packages/schema/src/runtime.ts`

Preconditions:
- `packages/schema/src/generated.ts` already contains `RebindRuntimeSessionConfiguredModelInput`.

Step 1:
- Action: Re-export the generated runtime session rebind input from the runtime schema surface so `@octopus/schema` resolves the type used by the desktop workspace client.
- Done when: `apps/desktop/src/tauri/workspace-client.ts` can import `RebindRuntimeSessionConfiguredModelInput` from `@octopus/schema` without TypeScript export errors.
- Verify: `pnpm -C apps/desktop exec vue-tsc --noEmit --pretty false`
- Stop if: the generated type name or ownership boundary differs from the desktop transport contract.

## Task 2: Repair ProjectSettings dialog state typings

Status: `done`

Files:
- Modify: `apps/desktop/src/views/project/useProjectSettings.ts`

Preconditions:
- Task 1 is complete or independent.

Step 1:
- Action: Split modal-open keys from save/error state keys so `grantModels`, `runtimeModels`, `grantTools`, `runtimeTools`, `grantActors`, and `runtimeActors` are valid typed keys where they are used.
- Done when: `useProjectSettings.ts` and `ProjectSettingsView.vue` no longer fail because save/error state objects are typed too narrowly.
- Verify: `pnpm -C apps/desktop exec vue-tsc --noEmit --pretty false`
- Stop if: the settings flow expects a broader refactor of dialog ownership instead of local key-type correction.

## Task 3: Re-run desktop package build

Status: `blocked`

Files:
- Modify: `docs/plans/2026-04-23-desktop-build-unblock.md`

Preconditions:
- Task 1 and Task 2 are done.

Step 1:
- Action: Re-run `pnpm build:desktop` and capture the final artifact path or the next hard blocker.
- Done when: the desktop package build succeeds or a new non-TypeScript blocker is identified with concrete output.
- Verify: `pnpm build:desktop`
- Stop if: the build fails in a different subsystem that requires broader product changes.

## Execution Checkpoint

- 2026-04-23:
  - Task 1 done. Re-exported `RebindRuntimeSessionConfiguredModelInput` from `packages/schema/src/runtime.ts`, so the desktop workspace client can import it from `@octopus/schema`.
  - Task 2 done. Split `useProjectSettings.ts` modal-open keys from save/error keys, which repaired the `grant*` / `runtime*` TypeScript key mismatch without changing runtime logic.
  - Verification passed: `pnpm -C apps/desktop exec vue-tsc --noEmit --pretty false`
  - Task 3 blocked on signing environment, not build compilation. `pnpm build:desktop` produced:
    - `/Users/goya/Work/weilaizhihuigu/super-agent/octopus/target/release/bundle/macos/Octopus.app`
    - `/Users/goya/Work/weilaizhihuigu/super-agent/octopus/target/release/bundle/dmg/Octopus_0.2.5_aarch64.dmg`
    - `/Users/goya/Work/weilaizhihuigu/super-agent/octopus/target/release/bundle/macos/Octopus.app.tar.gz`
  - Final blocker: `A public key has been found, but no private key. Make sure to set TAURI_SIGNING_PRIVATE_KEY environment variable.`
