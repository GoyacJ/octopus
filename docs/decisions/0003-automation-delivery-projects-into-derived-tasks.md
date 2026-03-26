# ADR 0003: Automation Delivery Projects Into Derived Tasks

- Status: Accepted
- Date: 2026-03-26
- Deciders: Repository maintainers
- Consulted: PRD, SAD, GA blueprint, Slice 3 task package
- Informed: Future automation, scheduler, and transport slices

---

## Context

Slice 3 introduces the first formal non-manual entrypoint through Automation and TriggerDelivery.

The repository already proves one governed execution chain:

- `Task -> Policy / Budget / Approval -> Run -> Artifact -> Audit / Trace`

The open design question for Slice 3 is how an Automation delivery should enter that chain without creating a parallel execution model or collapsing Automation into an untyped shortcut.

The relevant constraints are already fixed by source-of-truth docs:

- `Run` remains the only authority execution shell.
- `Task`, `Automation`, and other formal entry objects cannot bypass the Run system.
- Slice 3 must stay local, SQLite-backed, and Rust-only.
- Slice 3 must not silently pull in scheduler, transport, MCP, or Shared Knowledge depth.

## Decision

### 1. `Automation` remains a reusable definition, not the concrete execution request

`Automation` stores the repeatable definition, default execution action, governance inputs, and trigger ownership context.

It does not directly stand in for an individual execution attempt.

### 2. Each `TriggerDelivery` materializes one derived `Task`

Every accepted TriggerDelivery creates or reuses one derived Task that represents the concrete execution request for that delivery.

That derived Task:

- belongs to the same Workspace / Project scope as the Automation
- carries the resolved action and governance inputs
- is marked as `source_kind=automation`
- stores `automation_id` so lineage remains queryable

### 3. The execution shell remains a normal `Run`, but with `run_type=automation`

The derived Task is then executed through the existing governed runtime path, producing a normal Run.

That Run:

- remains the only authority execution shell
- stores `automation_id` and `trigger_delivery_id`
- uses `run_type=automation` to preserve source semantics

### 4. TriggerDelivery owns delivery-level idempotency and recovery state

`TriggerDelivery` is the formal record for:

- delivery dedupe
- delivery attempt count
- delivery payload capture
- delivery success/failure outcome
- explicit delivery retry entry

Run retry remains a lower-level execution recovery mechanism and is reused when the associated Run is retryable.

## Consequences

### Positive

- Slice 3 reuses the already verified Task/Run/Governance/Observation chain instead of creating a second execution path.
- Automation remains a stable reusable object rather than an overloaded execution record.
- Delivery-level dedupe and execution-level retry stay separate and easier to reason about.
- Later trigger types can reuse the same TriggerDelivery-to-derived-Task projection.

### Negative

- Each automation delivery persists an extra Task record.
- Query paths must now explain the relationship between Automation, TriggerDelivery, Task, and Run.
- Some Task fields now represent derived rather than user-authored requests.

### Trade-off

The repository accepts an extra persisted Task per delivery in exchange for preserving one governed execution chain and avoiding a second orchestration model.

## Rejected Alternatives

### 1. Let `Automation` create `Run` directly with no Task

Rejected because it would split the runtime into separate manual and automation execution paths and force duplicated governance/orchestration logic.

### 2. Treat `Automation` itself as the execution request

Rejected because a reusable definition and a single delivery attempt are different lifecycle objects with different idempotency and recovery needs.

### 3. Delay delivery records and rely only on Run dedupe

Rejected because Slice 3 explicitly needs TriggerDelivery state, delivery-level dedupe, and explicit recovery semantics that cannot be represented cleanly by Run alone.

## Follow-up

- Extend the same projection model to `cron`, `webhook`, and `MCP event` trigger types in later Slice 3 work.
- Revisit query/report surfaces once app or transport layers enter scope.
- Keep Shared Knowledge integration deferred until Slice 4.
