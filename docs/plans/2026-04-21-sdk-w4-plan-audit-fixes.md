# SDK W4 Plan Audit Fixes Implementation Plan

> **For Codex:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task.

## Goal

Repair the W4 SDK weekly plan so its public-surface targets, task ledger, and spec-drift handling are internally executable again.

## Architecture

This work stays in the documentation control plane. The main fix is to make `docs/plans/sdk/07-week-4-permissions-hooks-sandbox-context.md` consistent with the current contracts and control docs, then backfill the linked topology registry and spec-drift log that the weekly plan claims as its source of truth.

## Scope

- In scope:
- `docs/plans/sdk/07-week-4-permissions-hooks-sandbox-context.md`
- `docs/plans/sdk/02-crate-topology.md`
- `docs/sdk/README.md`
- `docs/plans/2026-04-21-sdk-w4-plan-audit-fixes.md`
- Out of scope:
- Any crate or code changes under `crates/**`
- Rewriting `docs/sdk/06-permissions-sandbox.md` or `docs/sdk/07-hooks-lifecycle.md` body text
- Changing W4 scope beyond the accepted audit findings and their directly required registry/fact-fix backfills

## Risks Or Open Questions

- `docs/plans/sdk/07-week-4-permissions-hooks-sandbox-context.md` is currently untracked in this worktree. The repair must preserve the existing draft content rather than recreating it from scratch.
- `docs/plans/sdk/README.md` and `docs/plans/sdk/00-overview.md` already have user-visible uncommitted edits. This plan should not touch them unless verification proves it is required.
- If closing the W4 contradictions requires changing `docs/sdk/06` or `docs/sdk/07` normative text instead of registering the drift in `docs/sdk/README.md`, execution must stop and leave that for a dedicated spec-change plan.

## Execution Rules

- Do not edit files until each accepted finding is mapped to an exact file and acceptance condition.
- Keep fixes limited to closing the accepted findings plus the minimum linked registry/fact-fix edits they require.
- Preserve existing user changes in the worktree; do not revert unrelated diffs.
- Update task status in this plan after each batch.

## Task Ledger

### Task 1: Repair W4 task contract and task-ledger wording

Status: `done`

Files:
- Modify: `docs/plans/sdk/07-week-4-permissions-hooks-sandbox-context.md`

Preconditions:
- The accepted audit findings have been re-verified against the current W4 draft and current contracts.

Step 1:
- Action: Repair the permissions and approval path so `PermissionGate`, `PermissionPolicy::evaluate`, and `ApprovalBroker` all target one coherent public contract with current `SessionEvent`, `AskPrompt`, and `AskAnswer` shapes.
- Done when: the W4 plan no longer references `PermissionOutcome::Continue`, `SessionEvent::RenderBlockEmitted`, `prompt.id`, or `AskAnswer.selected_option_id`, and the `PermissionGate` signature is aligned with the `ToolCategory` dependency it claims to need.
- Verify: `rg -n 'RenderBlockEmitted|selected_option_id|prompt\\.id|PermissionOutcome::Continue|未命中返回 `Continue`' docs/plans/sdk/07-week-4-permissions-hooks-sandbox-context.md && rg -n 'category_resolver|evaluate\\(ctx: &PermissionContext\\)' docs/plans/sdk/07-week-4-permissions-hooks-sandbox-context.md`
- Stop if: closing the inconsistency would require changing current `crates/octopus-sdk-contracts` code instead of repairing the W4 plan wording.

Step 2:
- Action: Repair the context/compaction and prompt-builder sections so public signatures, file lists, strategy behavior, and stability rules are self-consistent.
- Done when: `CompactionResult` has one field set throughout the plan, `maybe_compact` has one error model throughout the plan, Task 9 includes `compact.rs`, and tool-guidance generation consistently uses the deterministic registry path.
- Verify: `rg -n 'CompactionResult|maybe_compact|schemas_sorted\\(\\)|registry\\.iter\\(|compact\\.rs|read_back_to_message' docs/plans/sdk/07-week-4-permissions-hooks-sandbox-context.md`
- Stop if: the repaired signatures would force a W4 scope change beyond prompt/context internals.

Step 3:
- Action: Repair the credential-leak and hook-test wording so the hard gate uses current event names and explicit safe-summary rules instead of nonexistent redaction APIs or hook errors.
- Done when: the W4 plan no longer depends on `SecretVault::redact_for_event`, `SessionEvent::ToolInvoked`, or undeclared hook errors, and the approval/event tests state which current `SessionEvent` variants are allowed to carry approval or tool-execution data.
- Verify: `rg -n 'redact_for_event|ToolInvoked|CredentialLeak' docs/plans/sdk/07-week-4-permissions-hooks-sandbox-context.md && rg -n 'SessionEvent::Ask|SessionEvent::ToolExecuted|InjectNotAllowed' docs/plans/sdk/07-week-4-permissions-hooks-sandbox-context.md`
- Stop if: the hard gate cannot be expressed without adding a new Level 0 secret-redaction contract.

### Task 2: Backfill linked topology and Fact-Fix sources

Status: `done`

Files:
- Modify: `docs/plans/sdk/02-crate-topology.md`
- Modify: `docs/sdk/README.md`

Preconditions:
- Task 1 is done so the W4 plan spells out the required linked-document changes precisely.

Step 1:
- Action: Update `docs/plans/sdk/02-crate-topology.md §2.6 / §2.7 / §2.9` so the public-surface placeholders match the repaired W4 plan for `PermissionContext`, `PermissionGate`, `ApprovalBroker`, `Compactor`, and the W4 hook subset.
- Done when: `02-crate-topology.md` documents the repaired W4 placeholders for internal `PermissionContext`, `PermissionGate::check(&ToolCallRequest)`, `maybe_compact -> Result<Option<_>, _>`, and the repaired hook decision surface.
- Verify: `rg -n 'evaluate\\(&self, ctx: &PermissionContext\\)|check\\(&self, call: &ToolCallRequest\\)|maybe_compact\\(&self, session: &mut SessionView\\)' docs/plans/sdk/02-crate-topology.md && rg -n 'Rewrite\\(RewritePayload\\)|InjectNotAllowed|PreToolUse \\{ call: ToolCallRequest, category: ToolCategory \\}|PreCompact \\{ session: SessionId, ctx: CompactionCtx \\}' docs/plans/sdk/02-crate-topology.md`
- Stop if: the topology backfill would require inventing symbols that the repaired W4 plan does not actually expose.

Step 2:
- Action: Append a Fact-Fix entry in `docs/sdk/README.md` for the W4 execution baseline that freezes only the 8 hook events in the weekly plan and defers the rest of `docs/sdk/07` to later weeks.
- Done when: `docs/sdk/README.md` has a new Fact-Fix row that names the affected `07-hooks-lifecycle.md` lifecycle table and states the temporary W4 execution subset clearly.
- Verify: `rg -n '07-hooks-lifecycle\\.md|PreSampling|PostSampling|SubagentSpawn|OnToolError|PreFileWrite|PostFileWrite|07-week-4-permissions-hooks-sandbox-context\\.md' docs/sdk/README.md`
- Stop if: the drift cannot be described as a temporary W4 execution subset and instead requires rewriting the normative hook chapter now.

### Task 3: Verify and checkpoint

Status: `done`

Files:
- Modify: `docs/plans/2026-04-21-sdk-w4-plan-audit-fixes.md`

Preconditions:
- Tasks 1-2 are done.

Step 1:
- Action: Run targeted verification against the repaired files and update this plan with final statuses plus a checkpoint.
- Done when: each accepted finding is closed by a concrete doc change and the verification commands return the expected matches with no stale references left behind.
- Verify: `git diff --check -- docs/plans/sdk/07-week-4-permissions-hooks-sandbox-context.md docs/plans/sdk/02-crate-topology.md docs/sdk/README.md docs/plans/2026-04-21-sdk-w4-plan-audit-fixes.md && rg -n 'RenderBlockEmitted|selected_option_id|prompt\\.id|PermissionOutcome::Continue|ToolInvoked|redact_for_event|CredentialLeak|registry\\.iter\\(|read_back_to_message' docs/plans/sdk/07-week-4-permissions-hooks-sandbox-context.md && rg -n 'evaluate\\(&self, ctx: &PermissionContext\\)|check\\(&self, call: &ToolCallRequest\\)|maybe_compact\\(&self, session: &mut SessionView\\)|InjectNotAllowed' docs/plans/sdk/02-crate-topology.md && rg -n '07-hooks-lifecycle\\.md|PreSampling|PostSampling|SubagentSpawn|OnToolError|PreFileWrite|PostFileWrite|07-week-4-permissions-hooks-sandbox-context\\.md' docs/sdk/README.md`
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

## Checkpoint 2026-04-21 17:18

- Batch: Task 1 Step 1 -> Task 3 Step 1
- Completed:
  - Repaired `docs/plans/sdk/07-week-4-permissions-hooks-sandbox-context.md` for permission-policy fallthrough semantics, approval/event contracts, compactor signatures, prompt-builder stability wording, credential hard-gate wording, and explicit UI/runtime permission-mode migration handling
  - Backfilled `docs/plans/sdk/02-crate-topology.md` W4 placeholders for `SystemPromptBuilder`, `Compactor`, `PermissionPolicy`, `PermissionGate`, `ApprovalBroker`, and the 8-event hook subset
  - Appended the W4 hook-subset Fact-Fix in `docs/sdk/README.md`
- Verification:
  - `git diff --check -- docs/plans/sdk/07-week-4-permissions-hooks-sandbox-context.md docs/plans/sdk/02-crate-topology.md docs/sdk/README.md docs/plans/2026-04-21-sdk-w4-plan-audit-fixes.md` -> pass
  - `rg -n 'RenderBlockEmitted|selected_option_id|prompt\.id|PermissionOutcome::Continue|ToolInvoked|redact_for_event|CredentialLeak|registry\.iter\(|read_back_to_message' docs/plans/sdk/07-week-4-permissions-hooks-sandbox-context.md` -> pass (no matches)
  - `rg -n 'evaluate\(&self, ctx: &PermissionContext\)|check\(&self, call: &ToolCallRequest\)|maybe_compact\(&self, session: &mut SessionView\)|InjectNotAllowed|PreToolUse \{ call: ToolCallRequest, category: ToolCategory \}|PreCompact \{ session: SessionId, ctx: CompactionCtx \}' docs/plans/sdk/02-crate-topology.md` -> pass
  - `rg -n '07-hooks-lifecycle\.md|PreSampling|PostSampling|SubagentSpawn|OnToolError|PreFileWrite|PostFileWrite|07-week-4-permissions-hooks-sandbox-context\.md' docs/sdk/README.md` -> pass
- Blockers:
  - none
- Next:
  - ready to report
