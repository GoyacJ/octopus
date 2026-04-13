# Capability Runtime And Management Contract

This document records the canonical tools runtime path after the capability-runtime cutover.

General HTTP and OpenAPI policy lives in `docs/api-openapi-governance.md`. Runtime-config ownership and transport behavior live in `docs/runtime_config_api.md`. This document covers runtime capability planning, execution, and management projection only.

## Canonical Runtime Path

The runtime canonical flow is:

1. Providers emit canonical capability descriptors.
2. The planner builds a `CapabilityExecutionPlan` before each model request.
3. The model only sees `visible_tools` from that plan.
4. Skill discovery and resource availability are read from the same plan.
5. Execution resolves a `CapabilityHandle` and dispatches through a runtime-owned executor.

Canonical runtime types live in:

- `crates/tools/src/capability_runtime/provider.rs`
  - `CapabilitySpec`
  - `CapabilityRuntime`
  - `CapabilityTrustProfile`
  - `CapabilityScopeConstraints`
- `crates/tools/src/capability_runtime/planner.rs`
  - `CapabilityExecutionPlan`
  - `CapabilitySurfaceProjection`
  - `CapabilityPlanner`
- `crates/tools/src/capability_runtime/executor.rs`
  - `CapabilityExecutor`
  - `CapabilityDispatchKind`
- `crates/tools/src/capability_runtime/state.rs`
  - `SessionCapabilityState`
  - `SessionCapabilityStore`
- `crates/tools/src/capability_runtime/events.rs`
  - `CapabilityExecutionEvent`
  - `CapabilityExecutionRequest`
- `crates/tools/src/skill_runtime.rs`
  - `PromptSkillExecutor`
  - `SkillExecutionResult`

The runtime provider split is fixed to four source families:

- builtin
- local or bundled skill
- plugin
- MCP

Providers compile into `CapabilitySpec`. Runtime planning and execution must not branch on legacy registry types.

## Planning Contract

`CapabilityExecutionPlan` is the only canonical per-turn runtime projection. It contains:

- `visible_tools`
- `deferred_tools`
- `discoverable_skills`
- `available_resources`
- `hidden_capabilities`
- `activated_tools`
- `granted_tools`
- `pending_tools`
- `approved_tools`
- `auth_resolved_tools`
- `provider_fallbacks`

Planning is deny-before-expose:

- trust, permission, approval, auth, and runtime health gates apply before a capability is exposed to the model
- `visible_tools` is the only tool surface sent to providers
- prompt skills are discoverable only when a runtime executor exists
- resources are runtime-first capabilities and are planned separately from tool exposure

`SessionCapabilityState` is runtime-only session state. It stores:

- activation
- tool grants
- pending and approval state
- auth resolution state
- injected skill messages
- skill state updates
- model override
- reasoning effort override

It does not own asset catalogs or management-plane metadata.

## Prompt Skill Execution

`PromptSkillExecutor` is the single prompt-skill execution facade.

The allowed prompt-skill sources are:

- local skill
- bundled skill
- plugin skill
- MCP prompt

Every prompt-skill execution path must return `SkillExecutionResult`, including:

- injected messages
- tool grants
- model override
- effort override
- state updates

Plugin skills and MCP prompts must not bypass this contract with implicit shell injection.

## Provider Fallback Rules

Provider fallback is request-time translation only.

The current public rule is:

- when a ready resource capability carries a `provider_key`, the plan may record a provider fallback entry in `provider_fallbacks`
- the fallback is an adapter-side resource shim for the request path only

Fallback shims are not:

- builtin capabilities
- persistent session state
- management projection entries
- public replacement APIs for runtime resources

## Management Projection Contract

Capability management is capability-aware instead of catalog-family-aware.

`CapabilityManagementProjection` is split into:

- `entries`: capability-level rows
- `assets`: asset-level grouped manifests
- `skill_packages`
- `mcp_server_packages`

Projection rows must distinguish:

- `source_kind`
- `execution_kind`
- `capability_id`
- `resource_uri` when applicable

Asset and server package grouping must aggregate:

- `source_kinds`
- `execution_kinds`
- `prompt_names`
- `resource_uris`

Management projection is derived from asset and config state only. Session activation, grants, or per-turn runtime state must not mutate management projection results.

## Retired Entrypoints

These entrypoints are retired and must not return to the main runtime path:

- `global_mcp_registry()`
- `run_mcp_tool`
- `run_list_mcp_resources`
- `run_read_mcp_resource`
- builtin dispatch entries for `MCP`, `ListMcpResources`, and `ReadMcpResource`
- runtime use of `ToolRegistry` as a discovery or dispatch hub
- `McpToolRegistry`

`ToolRegistry` remains only as a static builtin manifest/helper surface.

## Reference

The design and migration rationale remain in `docs/plans/tools-runtime.md`.
