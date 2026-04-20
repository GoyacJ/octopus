# SDK W2 Plan Audit Fixes Implementation Plan

> **For Codex:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task.

## Goal

Repair the W2 SDK model weekly plan so its task contract, linked public-surface registry, and spec-drift handling are internally executable again.

## Architecture

This work stays in the documentation control plane. The main fix is to make `docs/plans/sdk/05-week-2-model.md` self-consistent with the current contracts and weekly-gate protocol, then backfill the linked registry and Fact-Fix docs that the weekly plan claims as its source of truth.

## Scope

- In scope:
- `docs/plans/sdk/05-week-2-model.md`
- `docs/plans/sdk/02-crate-topology.md`
- `docs/sdk/README.md`
- `docs/plans/2026-04-21-sdk-w2-plan-audit-fixes.md`
- Out of scope:
- Any crate or code changes under `crates/**`
- Rewriting `docs/sdk/11-model-system.md` normative text
- Changing W2 scope beyond the reviewed findings and their directly required registry/fact-fix backfills

## Risks Or Open Questions

- `docs/plans/sdk/05-week-2-model.md` is currently untracked in this worktree. The repair must preserve the existing draft content rather than recreating it from scratch.
- `docs/plans/sdk/README.md` already has user-visible uncommitted edits. This plan should not touch it unless verification proves it is required.
- If the accepted findings expose additional contradictions that would require changing `docs/sdk/11-model-system.md` body text, execution must stop and leave that for a dedicated spec-change plan.

## Execution Rules

- Do not edit files until each accepted finding is mapped to an exact file and acceptance condition.
- Keep fixes limited to closing the accepted findings plus the minimum linked registry/fact-fix edits they require.
- Preserve existing user changes in the worktree; do not revert unrelated diffs.
- Update task status in this plan after each batch.

## Task Ledger

### Task 1: Repair W2 task contract and weekly-gate wording

Status: `done`

Files:
- Modify: `docs/plans/sdk/05-week-2-model.md`

Preconditions:
- The six accepted review findings have been re-verified against the current W2 draft and linked docs.

Step 1:
- Action: Repair Task 3 and Task 6 so they no longer depend on nonexistent current-contract shapes, and so every newly introduced W2 public symbol is explicitly registered as a same-batch `02 §2.1 / §2.3` backfill requirement.
- Done when: Task 3 lists the missing `ResponseFormat` / `ThinkingConfig` / `CacheControlStrategy` / `ModelRequest` field backfills, and Task 6 no longer instructs implementers to emit `ToolUseStart / ToolUseDelta / ToolUseStop` against the current contracts without an explicit blocking rule.
- Verify: `rg -n 'ToolUseStart|ToolUseDelta|ToolUseStop|ResponseFormat|ThinkingConfig|CacheControlStrategy|max_tokens|temperature|stream' docs/plans/sdk/05-week-2-model.md`
- Stop if: closing the inconsistency would require changing current `crates/octopus-sdk-contracts` code instead of repairing the W2 plan wording.

Step 2:
- Action: Repair Task 4, Task 9, and the Exit State table so resolver selection, weekly-gate commands, and fallback test targets all match the actual documented public shape and gate protocol.
- Done when: the resolver rule no longer references `Model.status`, Task 9 includes the workspace-wide clippy gate plus the `00-overview.md §10` change-log update, and the Exit State table references an existing fallback test target.
- Verify: `rg -n 'status = Active|cargo clippy --workspace -- -D warnings|00-overview\\.md §10|fallback_trigger|--test fallback' docs/plans/sdk/05-week-2-model.md`
- Stop if: the weekly-gate fix would require relaxing `01-ai-execution-protocol.md` instead of aligning W2 to it.

Step 3:
- Action: Make the known `ModelRole` drift from `docs/sdk/11-model-system.md` explicit inside the W2 plan so the execution path points to a real Fact-Fix instead of a local comment.
- Done when: the W2 plan cites the existing or newly-added Fact-Fix entry for the temporary `rerank` exclusion and no longer treats it as an unregistered local exception.
- Verify: `rg -n 'rerank|Fact-Fix|docs/sdk/README\\.md' docs/plans/sdk/05-week-2-model.md`
- Stop if: documenting the drift reveals that W2 and `02-crate-topology.md` disagree on more than the single `rerank` omission.

### Task 2: Backfill linked registry and Fact-Fix sources

Status: `done`

Files:
- Modify: `docs/plans/sdk/02-crate-topology.md`
- Modify: `docs/sdk/README.md`

Preconditions:
- Task 1 is done so the W2 plan spells out the required linked-document changes precisely.

Step 1:
- Action: Expand `docs/plans/sdk/02-crate-topology.md §2.1 / §2.3` to include the W2 public-surface additions that the repaired plan now requires, including `ToolSchema` in contracts and the extra W2 model-layer types and fields.
- Done when: `02 §2.1 / §2.3` contains `ToolSchema`, `ResponseFormat`, `ThinkingConfig`, `CacheControlStrategy`, the extra `ModelRequest` fields, and the current `ProtocolAdapter`/fallback wording referenced by the W2 plan.
- Verify: `rg -n 'ToolSchema|ResponseFormat|ThinkingConfig|CacheControlStrategy|max_tokens|temperature|stream|auth_headers|complete_with_fallback' docs/plans/sdk/02-crate-topology.md`
- Stop if: backfilling the registry would require inventing symbols that the W2 plan does not actually expose.

Step 2:
- Action: Append a Fact-Fix entry in `docs/sdk/README.md` for the temporary W2/W0/02 decision to keep `ModelRole` at 10 public values and defer `rerank` beyond W2.
- Done when: `docs/sdk/README.md` has a new Fact-Fix row that names the affected `11-model-system.md §11.6.1` role set and states the temporary execution baseline clearly.
- Verify: `rg -n 'rerank|11-model-system\\.md §11\\.6\\.1|05-week-2-model\\.md|02-crate-topology\\.md' docs/sdk/README.md`
- Stop if: the drift cannot be described as a temporary execution-layer exception and instead requires changing the normative role set now.

### Task 3: Verify and checkpoint

Status: `done`

Files:
- Modify: `docs/plans/2026-04-21-sdk-w2-plan-audit-fixes.md`

Preconditions:
- Tasks 1-2 are done.

Step 1:
- Action: Run targeted verification against the repaired files and update this plan with final statuses plus a checkpoint.
- Done when: each accepted finding is closed by a concrete doc change and the verification commands return the expected matches with no stale references left behind.
- Verify: `rg -n 'ToolUseStart|ToolUseDelta|ToolUseStop|status = Active|fallback_trigger' docs/plans/sdk/05-week-2-model.md && rg -n 'ToolSchema|ResponseFormat|ThinkingConfig|CacheControlStrategy|max_tokens|temperature|stream|auth_headers|complete_with_fallback' docs/plans/sdk/02-crate-topology.md && rg -n 'rerank|11-model-system\\.md §11\\.6\\.1' docs/sdk/README.md`
- Stop if: verification exposes a new contradiction outside the planned file set.

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

## Checkpoint 2026-04-21 02:17

- Batch: Task 1 Step 1 -> Task 3 Step 1
- Completed: repaired `docs/plans/sdk/05-week-2-model.md` for current `AssistantEvent` baseline, public-surface backfill obligations, resolver/weekly-gate/fallback-test wording, and explicit `ModelRole` Fact-Fix linkage; backfilled `docs/plans/sdk/02-crate-topology.md §2.1 / §2.3`; appended the `rerank` Fact-Fix in `docs/sdk/README.md`; finalized this control plan status.
- Verification:
  - `git diff --check -- docs/plans/sdk/05-week-2-model.md docs/plans/sdk/02-crate-topology.md docs/sdk/README.md docs/plans/2026-04-21-sdk-w2-plan-audit-fixes.md` -> pass
  - `rg -n 'ToolUseStart|ToolUseDelta|ToolUseStop|AssistantEvent::Unknown|--test fallback_trigger' docs/plans/sdk/05-week-2-model.md` -> pass (no stale references)
  - `rg -n 'ToolUseStart|ToolUseDelta|ToolUseStop|AssistantEvent::Unknown|ResponseFormat|ThinkingConfig|CacheControlStrategy|max_tokens|temperature|stream|status = Active|Provider.status = Active|cargo clippy --workspace -- -D warnings|00-overview\\.md §10|--test fallback|fallback_trigger|rerank|Fact-Fix' docs/plans/sdk/05-week-2-model.md docs/plans/sdk/02-crate-topology.md docs/sdk/README.md` -> pass
- Blockers:
  - none
- Next:
  - ready to report
