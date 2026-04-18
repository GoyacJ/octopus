# Model Runtime Refactor Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Rebuild Octopus's model runtime into a registry-first, driver-based, policy-separated architecture with no compatibility shims and no unsupported catalog/runtime drift.

**Architecture:** Keep the existing `catalog -> configured model -> session policy -> runtime loop` backbone, but split provider-specific execution out of `executor.rs` into explicit protocol drivers. Add first-class `canonical model policy`, `auth resolution`, and `request policy` layers so the runtime loop only manages turn state, tool state, approvals, and event projection.

**Tech Stack:** Rust 2021, `octopus-core`, `octopus-runtime-adapter`, `api`, OpenAPI schemas under `contracts/openapi`, generated TS schema via `@octopus/schema`, Vue 3 desktop store/typecheck, `cargo test`, `cargo clippy`, `pnpm schema:generate`.

---

## Design Decisions

1. `octopus-core` remains the source of truth for catalog contracts and resolved execution targets.
2. `octopus-runtime-adapter` remains the runtime composition root, but no longer owns protocol-specific request assembly directly.
3. `api` becomes a protocol client library, not a model registry or alias policy owner.
4. Unsupported protocol families must not appear as normal executable choices in the catalog.
5. `simple completion` and `tool-enabled conversation runtime` are separate paths.
6. No compatibility wrappers remain after the refactor; rename, move, or delete modules instead of layering aliases.
7. Any `/api/v1/*` payload change must follow `contracts/openapi/src/** -> pnpm openapi:bundle -> pnpm schema:generate -> server/store/tests`.

## Architecture Principle Baseline

This refactor is not only a feature delivery task. It exists to make the model module satisfy the architectural baseline of `modular`, `high cohesion`, `low coupling`, and `deliberate pattern use`.

The current codebase already has the correct outer backbone:

- `catalog -> configured model -> session policy -> runtime loop`
- registry construction is separate from runtime turn execution
- resolved execution target is separate from frontend-facing catalog records

But the inner execution layer does not yet satisfy the baseline:

- `crates/octopus-runtime-adapter/src/executor.rs` is still a protocol-family switchboard instead of a true driver system
- `crates/octopus-runtime-adapter/src/execution_target.rs` still mixes execution target resolution with credential hydration
- `crates/api/src/providers/mod.rs` still owns alias and default-selection-like policy that should be canonical elsewhere
- `crates/octopus-runtime-adapter/src/agent_runtime_core.rs` is an oversized orchestrator and must not continue accumulating provider-specific concerns
- catalog declarations and runtime support still drift for some protocol families

The implementation in this plan must therefore optimize for structural quality first, not only for incremental functionality.

## Required Structural Rules

The end-state module graph must obey these rules:

1. `octopus-core`
   Owns contracts only. It may define catalog, execution target, and request-policy data structures, but it must not choose defaults, resolve secrets, or know transport details.
2. `registry`
   Owns model/provider/configured-model assembly only. It may validate records and resolve an executable target, but it must not hydrate secrets or assemble HTTP requests.
3. `canonical model policy`
   Owns default model IDs, canonical aliases, and fallback selection rules. No other module may silently redefine these decisions.
4. `auth resolution`
   Owns credential-source interpretation and secret or env expansion. It must be executable and testable without invoking the runtime loop.
5. `request policy`
   Owns base URL, headers, auth header mode, timeout, and other transport-facing request decisions. It must not be hidden inside driver implementations.
6. `driver registry`
   Owns protocol-family lookup only. It must be fail-closed and return an error for any unsupported family.
7. `protocol drivers`
   Own only provider-protocol request assembly, response normalization, and protocol-specific stream translation into runtime events.
8. `agent runtime core`
   Owns loop orchestration only: planning, approvals, tool progression, iteration boundaries, event persistence, and projection. It must not embed provider-specific payload branches.
9. `api`
   Owns protocol clients and transport helpers only. It must not become a second model registry, default policy source, or runtime selection engine.

## Preferred Patterns And Explicit Anti-Patterns

The refactor should use the following patterns deliberately:

- `Registry` for catalog assembly and target lookup
- `Strategy` for protocol drivers selected by `protocol_family`
- `Policy Object` for canonical model selection, auth resolution, and request transport rules
- `Facade` at the runtime adapter boundary only
- `Compiler` for snapshot or policy freeze steps

The refactor must explicitly avoid these anti-patterns:

- god modules
- switch-based protocol orchestration as the long-term architecture
- duplicated canonical defaults across `registry`, `api`, and runtime code
- hidden fallback behavior that only exists in one layer
- catalog entries that look selectable but are not actually executable

## Structural Success Conditions

The refactor is only complete if the following architectural questions can be answered clearly:

- When a new protocol family is added, only the driver registry, one driver module, and explicit execution-support metadata need to change.
- When credential rules change, only auth resolution and its tests need to change.
- When base URL or header precedence changes, only request policy and its tests need to change.
- When default model policy changes, only the canonical model policy source needs to change.
- The runtime loop can be read without needing to understand provider-specific HTTP payload details.

## Target Module Map

### `crates/octopus-runtime-adapter/src/model_runtime/`

- `mod.rs`
- `driver.rs`
- `driver_registry.rs`
- `canonical_model_policy.rs`
- `execution_support.rs`
- `auth.rs`
- `request_policy.rs`
- `simple_completion.rs`
- `drivers/mod.rs`
- `drivers/anthropic_messages.rs`
- `drivers/openai_chat.rs`
- `drivers/openai_responses.rs`
- `drivers/gemini_native.rs`

### `crates/octopus-runtime-adapter/tests/`

- `registry_execution_support.rs`
- `canonical_model_policy.rs`
- `model_auth_resolution.rs`
- `request_policy_resolution.rs`
- `protocol_drivers.rs`
- `simple_completion.rs`
- `runtime_turn_loop.rs`

### Contract/UI files likely to change

- `crates/octopus-core/src/lib.rs`
- `contracts/openapi/src/components/schemas/catalog.yaml`
- `contracts/openapi/src/components/schemas/runtime.yaml`
- `contracts/openapi/src/paths/catalog.yaml`
- `contracts/openapi/src/paths/runtime.yaml`
- `crates/octopus-server/src/workspace_runtime.rs`
- `apps/desktop/src/tauri/runtime_api.ts`
- `apps/desktop/src/tauri/workspace-client.ts`
- `apps/desktop/src/stores/catalog_normalizers.ts`
- `apps/desktop/src/stores/catalog.ts`
- `apps/desktop/src/stores/runtime_actions.ts`
- `apps/desktop/src/views/workspace/ModelsView.vue`
- `apps/desktop/src/views/workspace/ModelsTablePanel.vue`
- `apps/desktop/src/views/workspace/ModelDetailsDialog.vue`
- `apps/desktop/src/views/workspace/useModelsDraft.ts`
- `apps/desktop/src/views/workspace/models-security.ts`

## End State

- Catalog explicitly tells the UI which provider/model/surface combinations are executable now.
- Canonical model IDs and defaults come from one policy source.
- Runtime auth and request transport policy are independently testable.
- Protocol drivers are swappable strategies keyed by `protocol_family`.
- `agent_runtime_core.rs` owns loop orchestration only.
- Workspace console model management uses a true `list/detail` workbench surface instead of a `table + modal editor`.
- Credential source, validation health, and runtime executability are first-class UI state, not hidden implementation detail.
- Managed credentials persist as secure references only, with explicit provider inheritance and model override semantics.
- `vendor_native` and `realtime` are either fully implemented or removed from baseline declarations.

## Workspace Console Model UX And Credential Direction

This refactor includes the workspace console model surface. The current `ModelsView -> ModelsTablePanel -> ModelDetailsDialog -> useModelsDraft` flow is functional, but it is still a transactional `table + dialog` editor. That is not the target architecture for a long-lived operational model workbench.

The desktop design baseline already says model management belongs to the `List / Detail Page` archetype, and `docs/design/DESIGN.md` explicitly says the goal is not to copy Notion's visual language. The correct borrowing from Notion is information architecture and interaction quality only: stable browse context, progressive disclosure, persistent detail context, and fewer interruptive modals.

### Required UX Rules

1. The workspace model page must become a two-pane `list/detail` workbench surface.
2. The left pane owns browsing and selection only: model name, provider, enabled state, credential state, validation health, and execution support.
3. The right pane owns editing and inspection only. It should be sectioned into `Overview`, `Authentication`, `Routing`, `Quota`, and `Validation`.
4. The UI must distinguish:
   - declared model capability
   - executable runtime support
   - credential source and health
   - validation or reachability status
5. Create and edit flows must land in persistent context, not rely on a modal as the primary editor.
6. Validation must become durable page state with visible last-known result, not only a toast or one-shot success message.
7. The page must follow `docs/design/DESIGN.md` list/detail rules and `Calm Intelligence` language. No visual imitation of Notion is allowed.

### Required Credential Architecture Rules

1. Runtime config files remain reference-only for secrets. No new plaintext `credentialRef` write path is allowed.
2. OS secure storage plus `secret-ref:*` remains the managed-secret baseline.
3. Auth source must become explicit domain data, at minimum distinguishing:
   - managed secret
   - environment reference
   - provider-inherited credential
   - configured-model override
4. Provider credential fallback and model override precedence must be visible in contracts and UI, not hidden only inside registry resolution.
5. Unsupported reference kinds must not appear as supported UX choices until the runtime resolver can actually execute them.
6. Managed credential write plus config persistence must be atomic or compensating. Failed config save must not leave orphaned secure-store entries behind.
7. Inline plaintext migration remains one-way and fail-closed. The UI must surface migration failure or blocked state explicitly.

These rules apply to both:

- configured model `credentialRef`
- provider-level `credentialRefs.*`

In this plan, `provider-inherited credential` means the runtime-config-backed provider credential path consumed by the model registry. It does not mean the separate workspace-plane `ProviderCredentialRecord` list API unless a later task explicitly unifies those surfaces.

### Planning Consequences

- Task 4 must cover auth source modeling, secret lifecycle, and atomic or compensating credential persistence.
- Task 10 must cover the workspace console model page restructure into `list/detail`, plus explicit credential source and validation-health presentation.
- Task 4 must add fail-closed tests for unsupported reference schemes instead of only happy-path env or managed-secret resolution.
- Any contract change required for credential source, secret presence, or validation metadata must follow the existing OpenAPI and schema generation order in this plan.

### Task 1: Create the New Runtime Driver Skeleton

**Files:**
- Create: `crates/octopus-runtime-adapter/src/model_runtime/mod.rs`
- Create: `crates/octopus-runtime-adapter/src/model_runtime/driver.rs`
- Create: `crates/octopus-runtime-adapter/src/model_runtime/driver_registry.rs`
- Create: `crates/octopus-runtime-adapter/tests/protocol_drivers.rs`
- Modify: `crates/octopus-runtime-adapter/src/lib.rs`
- Delete later: `crates/octopus-runtime-adapter/src/executor.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn driver_registry_rejects_unknown_protocol_family() {
    let registry = ModelDriverRegistry::new(vec![]);
    let result = registry.driver_for("unknown_protocol");
    assert!(result.is_err());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p octopus-runtime-adapter --test protocol_drivers driver_registry_rejects_unknown_protocol_family -- --exact`
Expected: FAIL with unresolved `ModelDriverRegistry` or missing module errors.

**Step 3: Write minimal implementation**

```rust
pub trait ProtocolDriver: Send + Sync {
    fn protocol_family(&self) -> &'static str;
}

pub struct ModelDriverRegistry {
    drivers: HashMap<&'static str, Arc<dyn ProtocolDriver>>,
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p octopus-runtime-adapter --test protocol_drivers driver_registry_rejects_unknown_protocol_family -- --exact`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/octopus-runtime-adapter/src/lib.rs \
  crates/octopus-runtime-adapter/src/model_runtime \
  crates/octopus-runtime-adapter/tests/protocol_drivers.rs
git commit -m "refactor: add model runtime driver skeleton"
```

### Task 2: Make Execution Support Explicit in the Catalog Contract

**Files:**
- Modify: `crates/octopus-core/src/lib.rs`
- Modify: `crates/octopus-runtime-adapter/src/registry.rs`
- Modify: `crates/octopus-runtime-adapter/src/registry_baseline.rs`
- Modify: `contracts/openapi/src/components/schemas/catalog.yaml`
- Modify: `contracts/openapi/src/paths/catalog.yaml`
- Modify: `crates/octopus-server/src/workspace_runtime.rs`
- Modify: `apps/desktop/src/stores/catalog_normalizers.ts`
- Test: `crates/octopus-runtime-adapter/tests/registry_execution_support.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn catalog_marks_tool_runtime_support_per_surface() {
    let snapshot = build_test_catalog_snapshot();
    let minimax = snapshot.models.iter().find(|m| m.model_id == "MiniMax-M2.7").unwrap();
    assert_eq!(minimax.surface_bindings[0].runtime_support.tool_loop, true);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p octopus-runtime-adapter --test registry_execution_support catalog_marks_tool_runtime_support_per_surface -- --exact`
Expected: FAIL because `runtime_support` does not exist.

**Step 3: Write minimal implementation**

```rust
pub struct RuntimeExecutionSupport {
    pub prompt: bool,
    pub conversation: bool,
    pub tool_loop: bool,
    pub streaming: bool,
}
```

Update the registry to compute this fail-closed from the installed protocol drivers, not from baseline declarations. Unsupported protocol families must either:

- be removed from seeded default selections, or
- be present but marked unavailable and filtered out of normal picker UI.

**Step 4: Run test to verify it passes**

Run:

```bash
cargo test -p octopus-runtime-adapter --test registry_execution_support
pnpm openapi:bundle
pnpm schema:generate
pnpm -C apps/desktop typecheck
```

Expected: Rust tests PASS, OpenAPI bundles cleanly, desktop typecheck PASS.

**Step 5: Commit**

```bash
git add crates/octopus-core/src/lib.rs \
  crates/octopus-runtime-adapter/src/registry.rs \
  crates/octopus-runtime-adapter/src/registry_baseline.rs \
  crates/octopus-runtime-adapter/tests/registry_execution_support.rs \
  contracts/openapi/src/components/schemas/catalog.yaml \
  contracts/openapi/src/paths/catalog.yaml \
  crates/octopus-server/src/workspace_runtime.rs \
  apps/desktop/src/stores/catalog_normalizers.ts
git commit -m "refactor: expose executable runtime support in model catalog"
```

### Task 3: Centralize Canonical Model Policy and Remove Alias Drift

**Files:**
- Create: `crates/octopus-runtime-adapter/src/model_runtime/canonical_model_policy.rs`
- Modify: `crates/octopus-runtime-adapter/src/registry_baseline.rs`
- Modify: `crates/octopus-runtime-adapter/src/registry.rs`
- Modify: `crates/api/src/providers/mod.rs`
- Test: `crates/octopus-runtime-adapter/tests/canonical_model_policy.rs`
- Test: `crates/api/src/client.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn canonical_policy_and_registry_defaults_match() {
    let policy = CanonicalModelPolicy::default();
    assert_eq!(policy.default_conversation_model(), "claude-sonnet-4-5");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p octopus-runtime-adapter --test canonical_model_policy canonical_policy_and_registry_defaults_match -- --exact`
Expected: FAIL with missing policy type or mismatched default.

**Step 3: Write minimal implementation**

```rust
pub struct CanonicalModelPolicy {
    pub conversation_default: &'static str,
    pub fast_default: &'static str,
    pub responses_default: &'static str,
}
```

Then make `registry_baseline.rs` consume this policy. Reduce `crates/api/src/providers/mod.rs` to provider metadata and token limits only; do not let it pick runtime defaults independently.

**Step 4: Run test to verify it passes**

Run:

```bash
cargo test -p octopus-runtime-adapter --test canonical_model_policy
cargo test -p api client::tests -- --nocapture
```

Expected: PASS; no alias/default mismatch remains.

**Step 5: Commit**

```bash
git add crates/octopus-runtime-adapter/src/model_runtime/canonical_model_policy.rs \
  crates/octopus-runtime-adapter/src/registry_baseline.rs \
  crates/octopus-runtime-adapter/src/registry.rs \
  crates/octopus-runtime-adapter/tests/canonical_model_policy.rs \
  crates/api/src/providers/mod.rs \
  crates/api/src/client.rs
git commit -m "refactor: centralize canonical model policy"
```

### Task 4: Introduce a Dedicated Model Auth Resolution Layer

**Files:**
- Create: `crates/octopus-runtime-adapter/src/model_runtime/auth.rs`
- Modify: `crates/octopus-runtime-adapter/src/execution_target.rs`
- Modify: `crates/octopus-runtime-adapter/src/secret_store.rs`
- Modify: `crates/octopus-runtime-adapter/src/config_service.rs`
- Modify: `crates/octopus-runtime-adapter/src/runtime_config.rs`
- Modify: `crates/octopus-runtime-adapter/src/registry.rs`
- Modify: `crates/octopus-runtime-adapter/src/registry_resolution.rs`
- Modify: `crates/octopus-core/src/lib.rs`
- Modify: `contracts/openapi/src/components/schemas/runtime.yaml`
- Modify: `contracts/openapi/src/paths/runtime.yaml`
- Modify: `crates/octopus-server/src/workspace_runtime.rs`
- Modify: `apps/desktop/src/tauri/runtime_api.ts`
- Modify: `apps/desktop/src/tauri/workspace-client.ts`
- Test: `crates/octopus-runtime-adapter/tests/model_auth_resolution.rs`

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn resolves_secret_ref_and_env_ref_into_runtime_auth() {
    let auth = resolve_model_auth(test_target()).await.unwrap();
    assert_eq!(auth.mode, AuthMode::BearerToken);
}

#[tokio::test]
async fn rejects_unsupported_reference_schemes_fail_closed() {
    let result = resolve_model_auth(test_target_with_ref("op://vault/item")).await;
    assert!(result.unwrap_err().to_string().contains("unsupported credential reference"));
}

#[test]
fn reports_provider_inherited_auth_source_explicitly() {
    let auth = resolve_model_auth_source(test_provider_inherited_target()).unwrap();
    assert_eq!(auth.source, "provider_inherited");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p octopus-runtime-adapter --test model_auth_resolution resolves_secret_ref_and_env_ref_into_runtime_auth -- --exact`
Expected: FAIL because `resolve_model_auth` does not exist.

**Step 3: Write minimal implementation**

```rust
pub struct ResolvedModelAuth {
    pub mode: AuthMode,
    pub credential: String,
    pub source: String,
}
```

Move secret hydration out of the current ad-hoc `hydrate_execution_target_credentials()` flow and replace it with an explicit resolver returning `ResolvedModelAuth`.

The first complete version of this layer must also:

- classify the resolved auth source explicitly so provider-inherited credentials and model overrides are not hidden behavior
- keep `secret-ref:*` and `env:*` as the only first-class executable reference kinds unless additional resolvers are implemented
- handle both configured-model `credentialRef` and provider-level `credentialRefs.*` under the same auth-source model, instead of fixing only one branch
- migrate or explicitly block plaintext provider-level credential refs as part of the same fail-closed rule
- move credential persistence out of the frontend-led two-step flow and into an atomic or compensating backend boundary so a failed config save does not leak orphaned managed secrets

**Step 4: Run test to verify it passes**

Run:

```bash
cargo test -p octopus-runtime-adapter --test model_auth_resolution
pnpm openapi:bundle
pnpm schema:generate
pnpm -C apps/desktop typecheck
```

Expected: PASS

**Step 5: Commit**

```bash
git add crates/octopus-runtime-adapter/src/model_runtime/auth.rs \
  crates/octopus-runtime-adapter/src/execution_target.rs \
  crates/octopus-runtime-adapter/src/secret_store.rs \
  crates/octopus-runtime-adapter/tests/model_auth_resolution.rs
git commit -m "refactor: add dedicated model auth resolution"
```

### Task 5: Introduce a Dedicated Request Policy Layer

**Files:**
- Create: `crates/octopus-runtime-adapter/src/model_runtime/request_policy.rs`
- Modify: `crates/octopus-core/src/lib.rs`
- Modify: `crates/octopus-runtime-adapter/src/registry.rs`
- Test: `crates/octopus-runtime-adapter/tests/request_policy_resolution.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn request_policy_prefers_configured_base_url_then_surface_default() {
    let policy = resolve_request_policy(test_target(), test_auth()).unwrap();
    assert_eq!(policy.base_url, "https://api.minimaxi.com/anthropic");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p octopus-runtime-adapter --test request_policy_resolution request_policy_prefers_configured_base_url_then_surface_default -- --exact`
Expected: FAIL because `ResolvedRequestPolicy` does not exist.

**Step 3: Write minimal implementation**

```rust
pub struct ResolvedRequestPolicy {
    pub base_url: String,
    pub headers: BTreeMap<String, String>,
    pub auth_header: Option<(String, String)>,
    pub timeout_ms: Option<u64>,
}
```

Keep the first version intentionally small. Support only fields the product needs now:

- base URL precedence
- header injection
- auth header mode
- timeout

Do not add proxy/TLS knobs unless they are exercised by tests in this repo.

**Step 4: Run test to verify it passes**

Run: `cargo test -p octopus-runtime-adapter --test request_policy_resolution`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/octopus-runtime-adapter/src/model_runtime/request_policy.rs \
  crates/octopus-core/src/lib.rs \
  crates/octopus-runtime-adapter/src/registry.rs \
  crates/octopus-runtime-adapter/tests/request_policy_resolution.rs
git commit -m "refactor: add request policy resolution layer"
```

### Task 6: Replace `executor.rs` with Protocol Drivers for Anthropic and OpenAI Chat

**Files:**
- Create: `crates/octopus-runtime-adapter/src/model_runtime/drivers/anthropic_messages.rs`
- Create: `crates/octopus-runtime-adapter/src/model_runtime/drivers/openai_chat.rs`
- Create: `crates/octopus-runtime-adapter/src/model_runtime/drivers/mod.rs`
- Modify: `crates/octopus-runtime-adapter/src/model_runtime/driver.rs`
- Modify: `crates/octopus-runtime-adapter/src/model_runtime/driver_registry.rs`
- Modify: `crates/octopus-runtime-adapter/src/lib.rs`
- Delete: `crates/octopus-runtime-adapter/src/executor.rs`
- Test: `crates/octopus-runtime-adapter/tests/protocol_drivers.rs`

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn anthropic_driver_normalizes_message_response_into_events() {
    let events = run_driver_test("anthropic_messages").await.unwrap();
    assert!(matches!(events.last(), Some(AssistantEvent::MessageStop)));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p octopus-runtime-adapter --test protocol_drivers anthropic_driver_normalizes_message_response_into_events -- --exact`
Expected: FAIL because no driver implementation exists.

**Step 3: Write minimal implementation**

```rust
#[async_trait]
pub trait ProtocolDriver: Send + Sync {
    fn protocol_family(&self) -> &'static str;
    fn supports_tool_loop(&self) -> bool;
    async fn execute_conversation(
        &self,
        ctx: &ResolvedModelExecutionContext,
        request: &RuntimeConversationRequest,
    ) -> Result<RuntimeConversationExecution, AppError>;
}
```

Move the Anthropic/OpenAI Chat request assembly and response normalization into these protocol drivers.

**Step 4: Run test to verify it passes**

Run: `cargo test -p octopus-runtime-adapter --test protocol_drivers`
Expected: PASS for Anthropic and OpenAI Chat driver coverage.

**Step 5: Commit**

```bash
git add crates/octopus-runtime-adapter/src/model_runtime \
  crates/octopus-runtime-adapter/src/lib.rs \
  crates/octopus-runtime-adapter/tests/protocol_drivers.rs
git rm crates/octopus-runtime-adapter/src/executor.rs
git commit -m "refactor: replace monolithic executor with protocol drivers"
```

### Task 7: Add `simple completion` and Non-Tool Drivers for OpenAI Responses and Gemini

**Files:**
- Create: `crates/octopus-runtime-adapter/src/model_runtime/simple_completion.rs`
- Create: `crates/octopus-runtime-adapter/src/model_runtime/drivers/openai_responses.rs`
- Create: `crates/octopus-runtime-adapter/src/model_runtime/drivers/gemini_native.rs`
- Modify: `crates/octopus-runtime-adapter/src/model_runtime/driver.rs`
- Modify: `crates/octopus-runtime-adapter/src/model_runtime/driver_registry.rs`
- Test: `crates/octopus-runtime-adapter/tests/simple_completion.rs`
- Test: `crates/octopus-runtime-adapter/tests/protocol_drivers.rs`

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn responses_driver_refuses_tool_loop_but_supports_simple_completion() {
    let capability = driver_capability("openai_responses");
    assert!(!capability.tool_loop);
    assert!(capability.simple_completion);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p octopus-runtime-adapter --test simple_completion responses_driver_refuses_tool_loop_but_supports_simple_completion -- --exact`
Expected: FAIL because `simple_completion` capability does not exist.

**Step 3: Write minimal implementation**

```rust
pub async fn execute_simple_completion(
    ctx: &ResolvedModelExecutionContext,
    input: &str,
    system_prompt: Option<&str>,
) -> Result<ModelExecutionResult, AppError> {
    ctx.driver.execute_prompt(ctx, input, system_prompt).await
}
```

Use this path for `openai_responses` and `gemini_native` until tool loop support is intentionally implemented. Do not fake tool support.

**Step 4: Run test to verify it passes**

Run:

```bash
cargo test -p octopus-runtime-adapter --test simple_completion
cargo test -p octopus-runtime-adapter --test protocol_drivers
```

Expected: PASS

**Step 5: Commit**

```bash
git add crates/octopus-runtime-adapter/src/model_runtime/simple_completion.rs \
  crates/octopus-runtime-adapter/src/model_runtime/drivers/openai_responses.rs \
  crates/octopus-runtime-adapter/src/model_runtime/drivers/gemini_native.rs \
  crates/octopus-runtime-adapter/tests/simple_completion.rs \
  crates/octopus-runtime-adapter/tests/protocol_drivers.rs
git commit -m "refactor: separate simple completion from tool loop runtime"
```

### Task 8: Rewire the Runtime Turn Loop to Depend Only on Driver Capabilities

**Files:**
- Modify: `crates/octopus-runtime-adapter/src/agent_runtime_core.rs`
- Modify: `crates/octopus-runtime-adapter/src/execution_target.rs`
- Modify: `crates/octopus-runtime-adapter/src/run_context.rs`
- Test: `crates/octopus-runtime-adapter/tests/runtime_turn_loop.rs`

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn tool_enabled_turn_fails_closed_when_driver_has_no_tool_loop_support() {
    let result = submit_turn_against_driver("openai_responses").await;
    assert!(result.unwrap_err().to_string().contains("tool loop not supported"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p octopus-runtime-adapter --test runtime_turn_loop tool_enabled_turn_fails_closed_when_driver_has_no_tool_loop_support -- --exact`
Expected: FAIL because the turn loop still branches inside the old executor logic.

**Step 3: Write minimal implementation**

```rust
if request_has_tools && !ctx.driver_capability.tool_loop {
    return Err(AppError::runtime("tool loop not supported for selected protocol family"));
}
```

Remove protocol-family `match` statements from the runtime loop. The loop should only ask:

- does this driver support tool loop?
- does this driver support conversation execution?
- does this driver support simple completion?

**Step 4: Run test to verify it passes**

Run: `cargo test -p octopus-runtime-adapter --test runtime_turn_loop`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/octopus-runtime-adapter/src/agent_runtime_core.rs \
  crates/octopus-runtime-adapter/src/execution_target.rs \
  crates/octopus-runtime-adapter/src/run_context.rs \
  crates/octopus-runtime-adapter/tests/runtime_turn_loop.rs
git commit -m "refactor: make runtime turn loop driver-capability based"
```

### Task 9: Move Adapter Tests Out of the Monolith and Delete Dead Runtime Code

**Files:**
- Create: `crates/octopus-runtime-adapter/tests/`
- Modify: `crates/octopus-runtime-adapter/src/lib.rs`
- Delete: `crates/octopus-runtime-adapter/src/adapter_tests.rs`
- Delete: `crates/octopus-runtime-adapter/src/split_module_tests.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn test_layout_has_no_single_monolithic_adapter_test_module() {
    assert!(std::path::Path::new("crates/octopus-runtime-adapter/src/adapter_tests.rs").exists() == false);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p octopus-runtime-adapter --test protocol_drivers -- --nocapture`
Expected: Existing tests still compile from `adapter_tests.rs`; no new integration layout exists yet.

**Step 3: Write minimal implementation**

Move existing runtime model tests into focused integration files:

- registry behavior
- auth resolution
- request policy
- protocol drivers
- turn loop

Delete dead or shadow modules after migration instead of keeping duplicate entrypoints.

**Step 4: Run test to verify it passes**

Run:

```bash
cargo test -p octopus-runtime-adapter --tests
cargo clippy -p octopus-runtime-adapter --tests -- -D warnings
```

Expected: PASS, no duplicate dead test modules remain.

**Step 5: Commit**

```bash
git add crates/octopus-runtime-adapter/tests \
  crates/octopus-runtime-adapter/src/lib.rs
git rm crates/octopus-runtime-adapter/src/adapter_tests.rs \
  crates/octopus-runtime-adapter/src/split_module_tests.rs
git commit -m "refactor: split runtime adapter tests by component"
```

### Task 10: Final Contract, UI, and Governance Cleanup

**Files:**
- Modify: `contracts/openapi/src/components/schemas/catalog.yaml`
- Modify: `contracts/openapi/src/components/schemas/runtime.yaml`
- Modify: `contracts/openapi/src/paths/catalog.yaml`
- Modify: `contracts/openapi/src/paths/runtime.yaml`
- Modify: `apps/desktop/src/stores/catalog.ts`
- Modify: `apps/desktop/src/stores/catalog_normalizers.ts`
- Modify: `apps/desktop/src/stores/runtime_actions.ts`
- Modify: `apps/desktop/src/views/workspace/ModelsView.vue`
- Modify: `apps/desktop/src/views/workspace/ModelsTablePanel.vue`
- Modify: `apps/desktop/src/views/workspace/ModelDetailsDialog.vue`
- Modify: `apps/desktop/src/views/workspace/useModelsDraft.ts`
- Modify: `apps/desktop/src/views/workspace/models-security.ts`
- Modify: `docs/plans/model/2026-04-18-model-module-architecture.md`
- Modify: `docs/plans/model/2026-04-18-reference-projects-model-call-analysis.md`

**Step 1: Write the failing test**

```ts
it('hides non-executable model surfaces from the selectable runtime rows', () => {
  const rows = buildCatalogRows(snapshot)
  expect(rows.every(row => row.runtimeSupport.prompt || row.runtimeSupport.toolLoop)).toBe(true)
})

it('renders the workspace models page as persistent list-detail state instead of modal-only editing', () => {
  renderWorkspaceModels()
  expect(screen.getByTestId('workspace-models-list-pane')).toBeInTheDocument()
  expect(screen.getByTestId('workspace-models-detail-pane')).toBeInTheDocument()
})

it('shows provider-inherited credential state in the detail pane', () => {
  renderWorkspaceModelsWithProviderInheritedCredential()
  expect(screen.getByText(/provider inherited/i)).toBeInTheDocument()
})
```

**Step 2: Run test to verify it fails**

Run: `pnpm -C apps/desktop test`
Expected: FAIL because the UI does not yet read execution support metadata.

**Step 3: Write minimal implementation**

Update UI copy and filters so the model catalog distinguishes:

- declared capability
- executable runtime support

Then restructure the workspace console model page into the canonical `list/detail` workbench form:

- left list pane for browse and status scanning
- right detail pane for persistent editing and inspection
- explicit `Authentication` section showing credential source, secret health, inheritance or override state, and replacement or clear actions
- explicit `Validation` section showing last-known reachability state instead of relying only on transient toasts

Then refresh the two model docs to reflect the new architecture and remove any statements that are no longer true.

**Step 4: Run test to verify it passes**

Run:

```bash
pnpm openapi:bundle
pnpm schema:generate
pnpm -C apps/desktop typecheck
pnpm -C apps/desktop test
pnpm schema:check
```

Expected: PASS

**Step 5: Commit**

```bash
git add contracts/openapi/src/components/schemas/catalog.yaml \
  contracts/openapi/src/components/schemas/runtime.yaml \
  contracts/openapi/src/paths/catalog.yaml \
  contracts/openapi/src/paths/runtime.yaml \
  apps/desktop/src/tauri/runtime_api.ts \
  apps/desktop/src/tauri/workspace-client.ts \
  apps/desktop/src/stores/catalog.ts \
  apps/desktop/src/stores/catalog_normalizers.ts \
  apps/desktop/src/stores/runtime_actions.ts \
  apps/desktop/src/views/workspace/ModelsView.vue \
  apps/desktop/src/views/workspace/ModelsTablePanel.vue \
  apps/desktop/src/views/workspace/ModelDetailsDialog.vue \
  apps/desktop/src/views/workspace/useModelsDraft.ts \
  apps/desktop/src/views/workspace/models-security.ts \
  docs/plans/model/2026-04-18-model-module-architecture.md \
  docs/plans/model/2026-04-18-reference-projects-model-call-analysis.md
git commit -m "refactor: align catalog ui and docs with runtime execution model"
```

## Final Verification

Run:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
pnpm schema:generate
pnpm schema:check
pnpm check:desktop
```

Expected:

- all Rust code formatted
- no clippy warnings
- all workspace tests green
- schema parity passes
- desktop typecheck/tests green

## Acceptance Criteria

1. There is exactly one canonical model policy.
2. Catalog defaults, registry defaults, and runtime selection use the same canonical model IDs.
3. Unsupported protocol families are fail-closed in both backend selection and UI.
4. `agent_runtime_core.rs` contains no provider-specific request assembly logic.
5. Auth resolution is testable without invoking the runtime loop.
6. Request policy resolution is testable without invoking the runtime loop.
7. `simple completion` is its own execution path.
8. `vendor_native` and `realtime` are absent from normal selection unless fully implemented.
9. `crates/octopus-runtime-adapter/src/executor.rs` is gone.
10. `crates/octopus-runtime-adapter/src/adapter_tests.rs` is gone.
11. `crates/api` no longer defines a second canonical source for model aliases, defaults, or fallback provider choice.
12. A new protocol-family implementation does not require editing `agent_runtime_core.rs`.
13. A new credential-source rule does not require editing protocol drivers.
14. A new request-header or base-URL precedence rule does not require editing protocol drivers.
15. Catalog selectable entries and runtime executable entries are structurally consistent rather than conventionally aligned.

## Notes for Execution

- Prefer deleting dead paths over leaving wrappers.
- Do not manually edit `contracts/openapi/octopus.openapi.yaml` or `packages/schema/src/generated.ts`.
- Keep commits small and in task order.
- If a task exposes a cleaner module boundary than planned here, prefer the cleaner boundary and update the plan before continuing.
