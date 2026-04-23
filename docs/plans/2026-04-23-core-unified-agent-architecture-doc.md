# Core Unified Agent Architecture Document Plan

## Goal

Create a canonical architecture/design document under `docs/core` that integrates the strongest patterns from Claude Code, Hermes Agent, and OpenClaw into one actionable Octopus core architecture.

## Architecture

The new document will live in `docs/core` as a long-lived normative design artifact above the existing runtime and capability companion docs. It will define ownership boundaries, core runtime flow, plugin and capability surfaces, tool and skill execution, MCP integration, and rollout guidance without duplicating lower-level policy already owned by `docs/capability_runtime.md`, `docs/runtime_config_api.md`, or `docs/api-openapi-governance.md`.

## Scope

- In scope:
  - Create `docs/core/` as a new documentation subtree.
  - Add one primary architecture/design document for the unified core agent platform.
  - Cross-reference existing canonical docs where policy already exists.
  - Capture layered architecture, contracts, control flow, extension model, persistence boundaries, and rollout sequence.
- Out of scope:
  - Source code changes.
  - Changes to canonical transport or runtime-config policy docs.
  - Adding implementation tasks outside this document and plan.

## Risks Or Open Questions

- The new document must not contradict the current capability-runtime contract or runtime rebuild design.
- The new document must stay normative and concise instead of becoming a loose research summary.
- If `docs/core` already has an implied ownership model elsewhere in the repo, stop and reconcile before broadening the scope.

## Execution Rules

- Do not start document creation until the target file path, ownership boundary, and cross-reference strategy are explicit.
- Do not duplicate full policy text already owned by `docs/api-openapi-governance.md`, `docs/runtime_config_api.md`, or `docs/capability_runtime.md`.
- Stop when the new document would need to redefine an existing canonical policy instead of referencing it.
- Execute in small batches and update status in place after each batch.

## Task Ledger

### Task 1: Define document placement and structure

Status: `done`

Files:
- Create: `docs/core/unified-agent-platform-architecture.md`
- Modify: `docs/plans/2026-04-23-core-unified-agent-architecture-doc.md`

Preconditions:
- `docs/AGENTS.md` and `docs/plans/AGENTS.md` have been read.
- Existing runtime and capability design docs have been reviewed for overlap.

Step 1:
- Action: Confirm the target path, document role, and section structure for the new `docs/core` architecture doc.
- Done when: The doc outline is explicit enough to draft without changing ownership mid-stream.
- Verify: `test -f docs/plans/2026-04-23-core-unified-agent-architecture-doc.md`
- Stop if: The intended document would collide with an existing canonical core design document.

Step 2:
- Action: Draft the architecture document with clear sections for principles, planes, registries, execution loop, skills, commands, MCP, plugins, harnesses, persistence, security, and rollout.
- Done when: `docs/core/unified-agent-platform-architecture.md` exists and reads as a coherent standalone design.
- Verify: `test -f docs/core/unified-agent-platform-architecture.md && rg -n "^#|^##|^###" docs/core/unified-agent-platform-architecture.md`
- Stop if: The document requires inventing policy that belongs in another canonical governance file.

Notes:
- The document should synthesize reference-project strengths into a native Octopus design instead of restating each project separately.

### Task 2: Validate consistency and finish handoff

Status: `done`

Files:
- Modify: `docs/core/unified-agent-platform-architecture.md`
- Modify: `docs/plans/2026-04-23-core-unified-agent-architecture-doc.md`

Preconditions:
- Task 1 Step 2 is complete.

Step 1:
- Action: Review the new document against `docs/capability_runtime.md` and `docs/plans/runtime/agent-runtime-rebuild-design.md`, tighten wording, and remove duplicated policy.
- Done when: The final document stays aligned with existing canonical docs and uses cross-references where needed.
- Verify: `rg -n "docs/capability_runtime.md|docs/runtime_config_api.md|docs/api-openapi-governance.md" docs/core/unified-agent-platform-architecture.md`
- Stop if: A contradiction is found that cannot be resolved without changing another canonical document.

Step 2:
- Action: Update the plan status and append a checkpoint with files changed, verification results, blockers, and next step.
- Done when: The plan reflects the finished execution state.
- Verify: `tail -n 40 docs/plans/2026-04-23-core-unified-agent-architecture-doc.md`
- Stop if: Verification results are incomplete or the current state cannot be reconstructed from the plan.

### Task 3: Harden the canonical contracts and rollout order

Status: `done`

Files:
- Modify: `docs/core/unified-agent-platform-architecture.md`
- Modify: `docs/plans/2026-04-23-core-unified-agent-architecture-doc.md`

Preconditions:
- The existing architecture document and reference-project evidence have been reviewed.
- The follow-up scope stays inside this document and plan.

Step 1:
- Action: Tighten the registry and type model so `CapabilitySpec` remains the root execution contract without collapsing commands, skills, providers, channels, and harnesses into one runtime-callable object.
- Done when: The document explicitly defines the role, boundary, and required fields of `CapabilitySpec`, `SkillSpec`, `CommandSpec`, `ProviderSpec`, `ChannelSpec`, `HarnessSpec`, and `ResourceSpec`.
- Verify: `rg -n "CapabilitySpec|SkillSpec|CommandSpec|ProviderSpec|ChannelSpec|HarnessSpec|ResourceSpec|callable|control-plane|prompt-only" docs/core/unified-agent-platform-architecture.md`
- Stop if: The clarification would require changing another canonical governance document instead of this architecture doc.

Step 2:
- Action: Expand the surface assembler and runtime loop sections with concrete planner outputs, conflict rules, transcript-mirror requirements, and the relationship between assembled surfaces and actual execution.
- Done when: The document states how tool, skill, command, and prompt surfaces are assembled, what the runtime loop consumes, and how harness-native events map back into the Octopus transcript.
- Verify: `rg -n "ToolExecutionPlan|Prompt Builder|Runtime loop|transcript mirror|deferred|conflict|retry|resume|compaction" docs/core/unified-agent-platform-architecture.md`
- Stop if: The runtime-loop wording would redefine persistence or transport policy owned elsewhere.

Step 3:
- Action: Add an explicit plugin / bundle / inbound MCP / outbound MCP / harness integration order that follows the three reference projects without treating all sources as equally trusted.
- Done when: The document gives a staged adoption order and states what must be true before each source family is enabled.
- Verify: `rg -n "Integration Order|bundle|Inbound MCP|Outbound MCP|harness|manifest-first|translation" docs/core/unified-agent-platform-architecture.md`
- Stop if: The rollout order cannot be stated without unresolved source-of-truth decisions.

Notes:
- This pass treats `docs/capability_runtime.md` and older runtime design docs as non-authoritative for architecture decisions unless they are referenced only as legacy companion links.

### Task 4: Re-verify the document and refresh execution state

Status: `done`

Files:
- Modify: `docs/plans/2026-04-23-core-unified-agent-architecture-doc.md`
- Modify: `docs/core/unified-agent-platform-architecture.md`

Preconditions:
- Task 3 is complete.

Step 1:
- Action: Run document-level verification against the revised sections and confirm the new language is anchored in the requested reference implementations.
- Done when: Verification commands pass and the final document has explicit coverage for registry types, surface assemblers, runtime loop, and integration order.
- Verify: `rg -n "^## |^### " docs/core/unified-agent-platform-architecture.md && rg -n "Claude Code|Hermes Agent|OpenClaw" docs/core/unified-agent-platform-architecture.md`
- Stop if: The revised text introduces claims that cannot be supported by the inspected references.

Step 2:
- Action: Append a new checkpoint with files changed, verification results, blockers, and next step.
- Done when: The current execution state is reconstructable from the plan without re-reading the chat.
- Verify: `tail -n 80 docs/plans/2026-04-23-core-unified-agent-architecture-doc.md`
- Stop if: The checkpoint would omit verification results or current task state.

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

## Checkpoint 2026-04-23 22:27

- Batch: Task 1 Step 1 -> Task 2 Step 2
- Completed:
  - created `docs/core/unified-agent-platform-architecture.md`
  - aligned the new doc with existing runtime and capability companion docs
  - updated this plan to reflect final execution state
- Verification:
  - `test -f docs/plans/2026-04-23-core-unified-agent-architecture-doc.md` -> pass
  - `test -f docs/core/unified-agent-platform-architecture.md && rg -n "^#|^##|^###" docs/core/unified-agent-platform-architecture.md` -> pass
  - `rg -n "docs/capability_runtime.md|docs/runtime_config_api.md|docs/api-openapi-governance.md" docs/core/unified-agent-platform-architecture.md` -> pass
  - `tail -n 40 docs/plans/2026-04-23-core-unified-agent-architecture-doc.md` -> pass
- Blockers:
  - none
- Next:
  - none

## Checkpoint 2026-04-24 00:26

- Batch: Task 3 Step 1 -> Task 4 Step 2
- Completed:
  - hardened `docs/core/unified-agent-platform-architecture.md` around typed registry boundaries instead of a monolithic capability object
  - added explicit surface assembler contracts, runtime loop phases, transcript mirror rules, and harness source-of-truth rules
  - added staged extension integration order for native plugins, bundles, inbound MCP, outbound MCP, and harnesses
  - updated this plan to reflect the follow-up hardening pass
- Files changed:
  - `docs/core/unified-agent-platform-architecture.md` (modified)
  - `docs/plans/2026-04-23-core-unified-agent-architecture-doc.md` (modified)
- Verification:
  - `rg -n "CapabilitySpec|SkillSpec|CommandSpec|ProviderSpec|ChannelSpec|HarnessSpec|ResourceSpec|callable|control-plane|prompt-only" docs/core/unified-agent-platform-architecture.md` -> pass
  - `rg -n "ToolExecutionPlan|Prompt Builder|Runtime Loop Phases|transcript mirror|deferred|conflict|retry|resume|compaction" docs/core/unified-agent-platform-architecture.md` -> pass
  - `rg -n "Extension Integration Order|bundle|Inbound MCP|Outbound MCP|harness|manifest-first|translation" docs/core/unified-agent-platform-architecture.md` -> pass
  - `rg -n "^## |^### " docs/core/unified-agent-platform-architecture.md && rg -n "Claude Code|Hermes Agent|OpenClaw" docs/core/unified-agent-platform-architecture.md` -> pass
  - `tail -n 80 docs/plans/2026-04-23-core-unified-agent-architecture-doc.md` -> pass
- Blockers:
  - none
- Next:
  - none
