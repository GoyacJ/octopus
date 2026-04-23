# Session Store Startup Compat Plan

## Goal

Make desktop dev startup tolerate both legacy `runtime/events/*.jsonl` session files that were written without outer `event_id` and shared-DB table-name collisions inside `data/main.db`, so `pnpm dev:desktop` can boot through `SqliteJsonlSessionStore::open()`.

## Architecture

This repair belongs in `crates/octopus-sdk-session`, because both failures happen inside the session store while `open()` initializes the SQLite projection and replays append-only JSONL. The fix should keep current write format and caller behavior unchanged, while making session-store persistence coexist with other SQLite tables in the shared workspace database.

## Scope

- In scope:
  - Backward-compatible parsing for legacy JSONL session event lines under `crates/octopus-sdk-session/src/jsonl.rs`
  - Namespaced SQLite session-store tables and guarded migration from legacy generic `sessions/events` tables under `crates/octopus-sdk-session/src/sqlite/*.rs`
  - Regression coverage in `crates/octopus-sdk-session/tests/sqlite_jsonl.rs`
  - Targeted verification of the session-store tests and desktop startup path
- Out of scope:
  - Rewriting legacy `runtime/sessions/*.json` debug artifacts
  - Changing current JSONL write format
  - Broad runtime/session persistence redesign

## Risks Or Open Questions

- Legacy JSONL lines are older runtime event envelopes, not current `JsonlRecord` shape. The fallback parser must preserve stable ordering and avoid generating duplicate IDs within one session replay.
- If any legacy file is not parseable into the current `SessionEvent` domain at all, this repair should stop at an explicit corruption error instead of guessing a lossy mapping.
- `data/main.db` is shared with `octopus-infra` auth/session persistence. Session-store schema migration must not rename or mutate auth-owned `sessions` tables that do not contain `session_id`.

## Execution Rules

- Do not start implementation until each task has exact files, acceptance, verification, and stop conditions.
- Do not collapse shared-layer work into a desktop-local workaround.
- Stop when contract ownership, source of truth, or verification output is unclear.
- Execute in small batches and update status in place after each batch.

## Task Ledger

### Task 1: Define legacy JSONL compatibility path

Status: `done`

Files:
- Create: `docs/plans/runtime/2026-04-23-session-jsonl-compat.md`
- Modify: `crates/octopus-sdk-session/src/jsonl.rs`
- Modify: `crates/octopus-sdk-session/src/sqlite/stream.rs`

Preconditions:
- The startup crash is reproducible from current repository-root `runtime/events/*.jsonl` files.
- `SqliteJsonlSessionStore::open()` remains the single entry point for JSONL-to-SQLite reconciliation.

Step 1:
- Action: Document the ownership boundary and current failure mode for legacy JSONL replay in this plan.
- Done when: the plan states that the fix lives in `octopus-sdk-session` and targets `read_records()` compatibility.
- Verify: `test -f docs/plans/runtime/2026-04-23-session-jsonl-compat.md`
- Stop if: a more specific active execution plan already owns this exact regression.

Step 2:
- Action: Update JSONL loading so `read_records()` accepts both current `{ event_id, event }` records and legacy lines without outer `event_id`, producing deterministic `JsonlRecord.event_id` values for replay.
- Done when: reopening a legacy JSONL session no longer fails on missing `event_id`, and projection repair still receives a full `Vec<JsonlRecord>`.
- Verify: `cargo test -p octopus-sdk-session --test sqlite_jsonl test_open_repairs_db_projection_from_jsonl_tail`
- Stop if: legacy lines cannot be mapped to `SessionEvent` without inventing unsupported semantics.

### Task 2: Add regression coverage for legacy reopen

Status: `done`

Files:
- Modify: `crates/octopus-sdk-session/tests/sqlite_jsonl.rs`

Preconditions:
- Task 1 Step 2 is implemented locally.

Step 1:
- Action: Add a focused test fixture that writes a legacy JSONL line without outer `event_id`, reopens the store, and asserts replay/projection succeed.
- Done when: the new test fails on old code and passes with the compatibility logic, without depending on repository-local runtime artifacts.
- Verify: `cargo test -p octopus-sdk-session --test sqlite_jsonl`
- Stop if: the test needs unrelated runtime subsystems or networked providers to run.

### Task 3: Re-verify desktop startup path

Status: `done`

Files:
- Modify: `docs/plans/runtime/2026-04-23-session-jsonl-compat.md`

Preconditions:
- Task 1 and Task 2 are `done`.

Step 1:
- Action: Run the targeted Rust tests and retry `pnpm dev:desktop` far enough to confirm the missing-`event_id` crash is gone.
- Done when: test output is green and desktop startup no longer prints `session serde error: missing field \`event_id\``.
- Verify: `cargo test -p octopus-sdk-session --test sqlite_jsonl && pnpm dev:desktop`
- Stop if: a different startup failure replaces the serde error and requires a separate fix.

### Task 4: Namespace SQLite session-store tables

Status: `done`

Files:
- Modify: `crates/octopus-sdk-session/src/sqlite/schema.rs`
- Modify: `crates/octopus-sdk-session/src/sqlite/append.rs`
- Modify: `crates/octopus-sdk-session/src/sqlite/stream.rs`

Preconditions:
- Task 1 and Task 2 are `done`.
- The new startup failure is `session sqlite error: no such column: session_id in SELECT 1 FROM sessions WHERE session_id = ?1`.

Step 1:
- Action: Change session-store schema ownership from generic `sessions/events` to namespaced runtime tables, and only migrate legacy generic tables when the legacy `sessions` table is session-store-owned.
- Done when: `SqliteJsonlSessionStore::open()` creates or migrates `runtime_session_store_sessions` and `runtime_session_store_events` without mutating auth-owned `sessions`.
- Verify: `cargo test -p octopus-sdk-session sqlite::schema::`
- Stop if: existing repository policy requires session-store and auth persistence to share a single physical table.

Step 2:
- Action: Switch every session-store append, projection-repair, snapshot, wake, fork, and stream SQL path to the new namespaced table names.
- Done when: no runtime query inside `crates/octopus-sdk-session/src/sqlite/*.rs` still reads or writes generic `sessions/events`.
- Verify: `rg -n 'FROM sessions|INTO sessions|UPDATE sessions|DELETE FROM sessions|FROM events|INTO events|UPDATE events|DELETE FROM events' crates/octopus-sdk-session/src/sqlite`
- Stop if: another crate imports the old table names as part of a stable contract.

### Task 5: Add shared-DB regression coverage

Status: `done`

Files:
- Modify: `crates/octopus-sdk-session/src/sqlite/schema.rs`
- Modify: `crates/octopus-sdk-session/tests/sqlite_jsonl.rs`

Preconditions:
- Task 4 is implemented locally.

Step 1:
- Action: Add regression coverage for two cases: auth-owned `sessions` already exists in the shared DB, and legacy generic session-store tables need migration into the namespaced runtime tables.
- Done when: tests prove auth-owned `sessions` remains intact while session-store append/stream still works, and legacy generic session-store tables are migrated or reopened successfully.
- Verify: `cargo test -p octopus-sdk-session --test sqlite_jsonl && cargo test -p octopus-sdk-session sqlite::schema::`
- Stop if: the tests need desktop/Tauri startup instead of direct crate-level setup.

### Task 6: Re-verify desktop startup after SQLite fix

Status: `done`

Files:
- Modify: `docs/plans/runtime/2026-04-23-session-jsonl-compat.md`

Preconditions:
- Task 4 and Task 5 are `done`.

Step 1:
- Action: Re-run crate tests and `pnpm dev:desktop` far enough to confirm the SQLite column error is gone.
- Done when: startup no longer prints `session sqlite error: no such column: session_id`, and the previous missing-`event_id` regression stays fixed.
- Verify: `cargo test -p octopus-sdk-session && pnpm dev:desktop`
- Stop if: a different startup failure replaces the SQLite column error and requires a separate fix.

## Checkpoint 2026-04-23 07:05

- Batch: Task 1 Step 1
- Completed:
  - Confirmed dev desktop uses repository root as backend workspace in cargo-workspace mode.
  - Confirmed repository-root `runtime/events/*.jsonl` files are legacy envelopes with no outer `event_id`.
  - Confirmed `SqliteJsonlSessionStore::open()` crashes during `read_records()` before backend healthcheck succeeds.
- Verification:
  - `find runtime data -type f \( -path 'runtime/events/*.jsonl' -o -path 'runtime/sessions/*.json' -o -name 'main.db' \) | sort` -> pass
  - `python3 - <<'PY' ... 'event_id' in first line ... PY` -> pass (`False` for every current repository JSONL file)
- Blockers:
  - none
- Next:
  - Task 1 Step 2

## Checkpoint 2026-04-23 08:08

- Batch: Task 1 Step 2 -> Task 3 Step 1
- Completed:
  - Added backward-compatible JSONL parsing for current `{ event_id, event }` records, event-only legacy lines, and legacy runtime-envelope files that should be skipped by the session store.
  - Added regression tests for reopening legacy event-only JSONL and for skipping legacy runtime-envelope JSONL files.
  - Re-ran desktop startup and confirmed the previous `session serde error: missing field \`event_id\`` no longer appears.
- Verification:
  - `cargo test -p octopus-sdk-session --test sqlite_jsonl` -> pass
  - `cargo test -p octopus-sdk-session` -> pass
  - `pnpm dev:desktop` -> pass for this regression; startup reached `Running /.../octopus-desktop-shell` without the previous serde crash
- Blockers:
  - `cargo fmt --check` still reports unrelated pre-existing formatting drift in other crates outside this fix
- Next:
  - none

## Checkpoint 2026-04-23 22:58

- Batch: Task 4 Step 1
- Completed:
  - Confirmed the previous JSONL serde crash is fixed, but desktop startup now fails later on a SQLite query against `sessions.session_id`.
  - Confirmed `data/main.db` already contains an auth-owned `sessions` table from `octopus-infra`, while session-store code still queries generic `sessions/events`.
  - Started namespacing session-store tables in `schema.rs` so migration only renames legacy session-store tables and leaves auth-owned `sessions` untouched.
- Verification:
  - `sqlite3 data/main.db '.schema sessions'` -> auth-owned schema, no `session_id` column
  - `sqlite3 data/main.db '.schema events'` -> legacy session-store event schema
  - `cargo test -p octopus-sdk-session` -> expected fail while remaining SQL paths still reference generic `sessions/events`
- Blockers:
  - none
- Next:
  - Task 4 Step 2

## Checkpoint 2026-04-23 23:21

- Batch: Task 4 Step 2 -> Task 6 Step 1
- Completed:
  - Switched every runtime session-store append, projection-repair, snapshot, wake, fork, and stream SQL path from generic `sessions/events` to `runtime_session_store_sessions/runtime_session_store_events`.
  - Added regression coverage for auth-owned `sessions` coexistence, and for migrating legacy generic session-store tables into the namespaced runtime tables.
  - Re-ran desktop startup and confirmed it now reaches `Running /.../octopus-desktop-shell` without the previous `missing field event_id` or `no such column: session_id` failures.
- Verification:
  - `cargo test -p octopus-sdk-session` -> pass
  - `pnpm dev:desktop` -> pass for this regression; after clearing a stale Vite listener on port `15420`, startup reached `Running /.../octopus-desktop-shell` and stayed up without the previous session-store errors
- Blockers:
  - none
- Next:
  - none

## Checkpoint 2026-04-23 10:46

- Batch: Post-fix desktop dev re-verification
- Completed:
  - Re-ran `pnpm dev:desktop` in the current dirty workspace after the user's later report.
  - Confirmed startup still reaches `Running /Users/goya/Work/weilaizhihuigu/super-agent/octopus/target/debug/octopus-desktop-shell` without the previous `session serde error: missing field \`event_id\`` failure.
- Verification:
  - `pnpm dev:desktop` -> pass for this regression; shell binary launched after rebuild and did not print the old serde crash
- Blockers:
  - none
- Next:
  - none
