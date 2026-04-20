# SDK Control Doc Audit Fixes Implementation Plan

> **For Codex:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task.

## Goal

Repair the `docs/plans/sdk` control-plane documents so the audit findings are closed and the SDK refactor docs are internally executable again.

## Architecture

This plan only changes documentation under `docs/plans/sdk/` plus this control document. The work stays at the control-plane layer: fix broken section references, make the guard scans executable against the indexed file model, and align the W1 session JSONL path decision with the existing SDK spec surface.

## Scope

- In scope:
- `docs/plans/sdk/AGENTS.md`
- `docs/plans/sdk/README.md`
- `docs/plans/sdk/04-week-1-contracts-session.md`
- `docs/plans/2026-04-21-sdk-control-doc-audit-fixes.md`
- Out of scope:
- Any code or crate changes
- `docs/sdk/01-14` normative spec edits
- New weekly SDK plans beyond the existing `docs/plans/sdk/04-week-1-contracts-session.md`

## Risks Or Open Questions

- The current guard-scan wording conflates indexed plan rows with on-disk files. The fix must preserve the pre-registration model for future weeks instead of forcing placeholder files into existence.
- The W1 JSONL path needs one canonical shape. The repair should align with the existing SDK docs that already speak in terms of per-session JSONL event files.

## Execution Rules

- Do not edit docs before the affected file set and acceptance criteria are listed in the task.
- Keep changes limited to the audited problems and immediately adjacent broken references discovered while fixing them.
- Update task status in this file after each batch.
- Stop if any fix would require changing `docs/sdk/*` normative behavior instead of repairing the `docs/plans/sdk/*` control layer.

## Task Ledger

### Task 1: Repair control-plane governance rules

Status: `done`

Files:
- Modify: `docs/plans/sdk/AGENTS.md`
- Modify: `docs/plans/sdk/README.md`

Preconditions:
- Audit findings are limited to the `docs/plans/sdk` control plane and do not require `docs/sdk/*` spec changes.

Step 1:
- Action: Fix broken section references in `docs/plans/sdk/AGENTS.md` so exit-state, weekly-gate, and stop-condition references point at the actual sections in `00-overview.md` and `01-ai-execution-protocol.md`.
- Done when: no `docs/plans/sdk/AGENTS.md` references remain that point weekly exit state at `00-overview.md §4` or stop conditions at `01-ai-execution-protocol.md §4`.
- Verify: `rg -n '00-overview\\.md §4|01-ai-execution-protocol\\.md §4' docs/plans/sdk/AGENTS.md docs/plans/sdk/04-week-1-contracts-session.md`
- Stop if: fixing the references reveals the target sections themselves are missing or renamed.

Step 2:
- Action: Rewrite the `§5` guard-scan rules in `docs/plans/sdk/AGENTS.md` so they explicitly exclude `README.md` and `AGENTS.md`, allow `pending` rows in the index without backing files, and require every existing numbered plan file to be indexed plus every non-`pending` indexed row to have a backing file.
- Done when: the scan wording matches the actual indexed-plan model and no longer describes an impossible one-to-one equality between all `.md` files and all index rows.
- Verify: `rg -n 'README\\.md|AGENTS\\.md|pending' docs/plans/sdk/AGENTS.md`
- Stop if: the guard model cannot be expressed without changing the README index semantics.

Step 3:
- Action: Resolve the W0 mutability conflict by clarifying in `docs/plans/sdk/AGENTS.md` and `docs/plans/sdk/README.md` that `docs/sdk/*` normative spec fixes still flow through `docs/sdk/README.md` Fact-Fix, while `00/01/02/03` remain living control docs that may be amended with change logs when execution reveals control-plane gaps.
- Done when: the local rules no longer say both "must update `02-crate-topology.md`" and "W0 docs must not be rewritten."
- Verify: `rg -n 'Fact-Fix|00 / 01 / 02 / 03|02-crate-topology\\.md' docs/plans/sdk/AGENTS.md docs/plans/sdk/README.md`
- Stop if: this requires redefining the authority boundary between `docs/sdk/*` and `docs/plans/sdk/*`.

### Task 2: Repair W1 control-document references and registry usage

Status: `done`

Files:
- Modify: `docs/plans/sdk/04-week-1-contracts-session.md`

Preconditions:
- Task 1 is done so the directory-level control rules are stable.

Step 1:
- Action: Fix W1 references to `02-crate-topology.md` so contract discrepancy references point to `§5`, UI Intent IR registry references point to `§6`, and nearby broken shorthand references to `00-overview.md` use the actual section names.
- Done when: `04-week-1-contracts-session.md` no longer refers to `02 §4` for the discrepancy registry or `02 §5` for the UI intent registry.
- Verify: `rg -n '§4 契约差异清单|02 §4|02 §5 UI Intent IR|00 §W1|00-overview\\.md §W1' docs/plans/sdk/04-week-1-contracts-session.md`
- Stop if: any corrected reference depends on a section that does not exist in the target file.

Step 2:
- Action: Update the W1 Task 9 discrepancy-registry instructions so they append rows to the canonical `02-crate-topology.md §5` table instead of introducing a second ad hoc table schema.
- Done when: Task 9 Step 2 describes using the existing registry columns and no longer asks for a new `| SDK 类型 | OpenAPI 类型 | 差异 | 处理方针 | 决议周次 |` table.
- Verify: `rg -n 'SDK 类型 \\| OpenAPI 类型|§5 契约差异清单|# \\| 日期 \\| 来源 \\| 目标' docs/plans/sdk/04-week-1-contracts-session.md docs/plans/sdk/02-crate-topology.md`
- Stop if: the canonical registry itself is insufficient to record the W1 discrepancies.

### Task 3: Repair W1 JSONL path decision

Status: `done`

Files:
- Modify: `docs/plans/sdk/04-week-1-contracts-session.md`

Preconditions:
- Task 2 is done so the W1 document uses the correct registry and section vocabulary.

Step 1:
- Action: Replace the conflicting W1 JSONL sharding language with one canonical per-session path that matches the broader SDK docs, and keep the "no rotation in W1" decision explicit.
- Done when: `R4`, Task 7 Step 2, and Task 7 notes all describe the same per-session JSONL path and no run-based sharding remains in the W1 plan.
- Verify: `rg -n 'run_id|events\\.jsonl|runtime/events/<session>|session_id' docs/plans/sdk/04-week-1-contracts-session.md`
- Stop if: aligning the path would conflict with a stronger persistence rule elsewhere in the repository.

### Task 4: Verify and checkpoint

Status: `done`

Files:
- Modify: `docs/plans/2026-04-21-sdk-control-doc-audit-fixes.md`

Preconditions:
- Tasks 1-3 are done.

Step 1:
- Action: Run the targeted verification commands for the repaired files and confirm the audit findings are closed without introducing new broken references.
- Done when: all verification commands exit successfully and the plan file is updated with final task statuses plus a checkpoint.
- Verify: `tmp_existing=$(mktemp); tmp_indexed=$(mktemp); tmp_required=$(mktemp); find docs/plans/sdk -maxdepth 1 -type f -name '[0-9][0-9]-*.md' -exec basename {} \\; | sort > "$tmp_existing"; awk -F'\\`' '/^\\| \\`[0-9]{2}-/ {print $2}' docs/plans/sdk/README.md | sort > "$tmp_indexed"; awk -F'\\`' '/^\\| \\`[0-9]{2}-/ && $0 !~ /\\`pending\\`/ {print $2}' docs/plans/sdk/README.md | sort > "$tmp_required"; comm -23 "$tmp_existing" "$tmp_indexed"; comm -13 "$tmp_existing" "$tmp_required"`
- Stop if: any verification command still shows unresolved broken references or contradictory path decisions.

## Batch Checkpoint Format

After each batch, append a short checkpoint using this shape:

```md
## Checkpoint YYYY-MM-DD HH:MM

- Batch: Task 1 Step 1 -> Task 2 Step 2
- Completed: short list
- Verification:
  - `command` -> pass or fail
- Blockers:
  - none
- Next:
  - Task 3 Step 1
```

## Checkpoint 2026-04-21 00:15

- Batch: Task 1 Step 1 -> Task 4 Step 1
- Completed:
  - Repaired `docs/plans/sdk/AGENTS.md` section references, W0 mutability rules, and executable guard-scan semantics
  - Updated `docs/plans/sdk/README.md` to reflect pre-registered `pending` rows and living W0 control docs
  - Repaired `docs/plans/sdk/04-week-1-contracts-session.md` registry section references, canonical discrepancy-table usage, and per-session JSONL path decision
- Verification:
  - `tmp_existing=$(mktemp); tmp_indexed=$(mktemp); tmp_required=$(mktemp); find docs/plans/sdk -maxdepth 1 -type f -name '[0-9][0-9]-*.md' -exec basename {} \; | sort > "$tmp_existing"; awk -F'`' '/^\| `[0-9]{2}-/ {print $2}' docs/plans/sdk/README.md | sort > "$tmp_indexed"; awk -F'`' '/^\| `[0-9]{2}-/ && $0 !~ /`pending`/ {print $2}' docs/plans/sdk/README.md | sort > "$tmp_required"; printf 'existing_not_indexed:\n'; comm -23 "$tmp_existing" "$tmp_indexed"; printf 'required_missing_file:\n'; comm -13 "$tmp_existing" "$tmp_required"` -> pass (both sections empty)
  - `rg -n '§4 契约差异清单|02 §4|02 §5 UI Intent IR|00 §W1|00-overview\.md §W1|run_id|<jsonl_root>/<session_id>/events\.jsonl|SDK 类型 \| OpenAPI 类型' docs/plans/sdk/04-week-1-contracts-session.md docs/plans/sdk/AGENTS.md` -> pass (no matches)
  - `git diff --check` -> pass
- Blockers:
  - none
- Next:
  - ready to report the repaired control-plane docs and verification evidence
