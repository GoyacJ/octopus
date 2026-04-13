# Octopus Agent Platform Rebuild Design

This document records the canonical target architecture for the Octopus agent platform rebuild.

General HTTP and OpenAPI policy lives in `docs/api-openapi-governance.md`. Capability planning rules live in `docs/capability_runtime.md`. Runtime-config ownership and snapshot semantics live in `docs/runtime_config_api.md`. This document defines the platform-level architecture above those surfaces.

## North Star

Octopus is a general agent platform, not a coding-only runtime.

The first-class product objects are:

- `Agent`: an importable and exportable digital employee asset
- `Team`: an importable and exportable digital team asset

An asset is not a prompt wrapper. It packages:

- identity and role
- model and effort strategy
- capability policy
- memory policy
- permission envelope
- delegation and workflow behavior
- output and artifact expectations
- import or export metadata

The platform must support multiple task domains with one runtime trunk:

- coding
- research
- documentation
- browser and web tasks
- spreadsheet and data tasks
- operations and workflow tasks
- automation and long-running background tasks

## Evidence Baseline

This rebuild is derived from three source families and must stay aligned with them:

1. Local Claude Code source map under `docs/claude-code-sourcemap-main`
   - unified query or runtime loop
   - ordered tool orchestration
   - isolated subagent and team execution
   - permission and auth mediation
   - typed memory and memory freshness
   - append-only session storage and trace projection
2. Anthropic Claude Code documentation
   - layered memory, settings, skills, permissions, hooks, MCP, and Agent SDK contracts
   - declarative extension surfaces instead of hidden runtime mutation
3. OpenClaw documentation
   - gateway-first architecture
   - portable skills and plugins
   - asset registry and bundle translation
   - workflow, background task, and bridge boundaries

The rebuild must copy the strong patterns, not the exact product shell.

## Reference Inputs

The most important source inputs for this design are:

- local Claude Code sources
  - `docs/claude-code-sourcemap-main/restored-src/src/QueryEngine.ts`
  - `docs/claude-code-sourcemap-main/restored-src/src/query.ts`
  - `docs/claude-code-sourcemap-main/restored-src/src/services/tools/toolOrchestration.ts`
  - `docs/claude-code-sourcemap-main/restored-src/src/services/tools/StreamingToolExecutor.ts`
  - `docs/claude-code-sourcemap-main/restored-src/src/tools/AgentTool/runAgent.ts`
  - `docs/claude-code-sourcemap-main/restored-src/src/tools/AgentTool/forkSubagent.ts`
  - `docs/claude-code-sourcemap-main/restored-src/src/tools/shared/spawnMultiAgent.ts`
  - `docs/claude-code-sourcemap-main/restored-src/src/utils/agentContext.ts`
  - `docs/claude-code-sourcemap-main/restored-src/src/utils/permissions/permissions.ts`
  - `docs/claude-code-sourcemap-main/restored-src/src/memdir/memoryTypes.ts`
  - `docs/claude-code-sourcemap-main/restored-src/src/memdir/findRelevantMemories.ts`
- Anthropic documentation
  - [Claude Code overview](https://code.claude.com/docs/en/overview)
  - [Memory](https://code.claude.com/docs/en/memory)
  - [MCP](https://code.claude.com/docs/en/mcp)
  - [Settings](https://code.claude.com/docs/en/settings)
  - [Skills and slash commands](https://code.claude.com/docs/en/slash-commands)
  - [Sub-agents](https://docs.anthropic.com/en/docs/claude-code/sub-agents)
  - [Agent SDK overview](https://code.claude.com/docs/en/agent-sdk/overview)
- OpenClaw documentation
  - [Architecture](https://docs.openclaw.ai/concepts/architecture)
  - [Skills](https://docs.openclaw.ai/tools/skills)
  - [Plugins](https://docs.openclaw.ai/tools/plugin)
  - [Plugin Bundles](https://docs.openclaw.ai/plugins/bundles)
  - [TaskFlow](https://docs.openclaw.ai/automation/taskflow)
  - [ClawHub](https://docs.openclaw.ai/tools/clawhub)

## Non-Negotiable Defaults

The rebuild adopts these defaults:

- no compatibility-driven design
- no permanent legacy shim paths
- no dual execution trunk
- no prompt-centric agent execution model
- no demo or minimum-scope implementation posture
- no new god modules or oversized single-file orchestrators

`crates/octopus-runtime-adapter` may remain as a transport and persistence facade, but it must stop being an independent execution brain.

## Current Structural Gaps

The current system has eight structural problems that this rebuild must explicitly fix:

1. Execution still has two trunks.
   - `crates/octopus-runtime-adapter` still drives session execution through one-shot request handling.
   - `crates/runtime` and `crates/tools` already contain more advanced capability and loop primitives, but they are not the only path.
2. `crates/octopus-runtime-adapter/src/turn_submit.rs` still constructs state around a single provider response plus a simple approval insert.
3. `packages/schema/src/workspace-plane.ts` keeps `AgentRecord` and `UpsertAgentInput` too thin for runtime use. They remain prompt and tool reference objects, not executable manifests.
4. Existing bundle manifests package agents, teams, skills, and MCP servers, but they do not yet carry dependency, trust, translation, policy, or runtime semantics.
5. Team assets still skew toward membership lists instead of explicit team topology and worker lifecycle semantics.
6. Durable memory is not yet hard-separated from conversation summaries and session projection.
7. Session, event, trace, and summary projections are not yet designed around strict reconstructability.
8. Existing runtime planning documents are still too runtime-centric. They under-specify the asset plane, workflow plane, registry, and plugin or bridge boundaries.

## Canonical Platform Model

The canonical architecture has three planes:

1. `Asset Plane`
2. `Runtime Plane`
3. `Experience Plane`

No cross-plane bypass is allowed. The runtime consumes compiled manifests and policy snapshots. Product surfaces consume public contracts and projections. Asset import or export never bypasses native runtime contracts.

### Asset Plane

The asset plane owns portable definitions, trust, and lifecycle metadata.

#### Asset Kinds

The canonical asset kinds are:

- `agent`
- `team`
- `skill`
- `mcp-server`
- `plugin`
- `workflow-template`

These are durable product assets. They are not runtime session state.

#### AssetBundleManifest v2

`AssetBundleManifest v2` is the canonical import or export envelope.

It must include:

- `version`
- `bundleRoot`
- declared assets and their source IDs
- `dependencies`
- `trustMetadata`
- `compatibilityMapping`
- `policyDefaults`
- `importDiagnostics`
- optional registry metadata such as tags, publisher, revision, and release channel

Bundle import is translation-based:

- import into native Octopus assets
- downgrade unsupported constructs explicitly
- reject unsafe or untranslatable constructs explicitly

The platform must never execute foreign runtime semantics by accident during import.

#### Registry And Trust

The asset plane must support a registry model for:

- publishing
- versioning
- discovery
- trust review
- installation state
- health state
- dependency state

Trust metadata is mandatory for imported assets. Runtime planning may consume trust state, but trust is authored and managed in the asset plane.

#### Actor Manifest Compilation Boundary

Runtime execution never consumes raw asset rows.

Runtime always consumes compiled manifests:

- `ActorManifest` for agents
- `TeamManifest` for teams

Compilation happens at session start and produces a revisioned, immutable snapshot.

### Runtime Plane

The runtime plane is the only execution brain.

It owns:

- session and run lifecycle
- manifest compilation results
- turn context
- capability planning
- model loop
- approval and auth mediation
- tool, skill, MCP, and plugin execution
- subruns and workflow runs
- memory selection and write proposals
- append-only events and runtime projections

#### Session, Run, And Subrun Model

The runtime uses these durable containers:

- `Session`: a conversation lane bound to project, actor selection, model choice, permission ceiling, and config snapshot
- `Run`: a concrete execution attempt inside a session
- `Subrun`: a delegated run with lineage to its parent run and the tool call that spawned it

Each session freezes:

- `projectId`
- `conversationId`
- `selectedActorRef`
- `selectedConfiguredModelId`
- `executionPermissionMode`
- `configSnapshotId`
- `manifestRevision`
- `sessionPolicyRevision`

Each run or subrun carries:

- `sessionId`
- `runId`
- `runKind`
- `parentRunId`
- `actorRef`
- `delegatedByToolCallId`
- `approvalState`
- `usageSummary`
- `artifactRefs`
- `traceContext`

Only one foreground primary run may be active per session. Team workers and workflow workers may run concurrently as subruns under the same policy ceiling.

#### Canonical Runtime Flow

Every primary run follows the same flow:

1. `SessionStart`
2. `ConfigSnapshotResolve`
3. `ActorManifestCompile`
4. `SessionPolicyFreeze`
5. `TurnContextBuild`
6. `CapabilityExecutionPlanBuild`
7. `MemorySelection`
8. `ModelLoop`
9. `ApprovalOrAuthMediation`
10. `ToolOrSkillOrMcpExecute`
11. `EventPersist`
12. `RunFinalize`

The runtime loop is stateful, resumable, and multi-turn.

The runtime must support:

- streaming and partial events
- ordered tool concurrency
- retry and degraded result handling
- context compaction and resume
- max-turn guard
- budget and usage tracking

The runtime must not degrade back to stateless one-shot provider execution.

#### Capability System

The runtime capability model stays unified across all sources.

The canonical types are:

- `CapabilitySpec`
- `CapabilityExecutionPlan`
- `SessionCapabilityState`
- `CapabilityExecutor`

Supported provider families are fixed to:

- builtin
- skill
- MCP
- plugin

Planning is deny-before-expose.

The model only sees `visible_tools` and discoverable skills from the current plan. No legacy registry or dispatch path may bypass planning.

#### Skill Model

Skill remains a prompt capability, not a plain function tool.

Skill execution must return structured runtime output:

- `messagesToInject`
- `toolGrants`
- `modelOverride`
- `effortOverride`
- `stateUpdates`

Skill sources may be:

- local
- bundled
- plugin-provided
- MCP-provided

All skill execution must pass through the same runtime contract and approval model.

#### MCP And Plugin Boundaries

MCP is only a capability provider and bridge boundary.

It may provide:

- tool capabilities
- prompt or skill capabilities
- resource capabilities

MCP connection state directly affects capability surface:

- `ready`
- `pending`
- `authRequired`
- `approvalRequired`
- `degraded`
- `unavailable`

Plugin is a governed extension surface. Plugins may provide:

- capability providers
- channels
- hooks
- routes
- workflow templates

Plugins must still compile into native runtime contracts. They do not create sidecar execution trunks.

#### Team And Workflow Orchestration

`TeamManifest` is not a prompt variant. V1 is fixed to `leader-orchestrated`.

The team model must declare:

- leader actor
- member actors
- allowed delegation edges
- worker concurrency ceiling
- mailbox rules
- artifact handoff rules
- shared capability policy
- shared memory scope
- workflow affordances

The runtime must support:

- `spawn worker`
- `resume worker`
- `cancel worker`
- `background worker`
- mailbox handoff
- artifact handoff
- workflow lineage

Workflow and background tasks are first-class runtime objects. They are not future plugin-only additions.

#### Permission, Approval, And Auth

The platform has two different permission systems and must model both explicitly.

1. `Business Authorization`
   - decides whether a workspace, project, user, or asset may use an actor, capability family, external system, or memory scope
2. `Execution Permission Mode`
   - decides what the current session or run may do at execution time

The session-selected execution mode is the ceiling. Turn-level input may narrow that ceiling, but may not silently widen it.

Approval and auth handling is centralized in one broker. It must cover:

- tool execution approval
- MCP auth or elicitation
- memory write approval
- team spawn or escalation approval
- workflow escalation approval

Approval and auth are runtime mediation layers, not UI glue.

#### Memory And Knowledge

Durable memory is separate from conversation projection.

Durable memory types are fixed to:

- `user`
- `feedback`
- `project`
- `reference`

Durable scopes are fixed to:

- `user-private`
- `agent-private`
- `team-shared`
- `project-shared`
- `workspace-shared`

The runtime memory lifecycle is:

1. deterministic filter by scope, actor, project, freshness, and policy
2. side-model relevance selection with a bounded top-N result
3. runtime injection of memory summaries and freshness metadata
4. proposal-only memory write candidate generation
5. approval or policy review
6. durable save, rejection, or revalidation

Conversation checkpoints and summaries are projections. They are not durable memory records.

The runtime must never store the following as durable memory:

- code structure
- file paths and architecture that can be derived from the repo
- git history
- temporary task state
- current conversation noise
- data derivable from config or source control

#### Persistence And Observability

Persistence follows repository governance exactly.

`data/main.db` stores queryable projections and indexes only:

- sessions
- runs and subruns
- approvals
- manifest revisions
- memory metadata
- artifact metadata
- blob metadata

Disk stores large or primary bodies:

- `data/knowledge`
- `data/artifacts`
- `data/blobs`

Append-only runtime lifecycle events remain under:

- `runtime/events/*.jsonl`

Trace, telemetry, summary, and session projection are separate views over the same runtime facts. Active state must be reconstructable from SQLite projections, append-only events, and disk-backed bodies.

### Experience Plane

The experience plane is the public product surface over the asset and runtime planes.

It covers:

- desktop sessions
- browser-host sessions
- Tauri-host sessions
- channel or task entrypoints
- workflow and background task views
- artifact views
- approval queues
- memory and capability summaries

The experience plane does not implement execution logic.

#### Domain And Task Model

Coding is only one task domain.

The public task model must describe:

- task type
- input carriers
- external system dependencies
- output or artifact expectations
- approval requirements
- candidate actor and team routing
- capability surface summary

The same runtime must serve coding and non-coding tasks.

#### Output And Artifact Contract

Every actor and team must be able to declare output expectations:

- final answer contract
- artifact kinds
- handoff expectations
- background completion signals

Artifacts are first-class runtime outputs, not arbitrary byproducts.

#### Gateway Direction

The platform direction is gateway-first:

- one canonical runtime contract
- many product surfaces
- one asset registry
- one workflow substrate

This matches the OpenClaw lesson that the platform center of gravity must be the gateway and control plane, not an individual UI shell.

## Manifest Models

### ActorManifest

`ActorManifest` is the canonical compiled form for an agent.

It must include:

- identity and display metadata
- task-domain profile
- system prompt sections
- default model strategy
- default effort strategy
- capability policy
- permission envelope
- memory policy
- delegation policy
- approval preferences
- output contract
- selected builtin, skill, MCP, and plugin bindings

### TeamManifest

`TeamManifest` is the canonical compiled form for a team.

It must include:

- all `ActorManifest` requirements for the leader
- team topology
- member manifest references
- shared capability policy
- shared memory policy
- mailbox rules
- artifact handoff rules
- workflow affordances
- worker concurrency rules

The runtime must compile these manifests once at session start and reference them by revision.

## Public Contract Program

The rebuild requires public contract growth in at least these feature groups:

- `actor-manifest`
- `agent-runtime`
- `memory-runtime`
- `asset-bundle`
- `workflow-runtime`
- `runtime-policy`

The runtime transport must grow to include:

- session-level actor selection and manifest metadata
- session policy and capability summaries
- run and subrun lineage
- workflow lineage
- approval-layer metadata
- memory selection and proposal summaries
- planner, model, tool, skill, MCP, subrun, workflow, memory, approval, and trace event families
- import or export translation diagnostics

Knowledge and memory CRUD stay in the knowledge and workspace planes. Runtime only exposes selection, proposal, review, and projection state.

## Engineering Boundaries

The rebuild must enforce these structural rules:

- use state machines for turn execution and workflow execution
- use compilers for manifest and policy freeze steps
- use policy objects for business auth, execution permission, and memory write rules
- use provider strategies for builtin, skill, MCP, and plugin sources
- use brokers or mediators for approval and auth
- use projectors for session, trace, telemetry, and summary views
- keep feature contracts split into feature-based schema files

The rebuild must forbid:

- god modules
- oversized single-file orchestrators
- adapter-side business logic growth
- direct cross-plane mutation
- revival of a second execution trunk
- persistence of legacy compatibility shells beyond migration cutover

## Migration Direction

The migration order is fixed:

1. land this platform design and the paired implementation plan
2. rebuild asset contracts and bundle translation
3. introduce unified runtime core types and manifest compilation
4. move single-agent execution onto the new loop
5. add team, workflow, and background subruns
6. rebuild the memory plane
7. merge policy and approval handling into the planner and broker
8. update public contracts and host projections
9. delete duplicate legacy execution paths

During migration:

- all new runtime features must land only on the new trunk
- browser host and Tauri host must keep the same public contract shapes
- no phase is complete until its legacy predecessor is either removed or clearly fenced from future growth
