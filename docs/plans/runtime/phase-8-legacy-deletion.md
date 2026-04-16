# Phase 8 Legacy Deletion

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Delete the old execution trunk, duplicate dispatch infrastructure, legacy persistence fallbacks, and remaining compatibility shells so only one runtime trunk remains in code, tests, and docs.

**Architecture:** Phase 7 is assumed complete before this phase starts. At that point the public contract is already cut over and host parity is already enforced. Phase 8 is deletion-driven. It removes the remaining adapter-owned one-shot executor path, compatibility discovery or dispatch entrypoints, legacy configured-model fallbacks, legacy session or event recovery fallbacks, and stale documentation or tests that still describe the pre-rebuild runtime. The end state is simple: one compiled-manifest, policy-frozen, capability-planned, memory-aware, workflow-capable runtime trunk, with no second path waiting behind a helper or fallback.

**Tech Stack:** Runtime core in `crates/runtime`, capability and compatibility surfaces in `crates/tools`, runtime adapter and transport code in `crates/octopus-runtime-adapter`, remaining extraction helpers in `crates/compat-harness`, shared contract and adapter surfaces in `packages/schema` and `apps/desktop`, and runtime persistence governed by SQLite, JSONL, and disk-backed artifacts.

---

## Fixed Decisions

- Phase 8 is a deletion phase, not a compatibility phase. If a legacy path is still reachable, the phase is not complete.
- The primary runtime path is `manifest compile -> session policy freeze -> capability plan -> model loop -> executor -> memory or workflow or mediation projection`. No second execution root may remain behind a helper, registry, or fallback.
- Debug export JSON files, legacy runtime event files, or old session JSON shapes are not allowed to remain as hidden recovery dependencies.
- Compatibility helpers may survive only if they are strictly import, translation, or offline reference utilities. They may not participate in runtime discovery, planning, dispatch, or recovery.
- `ToolRegistry` is not allowed to remain a runtime discovery or execution dependency.
- The runtime-native `submit_turn(...)` entrypoint and capability-search result types such as `SkillDiscoveryOutput` are allowed to remain. Phase 8 removes legacy compat wrappers and duplicate execution roots, not the surviving runtime trunk surface.
- Legacy configured-model fallback generation is not allowed to remain the runtime source of truth once modern runtime config and registry paths are cut over.
- This phase includes stale tests, comments, and docs. Deleting code while leaving old architectural claims in place is not considered finished.

## Scope

This phase covers:

- deletion of adapter-owned one-shot execution roots and duplicate turn paths
- deletion or fencing of compatibility skill, tool, plugin, and registry dispatch leftovers
- deletion of legacy persistence, config, and session recovery fallbacks
- cleanup of stale runtime docs, tests, and comments that still describe the old trunk
- grep-level and test-level gates proving the old path is gone

This phase does not cover:

- new runtime feature work
- reintroducing fallback behavior for hosts that failed to cut over during Phase 7
- preserving permanent compatibility for obsolete runtime debug artifacts

## Current Baseline

The repository already deleted some early legacy shims, but the old trunk still exists in multiple places.

- `crates/octopus-runtime-adapter/src/executor.rs` still implements a turn-oriented `ExecutionResponse` model executor over provider `protocol_family`, which is evidence of an older adapter-owned execution root.
- `crates/octopus-runtime-adapter/src/execution_service.rs` still delegates submit-turn through `turn_submit::submit_turn(...)`, and `crates/octopus-runtime-adapter/src/lib.rs` still includes `mod turn_submit;`.
- `crates/octopus-runtime-adapter/src/persistence.rs` still carries fallback snapshot refs and still reads a legacy runtime debug events path if it exists.
- `crates/octopus-runtime-adapter/src/registry.rs` and `crates/octopus-runtime-adapter/src/registry_resolution.rs` still build `legacy_configured_models`.
- `crates/compat-harness/src/lib.rs` still depends on `ToolRegistry`, which shows compatibility extraction code is still coupled to runtime-adjacent registry types.
- `crates/tools/src/lib.rs` still re-exports `workspace_runtime` and compatibility-facing entrypoints, and `crates/tools/src/builtin_exec.rs` still exposes `SkillDiscovery` and `SkillTool` entrypoints.
- `crates/tools/src/capability_runtime/provider.rs` still contains compat handling for `SkillDiscovery` and `SkillTool`.
- `crates/runtime/src/lib.rs` still describes the crate as driving interactive and one-shot turns, and `crates/runtime/src/session/session_tests.rs` still carries legacy session JSON loading tests.
- Some deletion work has already started. For example, `crates/tools/src/split_module_tests.rs` already asserts that some legacy skill and MCP wrappers are removed from builtin dispatch. Phase 8 finishes that job instead of leaving partial cleanup behind.

Phase 8 starts from this real state: the new trunk exists, but the old trunk has not been fully deleted.

## Task 1: Delete The Adapter-Owned One-Shot Executor Path And Duplicate Turn Entry Root

**Files:**
- Modify: `crates/octopus-runtime-adapter/src/agent_runtime_core.rs`
- Modify: `crates/octopus-runtime-adapter/src/execution_service.rs`
- Modify: `crates/octopus-runtime-adapter/src/lib.rs`
- Modify or Replace: `crates/octopus-runtime-adapter/src/executor.rs`
- Delete or Inline: `crates/octopus-runtime-adapter/src/turn_submit.rs`
- Modify: `crates/runtime/src/conversation.rs`

**Implement:**
- remove the extra adapter-owned turn root so submit-turn and approval-resume behavior exist only through the native runtime core
- collapse or rename `executor.rs` so any remaining provider HTTP primitive is clearly a model driver internal to the new trunk rather than a legacy turn executor
- remove `turn_submit` as a separately meaningful execution path once the runtime core owns submit-turn end to end

**Deletion gate checks:**
```bash
rg -n "turn_submit|ExecutionResponse|RuntimeModelExecutor|execute_turn\\(" crates/octopus-runtime-adapter/src
```

**Verification:**
```bash
cargo test -p octopus-runtime-adapter
cargo test -p runtime
```

**Done when:**
- submit-turn and approval-resume reach the runtime only through the new trunk
- the adapter no longer exposes a meaningful one-shot execution root in naming or behavior

## Task 2: Delete Remaining Compat Discovery, Dispatch, And Registry Execution Roles

**Files:**
- Modify: `crates/tools/src/builtin_exec.rs`
- Modify: `crates/tools/src/capability_runtime/provider.rs`
- Modify: `crates/tools/src/skill_runtime.rs`
- Modify: `crates/tools/src/tool_registry.rs`
- Modify: `crates/tools/src/lib.rs`
- Modify: `crates/tools/src/split_module_tests.rs`
- Modify: `crates/compat-harness/src/lib.rs`

**Implement:**
- remove `SkillDiscovery` and `SkillTool` as runtime dispatch entrypoints on the main path
- remove direct runtime dependence on `ToolRegistry` for discovery or execution
- fence `compat-harness` so it is no longer coupled to runtime discovery or execution roles
- remove remaining runtime-facing exports of `workspace_runtime` helpers once they are no longer part of the execution truth
- keep only offline translation or reference uses that do not participate in runtime planning or dispatch

**Deletion gate checks:**
```bash
rg -n "\"SkillDiscovery\"|\"SkillTool\"|SkillDiscoveryInput|SkillToolInput|run_skill_discovery|run_skill_tool" crates/tools/src crates/octopus-runtime-adapter/src
rg -n "ToolRegistry" crates/tools/src crates/compat-harness/src crates/octopus-runtime-adapter/src
rg -n "workspace_runtime::|run_agent|run_worker_|run_task_|run_team_|run_cron_" crates/tools/src crates/octopus-runtime-adapter/src crates/runtime/src
```

**Verification:**
```bash
cargo test -p tools
cargo test -p compat-harness
```

**Done when:**
- the primary runtime path no longer depends on compat skill or tool entrypoints
- registry and harness code can no longer act as a second execution trunk

## Task 3: Delete Legacy Persistence, Config, And Recovery Fallbacks

**Files:**
- Modify: `crates/octopus-runtime-adapter/src/persistence.rs`
- Modify: `crates/octopus-runtime-adapter/src/runtime_config.rs`
- Modify: `crates/octopus-runtime-adapter/src/registry.rs`
- Modify: `crates/octopus-runtime-adapter/src/registry_resolution.rs`
- Modify: `crates/runtime/src/session/mod.rs`
- Modify: `crates/runtime/src/session/session_tests.rs`

**Implement:**
- remove fallback snapshot-ref generation that exists only because older records lacked proper runtime refs
- remove legacy runtime debug events file fallback reads when SQLite plus JSONL plus checkpoint artifacts are already the recovery truth
- remove `legacy_configured_models` generation once configured models come only from governed runtime config
- retire legacy session JSON compatibility behavior when it is no longer part of the supported recovery story

**Deletion gate checks:**
```bash
rg -n "legacy_configured_models|build_legacy_configured_models" crates/octopus-runtime-adapter/src
rg -n "legacy_path|runtime_debug_events_path|fallback_" crates/octopus-runtime-adapter/src/persistence.rs
rg -n "legacy session|loads_legacy_session_json|rejects_legacy_session_json" crates/runtime/src
```

**Verification:**
```bash
cargo test -p octopus-runtime-adapter
cargo test -p runtime session
```

**Done when:**
- runtime recovery depends only on the governed persistence layers
- config and session loading no longer silently revive legacy runtime shapes

## Task 4: Delete Stale Host, Test, And Documentation Compatibility Shells

**Files:**
- Modify: `packages/schema/src/*`
- Modify: `apps/desktop/src/tauri/runtime_api.ts`
- Modify: `apps/desktop/test/support/workspace-fixture-runtime.ts`
- Modify: `apps/desktop/test/openapi-transport.test.ts`
- Modify: `apps/desktop/test/runtime-store.test.ts`
- Modify: `apps/desktop/test/tauri-client-runtime.test.ts`
- Modify: `docs/plans/runtime/*.md`
- Modify: `crates/runtime/src/lib.rs`

**Implement:**
- remove stale comments, type aliases, fixture fields, and tests that preserve deleted runtime behavior
- update runtime crate docs so they describe only the surviving trunk
- delete compatibility assertions that exist only to protect obsolete paths
- keep tests focused on the new runtime path only

**Deletion gate checks:**
```bash
rg -n "one-shot|compat|legacy" packages/schema/src apps/desktop/src apps/desktop/test crates/runtime/src docs/plans/runtime
```

**Verification:**
```bash
pnpm -C apps/desktop exec vitest run test/openapi-transport.test.ts test/runtime-store.test.ts test/tauri-client-runtime.test.ts
```

**Done when:**
- public types, fixtures, tests, and docs no longer describe deleted runtime paths
- developers cannot misunderstand the surviving architecture from stale comments or test names

## Task 5: Final Program Exit Gate And Deletion Proof

**Files:**
- Verify only

**Run:**
```bash
pnpm openapi:bundle
pnpm schema:generate
pnpm schema:check
cargo test -p runtime
cargo test -p tools
cargo test -p compat-harness
cargo test -p octopus-infra
cargo test -p octopus-runtime-adapter
cargo test -p octopus-platform
cargo test -p octopus-server
pnpm -C apps/desktop exec vitest run test/tauri-client-host.test.ts test/tauri-client-runtime.test.ts test/openapi-transport.test.ts test/runtime-store.test.ts
rg -n "turn_submit|ExecutionResponse|RuntimeModelExecutor|execute_turn\\(|\"SkillDiscovery\"|\"SkillTool\"|SkillDiscoveryInput|SkillToolInput|run_skill_discovery|run_skill_tool|ToolRegistry|legacy_configured_models|build_legacy_configured_models|runtime_debug_events_path|workspace_runtime::" \
  crates/runtime/src crates/tools/src crates/compat-harness/src crates/octopus-runtime-adapter/src
git diff --stat -- \
  crates/runtime/src \
  crates/tools/src \
  crates/compat-harness/src \
  crates/octopus-runtime-adapter/src \
  crates/octopus-platform/src \
  crates/octopus-server/src \
  packages/schema/src \
  apps/desktop/src \
  apps/desktop/test \
  docs/plans/runtime
```

**Acceptance criteria:**
- only one execution trunk remains reachable in code
- all targeted runtime tests pass on the new path only
- no runtime recovery path depends on legacy session or debug JSON fallbacks
- no compat discovery, registry, or harness surface can act as a hidden execution path
- docs and tests describe only the surviving rebuilt platform

## Completion Fence

Phase 8 is complete only when the repository can honestly say all of the following:

- legacy executor, duplicate dispatch, and prompt-centric orchestration helpers are deleted
- runtime, memory, workflow, approval, and host projection all run on one trunk
- OpenAPI, generated types, server transport, adapters, and desktop consumers reflect only the surviving architecture
- recovery is governed by SQLite projections, JSONL events, and disk-backed artifacts without legacy fallback dependence

At that point the runtime rebuild program is structurally complete. No later phase is allowed to reintroduce a second execution path under a compatibility label.
