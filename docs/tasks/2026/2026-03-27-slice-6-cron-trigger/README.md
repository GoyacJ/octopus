# Slice 6 Cron Trigger

This task package records the first concrete GA trigger expansion slice after substrate generalization: add the minimum `cron` trigger path on top of the shared automation delivery projection model.

## Package Files

- [design-note.md](design-note.md)
- [contract-change.md](contract-change.md)
- [implementation-summary.md](implementation-summary.md)
- [verification.md](verification.md)
- [delivery-note.md](delivery-note.md)

## Task Definition

- Goal:
  - Implement the minimum `cron` trigger so due automation schedules can create deduped trigger deliveries and governed `run_type=automation` runs.
- Scope:
  - Create this task package and keep local design, contract, verification, and delivery notes here.
  - Add runtime support for due-trigger ticking and persisted `next_fire_at` schedule advancement.
  - Add the minimum remote-hub process loop that periodically calls the runtime tick API.
  - Add tests for due firing, duplicate ticking, reopen recovery, approval waiting, and retryable failure handling.
- Out of Scope:
  - Webhook or MCP-event ingress.
  - Desktop automation UI.
  - Distributed scheduling or complex DSL parsing.
- Acceptance Criteria:
  - A `cron` trigger can be persisted on an automation.
  - `tick_due_triggers(now)` projects at most one overdue fire window per due trigger and advances schedule metadata.
  - Duplicate ticks do not create duplicate deliveries or runs.
  - Reopen, approval, deny, and retry semantics remain aligned with the existing delivery model.
- Non-functional Constraints:
  - Keep scheduling local to the existing remote-hub/runtime process.
  - Avoid introducing a new scheduler service.
  - Preserve delivery dedupe and recovery semantics.
- MVP Boundary:
  - Minimal persisted schedule metadata and ticking only.
  - No calendar UI or schedule builder surface.
- Human Approval Points:
  - None.
- Source Of Truth Updates:
  - Update this task package.
  - Update shared trigger contracts if the `cron` shape is refined during implementation.
  - Update current-state docs if the tracked GA gap summary changes materially.
- Affected Modules:
  - `schemas/runtime`
  - `crates/runtime`
  - `apps/remote-hub`
  - `docs/tasks`
- Affected Layers:
  - Cross-language contracts
  - Rust runtime/orchestration layer
  - Remote-hub assembly layer
- Risks:
  - Overfiring or duplicate delivery creation on repeated ticks or reopen.
  - Encoding more scheduler behavior than the GA minimum requires.
  - Letting the remote-hub binary grow into a dedicated scheduler service.
- Validation:
  - Runtime integration tests for due/duplicate/reopen/approval/retry paths.
  - Remote-hub integration tests or focused process-loop tests where applicable.
