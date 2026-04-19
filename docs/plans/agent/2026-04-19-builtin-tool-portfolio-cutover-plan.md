# Octopus Built-in Tool Portfolio Clean-Cutover Implementation Plan

> **For Codex:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task.

**Goal:** 完整落实 `2026-04-19-octopus-tool-portfolio-benchmark.md` 的 built-in tool clean-cutover，把 Octopus 从当前 legacy built-in 集合切到 benchmark 定义的目标工具家族、统一命名、单点 catalog、派生 catalog view、runtime projection 与 transport contract。

**Architecture:** 以 `BuiltinCapabilityCatalog` 作为 built-in 唯一真相，以 `ToolCatalogView` 作为搜索、UI、调试和 profile/section 展示的唯一派生目录，以 `CapabilityExposureState` 作为 discovery/activation/exposure 的唯一运行时状态。所有新 built-in 家族都必须同时接入 catalog metadata、execution handler、planner/provider surface、runtime adapter projection、OpenAPI/schema contract 和测试；不允许继续在 `tool_registry.rs`、`builtin_exec.rs` 或 adapter summary 中保留第二套真相。

**Tech Stack:** Rust (`crates/tools`, `crates/octopus-runtime-adapter`), OpenAPI (`contracts/openapi/src/**`), generated transport schema (`packages/schema/src/**`), SQLite/runtime persistence, existing runtime config and session services.

---

## Target Built-in Portfolio

- Worker primitives:
  - `shell_exec`
  - `process_manage`
  - `read_file`
  - `write_file`
  - `edit_file`
  - `apply_patch`
  - `glob_search`
  - `grep_search`
  - `lsp_query`
  - `notebook_edit`
  - `web_search`
  - `web_fetch`
- Control-plane:
  - `tool_search`
  - `update_plan`
  - `ask_user`
  - `structured_output`
  - `sleep`
- Runtime config:
  - `runtime_config_get`
  - `runtime_config_validate_patch`
  - `runtime_config_save_patch`
- Orchestration and session:
  - `spawn_agent`
  - `send_agent_input`
  - `wait_agent`
  - `close_agent`
  - `session_status`
  - `session_history`
  - `session_search`
  - `session_yield`
- Automation:
  - `cron_create`
  - `cron_list`
  - `cron_delete`
  - `remote_trigger`
  - `monitor`
- Browser and artifact:
  - `browser_navigate`
  - `browser_snapshot`
  - `browser_click`
  - `browser_type`
  - `browser_scroll`
  - `browser_back`
  - `browser_press`
  - `browser_console`
  - `image_read`
  - `pdf_read`
- Optional guarded family:
  - `execute_code`
- Explicitly removed from model-visible built-in core:
  - `REPL`
  - `PowerShell`
  - `TestingPermission`
  - `TodoWrite`
  - `Config`
  - `AskUserQuestion`
  - `SendUserMessage`
  - `Brief`
  - `EnterPlanMode`
  - `ExitPlanMode`
  - all mixed-case canonical names such as `WebFetch`, `WebSearch`, `ToolSearch`, `NotebookEdit`, `LSP`

## Scope

- In scope:
  - built-in canonical naming cutover to `snake_case`
  - `BuiltinCapabilityCatalog` completion and `ToolCatalogView` derivation
  - P0, P1, P2 built-in families from the benchmark
  - runtime adapter/session/orchestration/config integration required to make those tools real
  - runtime capability projections, persistence, approval and resume behavior affected by the new tools
  - OpenAPI and `@octopus/schema` transport changes required by runtime/catalog surfaces
  - regression tests and dead-code cleanup for retired tool names and retired built-ins
- Out of scope:
  - compatibility aliases for legacy names in final state
  - historical data migration or backfill logic
  - plugin/MCP/provider-owned vertical tools such as `gateway`, `nodes`, `canvas`, `x_search`, music/video/TTS
  - browser/media/product features that bypass the built-in catalog boundary

## Clean-Cutover Rules

- Start from the current repository state that already includes the first-phase `BuiltinCapabilityCatalog` / exposure groundwork. Do not branch execution from an older pre-catalog revision.
- Final state must not expose legacy mixed-case canonical names or model-visible aliases. Temporary compile-time aliases are allowed only inside one execution batch and must be removed before the task is marked done.
- `ToolCatalogView` must stay derived from canonical capability metadata. `tool_registry.rs` may hold search helpers, but it must not become a second source of truth.
- `TestingPermission` may remain test-only behind `#[cfg(test)]` or harness-only wiring, but it must not appear in production `BuiltinCapabilityCatalog` or `ToolCatalogView`.
- Any new persistent state for sessions, automation, browser artifacts, or code execution must follow existing workspace persistence governance: structured state in SQLite projections, append-only runtime events in JSONL, large artifacts on disk with metadata references.
- For any `/api/v1/*` change, execution order is fixed: update `contracts/openapi/src/**`, run `pnpm openapi:bundle`, run `pnpm schema:generate`, then update adapters/tests. Do not hand-edit `contracts/openapi/octopus.openapi.yaml` or `packages/schema/src/generated.ts`.

## Risks Or Open Questions

- Browser tools need one adapter-owned browser session service. If execution cannot identify or create that single owner cleanly, browser tasks stop until the ownership boundary is resolved.
- `execute_code` is only acceptable if it can ship with a stronger isolation contract than `shell_exec`. If not, keep it blocked instead of downgrading it into a shell alias.
- `session_search` and `session_history` must read from runtime source-of-truth projections. If current persistence cannot answer them without side-channel scanning, add the projection first and do not fake the tool.

## Task Ledger

### Task 1: Finish the phase-1 catalog cutover and delete legacy production built-ins

Status: `pending`

Files:
- Modify: `crates/tools/src/builtin_catalog.rs`
- Modify: `crates/tools/src/builtin_exec.rs`
- Modify: `crates/tools/src/lib.rs`
- Modify: `crates/tools/src/tool_registry.rs`
- Modify: `crates/tools/src/capability_runtime/provider.rs`
- Modify: `crates/tools/src/capability_runtime/planner.rs`
- Modify: `crates/tools/src/capability_runtime/state.rs`
- Modify: `crates/octopus-runtime-adapter/src/capability_planner_bridge.rs`
- Modify: `crates/octopus-runtime-adapter/src/capability_executor_bridge.rs`
- Test: `crates/tools/src/split_module_tests.rs`
- Test: `crates/octopus-runtime-adapter/src/capability_runtime_tests.rs`

Preconditions:
- Current `main` already contains the first implementation-plan groundwork for `BuiltinCapabilityCatalog` and `CapabilityExposureState`.
- No new compatibility layer is introduced to preserve legacy mixed-case names.

Step 1:
- Action: Freeze the target production built-in matrix in `builtin_catalog.rs`, rename every canonical built-in to benchmark-approved `snake_case`, and remove `TestingPermission` plus all non-target production entries from the formal catalog.
- Done when: `BuiltinCapabilityCatalog` contains only target built-ins and internal-only helpers are no longer model-visible capabilities.
- Verify: `cargo test -p tools split_module_tests`
- Stop if: a supposedly removable legacy tool is still required as a model-visible contract and there is no benchmark-approved replacement.

Step 2:
- Action: Rewire `builtin_exec.rs`, `tool_registry.rs`, provider/planner state handling, and runtime bridges so the renamed canonical names are the only names used in planner input, exposure state, execution dispatch, and runtime summaries.
- Done when: runtime summaries, exposure snapshots, search surfaces, and dispatch paths no longer emit legacy mixed-case tool names.
- Verify: `cargo test -p tools split_module_tests && cargo test -p octopus-runtime-adapter capability_runtime_tests`
- Stop if: any runtime path requires dual-writing old and new tool names to stay functional.

Step 3:
- Action: Demote `TestingPermission` to test-only wiring and delete all production references to retired built-ins such as `TodoWrite`, `Config`, `AskUserQuestion`, `SendUserMessage`, `Brief`, `EnterPlanMode`, `ExitPlanMode`, `REPL`, and `PowerShell`.
- Done when: repository runtime code no longer treats those names as production built-ins, and tests only refer to them under explicit test-only coverage where still needed.
- Verify: `cargo test -p tools split_module_tests`
- Stop if: removing a retired built-in reveals a missing replacement family that must land first.

Notes:
- This task closes the review findings around `snake_case` cutover and `TestingPermission` still appearing in production catalog output.

### Task 2: Build `ToolCatalogView` as the only derived directory for search, UI, and profiles

Status: `pending`

Files:
- Create: `crates/tools/src/tool_catalog_view.rs`
- Modify: `crates/tools/src/lib.rs`
- Modify: `crates/tools/src/tool_registry.rs`
- Modify: `crates/tools/src/capability_runtime/provider.rs`
- Modify: `crates/tools/src/capability_runtime/planner.rs`
- Modify: `crates/octopus-runtime-adapter/src/capability_planner_bridge.rs`
- Modify: `contracts/openapi/src/components/schemas/catalog.yaml`
- Modify: `contracts/openapi/src/paths/catalog.yaml`
- Modify: `packages/schema/src/catalog.ts`
- Modify: `packages/schema/src/index.ts`
- Generated: `contracts/openapi/octopus.openapi.yaml`
- Generated: `packages/schema/src/generated.ts`
- Test: `crates/tools/src/split_module_tests.rs`
- Test: `crates/octopus-runtime-adapter/src/runtime_contract_tests.rs`

Preconditions:
- Task 1 is complete and canonical tool names are frozen.
- The catalog view remains derived from capability metadata instead of carrying independent registration state.

Step 1:
- Action: Introduce `ToolCatalogView` with explicit sections and profiles derived from `BuiltinCapabilityCatalog` plus extension descriptors, and keep display/search metadata out of the canonical catalog struct.
- Done when: there is one code path that materializes section/profile/search records for built-ins without duplicating the underlying tool matrix.
- Verify: `cargo test -p tools split_module_tests`
- Stop if: a section/profile requirement cannot be represented without duplicating canonical metadata.

Step 2:
- Action: Rewire `tool_search` indexing, planner/provider surface assembly, and adapter-facing catalog/debug surfaces to consume `ToolCatalogView` instead of ad-hoc `ToolSpec` lists.
- Done when: ToolSearch results, planner-facing visible/deferred surfaces, and catalog/debug endpoints all come from the same derived directory logic.
- Verify: `cargo test -p tools split_module_tests && cargo test -p octopus-runtime-adapter runtime_contract_tests`
- Stop if: ToolSearch still needs hand-maintained catalog-only records not derivable from canonical metadata.

Step 3:
- Action: Update OpenAPI catalog contracts and `@octopus/schema` exports so section/profile/search summaries are transport-visible where the product needs them.
- Done when: catalog transport schemas match the derived view and generated client types compile without handwritten parallel types.
- Verify: `pnpm openapi:bundle && pnpm schema:generate && pnpm schema:check`
- Stop if: transport consumers require a second handwritten catalog shape instead of the generated contract.

Notes:
- This task closes the benchmark requirement that `ToolCatalogView` be a formal derived projection instead of a second truth source.

### Task 3: Refactor worker primitives and land the missing P0 worker family

Status: `pending`

Files:
- Create: `crates/tools/src/file_runtime.rs`
- Create: `crates/tools/src/shell_runtime.rs`
- Create: `crates/tools/src/patch_runtime.rs`
- Create: `crates/tools/src/process_runtime.rs`
- Create: `crates/tools/src/notebook_runtime.rs`
- Create: `crates/tools/src/web_runtime.rs`
- Delete: `crates/tools/src/fs_shell.rs`
- Modify: `crates/tools/src/lsp_runtime.rs`
- Modify: `crates/tools/src/web_external.rs`
- Modify: `crates/tools/src/builtin_catalog.rs`
- Modify: `crates/tools/src/builtin_exec.rs`
- Modify: `crates/tools/src/lib.rs`
- Modify: `crates/tools/src/capability_runtime/provider.rs`
- Test: `crates/tools/src/split_module_tests.rs`
- Test: `crates/octopus-runtime-adapter/src/capability_runtime_tests.rs`

Preconditions:
- Tasks 1 and 2 are complete.
- Worker-family tools are still owned by `crates/tools`, not by ad-hoc adapter or shell scripts.

Step 1:
- Action: Split the overloaded `fs_shell.rs` responsibilities into family-specific runtime modules and define benchmark-approved canonical schemas for `shell_exec`, `process_manage`, `apply_patch`, `read_file`, `write_file`, `edit_file`, `glob_search`, `grep_search`, `notebook_edit`, `web_search`, `web_fetch`, and `lsp_query`.
- Done when: each worker family has a clear handler owner module and catalog entries point at those modules through explicit handler keys.
- Verify: `cargo test -p tools split_module_tests`
- Stop if: a new worker tool cannot be implemented without bypassing the shared permission/audit boundary.

Step 2:
- Action: Replace `bash`, `NotebookEdit`, `LSP`, `WebFetch`, and `WebSearch` with their canonical names, add new `apply_patch` and `process_manage` handlers, and fold host-specific REPL/PowerShell behavior into `shell_exec` backend options instead of standalone tools.
- Done when: the full benchmark P0 worker list exists in the catalog and retired worker tools are gone from the model-visible surface.
- Verify: `cargo test -p tools split_module_tests && cargo test -p octopus-runtime-adapter capability_runtime_tests`
- Stop if: `apply_patch` requires a second edit engine or `process_manage` requires a second background-process store outside current runtime governance.

Step 3:
- Action: Remove dead code and tests tied to the old worker tool names and keep provider/exposure logic aligned to the new worker families.
- Done when: grep over runtime code and tests no longer shows legacy worker canonical names outside benchmark/history docs.
- Verify: `! rg -n \"\\b(bash|NotebookEdit|WebFetch|WebSearch|LSP|REPL|PowerShell)\\b\" crates/tools crates/octopus-runtime-adapter && cargo test -p tools split_module_tests`
- Stop if: a retained occurrence is required by a non-doc runtime contract that has not yet been cut over.

Notes:
- `web_external.rs` should shrink to non-web responsibilities after this task; `remote_trigger` moves out in Task 6.

### Task 4: Replace legacy control-plane tools and split runtime-config tools by real ownership

Status: `pending`

Files:
- Create: `crates/tools/src/control_plane_runtime.rs`
- Create: `crates/tools/src/runtime_config_tools.rs`
- Modify: `crates/tools/src/builtin_catalog.rs`
- Modify: `crates/tools/src/builtin_exec.rs`
- Modify: `crates/tools/src/lib.rs`
- Modify: `crates/tools/src/tool_registry.rs`
- Modify: `crates/tools/src/capability_runtime/provider.rs`
- Modify: `crates/octopus-runtime-adapter/src/runtime_config.rs`
- Modify: `crates/octopus-runtime-adapter/src/config_service.rs`
- Modify: `crates/octopus-runtime-adapter/src/agent_runtime_core.rs`
- Modify: `crates/octopus-runtime-adapter/src/runtime_config_tests.rs`
- Modify: `contracts/openapi/src/components/schemas/runtime.yaml`
- Modify: `contracts/openapi/src/paths/runtime.yaml`
- Modify: `packages/schema/src/runtime-config.ts`
- Modify: `packages/schema/src/capability-runtime.ts`
- Modify: `docs/runtime_config_api.md`
- Generated: `contracts/openapi/octopus.openapi.yaml`
- Generated: `packages/schema/src/generated.ts`
- Test: `crates/tools/src/split_module_tests.rs`
- Test: `crates/octopus-runtime-adapter/src/runtime_config_tests.rs`
- Test: `crates/octopus-runtime-adapter/src/capability_runtime_tests.rs`

Preconditions:
- Tasks 1 through 3 are complete.
- Runtime config remains file-first and patch-based per repository governance.

Step 1:
- Action: Replace `TodoWrite` with `update_plan`, replace `AskUserQuestion` with structured `ask_user`, keep `tool_search`, `structured_output`, and `sleep`, and remove the retired control-plane built-ins from the model-visible catalog.
- Done when: the benchmark control-plane family is the only production control-plane family exposed to the model.
- Verify: `cargo test -p tools split_module_tests`
- Stop if: a removed control-plane tool is still carrying essential behavior that has not been remapped to `update_plan`, `ask_user`, or internal runtime flow.

Step 2:
- Action: Split `Config` into `runtime_config_get`, `runtime_config_validate_patch`, and `runtime_config_save_patch`, with direct ownership in the runtime config service rather than a generic stub handler.
- Done when: the three runtime-config tools map 1:1 to adapter/runtime-config semantics and preserve partial patch behavior, secret redaction, and validate-before-save flow.
- Verify: `cargo test -p octopus-runtime-adapter runtime_config_tests && cargo test -p tools split_module_tests`
- Stop if: save flow still requires whole-file overwrite or writes sensitive secrets back to plain config files.

Step 3:
- Action: Update runtime OpenAPI/schema contracts and the runtime-config companion doc so the new config tool behavior is transport-visible and documented in the canonical runtime-config API doc.
- Done when: runtime config transport types, docs, and tool handlers describe the same `get / validate_patch / save_patch` model.
- Verify: `pnpm openapi:bundle && pnpm schema:generate && pnpm schema:check`
- Stop if: any caller still depends on handwritten transport types instead of generated contracts.

Notes:
- `SendUserMessage` does not survive as a built-in in this plan. If background notification semantics are still needed later, reintroduce them as a dedicated notification family, not as a generic control-plane tool.

### Task 5: Promote orchestration and session tooling into first-class built-ins

Status: `pending`

Files:
- Create: `crates/tools/src/agent_runtime.rs`
- Create: `crates/tools/src/session_runtime.rs`
- Modify: `crates/tools/src/subagent_runtime.rs`
- Modify: `crates/tools/src/builtin_catalog.rs`
- Modify: `crates/tools/src/builtin_exec.rs`
- Modify: `crates/tools/src/lib.rs`
- Modify: `crates/tools/src/capability_runtime/provider.rs`
- Modify: `crates/tools/src/capability_runtime/state.rs`
- Modify: `crates/octopus-runtime-adapter/src/session_service.rs`
- Modify: `crates/octopus-runtime-adapter/src/subrun_orchestrator.rs`
- Modify: `crates/octopus-runtime-adapter/src/team_runtime.rs`
- Modify: `crates/octopus-runtime-adapter/src/workflow_runtime.rs`
- Modify: `crates/octopus-runtime-adapter/src/agent_runtime_core.rs`
- Modify: `crates/octopus-runtime-adapter/src/persistence.rs`
- Modify: `crates/octopus-runtime-adapter/src/capability_executor_bridge.rs`
- Modify: `contracts/openapi/src/components/schemas/runtime.yaml`
- Modify: `contracts/openapi/src/paths/runtime.yaml`
- Modify: `packages/schema/src/agent-runtime.ts`
- Modify: `packages/schema/src/runtime.ts`
- Generated: `contracts/openapi/octopus.openapi.yaml`
- Generated: `packages/schema/src/generated.ts`
- Test: `crates/tools/src/split_module_tests.rs`
- Test: `crates/octopus-runtime-adapter/src/approval_runtime_tests.rs`
- Test: `crates/octopus-runtime-adapter/src/runtime_persistence_tests.rs`

Preconditions:
- Tasks 1 through 4 are complete.
- Session and subrun state remain sourced from runtime aggregates and persistence, not ad-hoc manifest scanning.

Step 1:
- Action: Replace the current subagent-only public tool surface with benchmark-approved orchestration built-ins: `spawn_agent`, `send_agent_input`, `wait_agent`, and `close_agent`.
- Done when: agent orchestration uses canonical tool names and a dedicated handler boundary instead of exposing bespoke subagent internals directly.
- Verify: `cargo test -p tools split_module_tests && cargo test -p octopus-runtime-adapter approval_runtime_tests`
- Stop if: a new orchestration tool would bypass session policy, approval, or capability-state ownership.

Step 2:
- Action: Add `session_status`, `session_history`, `session_search`, and `session_yield`, backed by runtime session persistence and searchable projections rather than side-channel file inspection.
- Done when: session tools can answer current status, history, search, and yield semantics from runtime source-of-truth data.
- Verify: `cargo test -p octopus-runtime-adapter runtime_persistence_tests`
- Stop if: current persistence cannot answer one of these tools without inventing a second storage path.

Step 3:
- Action: Thread the new orchestration/session family through runtime contracts, resume/replay state, and capability summaries so suspended or restored runs keep the same semantics.
- Done when: runtime transport records and capability state snapshots faithfully represent orchestration/session tool calls and outcomes.
- Verify: `pnpm openapi:bundle && pnpm schema:generate && pnpm schema:check && cargo test -p octopus-runtime-adapter approval_runtime_tests && cargo test -p octopus-runtime-adapter runtime_persistence_tests`
- Stop if: resume/replay loses orchestration state or requires a compatibility bridge back to legacy subagent tool names.

Notes:
- `session_search` is not optional in this plan; it is part of the benchmarked core runtime tool surface.

### Task 6: Add the automation family and move `remote_trigger` under runtime control-plane ownership

Status: `pending`

Files:
- Create: `crates/tools/src/automation_runtime.rs`
- Modify: `crates/tools/src/web_external.rs`
- Modify: `crates/tools/src/builtin_catalog.rs`
- Modify: `crates/tools/src/builtin_exec.rs`
- Modify: `crates/tools/src/lib.rs`
- Modify: `crates/octopus-runtime-adapter/src/background_runtime.rs`
- Modify: `crates/octopus-runtime-adapter/src/workflow_runtime.rs`
- Modify: `crates/octopus-runtime-adapter/src/session_service.rs`
- Modify: `crates/octopus-runtime-adapter/src/execution_service.rs`
- Modify: `crates/octopus-runtime-adapter/src/persistence.rs`
- Modify: `contracts/openapi/src/components/schemas/runtime.yaml`
- Modify: `contracts/openapi/src/paths/runtime.yaml`
- Modify: `packages/schema/src/workflow-runtime.ts`
- Modify: `packages/schema/src/runtime.ts`
- Generated: `contracts/openapi/octopus.openapi.yaml`
- Generated: `packages/schema/src/generated.ts`
- Test: `crates/octopus-runtime-adapter/src/runtime_persistence_tests.rs`
- Test: `crates/octopus-runtime-adapter/src/runtime_contract_tests.rs`

Preconditions:
- Tasks 1 through 5 are complete.
- Automation state will use existing runtime persistence rules instead of ad-hoc files.

Step 1:
- Action: Create the automation family with `cron_create`, `cron_list`, `cron_delete`, `remote_trigger`, and `monitor`, and move `remote_trigger` out of the legacy mixed web module into an automation-owned handler.
- Done when: automation tooling is represented as one built-in family with catalog metadata, handler ownership, and profile rules.
- Verify: `cargo test -p tools split_module_tests`
- Stop if: automation logic needs a second control plane outside the runtime adapter.

Step 2:
- Action: Back cron and monitor behavior with durable runtime state in the existing runtime persistence stack and expose the scheduling/monitoring state through workflow/session summaries.
- Done when: scheduled or monitored work can survive resume/reload and remains auditable through runtime projections.
- Verify: `cargo test -p octopus-runtime-adapter runtime_persistence_tests && cargo test -p octopus-runtime-adapter runtime_contract_tests`
- Stop if: monitor or cron state would require an ungoverned new persistence path.

Step 3:
- Action: Update runtime transport contracts and generated schema types so automation family inputs/outputs are first-class runtime records instead of private adapter-only JSON.
- Done when: automation records are represented in OpenAPI/schema and compile cleanly across runtime consumers.
- Verify: `pnpm openapi:bundle && pnpm schema:generate && pnpm schema:check`
- Stop if: adapter or frontend code starts depending on handwritten transport shapes for automation.

Notes:
- This task turns the current `Sleep`/`RemoteTrigger` edge utilities into a coherent runtime control-plane family rather than leaving them as isolated tools.

### Task 7: Add browser and artifact readers behind a single runtime owner

Status: `pending`

Files:
- Create: `crates/tools/src/browser_runtime.rs`
- Create: `crates/tools/src/artifact_runtime.rs`
- Create: `crates/octopus-runtime-adapter/src/browser_runtime.rs`
- Modify: `crates/octopus-runtime-adapter/src/lib.rs`
- Modify: `crates/octopus-runtime-adapter/src/execution_service.rs`
- Modify: `crates/octopus-runtime-adapter/src/agent_runtime_core.rs`
- Modify: `crates/octopus-runtime-adapter/src/runtime_contract_tests.rs`
- Modify: `crates/tools/src/builtin_catalog.rs`
- Modify: `crates/tools/src/builtin_exec.rs`
- Modify: `crates/tools/src/lib.rs`
- Modify: `contracts/openapi/src/components/schemas/runtime.yaml`
- Modify: `contracts/openapi/src/paths/runtime.yaml`
- Modify: `packages/schema/src/artifact.ts`
- Modify: `packages/schema/src/runtime.ts`
- Generated: `contracts/openapi/octopus.openapi.yaml`
- Generated: `packages/schema/src/generated.ts`
- Test: `crates/tools/src/split_module_tests.rs`
- Test: `crates/octopus-runtime-adapter/src/runtime_contract_tests.rs`

Preconditions:
- Tasks 1 through 6 are complete.
- Browser ownership is resolved to one adapter-owned service boundary.

Step 1:
- Action: Create a single browser session service in the runtime adapter and wire `browser_navigate`, `browser_snapshot`, `browser_click`, `browser_type`, `browser_scroll`, `browser_back`, `browser_press`, and `browser_console` through `crates/tools`.
- Done when: browser tools do not shell out through ad-hoc commands and share one session/approval/audit boundary.
- Verify: `cargo test -p tools split_module_tests && cargo test -p octopus-runtime-adapter runtime_contract_tests`
- Stop if: execution cannot identify one browser owner and would need multiple inconsistent control paths.

Step 2:
- Action: Add `image_read` and `pdf_read` as artifact-reading built-ins with normalized outputs and governed artifact storage behavior.
- Done when: image/PDF reading is available as catalog-backed built-ins and any extracted artifacts follow repository storage rules.
- Verify: `cargo test -p tools split_module_tests`
- Stop if: artifact reading would store large content in SQLite or bypass existing artifact/blob storage rules.

Step 3:
- Action: Reflect browser/artifact tool records in runtime contracts, generated schema, and runtime summaries so UI/debug/runtime replay can reason about them consistently.
- Done when: browser and artifact family calls appear in runtime contracts with generated types and replay-safe metadata.
- Verify: `pnpm openapi:bundle && pnpm schema:generate && pnpm schema:check`
- Stop if: replay or audit metadata for browser sessions cannot be made deterministic.

Notes:
- Browser tools are phase 3 by design; do not start them before P0 and P1 families are stable.

### Task 8: Implement `execute_code` as a separately governed high-risk family

Status: `pending`

Files:
- Create: `crates/tools/src/code_execution_runtime.rs`
- Modify: `crates/tools/src/builtin_catalog.rs`
- Modify: `crates/tools/src/builtin_exec.rs`
- Modify: `crates/tools/src/lib.rs`
- Modify: `crates/octopus-runtime-adapter/src/execution_service.rs`
- Modify: `crates/octopus-runtime-adapter/src/approval_flow.rs`
- Modify: `crates/octopus-runtime-adapter/src/runtime_contract_tests.rs`
- Modify: `contracts/openapi/src/components/schemas/runtime.yaml`
- Modify: `packages/schema/src/runtime.ts`
- Generated: `contracts/openapi/octopus.openapi.yaml`
- Generated: `packages/schema/src/generated.ts`
- Test: `crates/tools/src/split_module_tests.rs`
- Test: `crates/octopus-runtime-adapter/src/approval_runtime_tests.rs`
- Test: `crates/octopus-runtime-adapter/src/runtime_contract_tests.rs`

Preconditions:
- Tasks 1 through 7 are complete.
- A stricter sandbox contract than `shell_exec` has been designed and approved in code before this task starts.

Step 1:
- Action: Implement `execute_code` as its own family with explicit language allowlist, timeout/memory/output limits, working-directory rules, and deterministic result contract.
- Done when: `execute_code` is not an alias to `shell_exec` and has materially stronger isolation and result semantics.
- Verify: `cargo test -p tools split_module_tests`
- Stop if: the implementation cannot provide stronger isolation than `shell_exec`.

Step 2:
- Action: Route `execute_code` through a stricter approval and policy path than normal worker tools and default it to disabled unless a profile explicitly allows it.
- Done when: policy, approval, and runtime summaries distinguish `execute_code` from standard shell execution.
- Verify: `cargo test -p octopus-runtime-adapter approval_runtime_tests && cargo test -p octopus-runtime-adapter runtime_contract_tests`
- Stop if: approval cannot differentiate `execute_code` risk from `shell_exec`.

Step 3:
- Action: Add runtime transport support and generated schema coverage for code-execution requests, outcomes, and blocked/approval states.
- Done when: code execution is fully represented in generated contracts and runtime replay metadata.
- Verify: `pnpm openapi:bundle && pnpm schema:generate && pnpm schema:check`
- Stop if: transport changes would expose an unstable or under-specified result model.

Notes:
- This task may remain `blocked` at the end of the broader cutover if the stronger sandbox cannot be proven. Do not water it down to claim benchmark completion for P0/P1/P2.

### Task 9: Run the cross-layer cleanup, generated-contract pipeline, and final verification sweep

Status: `pending`

Files:
- Modify: `contracts/openapi/src/root.yaml`
- Modify: `contracts/openapi/src/components/schemas/runtime.yaml`
- Modify: `contracts/openapi/src/components/schemas/catalog.yaml`
- Modify: `contracts/openapi/src/paths/runtime.yaml`
- Modify: `contracts/openapi/src/paths/catalog.yaml`
- Modify: `packages/schema/src/capability-runtime.ts`
- Modify: `packages/schema/src/catalog.ts`
- Modify: `packages/schema/src/runtime.ts`
- Modify: `packages/schema/src/agent-runtime.ts`
- Modify: `packages/schema/src/index.ts`
- Modify: `docs/openapi-audit.md`
- Generated: `contracts/openapi/octopus.openapi.yaml`
- Generated: `packages/schema/src/generated.ts`

Preconditions:
- Tasks 1 through 8 are complete or explicitly marked `blocked` with a justified stop condition.
- No remaining production runtime path depends on retired tool names.

Step 1:
- Action: Remove dead code, dead tests, and dead transport aliases that reference retired built-ins or pre-cutover catalog behavior.
- Done when: repository runtime code has one current built-in tool surface and benchmark-retired names remain only in historical docs or migration notes.
- Verify: `! rg -n \"\\b(WebFetch|WebSearch|ToolSearch|NotebookEdit|AskUserQuestion|TodoWrite|TestingPermission|REPL|PowerShell|SendUserMessage|Config|EnterPlanMode|ExitPlanMode|Brief)\\b\" crates packages contracts apps`
- Stop if: a remaining hit is part of a still-live runtime/API contract rather than dead code.

Step 2:
- Action: Run the full contract pipeline and cross-layer verification suite after all tool-family tasks have landed.
- Done when: Rust tests, OpenAPI bundle/generation, schema checks, and frontend compile/check all pass on the cutover branch.
- Verify: `cargo fmt --all && cargo test -p tools && cargo test -p octopus-runtime-adapter && pnpm openapi:bundle && pnpm schema:generate && pnpm schema:check && pnpm check:frontend`
- Stop if: generated contract changes break unrelated surfaces that need separate ownership decisions.

Step 3:
- Action: Update audit documentation to reflect the new transport/runtime coverage and mark the benchmark follow-through complete from an execution perspective.
- Done when: docs that track OpenAPI/runtime coverage match the implemented tool portfolio and no longer describe the legacy built-in surface as current.
- Verify: `pnpm schema:check`
- Stop if: documentation changes would introduce new policy instead of reflecting already-approved governance.

Notes:
- This task is where the repository should end with zero mixed-case canonical tool names in production code.

## Batch Checkpoint Format

After each execution batch, append a checkpoint in this format:

```md
## Checkpoint YYYY-MM-DD HH:MM

- Batch: Task N Step X -> Task M Step Y
- Completed:
  - short list
- Verification:
  - `command` -> pass or fail
- Blockers:
  - none
- Next:
  - Task N Step Z
```
