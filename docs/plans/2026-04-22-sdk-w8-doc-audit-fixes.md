# SDK W8 Doc Audit Fixes Implementation Plan

> **For Codex:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task.

## Goal

Repair the `docs/plans/sdk` control documents so W8 planning, ownership wording, and verification gates are internally consistent with each other and with the current repository state.

## Architecture

This plan stays in the documentation control plane. It updates the W8 weekly plan plus the W0 control docs that define ownership and workspace policy, and it does not change `docs/sdk/*` unless a spec contradiction is proven and must be routed through Fact-Fix.

## Scope

- In scope:
- `docs/plans/sdk/11-week-8-cleanup-and-split.md`
- `docs/plans/sdk/00-overview.md`
- `docs/plans/sdk/02-crate-topology.md`
- `docs/plans/sdk/README.md`
- `docs/plans/2026-04-22-sdk-w8-doc-audit-fixes.md`
- Out of scope:
- Any Rust, TypeScript, Cargo, OpenAPI, or schema implementation changes
- `docs/sdk/*` normative spec edits unless a separate Fact-Fix is proven necessary
- Rewriting W8 execution scope beyond fixing documented control-plane contradictions

## Risks Or Open Questions

- `00-overview.md` and `02-crate-topology.md` currently disagree on whether `octopus-sdk-session` routes through `octopus-persistence`; the audit repair must collapse that to one control-plane answer without inventing new runtime behavior.
- `00-overview.md` still hardcodes a W7/W8 `default-members` target that does not match the live `Cargo.toml`; the repair must either point to the canonical workspace section in `02-crate-topology.md` or restate the live policy consistently.
- W8 verification commands must distinguish production-path `Connection::open` cleanup from allowed temporary test-path usage; otherwise the plan becomes self-contradictory.

## Execution Rules

- Do not edit docs before each fix is tied to an exact file and acceptance condition in the task ledger.
- Keep fixes at the control-document layer; do not let this audit drift into code-planning expansion or `docs/sdk/*` rewrites.
- Update task status in this file after each execution batch.
- Stop if any repair requires changing canonical spec behavior instead of correcting `docs/plans/sdk/*` control wording.

## Task Ledger

### Task 1: Repair W8 plan structure and verification wording

Status: `done`

Files:
- Modify: `docs/plans/sdk/11-week-8-cleanup-and-split.md`

Preconditions:
- `docs/plans/sdk/AGENTS.md` remains the local format authority for weekly SDK plans.

Step 1:
- Action: Add the missing W8 control sections required by `docs/plans/sdk/AGENTS.md`, including explicit `Non-goal`, public-surface change registration, and retirement registration.
- Done when: the W8 document exposes the required plan structure without forcing readers to infer those sections from surrounding prose.
- Verify: `rg -n '^## (Goal|Non-goal|Scope|公共面变更登记|退役登记|Weekly Gate 对齐表（W8）|变更日志)$' docs/plans/sdk/11-week-8-cleanup-and-split.md`
- Stop if: the required sections would conflict with an already-canonical weekly-plan format in `docs/plans/sdk/`.

Step 2:
- Action: Rewrite the W8 `Connection::open` verification commands so they explicitly target production paths and do not count known test-only files such as `split_module_tests.rs` or `test_runtime_sdk.rs`.
- Done when: the W8 weekly gate and task verifies no longer contradict the stated rule that test paths may temporarily retain direct opens.
- Verify: `rg -n 'Connection::open|split_module_tests|test_runtime_sdk|tests/\*\*' docs/plans/sdk/11-week-8-cleanup-and-split.md`
- Stop if: the production-vs-test boundary cannot be expressed with repository path filters.

### Task 2: Align W0 control-plane ownership and workspace wording

Status: `done`

Files:
- Modify: `docs/plans/sdk/00-overview.md`
- Modify: `docs/plans/sdk/02-crate-topology.md`

Preconditions:
- Task 1 is done so W8 references the same ownership vocabulary the W0 docs will expose.

Step 1:
- Action: Align `00-overview.md` with `02-crate-topology.md` on the `octopus-persistence` boundary so `octopus-sdk-session` is no longer documented as routing through the business-side persistence crate.
- Done when: `00` and `02` present one control-plane answer for `SqliteJsonlSessionStore` ownership.
- Verify: `rg -n 'octopus-persistence|octopus-sdk-session|SqliteJsonlSessionStore' docs/plans/sdk/00-overview.md docs/plans/sdk/02-crate-topology.md`
- Stop if: the two documents describe materially different intended architectures that cannot be reconciled without changing the normative SDK spec.

Step 2:
- Action: Remove or reframe the stale hardcoded `default-members = 5 业务 crate + Tauri app` wording in `00-overview.md` so it matches the live `Cargo.toml` policy and the canonical workspace section in `02-crate-topology.md`.
- Done when: the W0 docs no longer contradict the current workspace `default-members` list.
- Verify: `rg -n 'default-members|5 业务 crate|Tauri app' docs/plans/sdk/00-overview.md docs/plans/sdk/02-crate-topology.md Cargo.toml`
- Stop if: the correct workspace policy cannot be stated without changing `Cargo.toml` itself.

### Task 3: Refresh index wording and verify the repaired document set

Status: `done`

Files:
- Modify: `docs/plans/sdk/README.md`
- Modify: `docs/plans/2026-04-22-sdk-w8-doc-audit-fixes.md`

Preconditions:
- Tasks 1-2 are done.

Step 1:
- Action: Update README wording only where needed so the index and last-update note accurately describe the W8 plan state after the audit repair.
- Done when: the README summary matches the repaired W8 control document without adding new policy text that belongs in `00/02`.
- Verify: `rg -n '11-week-8-cleanup-and-split|最后更新' docs/plans/sdk/README.md`
- Stop if: README updates would need to carry canonical rules instead of index-level status text.

Step 2:
- Action: Run targeted consistency scans, update this plan to `done`, and append a checkpoint with verification evidence.
- Done when: the repaired files pass the planned scans and this plan records the finished execution state.
- Verify: `rg -n '5 业务 crate|default-members|octopus-persistence|SqliteJsonlSessionStore|Connection::open' docs/plans/sdk/{00-overview.md,02-crate-topology.md,11-week-8-cleanup-and-split.md,README.md} Cargo.toml`
- Stop if: the scans reveal another unresolved control-plane contradiction outside this plan's scoped files.

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

## Checkpoint 2026-04-22 15:21

- Batch: Task 1 Step 1 -> Task 3 Step 2
- Completed:
  - Repaired `11-week-8-cleanup-and-split.md` so it now includes explicit `Non-goal` / public-surface registration / retirement registration sections
  - Aligned `00-overview.md` and `02-crate-topology.md` on the `octopus-persistence` boundary and on the live `default-members` policy
  - Refreshed `README.md` summary text to match the repaired W8 control document
- Verification:
  - `rg -n '^## (Goal|Non-goal|Scope|公共面变更登记|退役登记|Weekly Gate 对齐表（W8）|变更日志)$' docs/plans/sdk/11-week-8-cleanup-and-split.md` -> pass
  - `rg -n 'Connection::open|split_module_tests|test_runtime_sdk|tests/\*\*' docs/plans/sdk/11-week-8-cleanup-and-split.md` -> pass
  - `rg -n 'octopus-persistence|octopus-sdk-session|SqliteJsonlSessionStore' docs/plans/sdk/00-overview.md docs/plans/sdk/02-crate-topology.md` -> pass
  - `rg -n 'default-members|5 业务 crate|Tauri app' docs/plans/sdk/00-overview.md docs/plans/sdk/02-crate-topology.md Cargo.toml` -> pass
  - `find docs/plans/sdk -maxdepth 1 -type f -name '[0-9][0-9]-*.md' | sort` + `rg '^\| \`[0-9]{2}-' docs/plans/sdk/README.md` -> pass
  - `git diff --check` -> pass
- Blockers:
  - none
- Next:
  - ready to report the repaired control-plane documents
