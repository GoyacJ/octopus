# Implementation Plan Template

> **For Codex:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task.

## Goal

One sentence describing the user-visible or architecture-visible outcome.

## Architecture

Two to three sentences describing the intended ownership boundary, main flow, and why this is the right layer.

## Scope

- In scope:
- Out of scope:

## Risks Or Open Questions

- List anything that must be confirmed before execution starts.

## Execution Rules

- Do not start implementation until each task has exact files, acceptance, verification, and stop conditions.
- Do not collapse shared-layer work into a business-page-local workaround.
- Stop when contract ownership, source of truth, or verification output is unclear.
- Execute in small batches and update status in place after each batch.

## Task Ledger

### Task 1: <short title>

Status: `pending`

Files:
- Modify: `path/to/file`
- Create: `path/to/new-file`
- Test: `path/to/test-file`

Preconditions:
- Dependency or decision that must already be true.

Step 1:
- Action: specific action
- Done when: observable acceptance condition
- Verify: exact command
- Stop if: reason to pause and ask

Step 2:
- Action: specific action
- Done when: observable acceptance condition
- Verify: exact command
- Stop if: reason to pause and ask

Notes:
- Keep only execution-relevant notes here.

### Task 2: <short title>

Status: `pending`

Files:
- Modify: `path/to/file`
- Test: `path/to/test-file`

Preconditions:
- Dependency or decision that must already be true.

Step 1:
- Action: specific action
- Done when: observable acceptance condition
- Verify: exact command
- Stop if: reason to pause and ask

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
