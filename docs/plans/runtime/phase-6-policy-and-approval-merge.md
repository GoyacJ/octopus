# Phase 6 Policy And Approval Merge

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Merge business authorization, execution permission mode, approval brokering, and auth mediation into one deny-before-expose runtime control plane across capability, memory, team, workflow, and MCP boundaries.

**Architecture:** Phase 5 is assumed complete before this phase starts. The runtime already has a single capability trunk, durable subrun or workflow lineage, and typed memory selection or proposal state. Phase 6 adds a unified `policy compiler` and `approval or auth broker` on top of that foundation. The compiler merges actor policy, workspace authorization, runtime config enablement, execution permission ceiling, memory write rules, team escalation rules, workflow escalation rules, and provider auth state into a frozen session policy. The planner consumes that frozen policy to hide or defer blocked surfaces before exposure. The broker persists mediation state, approval requests, auth challenges, and resume checkpoints so the runtime can resume the specific suspended capability, memory proposal, worker escalation, or workflow node instead of rerunning a whole turn.

**Tech Stack:** OpenAPI runtime contracts under `contracts/openapi/src/**`, feature-based runtime policy and approval exports under `packages/schema/src/*`, policy and mediation modules in `crates/octopus-runtime-adapter`, capability planner and executor integration in `crates/tools`, policy primitives in `crates/octopus-core`, SQLite runtime projections, append-only JSONL runtime events, and shared desktop adapter and store consumers.

---

## Fixed Decisions

- Deny-before-expose becomes real runtime behavior in this phase. If policy blocks a capability, memory write, team escalation, workflow action, or MCP access, it must not be exposed as runnable surface area.
- Business authorization, execution permission ceiling, approval, and auth resolution are one control plane. They are not independent UI prompts or post-planning side flows.
- A running session remains bound to the frozen session policy captured at start or at explicit resume checkpoints. No runtime path may silently widen permissions mid-session.
- Approval and auth are target-specific runtime states. They attach to a capability call, memory proposal, worker escalation, workflow node, or MCP target, not to an undifferentiated turn blob.
- Memory write mediation uses the same broker as tool, team, workflow, and MCP mediation.
- The runtime must expose pending approval and auth state through the public contract rather than reconstructing it only in UI code.
- Every mediation event kind emitted by the runtime must be declared in OpenAPI and projected consistently through SQLite, JSONL, and the desktop adapter.
- This phase does not introduce a second policy engine in frontend stores or host adapters.

## Scope

This phase covers:

- unified session policy compilation and deny-before-expose planning
- approval and auth mediation across capability, memory, team, workflow, and MCP surfaces
- target-specific mediation checkpoints and resume behavior
- policy, approval, and auth projections in session and run transport
- runtime event and SQLite projection alignment for mediation state
- desktop and server parity on the same runtime control-plane contract

This phase does not cover:

- enterprise-wide access-control product redesign outside runtime mediation needs
- workflow-template authoring or automation UX redesign
- a separate side API for approvals or auth outside `/api/v1/runtime/*`
- undoing the Phase 5 rule that durable memory writeback remains proposal-only

## Current Baseline

The current repository already contains some policy and approval structure, but the control plane is not yet fully merged.

- `contracts/openapi/src/components/schemas/runtime.yaml` and generated `packages/schema/src/generated.ts` already expose `approvalLayer`, `targetKind`, `targetRef`, and `escalationReason` on approval records.
- Public runtime transport already exposes `sessionPolicy`, `executionPermissionMode`, and optional `pendingApproval` on session and run payloads.
- `crates/octopus-runtime-adapter/src/approval_flow.rs` is currently a thin delegator into `AgentRuntimeCore::resume_after_approval`.
- `crates/octopus-runtime-adapter/src/agent_runtime_core.rs` still creates a turn-level approval request centered on `tool_name = "runtime.turn"` when the requested permission mode exceeds the session ceiling. That path sets `approval_layer = "execution-permission"`, `target_kind = "runtime-turn"`, and `escalation_reason = "session ceiling requires approval"`.
- `crates/tools/src/capability_runtime/provider.rs` already has a mediation decision hook driven by `requiresApproval` and `requiresAuth`, and `SessionCapabilityStore` already remembers approved tools and auth-resolved tools.
- Generated runtime transport already includes capability-facing approval or auth flags such as `requiresApproval`, `requiresAuth`, and `authResolvedTools`.
- Even with those pieces in place, business authorization is not yet consistently merged into deny-before-expose planning, and approval remains mostly execution-time and turn-centric instead of target-specific across tool, memory, team, workflow, and MCP surfaces.

Phase 6 starts from that actual state: the public contract and internal runtime have mediation fragments, but not yet one merged control plane.

## Task 1: Harden Public Policy, Approval, And Auth Contracts Around Unified Mediation

**Files:**
- Modify: `contracts/openapi/src/components/schemas/runtime.yaml`
- Modify: `contracts/openapi/src/paths/runtime.yaml`
- Modify: `packages/schema/src/runtime-policy.ts`
- Modify: `packages/schema/src/runtime.ts`
- Modify: `packages/schema/src/index.ts`
- Modify: `apps/desktop/test/openapi-transport.test.ts`
- Modify: `apps/desktop/test/runtime-store.test.ts`

**Implement:**
- keep `docs/api-openapi-governance.md` as the canonical transport policy and make the runtime contract describe real mediation state rather than partial approval fragments
- extend session and run transport with unified approval and auth summaries
- separate pending mediation state from historical last outcome state
- keep browser host and Tauri host on the same public control-plane contract

**Required contract groups:**
- `RuntimePendingMediation`
- `RuntimeMediationOutcome`
- `RuntimeAuthChallengeSummary`
- `RuntimePolicyDecisionSummary`

**Required public fields:**
- on `RuntimeSessionDetail`:
  - `pendingMediation`
  - `authStateSummary`
  - `policyDecisionSummary`
- on `RuntimeRunSnapshot`:
  - `pendingMediation`
  - `lastMediationOutcome`
  - `approvalTarget`
  - `authTarget`
- on approval and auth records:
  - `approvalLayer`
  - `targetKind`
  - `targetRef`
  - `escalationReason`
  - `requiresApproval`
  - `requiresAuth`

**Verification:**
```bash
pnpm openapi:bundle
pnpm schema:generate
pnpm schema:check
pnpm -C apps/desktop exec vitest run test/openapi-transport.test.ts test/runtime-store.test.ts
```

**Done when:**
- session and run transport can represent pending and resolved mediation state across every runtime target family
- the runtime control-plane contract no longer depends on turn-centric approval-only assumptions

## Task 2: Build A Unified Session Policy Compiler And Runtime Approval Broker

**Files:**
- Create: `crates/octopus-runtime-adapter/src/policy_compiler.rs`
- Create: `crates/octopus-runtime-adapter/src/approval_broker.rs`
- Create: `crates/octopus-runtime-adapter/src/auth_mediation.rs`
- Modify: `crates/octopus-runtime-adapter/src/agent_runtime_core.rs`
- Modify: `crates/octopus-runtime-adapter/src/session_service.rs`
- Modify: `crates/octopus-runtime-adapter/src/lib.rs`
- Modify: `crates/octopus-core/src/runtime_policy.rs`

**Implement:**
- compile a frozen session policy from actor or team policy, workspace authorization, runtime config, execution permission ceiling, memory policy, team delegation policy, workflow affordance policy, and provider auth state
- replace scattered execution-time approval checks with one broker entrypoint
- let the broker return structured mediation outcomes for allow, deny, require approval, require auth, interrupted, or cancelled states
- preserve policy reasoning and target metadata for restart-safe resume

**Required policy compiler inputs:**
- actor or team manifest policy
- workspace authorization state
- execution permission mode ceiling
- runtime config enablement
- memory write policy
- workflow and team escalation policy
- provider or MCP auth state

**Required broker guarantees:**
- no runtime path may run a blocked target because a later layer forgot to re-check policy
- approval and auth requests are created with target-specific metadata, not only turn-level placeholders
- policy decisions remain reproducible from frozen inputs and persisted summaries

**Verification:**
```bash
cargo test -p octopus-core runtime_policy
cargo test -p octopus-runtime-adapter policy
```

**Done when:**
- one broker owns approval and auth mediation
- one compiled session policy governs planner exposure and executor permission decisions

## Task 3: Merge Deny-Before-Expose Into Capability, Memory, Team, Workflow, And MCP Planning

**Files:**
- Modify: `crates/octopus-runtime-adapter/src/capability_planner_bridge.rs`
- Modify: `crates/octopus-runtime-adapter/src/capability_executor_bridge.rs`
- Modify: `crates/octopus-runtime-adapter/src/memory_runtime.rs`
- Modify: `crates/octopus-runtime-adapter/src/memory_writer.rs`
- Modify: `crates/octopus-runtime-adapter/src/team_runtime.rs`
- Modify: `crates/octopus-runtime-adapter/src/workflow_runtime.rs`
- Modify: `crates/tools/src/capability_runtime/planner.rs`
- Modify: `crates/tools/src/capability_runtime/executor.rs`

**Implement:**
- feed compiled policy results into capability planning before model-visible surface generation
- feed memory write approval requirements into proposal review rather than leaving them as an external afterthought
- gate worker spawn, team escalation, workflow continuation, and MCP auth through the same broker
- represent hidden, deferred, approval-required, and auth-required surfaces as explicit runtime planning outcomes

**Required runtime rules:**
- blocked capabilities are removed before exposure to the model
- approval-required targets remain visible only when the runtime contract explicitly represents them as mediated or deferred state
- auth-required MCP and provider targets remain hidden or degraded until auth state is resolved
- policy and permission checks are consistent for primary runs, subruns, workflows, and memory proposals

**Verification:**
```bash
cargo test -p tools
cargo test -p octopus-runtime-adapter
```

**Done when:**
- deny-before-expose is enforced across capability, memory, team, workflow, and MCP surfaces
- no executor path widens permissions after planning

## Task 4: Replace Turn-Centric Approval Resume With Target-Specific Resume Checkpoints

**Files:**
- Modify: `crates/octopus-runtime-adapter/src/approval_flow.rs`
- Modify: `crates/octopus-runtime-adapter/src/agent_runtime_core.rs`
- Modify: `crates/octopus-runtime-adapter/src/persistence.rs`
- Modify: `crates/runtime/src/conversation/turn_orchestrator.rs`
- Modify: `crates/octopus-runtime-adapter/src/background_runtime.rs`
- Modify: `crates/octopus-runtime-adapter/src/workflow_runtime.rs`

**Implement:**
- stop treating approval resume as "resume the blocked turn"
- checkpoint the exact suspended target: capability call, memory proposal, worker escalation, workflow node, or MCP auth challenge
- resume the specific suspended target and then continue the runtime loop from that state
- preserve mediation lineage and reason across restart and reload

**Required checkpoint fields:**
- `approvalLayer`
- `targetKind`
- `targetRef`
- `requiredPermission`
- `requiresApproval`
- `requiresAuth`
- `reason`
- `capabilityId` or equivalent execution target id
- `providerKey` when applicable

**Verification:**
```bash
cargo test -p octopus-runtime-adapter approval
cargo test -p runtime conversation
```

**Done when:**
- approval or auth resolution resumes the blocked target instead of replaying a whole turn
- `runtime.turn` no longer needs to be the universal approval target placeholder

## Task 5: Persist Policy Decisions, Approval State, And Auth Mediation As First-Class Runtime Projections

**Files:**
- Modify: `crates/octopus-infra/src/infra_state.rs`
- Modify: `crates/octopus-runtime-adapter/src/persistence.rs`
- Modify: `crates/octopus-runtime-adapter/src/execution_events.rs`
- Modify: `crates/octopus-runtime-adapter/src/session_service.rs`
- Modify: `apps/desktop/test/tauri-client-runtime.test.ts`

**Implement:**
- add queryable SQLite projection fields for pending mediation, auth challenge state, denied exposure counts, approval lineage, and last mediation outcome
- store large mediation artifacts and checkpoint bodies under `runtime/` with hashes and refs in SQLite
- keep append-only mediation history in `runtime/events/*.jsonl`
- make recovery depend on SQLite plus JSONL plus checkpoint artifacts instead of frontend-local reconstruction

**Required event groups:**
- `policy.*`
- `approval.*`
- `auth.*`

**Required event fields:**
- `sessionId`
- `runId`
- `parentRunId`
- `iteration`
- `targetKind`
- `targetRef`
- `approvalLayer`
- `outcome`

**Verification:**
```bash
cargo test -p octopus-infra
cargo test -p octopus-runtime-adapter
pnpm -C apps/desktop exec vitest run test/tauri-client-runtime.test.ts
```

**Done when:**
- policy and mediation state can be queried and replayed after restart
- approvals and auth are no longer runtime side effects visible only in the live loop

## Task 6: Cut Over Desktop And Server Consumers, Then Remove Local Policy Interpretation

**Files:**
- Modify: `crates/octopus-server/src/workspace_runtime.rs`
- Modify: `crates/octopus-platform/src/workspace.rs`
- Modify: `apps/desktop/src/tauri/runtime_api.ts`
- Modify: `apps/desktop/src/stores/runtime.ts`
- Modify: `apps/desktop/src/views/project/ConversationView.vue`
- Modify: `apps/desktop/src/views/project/TraceView.vue`
- Modify: `apps/desktop/test/openapi-transport.test.ts`
- Modify: `apps/desktop/test/runtime-store.test.ts`
- Modify: `apps/desktop/test/tauri-client-runtime.test.ts`

**Implement:**
- expose policy, approval, and auth state through the shared runtime adapter rather than letting stores infer it
- keep browser host and Tauri host on the same control-plane surface
- stop treating frontend permission prompts as canonical runtime policy decisions
- update fixtures and views to consume target-specific mediation state

**Verification:**
```bash
pnpm -C apps/desktop exec vitest run test/openapi-transport.test.ts test/runtime-store.test.ts test/tauri-client-runtime.test.ts
cargo test -p octopus-platform
cargo test -p octopus-server
```

**Done when:**
- desktop consumes brokered mediation state from typed transport only
- no host-specific policy semantics leak back into the runtime contract

## Task 7: Phase-Level Validation And Control-Plane Acceptance Fence

**Files:**
- Verify only

**Run:**
```bash
pnpm openapi:bundle
pnpm schema:generate
pnpm schema:check
cargo test -p octopus-core
cargo test -p tools
cargo test -p octopus-infra
cargo test -p octopus-runtime-adapter
cargo test -p octopus-platform
cargo test -p octopus-server
pnpm -C apps/desktop exec vitest run test/openapi-transport.test.ts test/runtime-store.test.ts test/tauri-client-runtime.test.ts
```

**Acceptance criteria:**
- business deny removes blocked capabilities from exposure
- execution permission mode narrows actual runtime behavior, not just UI labels
- approval and auth events are projected consistently across tool, memory, team, workflow, and MCP boundaries
- pending mediation and last outcome state survive restart and reload
- no runtime path can bypass the merged control plane after planning

## Handoff To Later Phases

Phase 6 is complete only when later phases can safely assume:

- deny-before-expose is enforced by runtime behavior
- approval and auth are target-specific, persisted runtime facts
- memory proposal review uses the same broker as capability, team, workflow, and MCP mediation
- frontend hosts no longer invent control-plane state locally

Phase 7 can then focus on final public contract and host projection cutover instead of compensating for missing policy runtime behavior.
