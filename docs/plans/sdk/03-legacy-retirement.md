# 03 · Legacy 退役清单（Retirement Map）

> 本文档登记每一个**将被删除的旧 crate / 文件 / 公开符号**，以及它在新 SDK 中的替代位置。
> 目的有二：
> 1. 防止 W1–W8 任何一周出现"新符号上线、旧符号没下线"的双轨漂移。
> 2. 给 W7/W8 收口阶段提供一份**可机器核对**的退场清单。
>
> 配合：`02-crate-topology.md` 登记新公共面；本文档登记对应的旧公共面。

## 0. 使用规则

- **登记时机**：每个子 Plan 在"删除旧代码"的 Step 合入时，必须把相关行从"待退役"区移到"已退役"区，并写明 git revision。
- **守护命令**：`01-ai-execution-protocol.md §7.4` 的 `rg / find` 扫描在每个 PR 与每周 Weekly Gate 都会执行。出现命中但本表未登记 → Stop Condition #9 触发。
- **替代映射必须原子**：禁止"删除旧符号但未指定替代物"的 commit；如确实为废弃（无替代），标记 `deprecate-with-no-successor` 并在 §7 专列理由。
- **"替代位置"路径是目标态**：本文档"替代位置"一列中出现的 `octopus-platform::{config, governance, host, runtime_config, catalog, secret_vault, bootstrap, memory, session_bridge}` 等子模块，在当前 `octopus-platform/src/` 中**尚不存在**，它们将在 W4–W7 过程中按"业务侧薄壳"原则新建。现状下不存在即属预期，不得据此扫描判定失败。新建时必须遵守 `02-crate-topology.md §3.1` 的业务侧 crate 规约（薄壳 ≤ 300 行 / 不承载 SDK 能力）。
- **行数基准**：本文档所有代码规模数字取自 2026-04-20 `wc -l` 实测；变动超过 ±10% 时，下一批 PR 必须同步修订本文档。
- W7 `Task 7` 完成后，目录级退役状态以 `§9 已退役（Completed）` 为准；`§2–§7` 的模块级矩阵保留迁移历史，不再要求逐行把过程态 `pending / partial / replaced` 回写成目录删除态。

## 1. 总览：退役对象与替代去向

| 旧 crate | 代码规模（`*.rs` 总行，实测） | 退役周 | 主替代 |
|---|---|---|---|
| `crates/runtime` | **19 281** | W1 起拆 → W7 删目录 | `sdk-session` / `sdk-context` / `sdk-permissions` / `sdk-hooks` / `sdk-sandbox` / `sdk-mcp` / `sdk-tools`（bash/file_ops 部分） |
| `crates/tools` | **14 391**（含 `capability_runtime/*` 3 092 + `split_module_tests.rs` 5 433） | W3 拆 → W7 删目录 | `sdk-tools` / `sdk-subagent` |
| `crates/plugins` | **3 938** | W5 拆 → W7 删目录 | `sdk-plugin` / `sdk-hooks` |
| `crates/api` | **5 597** | W2 拆 → W7 删目录 | `sdk-model` |
| `crates/octopus-runtime-adapter` | **38 673** | W1–W6 持续迁移 → W7 删目录 | `sdk-core` / `sdk-session` / `sdk-model` / `sdk-permissions` / `sdk-subagent` / `sdk-context` / `octopus-platform`（桥接薄壳） / `octopus-persistence` |
| `crates/commands` | **5 010** | W6 合并入 `octopus-cli` → W7 删目录 | `octopus-cli` |
| `crates/compat-harness` | **385** | W7 删目录 | 无（完全废弃） |
| `crates/mock-anthropic-service` | **1 178** | W7 迁入 SDK tests → 删目录 | `octopus-sdk-model` 的 `tests/fixtures/` |
| `crates/rusty-claude-cli` | **12 844**（W6 重点工作量） | W6 合并入 `octopus-cli` → W7 删目录 | `octopus-cli` |
| `crates/octopus-desktop-backend` | **196** | W7 改名 → `octopus-desktop` | `octopus-desktop` |
| `crates/octopus-model-policy` | **143** | W2 内嵌入 `sdk-model` → W7 删目录 | `octopus-sdk-model::RoleRouter` |

> **W6 警示**：`rusty-claude-cli` 实测 12 844 行（是首稿估计的 8 倍以上）。W6 子 Plan 必须把"合并 CLI"作为独立 Task 组（不少于 3 个 Task），而不是子代理 / 插件周的附属动作。

## 2. `crates/runtime` 逐模块退役

> 源规模：~19 300 行；`lib.rs` re-export 60+ 符号。退役动作分散在 W1–W7。

### 2.1 模块 → 新位置映射

| 旧模块（`crates/runtime/src/`） | 主要公开符号（节选） | 替代位置 | 迁移周 | 状态 |
|---|---|---|---|---|
| `session.rs` | `Session / ConversationMessage / ContentBlock / MessageRole / SessionCompaction / SessionFork / SessionError` | `octopus-sdk-contracts`（数据 IR）+ `octopus-sdk-session`（Store） | W1 | pending |
| `session_control.rs`（879） | `SessionStore`（旧 trait）、会话控制 | `octopus-sdk-session::SqliteJsonlSessionStore` | W1 | pending |
| `conversation.rs` + `conversation/*` | `ConversationRuntime / ApiClient(旧) / ApiRequest / AssistantEvent / PromptCacheEvent / RuntimeError / StaticToolExecutor / ToolError / ToolExecutionOutcome / ToolExecutor / TurnSummary / AutoCompactionEvent` | `octopus-sdk-core::AgentRuntime`（Brain Loop）+ `octopus-sdk-contracts::AssistantEvent` | W6 | partial/replaced |
| `prompt.rs`（803） | `load_system_prompt / SystemPromptBuilder / ProjectContext / ContextFile / PromptBuildError / FRONTIER_MODEL_NAME / SYSTEM_PROMPT_DYNAMIC_BOUNDARY / prepend_bullets` | `octopus-sdk-context::SystemPromptBuilder` | W4 | replaced |
| `compact.rs` + `summary_compression.rs` | `compact_session / estimate_session_tokens / format_compact_summary / get_compact_continuation_message / should_compact / CompactionConfig / CompactionResult / compress_summary_text` | `octopus-sdk-context::Compactor` | W4 | replaced |
| `config.rs` + `config_merge.rs` + `config_patch.rs` + `config_secrets.rs` + `config_sources.rs` + `config/` | `ConfigDocument / ConfigEntry / ConfigError / ConfigLoader / ConfigSource / RuntimeConfig / RuntimeFeatureConfig / RuntimeHookConfig / RuntimePermissionRuleConfig / RuntimePluginConfig / ResolvedPermissionMode / apply_config_patch / CLAW_SETTINGS_SCHEMA_NAME` | `octopus-platform::config`（业务侧运行时配置服务）；SDK 只消费由业务注入的 `AgentRuntimeBuilder` 字段，不持有 config loader | W7 | pending |
| `bash.rs` + `bash_validation.rs` | `execute_bash / BashCommandInput / BashCommandOutput / bash_validation::*` | `octopus-sdk-tools::builtin::BashTool`（走 MCP in-process shim） | W3 | pending |
| `file_ops.rs` | `edit_file / glob_search / grep_search / read_file / write_file / *Output / StructuredPatchHunk / TextFilePayload` | `octopus-sdk-tools::builtin::{FileReadTool, FileWriteTool, FileEditTool, GlobTool, GrepTool}` | W3 | pending |
| `mcp*.rs`（含 `mcp_stdio/`） | `McpServerManager / McpServerManagerError / McpStdioProcess / McpClientAuth / McpClientBootstrap / McpClientTransport / McpManagedProxyTransport / McpRemoteTransport / McpSdkTransport / McpStdioTransport / spawn_mcp_stdio_process / JsonRpc* / ManagedMcpTool / ManagedMcpPrompt / McpTool / McpResource / McpPrompt / ...`（约 40 个符号） | `octopus-sdk-mcp::{McpClient, McpServerManager, StdioTransport, HttpTransport, SdkTransport, JsonRpc*}` | W3 | pending |
| `mcp_lifecycle_hardened.rs` | `McpLifecyclePhase / McpLifecycleState / McpLifecycleValidator / McpErrorSurface / McpDegradedReport / McpFailedServer / McpPhaseResult` | `octopus-sdk-mcp::McpServerManager`（内部） | W3 | pending |
| `permissions.rs` + `permission_enforcer.rs` + `policy_engine.rs` | `PermissionMode / PermissionPolicy / PermissionContext / PermissionOutcome / PermissionOverride / PermissionPromptDecision / PermissionPrompter / PermissionRequest / PermissionEnforcer / EnforcementResult / PolicyEngine / PolicyRule / PolicyAction / PolicyCondition / evaluate / GreenLevel / DiffScope / ReviewStatus / LaneContext / LaneBlocker / ReconcileReason` | `octopus-sdk-permissions::{PermissionMode, PermissionPolicy, PermissionGate, ApprovalBroker, PermissionOutcome}` + 业务侧 `octopus-platform::governance`（保留 `LaneContext / LaneBlocker` 等业务域语义） | W4 | replaced |
| `hooks.rs` | `HookAbortSignal / HookEvent / HookProgressEvent / HookProgressReporter / HookRunResult / HookRunner` | `octopus-sdk-hooks::{HookEvent, HookRunner, HookDecision, Hook}` | W4 | replaced |
| `plugin_lifecycle.rs` | `PluginLifecycle / PluginLifecycleEvent / PluginState / DegradedMode / DiscoveryResult / PluginHealthcheck / ResourceInfo / ServerHealth / ServerStatus / ToolInfo` | `octopus-sdk-plugin::{PluginLifecycle, PluginRegistry}` | W5 | replaced |
| `sandbox.rs` | `SandboxConfig / SandboxStatus / SandboxRequest / SandboxDetectionInputs / build_linux_sandbox_command / detect_container_environment / resolve_sandbox_status / LinuxSandboxCommand / ContainerEnvironment / FilesystemIsolationMode` | `octopus-sdk-sandbox::{SandboxSpec, SandboxBackend, BubblewrapBackend, SeatbeltBackend}` | W4 | replaced |
| `oauth.rs` | `OAuthTokenSet / OAuth* / PkceChallengeMethod / PkceCodePair / save_oauth_credentials / load_oauth_credentials / clear_oauth_credentials / credentials_path / code_challenge_s256 / generate_pkce_pair / generate_state / loopback_redirect_uri / parse_oauth_callback_query / parse_oauth_callback_request_target` | `octopus-sdk::SecretVault` 实现 + 业务侧 `octopus-platform::auth`（OAuth flow 业务侧执行，vault 存 token） | W2（与模型认证耦合）W4（与凭据零暴露合约同时验证） | pending |
| `usage.rs` | `TokenUsage / UsageTracker / ModelPricing / UsageCostEstimate / pricing_for_model / format_usd` | `octopus-sdk-observability::UsageLedger` + `octopus-sdk-contracts::Usage` | W6 | partial |
| `task_packet.rs` + `task_registry.rs` | `TaskPacket / ValidatedPacket / validate_packet / TaskPacketValidationError` | `octopus-platform`（业务域 task）；SDK 不承担业务任务包语义 | W7 | pending |
| `lane_events.rs` | `LaneEvent / LaneCommitProvenance / LaneEventBlocker / LaneEventName / LaneEventStatus / LaneFailureClass / dedupe_superseded_commit_events` | `octopus-platform::governance`（业务域） | W7 | pending |
| `recovery_recipes.rs` | `RecoveryRecipe / RecoveryContext / RecoveryEvent / RecoveryResult / RecoveryStep / EscalationPolicy / FailureScenario / attempt_recovery / recipe_for` | `octopus-sdk-core`（内部失败处理；不作为公共面暴露） | W6 | pending |
| `remote.rs` | `RemoteSessionContext / UpstreamProxyBootstrap / UpstreamProxyState / DEFAULT_REMOTE_BASE_URL / DEFAULT_SESSION_TOKEN_PATH / DEFAULT_SYSTEM_CA_BUNDLE / NO_PROXY_HOSTS / UPSTREAM_PROXY_ENV_KEYS / inherited_upstream_proxy_env / no_proxy_list / read_token / upstream_proxy_ws_url` | `octopus-platform::host`（业务层的上游代理集成） | W7 | pending |
| `sse.rs` | `IncrementalSseParser / SseEvent` | `octopus-sdk-model`（内部 util） | W2 | pending |
| `stale_base.rs` / `stale_branch.rs` / `branch_lock.rs` / `green_contract.rs` | git lane 业务 | `octopus-platform::governance` | W7 | pending |
| `trust_resolver.rs` | `TrustResolver / TrustPolicy / TrustConfig / TrustDecision / TrustEvent` | `octopus-platform::governance` | W7 | pending |
| `bootstrap.rs` | `BootstrapPhase / BootstrapPlan` | `octopus-platform::bootstrap` | W7 | pending |
| `worker_boot.rs`（1168） | `Worker / WorkerRegistry / WorkerEvent / WorkerEventKind / WorkerEventPayload / WorkerFailure / WorkerFailureKind / WorkerPromptTarget / WorkerReadySnapshot / WorkerStatus / WorkerTrustResolution` | **W5 不作为 `octopus-sdk-subagent` 实现来源**；当前文件保留 trust gate / ready-for-prompt / prompt-misdelivery 控制面。若 W7 业务侧改接通用 subagent SDK，再由 `octopus-platform::team` 承接团队态映射。 | W7 | pending |
| `lsp_client.rs` | `LspRegistry` | `octopus-sdk-tools`（内部）或 `octopus-sdk-plugin::LspServer` 扩展点 | W5 | pending |

### 2.2 彻底废弃（deprecate-with-no-successor）

- `test_env_lock` 测试辅助 → 随 `runtime` crate 退场，不迁移。
- `split_module_tests.rs` 风格的千行测试合集 → 不迁移；按 `02-crate-topology.md §4.2` 拆小测试文件。

---

## 3. `crates/tools` 逐模块退役

### 3.1 模块映射

| 旧模块（`crates/tools/src/`） | 主要符号 | 替代位置 | 迁移周 | 状态 |
|---|---|---|---|---|
| `tool_registry.rs` | `mvp_tool_specs / RuntimeToolDefinition / ToolSearchOutput / ToolSpec / search_tool_specs / normalize_tool_search_query / canonical_tool_token / deferred_tool_specs / execute_tool_search / permission_mode_from_plugin` | `octopus-sdk-tools::ToolRegistry / ToolSpec` | W3 | pending |
| `builtin_catalog.rs` + `builtin_exec.rs` | `builtin_capability_catalog / BuiltinCapability / BuiltinCapabilityCatalog / BuiltinCapabilityCategory / BuiltinHandlerKey / BuiltinRoleAvailability / execute_tool / enforce_permission_check / *Input`（AskUserQuestion/Brief/Config/…） | `octopus-sdk-tools::builtin::*` | W3 | pending |
| `capability_runtime/**`（planner/executor/provider/state/exposure/events） | `CapabilityRuntime / CapabilityExecutor / CapabilityPlanner / CapabilityProvider / CapabilitySpec / CapabilityHandle / CapabilitySurface / CapabilityExecutionPlan / CapabilityCompiler / CapabilityDispatchKind / CapabilityExecutionEvent / CapabilityExecutionPhase / CapabilityExecutionRequest / CapabilityMediationDecision / CapabilityConcurrencyPolicy / CapabilityExecutionKind / CapabilitySourceKind / CapabilityVisibility / CapabilityActivation / CapabilityProfile / CapabilityRequestOverride / ManagedMcpRuntime / McpCapabilityDescriptor / McpCapabilityProvider / McpConnectionProjection / mcp_resource_capability_descriptor / mcp_tool_capability_descriptor / permission_mode_for_mcp_tool / apply_skill_session_overrides / SessionCapabilityState / SessionCapabilityStore / SharedSessionCapabilityState` | **无**（整套删除；取舍 #2）；`ToolRegistry + MCP + PermissionGate` 三件套替代 | W3 | done |
| `fs_shell.rs` | `run_bash / run_edit_file / run_glob_search / run_grep_search / run_notebook_edit / run_powershell / run_read_file / run_repl / run_write_file / workspace_test_branch_preflight / *Input` | `octopus-sdk-tools::builtin::{BashTool, FileReadTool, FileWriteTool, FileEditTool, GlobTool, GrepTool}` + 去除 PowerShell / Repl / Notebook（保留 Notebook 作为 plugin 示例） | W3 | pending |
| `lsp_runtime.rs` | `run_lsp / LspInput` | `octopus-sdk-plugin::LspServer` 扩展点（最小 stub；W5 落地） | W5 | pending |
| `skill_runtime.rs`（1107） | `SkillDiscoveryOutput / SkillExecutionResult / SkillStateUpdate` | `octopus-sdk-tools::builtin::SkillTool` + `octopus-sdk-context`（Skill 按需加载） | W3 | pending |
| `subagent_runtime.rs`（1024） | `spawn_subagent_job / spawn_subagent_task / spawn_subagent_with_job / spawn_subagent_with_job_detailed / AgentInput / AgentJob / AgentOutput / AgentSpawnFailure / SubagentToolExecutor / agent_permission_policy / allowed_tools_for_subagent / classify_lane_failure / derive_agent_state / final_assistant_text / iso8601_now / maybe_commit_provenance / persist_agent_terminal_state / push_output_block` | `octopus-sdk-subagent::OrchestratorWorkers / GeneratorEvaluator` + `octopus-sdk-tools::builtin::AgentTool`；但当前文件已降为 TODO stub，W5 只由 greenfield SDK 覆盖职责边界，不从此处迁实现。 | W5 | partial |
| `web_external.rs` | `run_web_fetch / run_web_search / run_remote_trigger / WebFetchInput / WebSearchInput / RemoteTriggerInput` | `octopus-sdk-tools::builtin::{WebFetchTool, WebSearchTool}`；`RemoteTrigger` 作为 plugin 示例 | W3 | pending |
| `lane_completion.rs` | lane 业务 | `octopus-platform::governance` | W7 | pending |
| `split_module_tests.rs`（5433） | 整体测试合集 | 拆分到 `octopus-sdk-tools/tests/<feature>.rs` | W3 | **must-split** |

---

## 4. `crates/plugins` 退役

| 旧模块 | 主要符号 | 替代位置 | 迁移周 | 状态 |
|---|---|---|---|---|
| `manifest.rs` | `PluginManifest / PluginManifestValidationError / PluginCommandManifest / PluginToolManifest / PluginToolDefinition / PluginToolPermission / PluginPermission / PluginHooks / PluginLifecycle / load_plugin_from_directory` | `octopus-sdk-plugin::{PluginManifest, PluginComponent, PluginCompat, PluginError}` | W5 | replaced |
| `discovery.rs`（1124） | `PluginManager / PluginManagerConfig / PluginRegistry / PluginRegistryReport / PluginSummary / RegisteredPlugin / InstalledPluginRecord / InstalledPluginRegistry / InstallOutcome / UpdateOutcome / PluginMetadata / PluginInstallSource / PluginKind / PluginError / PluginLoadFailure / builtin_plugins` | `octopus-sdk-plugin::{PluginRegistry, PluginLifecycle}` + 业务侧分发源 | W5 | partial |
| `hooks.rs` | `HookEvent / HookRunResult / HookRunner` | `octopus-sdk-hooks`（**与 `runtime::hooks` 合并**，不得重复实现） | W4–W5 | replaced |
| `hook_dispatch.rs` | `PluginTool` | `octopus-sdk-plugin` 内部 | W5 | replaced |
| `lifecycle.rs` | `Plugin / PluginDefinition / BuiltinPlugin / BundledPlugin / ExternalPlugin` | `octopus-sdk-plugin::PluginComponent` + builtin/bundled/external 通过 `PluginSource` 区分 | W5 | replaced |
| `split_module_tests.rs`（1160） | 整体测试合集 | 拆分到 `octopus-sdk-plugin/tests/` | W5 | **must-split** |

---

## 5. `crates/api` 退役

| 旧模块 | 主要符号 | 替代位置 | 迁移周 | 状态 |
|---|---|---|---|---|
| `client.rs` | `ProviderClient / MessageStream / OAuthTokenSet / AuthSource / read_base_url / read_xai_base_url / oauth_token_is_expired / resolve_saved_oauth_token / resolve_startup_auth_source` | `octopus-sdk-model::{ModelProvider, Provider}` + `octopus-sdk::SecretVault` | W2 | pending |
| `error.rs` | `ApiError` | `octopus-sdk-model::ModelError` | W2 | pending |
| `http_client.rs` | `build_http_client / build_http_client_or_default / build_http_client_with / ProxyConfig` | `octopus-sdk-model`（内部 util） | W2 | pending |
| `prompt_cache.rs` | `PromptCache / PromptCacheConfig / PromptCachePaths / PromptCacheRecord / PromptCacheStats / CacheBreakEvent` | `octopus-sdk-model::{PromptCache, CacheBreakpoint}`；prompt cache 稳定性守护 | W2 | pending |
| `providers/anthropic.rs` | `AnthropicClient (= ApiClient)` + OAuth/stream/tests | `octopus-sdk-model::adapters::anthropic_messages` | W2 | pending |
| `providers/openai_compat.rs` | `OpenAiCompatClient / OpenAiCompatConfig` | `octopus-sdk-model::adapters::openai_chat` | W2 | pending |
| `providers/mod.rs` | `max_tokens_for_model / resolve_model_alias / ProviderKind` | `octopus-sdk-model`（内部）；Provider 注册迁入新 `ProviderRegistry` | W2 | pending |
| `providers/request_assembly.rs` / `response_normalization.rs` / `stream_parsing.rs` / `provider_errors.rs` | 协议装配/解析 | `octopus-sdk-model::adapters::*`（分适配器） | W2 | pending |
| `sse.rs` / `types.rs` | IR 与 SSE 工具 | `octopus-sdk-contracts`（`ContentBlock` 等）+ `octopus-sdk-model`（内部 SSE parser） | W1（IR 先立）+ W2 | pending |

### 5.1 协议适配器对齐

adapter 侧 `octopus-runtime-adapter/src/model_runtime/drivers/*`（`anthropic_messages / gemini_native / openai_chat / openai_responses`）已经具备雏形（4 个文件、~760 行），**优先复用它们作为 `sdk-model::adapters::*` 的起点**；W2 的核心工作是：
1. 把 `api::providers::*` 的 client/auth/streaming/prompt_cache 与 adapter 的 4 drivers 合并。
2. 补 `vendor_native` 第 5 个 protocol family 的 stub。

---

## 6. `crates/octopus-runtime-adapter` 退役（重头戏）

> 源规模：~32 600 行。退役动作跨 W1–W7，每周认领一块。

### 6.1 按"职责域"分组迁移

| 职责域 | 相关文件（示例） | 替代位置 | 迁移周 | 状态 |
|---|---|---|---|---|
| **Agent Runtime Core（Brain Loop）** | `agent_runtime_core.rs`（6024）、`subrun_orchestrator.rs`（288）、`team_runtime.rs`（918）、`workflow_runtime.rs`（367）、`background_runtime.rs`、`handoff_runtime.rs`、`mailbox_runtime.rs`、`worker_runtime.rs`、`run_context.rs` | `octopus-sdk-core::AgentRuntime`（单体循环）+ `octopus-sdk-subagent::{OrchestratorWorkers, GeneratorEvaluator}`（团队/工作流编排以通用模式落地，不再区分 `team/workflow/worker/handoff/mailbox` 四套 runtime） | W5（subagent）+ W6（core） | partial/replaced |
| **Model Runtime** | `model_runtime/mod.rs` + `drivers/*` + `driver.rs / driver_registry.rs / conversation_driver.rs / generation_driver.rs / auth.rs / request_policy.rs / simple_completion.rs / stream_bridge.rs / canonical_model_policy.rs`、`model_budget.rs`（499） | `octopus-sdk-model::{ModelProvider, ProtocolAdapter, Provider, Surface, Model, RoleRouter, FallbackPolicy}` | W2 | pending |
| **Session / Persistence** | `persistence.rs`（2593）、`session_service.rs`（437）、`session_policy.rs`（124）、`snapshot_store.rs`、`adapter_state.rs`、`actor_context.rs`、`actor_manifest.rs`（621）、`adapter_test_support.rs`（584） | `octopus-sdk-session::SqliteJsonlSessionStore` + `octopus-platform::session_bridge`（业务域会话薄壳） | W1 + W7 | pending |
| **Capability Bridge** | `capability_executor_bridge.rs`（298）、`capability_planner_bridge.rs`（593）、`capability_state.rs` | **无**（整套删除；取舍 #2） | W3 | done |
| **Approval / Permissions / Policy** | `approval_broker.rs`（468）、`approval_flow.rs`、`policy_compiler.rs`（550） | `octopus-sdk-permissions::{PermissionGate, ApprovalBroker, PermissionPolicy}` | W4 | replaced |
| **Memory** | `memory_runtime.rs`、`memory_selector.rs`（485）、`memory_writer.rs`（802）、`memory_runtime_tests.rs`（851） | `octopus-sdk-context::MemoryBackend`（trait，默认实现 `DurableScratchpad`）+ 业务侧 `octopus-platform::memory`（知识库 / 记忆提议业务语义） | W4 | replaced |
| **Config Snapshot / Runtime Config** | `runtime_config.rs`（790）、`config_service.rs`（1021）、`runtime_config_tests.rs`（993） | `octopus-platform::runtime_config`（业务侧，文件为主）+ `octopus-sdk-session`（snapshot 落首事件） | W7 | pending |
| **Model Registry** | `registry.rs`（673）、`registry_baseline.rs`（936）、`registry_overrides.rs`（737）、`registry_parse.rs`、`registry_resolution.rs` | `octopus-sdk-model::ModelCatalog`（静态默认 + 覆写层）+ `octopus-platform::catalog`（workspace 覆写面） | W2 | pending |
| **Execution Events / Event Bus** | `execution_events.rs`（1829）、`event_bus.rs`、`execution_service.rs`、`execution_target.rs`、`trace_context.rs` | `octopus-sdk-contracts::SessionEvent` + `octopus-sdk-observability::Tracer` | W6 | partial |
| **Secret Store** | `secret_store.rs`（371）、`auth_mediation.rs` | `octopus-sdk::SecretVault` 的业务侧默认实现（SQLite 加密存储）→ 放 `octopus-platform::secret_vault` | W2 | pending |
| **Tests** | `*_tests.rs`（多个，几千到几万行） | 随对应模块迁入 SDK tests/ 或业务 tests/；拆分到 ≤ 800 行 | 对应周 | pending |

### 6.2 adapter 顶层门面

`RuntimeAdapter`（`lib.rs:135`）退役后由 `octopus-sdk::AgentRuntime` + `octopus-platform::PlatformServices` 联合替代。业务侧 `octopus-server` / `octopus-desktop-backend` 的接入口改为：

```rust
let sdk = octopus_sdk::AgentRuntimeBuilder::new()
    .with_session_store(Arc::new(SqliteJsonlSessionStore::open(&db, &jsonl_root)?))
    .with_model_provider(platform.catalog.build_provider()?)
    .with_secret_vault(platform.secret_vault.clone())
    .with_tool_registry(build_tool_registry(&config))
    .with_permission_gate(platform.permissions.gate())
    .with_ask_resolver(platform.approvals.ask_resolver())
    .with_sandbox_backend(sandbox_backend_for_host())
    .with_plugin_registry(platform.plugins.registry())
    .with_plugins_snapshot(platform.plugins.snapshot())
    .with_tracer(platform.observability.tracer())
    .with_task_fn(platform.subagents.task_fn())
    .build()?;
```

---

## 7. 其它 crate 退役速览

### 7.1 `crates/commands`（W6 合并）

| 模块 | 替代位置 |
|---|---|
| `command_parser.rs` | W6 仅落 `octopus-cli::run_once::main_with_args` 最小入口；完整 parser 留 W7 cutover |
| `automation_commands.rs`（1030） | `octopus-cli::automation` |
| `workspace_commands.rs`（1098） | `octopus-cli::workspace` |
| `project_commands.rs` | `octopus-cli::project` |
| `runtime_commands.rs` | `octopus-cli::run_once`（W6 最小单会话单回合）；完整 runtime 命令面留 W7 |
| `config_commands.rs` | `octopus-cli::config` |

### 7.2 `crates/compat-harness`（W7 删除，无替代）

纯兼容层；随 adapter 下线一并移除。`lib.rs` 的内部使用点在 W6 前必须全部清理。

### 7.3 `crates/mock-anthropic-service`（W7 移 fixture）

迁入 `crates/octopus-sdk-model/tests/fixtures/mock_anthropic.rs`；不再作为独立 binary。

### 7.4 `crates/rusty-claude-cli`（W6 合并为 `octopus-cli`）

`claw` binary 保留。W6 已先落 `octopus-cli/src/{main.rs,run_once.rs}` 的最小 run path，并切到 `octopus-sdk`；`init.rs / input.rs / render.rs` 等全量 CLI 语义留 W7 继续平移。

### 7.5 `crates/octopus-desktop-backend`（W7 改名 `octopus-desktop`）

`backend.rs / bootstrap.rs / commands.rs / runtime.rs / services.rs / state.rs / updates.rs / error.rs / lib.rs / main.rs` 内容整改：移除对 `octopus-runtime-adapter` 的依赖，改为对 `octopus-sdk` + `octopus-platform`。

### 7.6 `crates/octopus-model-policy`（W2 并入）

全部内容（~500 行）迁入 `octopus-sdk-model::role_router` + `octopus-sdk-model::fallback_policy`；原 crate 删除。

### 7.7 `crates/octopus-core/src/lib.rs`（W8 拆分，不删除 crate）

3861 行按 12 个域切分：

| 新文件 | 内容 |
|---|---|
| `workspace.rs` | Workspace 相关类型 |
| `project.rs` | Project / Team / Agent 关联类型 |
| `task.rs` | Task / Run / Intervention |
| `deliverable.rs` | Deliverable / Version / Promotion |
| `agent.rs` | Agent 定义 / 资产 |
| `team.rs` | Team manifest |
| `runtime_config.rs` | `RuntimeConfig*` 业务侧 |
| `runtime_session.rs` | `RuntimeSession* / RuntimeRun*`（DTO，不是 SDK 的） |
| `memory.rs` | `RuntimeMemory* / MemoryProposal*` |
| `observation.rs` | 观测与 audit |
| `artifact.rs` | ArtifactRef / Version |
| `common.rs` | `AppError / timestamp_now / 权限常量` 等通用 |

（部分类型可能迁去 `octopus-sdk-contracts`；W1 梳理。）

### 7.8 `crates/octopus-infra`（W8 拆分，不删除 crate）

- `infra_state.rs`（5176）按 `workspace / project / agent / team / access / auth` 切分。
- `agent_assets.rs`（4577）按 `manifest / bundle / resources / seed` 切分。
- `projects_teams.rs`（4961）切 `projects.rs / teams.rs / team_links.rs / project_members.rs`。
- `access_control.rs`（2983）切 `users.rs / roles.rs / policies.rs / menus.rs`。
- `auth_users.rs`（1866）切 `credentials.rs / sessions.rs / oauth_records.rs`。
- `resources_skills.rs`（2843）切 `resources.rs / skills.rs`。
- 单文件 ≤ 800 行。

---

## 8. 守护扫描（自动核对本文档的完整性）

每周 Weekly Gate 时执行。全部返回 0 hits 才允许进入下一周（W7 / W8 除外，见下）。

```bash
# 遗留 crate 是否还在 Cargo workspace
rg '^\s*"crates/(runtime|tools|plugins|api|octopus-runtime-adapter|commands|compat-harness|mock-anthropic-service|rusty-claude-cli|octopus-desktop-backend|octopus-model-policy)"' Cargo.toml

# 业务侧是否仍 use 遗留 crate
rg 'use (runtime|tools|plugins|api|octopus_runtime_adapter|commands|compat_harness|octopus_model_policy)::' crates/octopus-{core,platform,persistence,server,desktop,cli}

# Capability Planner 残留
rg 'capability_runtime|CapabilityPlanner|CapabilitySurface|CapabilityExecutionPlan|CapabilityCompiler|CapabilityExecutionEvent|CapabilityExecutionPhase|CapabilityExecutionRequest|CapabilityMediationDecision' crates/

# 遗留 debug session JSON 作为恢复源
rg 'runtime/sessions/.*\.json' crates/ --glob '!**/tests/**' --glob '!**/fixtures/**'

# 单文件行数硬约束
find crates -type f -name '*.rs' -exec wc -l {} + | awk '$2 != "total" && $1 > 800 { print }'
```

### 退役阶段特例

- W7 结束前 `Cargo.toml` 扫描命中 → 阻塞本周。
- W7 结束后 `crates/{runtime,tools,plugins,api,octopus-runtime-adapter,commands,compat-harness,mock-anthropic-service,rusty-claude-cli,octopus-desktop-backend,octopus-model-policy}` 目录不存在 → 上述命令自然返回 0 hits。
- W8 结束前上述行数检查命令必须为 0。

---

## 9. 已退役（Completed）

> 格式：移动记录行到此节，附 commit SHA。本节初始为空。

| 日期 | 移除内容 | 替代位置 | Commit |
|---|---|---|---|
| 2026-04-22 | `crates/runtime` | `octopus-sdk-session` / `octopus-sdk-context` / `octopus-sdk-permissions` / `octopus-sdk-hooks` / `octopus-sdk-sandbox` / `octopus-sdk-mcp` / `octopus-sdk-tools` | working tree |
| 2026-04-22 | `crates/tools` | `octopus-sdk-tools` / `octopus-sdk-subagent` | working tree |
| 2026-04-22 | `crates/plugins` | `octopus-sdk-plugin` / `octopus-sdk-hooks` | working tree |
| 2026-04-22 | `crates/api` | `octopus-sdk-model` | working tree |
| 2026-04-22 | `crates/octopus-runtime-adapter` | `octopus-sdk` + `octopus-platform` bridge | working tree |
| 2026-04-22 | `crates/commands` | `octopus-cli` | working tree |
| 2026-04-22 | `crates/compat-harness` | deprecate-with-no-successor | working tree |
| 2026-04-22 | `crates/mock-anthropic-service` | `octopus-sdk-model` tests/fixtures | working tree |
| 2026-04-22 | `crates/rusty-claude-cli` | `octopus-cli` | working tree |
| 2026-04-22 | `crates/octopus-desktop-backend` | `crates/octopus-desktop` | working tree |
| 2026-04-22 | `crates/octopus-model-policy` | `octopus-sdk-model::RoleRouter` | working tree |

---

## 10. 变更日志

| 日期 | 变更 | 责任人 |
|---|---|---|
| 2026-04-20 | 首稿：10 个旧 crate 的模块/符号级退役矩阵 + 守护扫描 | Architect |
| 2026-04-20 | P0 事实修订：§1 总览表行数改为 2026-04-20 `wc -l` 实测值（共校正 7 项：`runtime 19 281`、`tools 14 391`、`plugins 3 938`、`api 5 597`、`octopus-runtime-adapter 38 673`、`commands 5 010`、`rusty-claude-cli 12 844`、`compat-harness 385`、`mock-anthropic-service 1 178`、`octopus-desktop-backend 196`、`octopus-model-policy 143`），并就 `rusty-claude-cli` 实际体量为 W6 工作量给出警示 | Architect |
| 2026-04-20 | P1 修订：§0 增加"目标态路径"说明，明确 `octopus-platform::{config/governance/host/…}` 等子模块属 W4–W7 新建目标态，当前不存在即属预期 | Architect |
| 2026-04-21 | 审计修复：`§8` 的 Weekly Gate 守护扫描把单文件 ≤ 800 行检查从 `find -size +800` 改为 `wc -l + awk` 行数版，并同步把 W8 特例说明改成引用该命令 | Codex |
| 2026-04-21 | W5 Weekly Gate 收尾：`runtime::plugin_lifecycle`、`plugins::{manifest,hooks,hook_dispatch,lifecycle}` 已切到 `replaced`；`plugins::discovery` 保持 `partial`，`tools::subagent_runtime` 维持 greenfield SDK 覆盖边界；`worker_boot` 继续留在 W7，并明确 W5 只保留 non-source 说明 | Codex |
| 2026-04-22 | W7 Task 7：11 个 legacy crate 目录已从 workspace 实盘删除；`§9 已退役` 新增目录级完成记录，并声明目录删除态以 `§9` 为准 | Codex |
| 2026-04-22 | W7 Weekly Gate 收尾：11 个 legacy crate 的删除态经 `cargo build --workspace`、`cargo clippy --workspace -- -D warnings`、desktop 全量测试、legacy grep、`ls crates/` 与 Phase 4/8 治理脚本复核通过，W7 目录级退役状态冻结。 | Codex |
