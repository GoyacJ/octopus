# SDK W5 Plan Audit Fixes Implementation Plan

> **For Codex:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task.

## Goal

Repair the W5 SDK weekly plan so its subagent/plugin architecture, task ledger, and linked control docs are executable against the current codebase again.

## Architecture

This work stays in the documentation control plane. The main fix is to make `docs/plans/sdk/08-week-5-subagent-plugin.md` line up with the actual runtime surfaces already present in `octopus-sdk-tools`, `octopus-sdk-hooks`, `octopus-sdk-context`, `octopus-sdk-contracts`, and `octopus-sdk-session`, then backfill the linked topology, retirement map, and spec-drift log that the W5 plan treats as source of truth.

## Scope

- In scope:
- `docs/plans/sdk/08-week-5-subagent-plugin.md`
- `docs/plans/sdk/01-ai-execution-protocol.md`
- `docs/plans/sdk/02-crate-topology.md`
- `docs/plans/sdk/00-overview.md`
- `docs/plans/sdk/03-legacy-retirement.md`
- `docs/sdk/README.md`
- `docs/plans/2026-04-21-sdk-w5-plan-audit-fixes.md`
- Out of scope:
- Any crate or code changes under `crates/**`
- Rewriting `docs/sdk/05-sub-agents.md` or `docs/sdk/12-plugin-system.md` body text
- Editing `docs/plans/sdk/README.md`

## Risks Or Open Questions

- `docs/plans/sdk/08-week-5-subagent-plugin.md` is currently an untracked draft in this worktree. The repair must preserve that draft content and only close the accepted contradictions.
- `docs/plans/sdk/00-overview.md` and `docs/plans/sdk/README.md` already have user-visible uncommitted edits. This repair should not touch them unless verification proves it is required.
- If closing the W5 contradictions requires normative changes in `docs/sdk/05-sub-agents.md` or `docs/sdk/12-plugin-system.md` instead of a Fact-Fix entry, execution must stop and leave that for a dedicated spec-change plan.

## Execution Rules

- Do not edit files until each accepted finding is mapped to an exact file and acceptance condition.
- Keep fixes limited to closing the accepted findings plus the minimum linked registry/fact-fix edits they require.
- Preserve existing user changes in the worktree; do not revert unrelated diffs.
- Update task status in this plan after each batch.

## Task Ledger

### Task 1: Repair W5 architecture and task-ledger contracts

Status: `done`

Files:
- Modify: `docs/plans/sdk/08-week-5-subagent-plugin.md`

Preconditions:
- The accepted audit findings have been re-verified against the current W5 draft and current runtime/session/tool/hook contracts.

Step 1:
- Action: Repair the W5 plugin registration model so declaration structs stay metadata-only and runtime registration explicitly targets executable tool/hook/provider surfaces instead of declaration-only registries.
- Done when: the W5 plan no longer claims `ToolRegistration` / `HookRegistration` only accept `ToolDecl` / `HookDecl` and the plugin registration flow clearly distinguishes declaration metadata from executable runtime records.
- Verify: `rg -n 'fn register\\(&self, decl: ToolDecl\\)|fn register\\(&self, decl: HookDecl\\)|只含数据字段（id/name/schema/source/…），不持有 trait object|PluginApi.*register_tool\\(&mut self, decl: ToolDecl\\)' docs/plans/sdk/08-week-5-subagent-plugin.md`
- Stop if: repairing the contract would require changing current `octopus-sdk-tools` or `octopus-sdk-hooks` code instead of fixing the W5 plan wording.

Step 2:
- Action: Repair the subagent runtime surface so `SubagentContext` and the related tasks depend on a runtime-readable/executable tool view (`ToolRegistry` or equivalent filtered directory), not registration-only traits.
- Done when: `SubagentContext.tools` and Task 2 stop using `Arc<dyn ToolRegistration>`, and the W5 plan states that prompt building, allowlist filtering, and dispatch flow all work from a child `ToolRegistry`/tool directory derived from the parent runtime registry.
- Verify: `rg -n 'SubagentContext|ToolRegistration|ToolRegistry|ToolDirectory|call_tool|filtered child registry' docs/plans/sdk/08-week-5-subagent-plugin.md`
- Stop if: the repaired subagent contract would force a W4/W3 public-surface rewrite outside the planned W5 doc set.

Step 3:
- Action: Repair the hook declaration layer and stale legacy inventory so W5 uses a static hook point enum for manifests and treats subagent/plugin implementation as a greenfield SDK build, not a migration from nonexistent `worker_boot`/`subagent_runtime` abstractions.
- Done when: `HookDecl` no longer uses runtime `HookEvent` as a declaration field, D3 and related retirement text stop claiming `worker_boot` or `subagent_runtime` currently provide fan-out/fan-in abstractions, and the W5 plan says the runtime sources are `docs/sdk/05-sub-agents.md` plus current SDK crate boundaries.
- Verify: `rg -n 'HookDecl|HookPoint|HookEvent|worker_boot|subagent_runtime|fan-out|fan-in|greenfield|绿色实现' docs/plans/sdk/08-week-5-subagent-plugin.md`
- Stop if: the repair would require redefining current runtime ownership in `03-legacy-retirement.md` beyond the accepted audit findings.

Step 4:
- Action: Front-load the `plugins_snapshot` contract work as a dedicated contracts/session task instead of a trailing session patch.
- Done when: the W5 plan states that `plugins_snapshot` first expands Level 0 contracts and session snapshot/store surfaces before any replay/session-store task, and Task 10 no longer understates the blast radius to fixtures/schema/OpenAPI.
- Verify: `rg -n 'plugins_snapshot|PluginsSnapshot|SessionStarted|SessionSnapshot|SessionStore|OpenAPI|schema' docs/plans/sdk/08-week-5-subagent-plugin.md`
- Stop if: closing the gap requires directly editing generated schema/OpenAPI artifacts in this batch.

### Task 2: Backfill topology for W5 public-surface corrections

Status: `done`

Files:
- Modify: `docs/plans/sdk/02-crate-topology.md`

Preconditions:
- Task 1 is done so the W5 plan spells out the required linked-document changes precisely.

Step 1:
- Action: Update `docs/plans/sdk/02-crate-topology.md §2.1 / §2.10 / §2.11` so the W5 placeholders match current runtime ownership: static hook point declarations, runtime tool registries for subagents, executable plugin registration surfaces, and the real `plugins_snapshot` expansion point.
- Done when: `02-crate-topology.md` no longer describes decl-only registration or `HookDecl.event: HookEvent`, and it documents `SubagentContext` against `ToolRegistry` plus a contracts-first `PluginsSnapshot` contract.
- Verify: `rg -n 'HookPoint|ToolRegistry|PluginsSnapshot|PluginRuntimeTool|PluginRuntimeHook|ExecutableTool|ExecutableHook' docs/plans/sdk/02-crate-topology.md`
- Stop if: the topology backfill would require inventing symbols that the repaired W5 plan does not actually expose.

### Task 3: Backfill retirement map and Fact-Fix baseline

Status: `done`

Files:
- Modify: `docs/plans/sdk/03-legacy-retirement.md`
- Modify: `docs/sdk/README.md`

Preconditions:
- Tasks 1-2 are done so the linked retirement and spec-drift wording can point to a stable W5 execution baseline.

Step 1:
- Action: Update `03-legacy-retirement.md` so `worker_boot.rs` and `subagent_runtime.rs` are described as stale control-plane or TODO remnants, not current SDK implementation sources, and so W5 retirement status matches the repaired scope.
- Done when: `03-legacy-retirement.md` no longer claims W5 subagent SDK work is migrating active fan-out/fan-in logic from `worker_boot.rs` or `subagent_runtime.rs`.
- Verify: `rg -n 'worker_boot|subagent_runtime|TODO|stub|trust gate|prompt misdelivery' docs/plans/sdk/03-legacy-retirement.md`
- Stop if: closing the contradiction would require changing the retirement week allocation itself instead of the wording.

Step 2:
- Action: Append a Fact-Fix entry in `docs/sdk/README.md` for the W5 execution baseline that freezes declaration-vs-runtime separation, `HookPoint` for plugin manifests, greenfield subagent/plugin implementation, and contracts-first `plugins_snapshot`.
- Done when: `docs/sdk/README.md` has a new W5 Fact-Fix row naming `05-sub-agents.md`, `12-plugin-system.md`, and `08-week-5-subagent-plugin.md`, with the repaired execution baseline stated clearly.
- Verify: `rg -n '08-week-5-subagent-plugin\\.md|HookPoint|plugins_snapshot|greenfield|declaration|runtime' docs/sdk/README.md`
- Stop if: the drift cannot be described as a temporary W5 execution baseline and instead requires rewriting normative spec chapters now.

### Task 4: Verify and checkpoint

Status: `done`

Files:
- Modify: `docs/plans/2026-04-21-sdk-w5-plan-audit-fixes.md`

Preconditions:
- Tasks 1-3 are done.

Step 1:
- Action: Run targeted verification against the repaired files and update this plan with final statuses plus a checkpoint.
- Done when: each accepted finding is closed by a concrete doc change and the verification commands return the expected matches with no stale references left behind.
- Verify:
  - `git diff --check -- docs/plans/sdk/08-week-5-subagent-plugin.md docs/plans/sdk/02-crate-topology.md docs/plans/sdk/03-legacy-retirement.md docs/sdk/README.md docs/plans/2026-04-21-sdk-w5-plan-audit-fixes.md`
  - `! rg -n 'fn register\\(&self, decl: ToolDecl\\)|fn register\\(&self, decl: HookDecl\\)|register_tool\\(&mut self, decl: ToolDecl\\)|register_hook\\(&mut self, decl: HookDecl\\)|Arc<dyn ToolRegistration>|SubagentContext\\.tools: Arc<dyn ToolRegistration>|HookDecl \\{ id: String, event: HookEvent' docs/plans/sdk/08-week-5-subagent-plugin.md`
  - `rg -n 'PluginToolRegistration|PluginHookRegistration|前置合同硬门禁|W5 不再把这两处当成 subagent SDK 的实现来源|SubagentContext\\.tools: Arc<ToolRegistry>|HookDecl \\{ id: String, point: HookPoint' docs/plans/sdk/08-week-5-subagent-plugin.md`
  - `rg -n 'PluginToolRegistration|PluginHookRegistration|HookPoint|SubagentContext|ToolRegistry|plugins_snapshot|SessionSnapshot' docs/plans/sdk/02-crate-topology.md`
  - `rg -n 'worker_boot.*W5 不作为|subagent_runtime.*TODO stub|greenfield SDK 覆盖职责边界|trust gate|prompt misdelivery' docs/plans/sdk/03-legacy-retirement.md`
  - `rg -n '08-week-5-subagent-plugin\\.md|HookPoint|plugins_snapshot|greenfield|declaration|runtime' docs/sdk/README.md`
- Stop if: verification exposes a new contradiction outside the planned file set.

### Task 5: Close residual follow-up gaps from the second W5 review

Status: `done`

Files:
- Modify: `docs/plans/sdk/08-week-5-subagent-plugin.md`
- Modify: `docs/plans/sdk/02-crate-topology.md`
- Modify: `docs/plans/sdk/00-overview.md`
- Modify: `docs/plans/2026-04-21-sdk-w5-plan-audit-fixes.md`

Preconditions:
- Task 1-4 are done and the residual findings have been re-verified against the current draft.

Step 1:
- Action: Remove the remaining Level 0 leaks from the W5 plan and linked topology by switching `SubagentSpec.model_role` / `ModelProviderDecl.provider_ref` to opaque string keys, keeping `SubagentError::Provider` as `reason: String`, and moving canonical model normalization behind Level 1 helpers.
- Done when: the W5 plan and topology no longer say `model_role: ModelRole`, `ModelProviderDecl { ... provider_id: ProviderId }`, or `Provider(#[from] ModelError)` for Level 0 declarations.
- Verify: `! rg -n 'model_role: ModelRole|ModelProviderDecl \\{ pub id: String, pub provider_id: ProviderId \\}|Provider\\(#\\[from\\] ModelError\\)' docs/plans/sdk/08-week-5-subagent-plugin.md docs/plans/sdk/02-crate-topology.md`
- Stop if: closing the gap requires changing current SDK crate code instead of the plan/topology docs.

Step 2:
- Action: Tighten the remaining W5 plan wording around `plugins_snapshot`, bundled noop plugin layout, `worker_boot` migration-week mirroring, and the `HookPoint`/`HookEvent` mapping risk.
- Done when: the W5 plan routes `plugins_snapshot` fallback through `session.plugins_snapshot`, the noop plugin uses `src/bundled.rs` plus a manifest fixture, `worker_boot` is mirrored as a W7 review item, and R13 is present.
- Verify: `rg -n 'session\\.plugins_snapshot|src/bundled\\.rs|worker_boot|R13|HookPoint 与 `HookEvent`|provider_ref: String|model_role: String' docs/plans/sdk/08-week-5-subagent-plugin.md`
- Stop if: the repair would require changing `03-legacy-retirement.md` again beyond the already accepted W7 mirror.

Step 3:
- Action: Sync `00-overview.md` so W5 exit state wording matches the repaired plan, then extend this execution plan with the follow-up checkpoint and verification record.
- Done when: `00-overview.md` no longer promises trait-object plugin access for `SkillRegistry / ModelProvider`, and this plan records the second review batch explicitly.
- Verify:
  - `rg -n 'ToolRegistry / HookRunner.*executable runtime registration|SkillDecl / ModelProviderDecl.*metadata \\+ builder slot' docs/plans/sdk/00-overview.md docs/plans/sdk/08-week-5-subagent-plugin.md`
  - `git diff --check -- docs/plans/sdk/08-week-5-subagent-plugin.md docs/plans/sdk/02-crate-topology.md docs/plans/sdk/00-overview.md docs/plans/2026-04-21-sdk-w5-plan-audit-fixes.md`
- Stop if: verification exposes a new W5 inconsistency that needs changes outside the docs control plane.

### Task 6: Close the third W5 audit batch on session fallback, public-surface registration, and line-count guards

Status: `done`

Files:
- Modify: `docs/plans/sdk/08-week-5-subagent-plugin.md`
- Modify: `docs/plans/sdk/02-crate-topology.md`
- Modify: `docs/plans/sdk/00-overview.md`
- Modify: `docs/sdk/README.md`
- Modify: `docs/plans/2026-04-21-sdk-w5-plan-audit-fixes.md`

Preconditions:
- Task 1-5 are done and the third audit batch has been re-verified against the current draft wording.

Step 1:
- Action: Split the W5 `plugins_snapshot` session contract into two explicit completion branches across the weekly plan and linked control docs: embedded in `SessionStarted` when possible, or a second `session.plugins_snapshot` event immediately after `session.started` when the first-event contract cannot be extended.
- Done when: Task 10, W5 exit-state wording, `02 §2.1 / §2.2 / §5`, and the W5 Fact-Fix row all describe the same two branches and replay contract instead of hard-coding only the embedded-first-event shape.
- Verify: `rg -n 'session\\.plugins_snapshot|SessionPluginsSnapshot|test_append_session_plugins_snapshot|test_snapshot_replay_second_event|SessionStarted.*Option<PluginsSnapshot>|SessionStarted 或紧随其后的 `session\\.plugins_snapshot`' docs/plans/sdk/08-week-5-subagent-plugin.md docs/plans/sdk/02-crate-topology.md docs/plans/sdk/00-overview.md docs/sdk/README.md`
- Stop if: closing the mismatch would require normative spec rewrites outside the docs control plane.

Step 2:
- Action: Register the missing `SessionStore` helpers in `02-crate-topology.md`, remove the stale `octopus-sdk-model` dependency from W5 Task 7, and replace every size-based 800-line guard used by this control-doc set with a real line-count command.
- Done when: `02 §2.2` explicitly exposes `append_session_started(..., Option<PluginsSnapshot>)` and `new_child_session(...)`, Task 7 no longer says `octopus-sdk-plugin` depends on `octopus-sdk-model`, and both W5/W8/DoD guards use `wc -l`-based checks instead of `find -size`.
- Verify:
  - `rg -n 'append_session_started|new_child_session' docs/plans/sdk/02-crate-topology.md docs/plans/sdk/08-week-5-subagent-plugin.md`
  - `! rg -n 'octopus-sdk-contracts / -model / -tools / -hooks|find .* -size \\+800' docs/plans/sdk/08-week-5-subagent-plugin.md docs/plans/sdk/00-overview.md`
- Stop if: the fix would require changing real crate manifests or code instead of the control docs.

Step 3:
- Action: Re-run targeted doc verification and append a checkpoint for this audit batch.
- Done when: the accepted findings are closed with no stale wording left in the touched files, and this plan records the batch explicitly.
- Verify: `git diff --check -- docs/plans/sdk/08-week-5-subagent-plugin.md docs/plans/sdk/02-crate-topology.md docs/plans/sdk/00-overview.md docs/sdk/README.md docs/plans/2026-04-21-sdk-w5-plan-audit-fixes.md`
- Stop if: verification exposes a new contradiction outside these files.

### Task 7: Close the fourth W5 audit batch on expose wording and shared line-count guards

Status: `done`

Files:
- Modify: `docs/plans/sdk/08-week-5-subagent-plugin.md`
- Modify: `docs/plans/sdk/01-ai-execution-protocol.md`
- Modify: `docs/plans/sdk/03-legacy-retirement.md`
- Modify: `docs/plans/2026-04-21-sdk-w5-plan-audit-fixes.md`

Preconditions:
- Task 1-6 are done and the fourth audit batch has been re-verified against the current control-doc wording.

Step 1:
- Action: Repair the W5 Architecture `expose` wording so `PluginRegistry::get_snapshot()` feeds session-start persistence generically instead of hard-coding first-event delivery.
- Done when: the `expose` phase explicitly allows the snapshot to be embedded in `SessionStarted` or emitted as the immediate `session.plugins_snapshot` follow-up event.
- Verify: `rg -n 'session start 持久化输入|session\\.plugins_snapshot 次事件载荷' docs/plans/sdk/08-week-5-subagent-plugin.md`
- Stop if: closing the wording gap would require changing the already-accepted dual-branch contract in linked docs.

Step 2:
- Action: Replace the remaining size-based 800-line guards in the shared execution protocol and retirement Weekly Gate with the same `wc -l + awk` line-count check used by the repaired W5 docs.
- Done when: `01 §7.4` and `03 §8` no longer contain `find ... -size +800`, and `03` no longer references `find +800` in its W8 special-case note.
- Verify: `! rg -n 'find .* -size \\+800|find \\+800' docs/plans/sdk/01-ai-execution-protocol.md docs/plans/sdk/03-legacy-retirement.md`
- Stop if: the fix would require rewriting historical completed-week checkpoints outside these shared control docs.

Step 3:
- Action: Re-run targeted verification and append a checkpoint for this audit batch.
- Done when: the three accepted findings are closed and this plan records the batch explicitly.
- Verify: `git diff --check -- docs/plans/sdk/08-week-5-subagent-plugin.md docs/plans/sdk/01-ai-execution-protocol.md docs/plans/sdk/03-legacy-retirement.md docs/plans/2026-04-21-sdk-w5-plan-audit-fixes.md`
- Stop if: verification exposes a new contradiction outside these files.

## Batch Checkpoint Format

After each batch, append a short checkpoint using this shape:

```md
## Checkpoint YYYY-MM-DD HH:MM

- Batch: Task 1 Step 1 -> Task 2 Step 1
- Completed: short list
- Verification:
  - `command` -> pass or fail
- Blockers:
  - none
- Next:
  - Task 2 Step 2
```

## Checkpoint 2026-04-21 20:12

- Batch: Task 1 Step 1 -> Task 4 Step 1
- Completed:
  - Repaired the W5 weekly plan so plugin registration is split into declaration metadata and executable runtime registration, `SubagentContext.tools` points back to `ToolRegistry`, `HookDecl` uses `HookPoint`, legacy `worker_boot/subagent_runtime` are treated as non-source boundaries, and `plugins_snapshot` is front-loaded as a contracts/session gate
  - Backfilled `docs/plans/sdk/02-crate-topology.md`, `docs/plans/sdk/03-legacy-retirement.md`, and `docs/sdk/README.md` so the W5 execution baseline matches the repaired plan, including the `plugins_snapshot` discrepancy entry and Fact-Fix row
  - Fixed the `02-crate-topology.md` discrepancy registry numbering after inserting the new `plugins_snapshot` row and updated this plan's verification commands so negative checks are executable
- Verification:
  - `git diff --check -- docs/plans/sdk/08-week-5-subagent-plugin.md docs/plans/sdk/02-crate-topology.md docs/plans/sdk/03-legacy-retirement.md docs/sdk/README.md docs/plans/2026-04-21-sdk-w5-plan-audit-fixes.md` -> pass
  - `! rg -n 'fn register\(&self, decl: ToolDecl\)|fn register\(&self, decl: HookDecl\)|register_tool\(&mut self, decl: ToolDecl\)|register_hook\(&mut self, decl: HookDecl\)|Arc<dyn ToolRegistration>|SubagentContext\.tools: Arc<dyn ToolRegistration>|HookDecl \{ id: String, event: HookEvent' docs/plans/sdk/08-week-5-subagent-plugin.md` -> pass
  - `rg -n 'PluginToolRegistration|PluginHookRegistration|前置合同硬门禁|W5 不再把这两处当成 subagent SDK 的实现来源|SubagentContext\.tools: Arc<ToolRegistry>|HookDecl \{ id: String, point: HookPoint' docs/plans/sdk/08-week-5-subagent-plugin.md` -> pass
  - `rg -n 'PluginToolRegistration|PluginHookRegistration|HookPoint|SubagentContext|ToolRegistry|plugins_snapshot|SessionSnapshot' docs/plans/sdk/02-crate-topology.md` -> pass
  - `rg -n 'worker_boot.*W5 不作为|subagent_runtime.*TODO stub|greenfield SDK 覆盖职责边界|trust gate|prompt misdelivery' docs/plans/sdk/03-legacy-retirement.md` -> pass
  - `rg -n '08-week-5-subagent-plugin\.md|HookPoint|plugins_snapshot|greenfield|declaration|runtime' docs/sdk/README.md` -> pass
- Blockers:
  - none
- Next:
  - ready to report

## Checkpoint 2026-04-21 20:48

- Batch: Task 5 Step 1 -> Task 5 Step 3
- Completed:
  - Repaired the residual Level 0 leaks so the W5 plan/topology now treat `SubagentSpec.model_role` and `ModelProviderDecl.provider_ref` as opaque string keys, and `SubagentError::Provider` stays a contracts-local `reason: String`
  - Tightened W5 wording for `plugins_snapshot` fallback, bundled noop plugin layout, `worker_boot` W7 mirroring, and added R13 to keep `HookPoint` and `HookEvent` explicitly mapped
  - Synced `00-overview.md` exit-state wording with the repaired W5 plan and extended this execution plan to record the follow-up review batch
- Verification:
  - `! rg -n 'model_role: ModelRole|ModelProviderDecl \{ pub id: String, pub provider_id: ProviderId \}|Provider\(#\[from\] ModelError\)|bundled/example-noop-tool/src/lib.rs|HookRunner / ToolRegistry / SkillRegistry / ModelProvider|W5 保持 `pending`' docs/plans/sdk/08-week-5-subagent-plugin.md docs/plans/sdk/02-crate-topology.md docs/plans/sdk/00-overview.md` -> pass
  - `rg -n 'provider_ref: String|model_role: String|Provider \{ reason: String \}|session\.plugins_snapshot|src/bundled\.rs|R13|ToolRegistry / HookRunner.*executable runtime registration|SkillDecl / ModelProviderDecl.*metadata \+ builder slot' docs/plans/sdk/08-week-5-subagent-plugin.md docs/plans/sdk/02-crate-topology.md docs/plans/sdk/00-overview.md` -> pass
  - `git diff --check -- docs/plans/sdk/08-week-5-subagent-plugin.md docs/plans/sdk/02-crate-topology.md docs/plans/sdk/00-overview.md docs/plans/2026-04-21-sdk-w5-plan-audit-fixes.md` -> pass
- Blockers:
  - none
- Next:
  - ready to report

## Checkpoint 2026-04-21 21:26

- Batch: Task 6 Step 1 -> Task 6 Step 3
- Completed:
  - Split the W5 `plugins_snapshot` contract into an explicit two-branch shape across the weekly plan, topology, overview, and Fact-Fix row: preferred `SessionStarted` embedding plus fallback `session.plugins_snapshot`
  - Backfilled `02 §2.2` with `append_session_started(..., Option<PluginsSnapshot>)` and `new_child_session(...)`, removed the stale `octopus-sdk-model` dependency from Task 7, and aligned Task 10 / exit-state wording with the fallback branch
  - Replaced size-based 800-line guards in the W5 plan and global overview with real line-count checks
- Verification:
  - `rg -n 'session\.plugins_snapshot|SessionPluginsSnapshot|test_append_session_plugins_snapshot|test_snapshot_replay_second_event|SessionStarted.*Option<PluginsSnapshot>|SessionStarted 或紧随其后的 `session\.plugins_snapshot`' docs/plans/sdk/08-week-5-subagent-plugin.md docs/plans/sdk/02-crate-topology.md docs/plans/sdk/00-overview.md docs/sdk/README.md` -> pass
  - `rg -n 'append_session_started|new_child_session' docs/plans/sdk/02-crate-topology.md docs/plans/sdk/08-week-5-subagent-plugin.md` -> pass
  - `! rg -n 'octopus-sdk-contracts / -model / -tools / -hooks|find .* -size \+800' docs/plans/sdk/08-week-5-subagent-plugin.md docs/plans/sdk/00-overview.md` -> pass
  - `git diff --check -- docs/plans/sdk/08-week-5-subagent-plugin.md docs/plans/sdk/02-crate-topology.md docs/plans/sdk/00-overview.md docs/sdk/README.md docs/plans/2026-04-21-sdk-w5-plan-audit-fixes.md` -> pass
- Blockers:
  - none
- Next:
  - ready to report

## Checkpoint 2026-04-21 22:04

- Batch: Task 7 Step 1 -> Task 7 Step 3
- Completed:
  - Repaired the W5 `Architecture/expose` wording so `PluginRegistry::get_snapshot()` now feeds session-start persistence generically instead of implying embedded-first-event delivery only
  - Replaced the remaining shared size-based 800-line guards in `01-ai-execution-protocol.md` and `03-legacy-retirement.md` with the same `wc -l + awk` line-count check already used by the repaired W5 control docs
  - Synced the retirement Weekly Gate note and extended this execution plan to record the fourth audit batch
- Verification:
  - `rg -n 'session start 持久化输入|session\.plugins_snapshot 次事件载荷' docs/plans/sdk/08-week-5-subagent-plugin.md` -> pass
  - `! rg -n 'find .* -size \+800|find \+800' docs/plans/sdk/01-ai-execution-protocol.md docs/plans/sdk/03-legacy-retirement.md` -> pass
  - `git diff --check -- docs/plans/sdk/08-week-5-subagent-plugin.md docs/plans/sdk/01-ai-execution-protocol.md docs/plans/sdk/03-legacy-retirement.md docs/plans/2026-04-21-sdk-w5-plan-audit-fixes.md` -> pass
- Blockers:
  - none
- Next:
  - ready to report
