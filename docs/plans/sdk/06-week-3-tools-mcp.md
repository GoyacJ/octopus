# W3 · `octopus-sdk-tools` + `octopus-sdk-mcp`

> 本文档遵循 `docs/plans/sdk/AGENTS.md` 与 `docs/plans/PLAN_TEMPLATE.md`；执行规约见 `docs/plans/sdk/01-ai-execution-protocol.md`。
>
> 阅读顺序：**本文件 →** `docs/sdk/03-tool-system.md` → `02-crate-topology.md §2.4 / §2.5 / §5 / §8` → `03-legacy-retirement.md §3 + §6.1（Capability Bridge 行） + §8 守护扫描`。

## Goal

产出 **两个零业务语义的新 crate**——`crates/octopus-sdk-tools`（Level 2）与 `crates/octopus-sdk-mcp`（Level 2），落地 `docs/sdk/03` 的 **`trait Tool` + `ToolRegistry` + `ToolContext` + `partition_tool_calls`**、**15 个内置工具（10 full + 4 W5-stub + 1 W6-stub）**、**MCP 三 transport（stdio / http / sdk-inprocess）与 `McpServerManager` 生命周期**；并在本周末 **整块删除 `crates/tools/src/capability_runtime/**` + `crates/octopus-runtime-adapter` 的三个 capability bridge 文件**，把 `Capability Planner / Surface / Exposure` 三段式（取舍 #2）永久下线。本周结束时，`rg "capability_runtime|CapabilityPlanner|CapabilitySurface" crates/` 必须 **0 命中**。

## Architecture

- **Level 2 · tools**：引用 Level 0 `octopus-sdk-contracts`；允许 `tokio / async-trait / futures / serde / serde_json / globset / ignore / regex / reqwest / sha2 / bytes / tracing / tempfile / dunce`；禁止引用 `rusqlite`、业务域 crate、`tauri`、`axum`、Level 3+ 任意 crate。允许对 `octopus-sdk-mcp` 保持**单向窄依赖**，但仅限两类用途：① 为 `mcp::ToolDirectory` 实现 trait；② 复用 `McpTool / McpToolResult / McpError` 做 SDK in-process shim adapter。`tools` 不得持有 `McpClient / McpServerManager / transport` 生命周期逻辑。
- **Level 2 · mcp**：引用 Level 0 `octopus-sdk-contracts`；允许 `tokio / tokio-util / async-trait / serde / serde_json / reqwest / futures / tracing / url`；**不依赖** `octopus-sdk-tools`。W3 的 `mcp` 只定义 pull-based `ToolDirectory` + `SdkTransport::from_directory(...)`；由上层把任意 directory 注入。把外部 MCP tool 包成 `Arc<dyn Tool>` 的反向 adapter **不在 W3 落地**，延到 W5 `octopus-sdk-plugin` / W6 `octopus-sdk-core` 决策，避免 `mcp → tools` 反向依赖。
- **`trait Tool` 的设计原则**：仅声明"execute（输入→输出）+ spec（宣告） + 并发安全判定"；不感知 session / brain loop / compaction / hooks。钩子由 W4 `octopus-sdk-hooks` 在调度外层切入，不污染 Tool 实现。
- **内置 vs MCP 统一为一种协议（取舍 #4）**：通过 **schema 同构**实现（`ToolSpec ≡ McpTool`），不强制每次调用走 JSON-RPC 往返。W3 只落 **内置工具 → MCP** 的 in-process shim：`McpClient + SdkTransport` 可以 list/call `ToolRegistry` 里的内置工具。外部 MCP server 反向挂入 `ToolRegistry` 的 adapter 不在本周交付，延 W5/W6 与 plugin/core 一并决策。测试阶段覆盖一次 round-trip 即可证明协议统一（R4）。
- **权限握手下沉（Contracts 补丁，沿用 W2 `ToolSchema` 下沉先例）**：在 `octopus-sdk-contracts` 新增 **最小** `trait PermissionGate` + `PermissionOutcome` + `PermissionMode`；W3 `ToolContext` 持有 `Arc<dyn PermissionGate>`，W4 `octopus-sdk-permissions` 提供真实实现（`PermissionPolicy / ApprovalBroker / `四态 mode`语义`）。W3 的 `ToolRegistry::check_before_execute()` 只调用 `check()` 并根据 `Allow / Deny / AskApproval` 三变体分支；`RequireAuth` 变体由 W4 追加。
- **沙箱占位（W4 升级）**：`ToolContext.sandbox: SandboxHandle` 为最小值类型（内部只持 `cwd: PathBuf + env_allowlist: Vec<String>`），真实 Bubblewrap / Seatbelt 后端在 W4 落地。W3 `BashTool` 直接用 `tokio::process::Command` 在 `cwd` 下执行；`ToolSpec.description` 明示"未沙箱化"，交由调用方（W6 Brain Loop）在合适 mode 下放行。
- **Prompt Cache 稳定（C1）**：`ToolRegistry::schemas_sorted()` 返回的 `ToolSpec` 序列 **确定性排序 + 字节稳定序列化**——排序键为 `(category_priority, name)`；SHA-256 指纹 `tools_fingerprint()` 作为 W4 Compactor / W6 Brain Loop 的缓存键守护。排序契约测试是 **本周硬门禁**。
- **MCP 进程边界（R3）**：`McpServerManager` 持有 `Arc<Mutex<HashMap<ServerId, ServerHandle>>>`；每个 `ServerHandle` 在 `Drop` 时必须 **同步 kill + wait**（不仅仅是 `drop(child)`），避免 tokio::process::Child 默认"不杀"行为造成僵尸进程。集成测试覆盖 `drop(manager) → ps` 无残留。
- **并行保留边界**：`crates/api` / `crates/octopus-model-policy` / `crates/runtime` / 大部分 `crates/tools` 与 `crates/octopus-runtime-adapter` 本周 **仍不删**，仅对"capability_runtime/** + 三个 capability bridge"做精准外科切除（见 Task 9）。

## Scope

- In scope：
  - 新建 `crates/octopus-sdk-tools/` 与 `crates/octopus-sdk-mcp/` 两个 crate 骨架（含 `Cargo.toml` / `src/lib.rs` / `tests/`）。
  - `02 §2.4` 全部数据符号：`Tool / ToolSpec / ToolCategory / ToolRegistry / ToolContext / ToolResult / ToolError / ToolCallRequest / ExecBatch / partition_tool_calls / BASH_MAX_OUTPUT_DEFAULT / BASH_MAX_OUTPUT_UPPER_LIMIT / DEFAULT_TOOL_MAX_CONCURRENCY` 的最小签名。
  - `02 §2.5` 全部数据符号：`McpTransport / McpClient / McpServerManager / JsonRpcRequest / JsonRpcResponse / JsonRpcNotification / McpError / McpTool / McpPrompt / McpResource / McpToolResult / StdioTransport / HttpTransport / SdkTransport`。
  - **`octopus-sdk-contracts` 新增 Level 0 权限握手面**（W1 补丁，沿用 W2 对 `ToolSchema` 的 B3 先例）：`trait PermissionGate` / `PermissionOutcome` / `PermissionMode` / `ToolCallRequest`。`ToolCallRequest` 下沉到 contracts 是因为权限层也需引用同一值形状（Tools 与 Permissions 同为 Level 2，不可横向依赖）。
  - **15 个内置工具**落地（**10 full + 5 stub**，满足 `00 §3 W3` 出口"15 个内置工具以 `trait Tool` 形式存在"的唯一硬指标；`full` 与 `stub` 的分布是实现层取舍，非 `00` 硬门禁）：
    - **10 full**：`FileReadTool / FileWriteTool / FileEditTool / GlobTool / GrepTool / BashTool / WebFetchTool / AskUserQuestionTool / TodoWriteTool / SleepTool`。
    - **4 W5-stub**（返回 `ToolError::NotYetImplemented { crate_name: "octopus-sdk-subagent", week: "W5" }`，`ToolSpec.description` 前缀 `[STUB · W5]`）：`AgentTool / SkillTool / TaskListTool / TaskGetTool`。
    - **1 W6-stub**（返回 `ToolError::NotYetImplemented { crate_name: "octopus-sdk-tools::web_search", week: "W6" }`，`ToolSpec.description` 前缀 `[STUB · W6]`）：`WebSearchTool`——**降级理由**：W3 无稳定的 `SearchProvider` 抽象归属（当 `octopus-sdk-plugin` 在 W5 落地后再迁）、无 vendor key 注入通路（W6 Brain Loop 才装配），强行在 W3 绑定 vendor API 会污染 `octopus-sdk-tools` 的纯度；W3 只保留 `WebSearchTool` 的 `impl Tool` 壳与 `ToolSpec`，满足"15"出口。
  - MCP `stdio / http / sdk`三 transport 的 full impl + 集成测试（含 **process drop 安全 · R3**、`notifications/tools/list_changed` 刷新、错误传播）。
  - `partition_tool_calls(calls, registry) → Vec<ExecBatch>`：**工具级**并发/串行——`tool.is_concurrency_safe(&input) == true` 的连续段聚合为 `Concurrent` 批次（上限 `DEFAULT_TOOL_MAX_CONCURRENCY = 10`），否则独立 `Serial` 批次；未注册工具强制 `Serial`。**不包含** `ResourceKey` 级细粒度桶（同一文件路径并发写的资源冲突检测），此语义 **延 W4**：届时由 `octopus-sdk-permissions::PermissionPolicy` + `octopus-sdk-hooks::HookRunner` 在调度外层兜底，`partition_tool_calls` 签名不变，只在 W4 补充 "serial bucket override" 辅助（登记到 `02 §5`）。
  - **Prompt cache 稳定性硬门禁**：`ToolRegistry::schemas_sorted()` 3 次调用字节一致；`tools_fingerprint()` 单调稳定。
  - **Bash 输出截断硬门禁**：`BASH_MAX_OUTPUT_DEFAULT = 30_000` 字符（非 token）写为 `pub const`；`BASH_MAX_OUTPUT_LENGTH` 环境变量覆盖路径可达 `BASH_MAX_OUTPUT_UPPER_LIMIT = 150_000`。
  - **capability 系列退役**：删除 `crates/tools/src/capability_runtime/**`（6 文件 / ~3 092 行）+ `crates/octopus-runtime-adapter/src/capability_{executor_bridge,planner_bridge,state,runtime_tests}.rs`（4 文件 / ~1 800 行）+ 所有 call-site 的 `use` 与函数体清理（跨 ~30 文件）。
  - `02 §2.4 / §2.5` 同批次回填新增 `pub` 符号；`§5 契约差异清单` 登记与 `contracts/openapi/src/**` 的差异。
- Out of scope：
  - `crates/tools/src/{builtin_catalog, builtin_exec, fs_shell, skill_runtime, web_external, tool_registry, split_module_tests}.rs` 的**目录级删除**（留 W7；本周只做"符号替换 + 不再被 SDK 引用"）。
  - `crates/runtime/src/{mcp.rs, mcp_client.rs, mcp_lifecycle_hardened.rs, mcp_stdio.rs, bash.rs, file_ops.rs}` 的删除（留 W7；本周只做"新 SDK 实现替代它们的能力，不改旧代码"）。
  - `subagent_runtime.rs`（1 024 行）的迁移——归 W5；W3 只为 `AgentTool` 留 stub。
  - 真实 Bubblewrap / Seatbelt 沙箱后端（归 W4 `octopus-sdk-sandbox`）。
  - 真实 `PermissionPolicy` / `ApprovalBroker` / `PermissionMode` 四态语义判定（归 W4 `octopus-sdk-permissions`）；W3 只提供 trait 形状 + `Allow/Deny/AskApproval` 三变体。
  - `HookRunner` 对 Tool 调度的切入（归 W4 `octopus-sdk-hooks`）。
  - `Compactor` 对 tool result 的清理策略（归 W4 `octopus-sdk-context`）。
  - `Code Execution Mode with MCP`（`docs/sdk/03 §3.6.3`；留 plugin 扩展点，不入 W3）。
  - `NotebookEditTool / PowerShellTool / LspTool / EnterPlanModeTool`（非 15 内置工具，`docs/sdk/03 §3.4 注` 已声明首版不强制）。
  - **`task_output` 工具**（`docs/sdk/03-tool-system.md §3.4` 表格 line 290 中与 `task_list / task_get` 并列的第 3 个 "monitor" 类工具）：W3 不落地 `TaskOutputTool`。依据——`00-overview.md §3 W3 出口状态` 与 `02-crate-topology.md §2.4 builtin` 登记的 15 工具名单 **均未包含** `TaskOutputTool`；本 Plan 承接 "15 工具" 出口口径，不扩容第 16 个。解决路径：Task 10 Step 2 在 `docs/sdk/README.md ## Fact-Fix 勘误` 追加 **`[Fact-Fix] 03-tool-system.md §3.4 · task_output`** 条目，只说明 "SDK 首版 15 工具不包含独立 `TaskOutputTool`，W3 当前 contracts 也未定义等价 retrieval 契约；若后续要折叠进 `task_get` 或新增 session 事件，必须先登记到 `02 §2.1 / §2.4`，再由 W5 `octopus-sdk-subagent` 决策"。

## Risks Or Open Questions

| # | 风险 / 问题 | 决策建议 | 触发 Stop Condition |
|---|---|---|---|
| R1 | Prompt Cache 稳定性：`ToolRegistry::schemas_sorted()` 若在"注册顺序 ≠ 字典序"时返回注册顺序，会在 W4/W6 把工具列表拼进 system prompt 时击穿缓存（`docs/sdk/03 §3.2.1 C1`）。 | `schemas_sorted()` 按 `(category_priority, name)` 双键排序，`category_priority` 为 `Read=0, Write=1, Network=2, Shell=3, Subagent=4, Skill=5, Meta=6`；序列化 `ToolSpec.input_schema` 前统一 `serde_json::Value` 的 object key 字典序。契约测试 `tools/registry_stability.rs` 3 次调用字节一致；指纹 SHA-256 固定。 | 命中率潜在下降 → Stop #4 |
| R2 | Tool 调度 vs Hooks / Compactor 的耦合点：若 W3 `ToolRegistry::dispatch()` 直接实现"调度流水线"（`docs/sdk/03 §3.5.1` 的 8 阶段），W4 `HookRunner` / `Compactor` 切入会改动 dispatch 签名 → 公共面抖动。 | **不在 W3 实现 dispatch 流水线**。`ToolRegistry::dispatch()` **不落**。W3 只提供 `Tool::execute + ToolRegistry::get + partition_tool_calls`；完整流水线由 W6 `octopus-sdk-core::BrainLoop` 拼装，钩子插入点由 W4 负责。 | 若发现 W3 某处必须有 dispatcher → Stop #2（层越权） |
| R3 | MCP 子进程泄漏（`docs/plans/sdk/00-overview.md §6 R3`）：`tokio::process::Child` 默认在 drop 时不 kill，只是 detach；一旦测试异常退出或 `McpServerManager` 提前释放，会残留僵尸 stdio 子进程。 | `StdioProcessGuard` 封装 `Child`，`Drop` 实现调用 `self.child.start_kill()` + `block_on(self.child.wait())`（用 `tokio::task::block_in_place`）。集成测试 `tests/mcp_process_lifecycle.rs` 跑 50 次启动/释放循环后断言 `ps -o pid,ppid,comm -p $(...)` 无 `mcp-echo-server` 残留。Windows 路径用 `TerminateProcess`（`winapi` feature-gated，W3 先 Linux/macOS 覆盖，Windows 留 `TODO(W8)`）。 | process 残留 → Stop #7（验证失败）或 #10（workspace test 连锁失败） |
| R4 | 15 个内置工具面并非本周都要"full"：**5 个 stub**（`AgentTool / SkillTool / TaskListTool / TaskGetTool` 四 W5-stub + `WebSearchTool` 一 W6-stub）在 W3 返回 `ToolError::NotYetImplemented`；若 stub 被 W6 Brain Loop 误认为 "真能跑"，E2E 会失败。 | W5-stub 的 `ToolSpec.description` 前 80 字符以 `"[STUB · W5]"` 开头、W6-stub 以 `"[STUB · W6]"` 开头；`execute()` 返回的 `ToolResult.is_error = true`；Registry 在 `schemas_sorted()` 输出时保留，但 `dispatch`（W6 加）会对 stub 在非 `plan` mode 下 hard-fail。 | 若 W3 出口后某业务链路实际需要 `AgentTool`/`SkillTool`/`WebSearchTool` 真执行 → Stop #8（跨周契约断裂） |
| R5 | "内置工具走 MCP in-process shim"（取舍 #4）是否要求每次调用都走 JSON-RPC 序列化？ | **不要求**。内置 `ToolRegistry::execute_by_name(name, input, ctx)` 走直连（零序列化）；`SdkTransport::from_directory(registry.as_directory())` 暴露一条 shim path 让 MCP 客户端也能调到同一 Tool；集成测试 `tests/mcp_sdk_transport_roundtrip.rs` 覆盖 **1 个 builtin tool 走 shim 跑通**即算满足。 | 若 spec 审计判定"必须全走 shim" → Stop #1（规范冲突） |
| R6 | `trait PermissionGate` 下沉到 Level 0 contracts 是否违反"SDK 对业务仅暴露 4 类 trait"的承诺（`README.md §关键不变量 #6`）？ | 不违反：四类 trait 指"业务侧看得见的入口"（`AgentRuntime / SessionStore / ModelProvider / SecretVault`）；`PermissionGate` 是 SDK 内部层间协议，业务看不见。contracts 新增不扩业务面；与 W2 `ToolSchema` 下沉完全对称（复用同一先例）。 | 若评审判定 contracts 公共面膨胀超预期 → 回退为"tools 内定义 handshake 浅 trait，permissions W4 注入 adapter" | — |
| R7 | Capability 删除的"call-site 重写"规模：**扩展扫描口径**（原 6 token 不足以覆盖仅持 `capability_state_ref / load_capability_store / persist_capability_store` 的活调用点，典型漏网：`execution_events.rs`、`persistence.rs`、`approval_runtime_tests.rs`、`adapter_state.rs`、`memory_selector.rs`、`subrun_orchestrator.rs`、`team_runtime.rs`、`runtime_persistence_tests.rs`）。用 `Capability Scan Superset（下定义）`统计命中 ≈ **35 + 个文件**（以 `octopus-runtime-adapter` 为主力，外圈覆盖 `crates/octopus-server::workspace_runtime`、`crates/octopus-platform::runtime`、`crates/octopus-core::lib`、`crates/octopus-infra::infra_state`、`crates/rusty-claude-cli::main`）。单 PR ≤ 800 行约束下必须拆 **至少 4 个 sub-PR**。 | Task 9 拆 4 sub-Steps（9a 审计与分类 / 9b adapter 内部替代 / 9c 外圈 legacy crate call-site 清理 / 9d 文件删除 + 守护扫描）；**每个 sub-Step 单 PR ≤ 800 行**；若审计后发现调用面实际超 4 个 PR 能承载 → 追加 9e。因为这批 legacy crate 将在 W7 统一删除，9c 允许采用 "panic-stub + TODO(W7-RETIRE)" 最短路径（不要求迁到新 SDK 实现，只要求 call-site 不再 `use` 被删符号）。**审计命令统一使用 Capability Scan Superset**；**Weekly Gate（Task 10 Step 3）** 对外仍沿用 `00-overview.md §3 W3` 的 3-token 硬门禁作为对外口径，但同时叠加 Capability Scan Superset = 0 作为 W3 内部的更严保险。 | 若 adapter 某处必须在 W3 即切到新 SDK dispatch（例如 Brain loop 路径）→ Stop #8（跨周契约：Brain Loop 属 W6） |

#### Capability Scan Superset 定义（R7 / Task 9 全程共用）

统一正则，供 `rg` / `grep -E` 使用（`<SUPER>`）：

```
capability_runtime|CapabilityPlanner|CapabilitySurface|CapabilityExecutor|CapabilityExposure|CapabilityState|CapabilityStore|capability_executor_bridge|capability_planner_bridge|capability_state|capability_state_ref|load_capability_store|persist_capability_store
```

约定：
- `<SUPER>` 命中 > 0 → Task 9 内部不得声明完成；
- `00-overview.md §3 W3` 的 3-token 口径（`capability_runtime|CapabilityPlanner|CapabilitySurface`）= 0 仍是对外硬门禁，由 Task 10 Step 3 勾选；
- 出现 `<SUPER>` 命中 0 而 3-token 命中 > 0 的情况（不可能但需兜底）→ Stop #1（规范自洽断裂）。
| R8 | `octopus-runtime-adapter::persistence.rs`、`session_service.rs`、`agent_runtime_core.rs` 等文件同时被 W1（session 下沉）、W2（model 下沉）、W3（capability 退役）反复修改 → 合入冲突 + 行数爆炸 | W3 的 Task 9b 仅触碰与 capability 相关的函数/方法（按 `rg capability` 定位），其它行不动；若某函数同时使用 capability + session/model 又无法分离 → 登记到 `02 §5 契约差异清单` 并延到 W7 统一下线。 | 若冲突到无法分离 → Stop #6（PR 无法再拆） |
| R9 | `AskUserQuestionTool` 的交互语义：W3 没有 Brain Loop（W6），谁接住 `AskPrompt` 并返回用户答案？ | W3 `AskUserQuestionTool::execute()` 通过 `ToolContext.ask_resolver: Arc<dyn AskResolver>` 注入的 trait 异步获取答案；默认 `NoopAskResolver` 直接 `Err(ToolError::Interactive)`。W3 单元测试用 `MockAskResolver` 注入预设答案。真实对接业务交互层在 W6 Brain Loop。 | 若发现 `AskResolver` 签名需要跨 crate 共享 → 下沉到 contracts（与 PermissionGate 同等) |
| R10 | `WebFetchTool` / `WebSearchTool` 的外部网络：W3 CI 不能真连网 | 单元测试用 `wiremock`（W2 已引入 dev-dep）；集成测试 `tests/web_fetch_mock.rs` 只跑 mock；真实 provider 拨测放 W6 E2E。 | 若某 vendor SDK 强制网络 握手 → 记 `02 §5` |

## 本周 `02 §2.1 / §2.4 / §2.5` 公共面修订清单（同批次回填）

> 以下 22 处签名修订必须在 Task 1 / Task 2 / Task 3 / Task 4 / Task 5 / Task 6 / Task 10 合入批次内**同 PR** 回填到 `02-crate-topology.md`，否则视为 `Stop Condition #1` 裸增。

### `02 §2.1 octopus-sdk-contracts`（W1 补丁 · Level 0 下沉）

| # | 位置 | 修订类型 | 内容 |
|---|---|---|---|
| 1 | `§2.1` 新增类型 | 类型新增 | `ToolCallRequest { id: String, name: String, input: serde_json::Value }` |
| 2 | `§2.1` 新增类型 | 类型新增 | `PermissionMode { Default, AcceptEdits, BypassPermissions, Plan }` |
| 3 | `§2.1` 新增类型 | 类型新增 | `PermissionOutcome { Allow, Deny { reason: String }, AskApproval { prompt: AskPrompt } }`（`RequireAuth` 由 W4 追加） |
| 4 | `§2.1` 新增 trait | trait 新增 | `trait PermissionGate: Send + Sync { async fn check(&self, call: &ToolCallRequest) -> PermissionOutcome; }` |
| 5 | `§2.1` 新增 trait | trait 新增 | `trait AskResolver: Send + Sync { async fn resolve(&self, prompt_id: &str, prompt: &AskPrompt) -> Result<AskAnswer, AskError>; }` + `AskAnswer / AskError` |
| 6 | `§2.1` 新增 trait | trait 新增 | `trait EventSink: Send + Sync { fn emit(&self, event: SessionEvent); }` |

### `02 §2.4 octopus-sdk-tools`

| # | 位置 | 修订类型 | 内容 |
|---|---|---|---|
| 7 | `§2.4` `Tool` trait | 保持 | `spec / is_concurrency_safe / execute` 三方法保持；`execute` 返回 `Result<ToolResult, ToolError>` |
| 8 | `§2.4` `ToolSpec` | 字段保持 | 4 字段；`input_schema` 为 `serde_json::Value`（JSON Schema v7） |
| 9 | `§2.4` `ToolCategory` | 保持 | 7 变体；新增 `category_priority()` 常量函数固定排序权重 |
| 10 | `§2.4` 新增类型 | 类型新增 | `ToolError`（`Validation / Permission / Execution / Timeout / Cancelled / NotYetImplemented { crate_name: &'static str, week: &'static str } / Transport(#[from] reqwest::Error) / Serialization(#[from] serde_json::Error) / Sandbox { reason: String }`） |
| 11 | `§2.4` `ToolContext` | 字段调整 | 去 `session: SessionId`（挪到 `session_id: SessionId`），新增 `ask_resolver: Arc<dyn AskResolver>`、`secret_vault: Arc<dyn SecretVault>`、`permissions: Arc<dyn PermissionGate>`、`sandbox: SandboxHandle`（W3 占位类型）、`working_dir: PathBuf`、`cancellation: CancellationToken`、`event_sink: Arc<dyn EventSink>`（`EventSink` 从 contracts re-export） |
| 12 | `§2.4` 新增类型 | 类型新增 | `SandboxHandle { pub cwd: PathBuf, pub env_allowlist: Vec<String> }`（W3 占位；W4 `octopus-sdk-sandbox` 升级） |
| 13 | `§2.4` `ToolRegistry` | 方法补齐 | `new / register / get / schemas_sorted / tools_fingerprint / iter`；**不含 `dispatch`**（R2 决策） |
| 14 | `§2.4` `partition_tool_calls` | 签名细化 | `fn partition_tool_calls<'a>(calls: &'a [ToolCallRequest], registry: &ToolRegistry) -> Vec<ExecBatch<'a>>`；`ExecBatch<'a> { Concurrent(Vec<&'a ToolCallRequest>), Serial(Vec<&'a ToolCallRequest>) }` |
| 15 | `§2.4` 常量 | 常量锁定 | `BASH_MAX_OUTPUT_DEFAULT: usize = 30_000`、`BASH_MAX_OUTPUT_UPPER_LIMIT: usize = 150_000`、`DEFAULT_TOOL_MAX_CONCURRENCY: usize = 10` |
| 16 | `§2.4` `builtin` 模块 | 符号新增 | 15 个 struct：`FileReadTool / FileWriteTool / FileEditTool / GlobTool / GrepTool / BashTool / WebSearchTool / WebFetchTool / AskUserQuestionTool / TodoWriteTool / SleepTool / AgentTool / SkillTool / TaskListTool / TaskGetTool`（4 个 W5-stub + 1 个 W6-stub） |

### `02 §2.5 octopus-sdk-mcp`

| # | 位置 | 修订类型 | 内容 |
|---|---|---|---|
| 17 | `§2.5` `McpTransport` | 保持 | 1 方法 `call`；新增 `async fn notify(&self, msg: JsonRpcNotification) -> Result<(), McpError>` |
| 18 | `§2.5` 新增类型 | 类型新增 | `JsonRpcRequest / JsonRpcResponse / JsonRpcNotification / JsonRpcError { code: i32, message: String, data: Option<serde_json::Value> }` |
| 19 | `§2.5` 新增类型 | 类型新增 | `McpError`（`Transport / Protocol / Timeout / Handshake / ServerCrashed { server_id, exit_code } / ToolNotFound / InvalidResponse { body_preview }`） |
| 20 | `§2.5` 新增类型 | 类型新增 | `InitializeResult { protocol_version: String }` |
| 21 | `§2.5` `McpClient` | 字段 | `transport: Arc<dyn McpTransport>`、`server_id: String`、`initialized: AtomicBool`；方法 `initialize / list_tools / call_tool / list_prompts / list_resources` 全 `async fn` |
| 22 | `§2.5` 新增类型 | 类型新增 | `McpLifecyclePhase { Starting, Ready, Degraded, Stopped }` |
| 23 | `§2.5` 新增类型 | 类型新增 | `McpServerSpec { server_id: String, transport: McpServerTransport }` 与 `McpServerTransport::{ Stdio { command, args, env, transport }, Http { transport }, Sdk { transport } }` |
| 24 | `§2.5` `McpServerManager` | 方法新增 | `spawn / shutdown / list_servers / get_client`；内部 `Drop` 保证 kill + wait 所有 stdio 子进程（R3） |
| 25 | `§2.5` transport 三 impl | 符号新增 | `StdioTransport / HttpTransport / SdkTransport` + 各自构造器 `StdioTransport::spawn(cmd, args, env)` / `HttpTransport::new(url, headers)` / `SdkTransport::from_directory(dir: Arc<dyn ToolDirectory>)` |
| 26 | `§2.5` 新增 trait | trait 新增 | `#[async_trait] trait ToolDirectory: Send + Sync { fn list_tools(&self) -> Vec<McpTool>; async fn call_tool(&self, name: &str, input: serde_json::Value) -> Result<McpToolResult, McpError>; }`（**全 MCP 原生类型**，不触 `octopus-sdk-tools` 的任何符号；由 `octopus-sdk-tools::ToolRegistry` 在 tools crate 内 `impl ToolDirectory for ToolRegistry`，方向保持 `tools → mcp`。详见 Task 4 Step 4 + Task 6 Step 3 + Task 8 Step 4 三处一致化决策。） |

任何额外出现的 `pub` 符号都必须在 Task 10 Step 1 之前追加到本表与 `02 §2.1 / §2.4 / §2.5`，否则 Weekly Gate 阻断。

---

## Execution Rules

- 遵循 `01-ai-execution-protocol.md`：三层 Checklist + Stop Conditions 1–11 全部生效。
- 每个 Task 原子、单 PR ≤ 800 行；违反 → 拆 sub-Task。
- 公共面（`pub` 符号）变动 → 同一 PR 必须更新 `02-crate-topology.md §2.1 / §2.4 / §2.5`；违反 → Stop Condition #1。
- 任何与 `contracts/openapi/src/**` 的字段差异 → 登记到 `02 §5`，**不**直接改 openapi。
- `crates/api / crates/octopus-model-policy / crates/runtime / crates/tools / crates/octopus-runtime-adapter / crates/plugins` 的改动本周**仅允许**发生在 Task 9 定义的 **capability 退役范围**；其他业务语义改动一律 defer 到 W7。
- 单文件 ≤ 800 行；`src/lib.rs` ≤ 80 行（仅 `mod` + `pub use`）。
- `default-members` 在本周结束时追加 `crates/octopus-sdk-tools` 与 `crates/octopus-sdk-mcp`。**阶段性偏离登记**：`02-crate-topology.md §8` 的最终口径是"仅保留 5 个业务 crate + Tauri app"；W1–W6 为了让 SDK crate 进入默认编译闭包、获得 `cargo build` 的基线守护，**临时** 把 `octopus-sdk-contracts / session / model / tools / mcp / permissions / sandbox / hooks / context / plugin / subagent / core` 逐步追加进 `default-members`。回收点 = **W7 legacy 整合清理**：W7 在删除 `crates/runtime / crates/tools / crates/api / crates/plugins / crates/octopus-runtime-adapter / crates/commands` 等旧 crate 的同一批 PR 中，把 `default-members` 收敛回 §8 目标口径；本 Plan 不再重复约束，只在变更日志与 `03-legacy-retirement.md` 的 "W7 整合清单" 留指针。
- 禁止引入 `rusqlite` 到 `octopus-sdk-tools / octopus-sdk-mcp` 的 `[dependencies]`；禁止使用 `env::var` 读取任何凭据（必须经 `SecretVault`）；`octopus-sdk-mcp` 禁止依赖 `octopus-sdk-tools`；`octopus-sdk-tools` 只允许为 `ToolDirectory` impl 与 `ToolResult ↔ McpToolResult` adapter 对 `octopus-sdk-mcp` 保持窄依赖，不得持有 `McpClient / McpServerManager / transport` 生命周期逻辑。
- MCP 子进程生命周期测试不得在 CI 被 skip；任何 `#[ignore]` 需在 `02 §5` 标注。
- Task 9 的每个 sub-Step 合入前，必须跑 `cargo build --workspace` 与 `cargo clippy --workspace -- -D warnings` 均绿；任一失败立即回滚。

---

## Active Work

- Current task: `Done（W3 Weekly Gate 已完成）`
- Current step: `Completed：Task 10 Step 3（全部门禁通过，W3 收口）`
- Execution mode: `continuous-by-task`

### Pre-Task Checklist（Task 1，启动前勾选）

- [x] 已阅读本子 Plan 的 `Goal` / `Architecture` / `Scope`。
- [x] 已阅读 `00-overview.md §1 10 项取舍`（特别是 #2 删 Capability Planner、#4 内置工具走 MCP shim），且当前任务未违反。
- [x] 已阅读 `docs/sdk/03-tool-system.md §3.1 / §3.2 / §3.3 / §3.4 / §3.5 / §3.6`。
- [x] 已阅读 `02-crate-topology.md §1 依赖图 / §2.4 / §2.5`。
- [x] 已阅读 `03-legacy-retirement.md §3（crates/tools）/ §6.1 Capability Bridge 行 / §8 守护扫描`。
- [x] 已识别本 Task 涉及的 SDK 对外公共面变更（是 / 否）。
  - 当前判断：`是`（§2.1 contracts 新增 5 项 + §2.4/§2.5 全部公共面本周首次落地）。
- [x] 已识别是否涉及 `contracts/openapi/src/**` 或 `packages/schema/src/**`。
  - 当前判断：`否`（差异走 `02 §5` 登记，不改 OpenAPI）。
- [x] 已识别是否涉及 `docs/sdk/14` UI Intent IR 变更（是 / 否）。
  - 当前判断：`否`（`AskPrompt` / `RenderBlock` 沿用 W1 现状）。
- [x] Preconditions 已全部满足；未满足项已在 `Open Questions` 中登记。
- [x] 当前 git 工作树干净或有明确切分；本批次计划 diff ≤ 800 行（不含 generated）。
- [x] 已识别所有 `Stop if:` 条款；遇到任一条件 → 立即停止并汇报。

---

## Task Ledger

### Task 1：两 crate 骨架 + workspace 登记 + contracts 权限握手下沉

Status: `done`

Files:
- Create: `crates/octopus-sdk-tools/Cargo.toml`
- Create: `crates/octopus-sdk-tools/src/lib.rs`
- Create: `crates/octopus-sdk-mcp/Cargo.toml`
- Create: `crates/octopus-sdk-mcp/src/lib.rs`
- Create: `crates/octopus-sdk-contracts/src/permission.rs`（新增 `PermissionGate / PermissionOutcome / PermissionMode / ToolCallRequest`）
- Create: `crates/octopus-sdk-contracts/src/ask_resolver.rs`（新增 `AskResolver / AskAnswer / AskError`）
- Modify: `crates/octopus-sdk-contracts/src/lib.rs`（`mod permission; mod ask_resolver; pub use ...;` 行数守约 ≤ 80）
- Modify: `Cargo.toml`（workspace `default-members` 追加 `"crates/octopus-sdk-tools", "crates/octopus-sdk-mcp"`）
- Modify: `docs/plans/sdk/02-crate-topology.md §2.1`（同批回填"本周公共面修订清单" #1–#5）
- Modify: `docs/plans/sdk/README.md`（W3 行状态 `pending → in_progress`）

Preconditions：W2（`05-week-2-model.md`）状态 `done`；`cargo test -p octopus-sdk-contracts -p octopus-sdk-model -p octopus-sdk-session` 全绿；`02-crate-topology.md §2.1 / §2.4 / §2.5` 签名与本 Plan Scope 一致；本 Plan 已登记到 `docs/plans/sdk/README.md`。

Step 1：
- Action：创建 `octopus-sdk-tools` / `octopus-sdk-mcp` 两 crate 骨架。
  - `octopus-sdk-tools/Cargo.toml` `[dependencies]` 允许 `serde / serde_json / thiserror / async-trait / tokio / futures / reqwest / bytes / tracing / sha2 / globset / ignore / regex / tempfile / dunce / octopus-sdk-contracts`；禁止 `rusqlite / tauri / axum / octopus-core / octopus-platform / octopus-sdk-mcp`。
  - `octopus-sdk-mcp/Cargo.toml` `[dependencies]` 允许 `serde / serde_json / thiserror / async-trait / tokio / tokio-util / futures / reqwest / bytes / tracing / url / octopus-sdk-contracts`；禁止 `rusqlite / tauri / axum / octopus-core / octopus-platform / octopus-sdk-tools`。
  - 两个 `src/lib.rs` 仅含 `mod` 声明与受控 `pub use`，各 ≤ 80 行，先声明空 `mod` stub。
- Done when：`cargo build -p octopus-sdk-tools -p octopus-sdk-mcp` 成功；`wc -l crates/octopus-sdk-tools/src/lib.rs crates/octopus-sdk-mcp/src/lib.rs` 各 ≤ 80；`rg 'rusqlite|tauri|axum|octopus-core|octopus-platform' crates/octopus-sdk-{tools,mcp}/Cargo.toml` 无结果；`rg 'octopus-sdk-mcp' crates/octopus-sdk-tools/Cargo.toml` 无结果；`rg 'octopus-sdk-tools' crates/octopus-sdk-mcp/Cargo.toml` 无结果。
- Verify：`cargo build -p octopus-sdk-tools -p octopus-sdk-mcp && rg -n '^(rusqlite|tauri|axum|octopus-core|octopus-platform|octopus-sdk-mcp|octopus-sdk-tools)' crates/octopus-sdk-tools/Cargo.toml crates/octopus-sdk-mcp/Cargo.toml`
- Stop if：workspace `[workspace.dependencies]` 中 `globset` / `ignore` / `tokio-util` 缺失且版本锁策略不允许 W3 新增 → Stop #8（等待人决策）。

Step 2：
- Action：在 `octopus-sdk-contracts/src/permission.rs` 落地 `ToolCallRequest { id, name, input }`、`PermissionMode { Default, AcceptEdits, BypassPermissions, Plan }`（`#[serde(rename_all = "snake_case")]`）、`PermissionOutcome { Allow, Deny { reason: String }, AskApproval { prompt: AskPrompt } }`（`RequireAuth` 预留注释 "W4 to add"）、`trait PermissionGate: Send + Sync { async fn check(&self, call: &ToolCallRequest) -> PermissionOutcome; }`（`#[async_trait]`）。`ask_resolver.rs` 落地 `trait AskResolver` + `AskAnswer { prompt_id, option_id, text }` + `AskError { NotResolvable, Timeout, Cancelled }`。
- Done when：`cargo test -p octopus-sdk-contracts permission:: ask_resolver::` 全绿；JSON round-trip 测试覆盖 `PermissionOutcome::AskApproval` + `PermissionMode::BypassPermissions`；`02 §2.1` 已同批补入修订清单 #1–#5。
- Verify：`cargo test -p octopus-sdk-contracts permission:: ask_resolver:: && rg -n 'PermissionGate|PermissionOutcome|PermissionMode|ToolCallRequest|AskResolver' docs/plans/sdk/02-crate-topology.md`
- Stop if：`AskPrompt` 签名与 `octopus-sdk-contracts::ui_intent::AskPrompt` 不兼容（例如 `AskPrompt` 当前缺 `id` 字段） → 先补 `ui_intent`（W1 小修订）再继续；若改动外泄到 `contracts/openapi/src/**` → Stop #3。

Step 3：
- Action：更新 workspace `Cargo.toml` 的 `default-members` 追加 `"crates/octopus-sdk-tools", "crates/octopus-sdk-mcp"`（W7 业务收敛前先登记；与 W1 / W2 先例一致）；把 `docs/plans/sdk/README.md §文档索引` 的 W3 行状态由 `pending` 改为 `in_progress`。
- Done when：`cargo metadata --format-version=1 --no-deps | jq -r '.workspace_default_members[]' | rg 'octopus-sdk-(tools|mcp)'` 两条均命中；`rg '^\| `06-week-3-tools-mcp\.md` \| .* \| `in_progress` \|' docs/plans/sdk/README.md` 命中。
- Verify：`cargo build && cargo metadata --format-version=1 --no-deps | jq -r '.workspace_default_members[]' | rg 'octopus-sdk-'`
- Stop if：workspace build 因 `octopus-sdk-contracts` 新增符号未被某下游使用者消费而 warning-as-error → 调整 clippy allow 策略，不延迟本 Task。

Notes：
- crate 目录与包名均为 `octopus-sdk-tools` / `octopus-sdk-mcp`（短横线）。
- 本 Task 不新增 workspace 级依赖版本条目；如需新增 `globset` / `ignore` / `tokio-util`，以 `workspace = true` 形式引用已存在项；若不存在则作为本 Task 的 Step 4 单独提交并在 PR 说明锁版本（与 W2 `sha2 / bytes` 的处理一致）。
- `PermissionGate` 下沉是本周唯一对 contracts 的改动；类比 W2 `ToolSchema` 的 B3 决策，本 crate 不再出现同名符号。

---

### Task 2：`Tool` trait + `ToolSpec` + `ToolCategory` + `ToolRegistry`（确定性排序）

Status: `done`

Files:
- Create: `crates/octopus-sdk-tools/src/tool.rs`（`trait Tool` + `async_trait`）
- Create: `crates/octopus-sdk-tools/src/spec.rs`（`ToolSpec / ToolCategory`）
- Create: `crates/octopus-sdk-tools/src/registry.rs`（`ToolRegistry + tools_fingerprint`）
- Modify: `crates/octopus-sdk-tools/src/lib.rs`（模块声明 + `pub use`）

Preconditions：Task 1 完成。

Step 1：
- Action：落地 `ToolCategory { Read, Write, Network, Shell, Subagent, Skill, Meta }` + `fn category_priority(self) -> u8`（`Read=0, Write=1, Network=2, Shell=3, Subagent=4, Skill=5, Meta=6`）；`ToolSpec { name: String, description: String, input_schema: serde_json::Value, category: ToolCategory }`（`#[derive(Debug, Clone, Serialize, Deserialize)]`）。提供 `ToolSpec::to_mcp(&self) -> octopus_sdk_contracts::ToolSchema` 便于 MCP 侧 reuse（W1 已下沉的 `ToolSchema`）。
- Done when：`cargo test -p octopus-sdk-tools spec::` 通过；round-trip JSON 测试覆盖 7 个 `ToolCategory` 变体。
- Verify：`cargo test -p octopus-sdk-tools spec::`
- Stop if：`ToolSchema` 的 3 字段（`name / description / input_schema`）与 `ToolSpec` 有语义差异 → Stop #1（W1 契约冲突）。

Step 2：
- Action：落地 `trait Tool: Send + Sync { fn spec(&self) -> &ToolSpec; fn is_concurrency_safe(&self, input: &serde_json::Value) -> bool; async fn execute(&self, ctx: ToolContext, input: serde_json::Value) -> Result<ToolResult, ToolError>; }`（`#[async_trait]`）。`ToolResult / ToolError` 的最小骨架先 stub（具体字段在 Task 3 完成），本 Step 只保证 trait 对象安全。
- Done when：`cargo check -p octopus-sdk-tools` 通过；`Arc<dyn Tool>` 可构造。
- Verify：`cargo check -p octopus-sdk-tools`
- Stop if：`async fn` 与 `dyn Tool` 对象安全冲突 → 使用 `async_trait` 展开；不自行实现 `Box<dyn Future>`。

Step 3：
- Action：落地 `ToolRegistry { tools: BTreeMap<String, Arc<dyn Tool>> }`；方法 `new() -> Self`、`register(&mut self, tool: Arc<dyn Tool>) -> Result<(), RegistryError>`（禁止重名）、`get(&self, name: &str) -> Option<Arc<dyn Tool>>`、`iter(&self) -> impl Iterator<Item = (&str, &Arc<dyn Tool>)>`、`schemas_sorted(&self) -> Vec<&ToolSpec>`、`tools_fingerprint(&self) -> String`（SHA-256 hex，输入 = `schemas_sorted()` 的 `{name}\0{category_priority}\0{input_schema_canonical_json}` 以 `\n` 分隔）。`schemas_sorted` 的排序规则：`(spec.category.category_priority(), spec.name.as_str())`。序列化 JSON 必须走 `serde_json::to_string(&SortedMap)` 固化 object key 字典序。
- Done when：`cargo test -p octopus-sdk-tools registry::` 通过；`registry_stability_byte_equal` 3 次调用 `schemas_sorted` + `serde_json::to_string` 字节一致；`registry_fingerprint_changes_on_new_tool` 注册新工具后指纹变化；`registry_fingerprint_stable_on_reorder` 不同注册顺序但相同工具集合指纹一致。
- Verify：`cargo test -p octopus-sdk-tools registry::`
- Stop if：`BTreeMap` 无法保证 `input_schema` 内嵌对象的 key 排序（`serde_json::Value::Object(Map<String, Value>)` 默认保留插入顺序） → 改用 `serde_json::to_value(&tool_spec).and_then(canonical_json_sort)` 的 helper；若 helper 复杂度超预期，抽 `src/canonical_json.rs` 独立模块。

Notes：
- `ToolRegistry` 不做 "register(hot-op)" ——运行中工具集不变（`docs/sdk/03 §3.9.1`）；若 W5 plugin 需要热注册 → 届时在 `octopus-sdk-plugin` 内做快照再注入，不改本 trait 签名。
- `tools_fingerprint` 的 canonical JSON 实现可 ~80 行；必要时在本 Task 补 `crates/octopus-sdk-tools/src/canonical_json.rs`（仍在本 Task diff 预算内）。

---

### Task 3：`ToolContext` + `ToolResult` + `ToolError` + `partition_tool_calls`

Status: `done`

Files:
- Create: `crates/octopus-sdk-tools/src/context.rs`（`ToolContext + SandboxHandle`）
- Create: `crates/octopus-sdk-tools/src/result.rs`（`ToolResult`）
- Create: `crates/octopus-sdk-tools/src/error.rs`（`ToolError / RegistryError`）
- Create: `crates/octopus-sdk-tools/src/partition.rs`（`ExecBatch / partition_tool_calls`）
- Create: `crates/octopus-sdk-tools/src/constants.rs`（`BASH_MAX_OUTPUT_DEFAULT / BASH_MAX_OUTPUT_UPPER_LIMIT / DEFAULT_TOOL_MAX_CONCURRENCY`）
- Modify: `crates/octopus-sdk-tools/src/lib.rs`

Preconditions：Task 2 完成。

Step 1：
- Action：落地 `SandboxHandle { pub cwd: PathBuf, pub env_allowlist: Vec<String> }`；落地 `ToolContext { pub session_id: SessionId, pub permissions: Arc<dyn PermissionGate>, pub sandbox: SandboxHandle, pub session_store: Arc<dyn SessionStore>, pub secret_vault: Arc<dyn SecretVault>, pub ask_resolver: Arc<dyn AskResolver>, pub event_sink: Arc<dyn EventSink>, pub working_dir: PathBuf, pub cancellation: CancellationToken }`（`tokio_util::sync::CancellationToken`）。`EventSink` 从 contracts re-export（若 W1 contracts 缺，本 Step 补一个最小 `trait EventSink { fn emit(&self, event: SessionEvent); }`）。
- Done when：`cargo check -p octopus-sdk-tools` 通过；`ToolContext` 可在单元测试中用 `MockPermissionGate / MockAskResolver / MockEventSink` 构造。
- Verify：`cargo check -p octopus-sdk-tools --tests`
- Stop if：`SessionStore` 的真实 trait 需要 `async fn append_event(...)`，但 `EventSink` 想要同步 `emit` → 保持两个独立 trait，`event_sink` 在 W3 只用于同步发送 `SessionEvent::ToolCall { .. }` 等轻量事件；重事件（session.started）仍走 session_store（W6 Brain Loop 负责）。

Step 2：
- Action：落地 `ToolResult { pub content: Vec<ContentBlock>, pub is_error: bool, pub duration_ms: u64, pub render: Option<RenderBlock> }`（`ContentBlock / RenderBlock` 复用 contracts）。落地 `ToolError` 9 个变体（修订清单 #9），派生 `thiserror::Error + Debug + Send + Sync`；`fn as_tool_result(&self) -> ToolResult` 把 error 转成边界形状（`is_error = true`，`content = vec![ContentBlock::Text { text: format!("{self}") }]`，带 `remediation` 附在 text 末尾）。落地 `RegistryError { DuplicateName(String), InvalidSpec(String) }`。
- Done when：`cargo test -p octopus-sdk-tools error::` + `result::` 全绿；`tool_error_becomes_boundary_result` 断言 `is_error = true` 且 text 含 "remediation:"。
- Verify：`cargo test -p octopus-sdk-tools error:: result::`
- Stop if：`ContentBlock` 变体缺 `Text { text }` 或 `RenderBlock` 变体命名与 `docs/sdk/14` 不一致 → Stop #1 / Stop #8。

Step 3：
- Action：落地 `src/constants.rs` 三常量；落地 `src/partition.rs` 的 `ExecBatch<'a>` 与 `partition_tool_calls<'a>(calls: &'a [ToolCallRequest], registry: &ToolRegistry) -> Vec<ExecBatch<'a>>`。算法：遍历 `calls`，对每个 call 查 `registry.get(name)`：
  1. 若 tool 未注册 → 放入单独 `Serial(vec![call])`（调度方自行报错）；
  2. 若 `tool.is_concurrency_safe(&call.input) == true` → 追加到"当前 concurrent 批次"，直到批次数 = `DEFAULT_TOOL_MAX_CONCURRENCY` 或遇到写工具；批次上限后开新 concurrent 批次；
  3. 若 `is_concurrency_safe == false` → 先 flush 当前 concurrent 批次，再追加 `Serial(vec![call])`。
- Done when：`cargo test -p octopus-sdk-tools partition::` 通过 4 组测试：(a) 全只读 8 个 call → 单 concurrent 批次；(b) 11 个只读 → 拆两批 `[10, 1]`；(c) `[read, write, read]` → `[Concurrent([read]), Serial([write]), Concurrent([read])]`；(d) 未注册 tool 归入独立 `Serial`。
- Verify：`cargo test -p octopus-sdk-tools partition::`
- Stop if：W3 `partition_tool_calls` 仅做工具级粒度（与 Scope 的明文承诺一致）。若下游（W6 Brain Loop 或测试）临时要求"同一 `ResourceKey`（文件路径归一到绝对路径）并发写必须走 serial bucket" → **不在 W3 实现**；在 `02 §5` 登记 `W4 增强项 · partition_tool_calls.resource_bucket`，W4 由 `HookRunner / PermissionPolicy` 在调度外层兜底（或由 `partition_tool_calls` 的 `v2` 辅助接口在 W4 追加，不改本周签名）。

Notes：
- `ToolContext` 构造方式统一用 `ToolContextBuilder`，避免字段追加导致全仓 call-site 改写（W4 新增字段时升级成本低）。
- W3 不实现 dispatcher（R2 决策）；`partition_tool_calls` 只返回分片，真正的并发执行由 W6 Brain Loop 基于 `tokio::spawn` 实现。

---

### Task 4：MCP 核心类型（`JsonRpc*` + `McpError` + `McpTool/Prompt/Resource`）

Status: `done`

Files:
- Create: `crates/octopus-sdk-mcp/src/jsonrpc.rs`
- Create: `crates/octopus-sdk-mcp/src/types.rs`（`McpTool / McpPrompt / McpResource / McpToolResult`）
- Create: `crates/octopus-sdk-mcp/src/error.rs`（`McpError`）
- Create: `crates/octopus-sdk-mcp/src/directory.rs`（`trait ToolDirectory`）
- Modify: `crates/octopus-sdk-mcp/src/lib.rs`

Preconditions：Task 1、Task 2 完成（需要 `JsonRpc* / McpTool / McpToolResult / McpError` 基础类型先就位）。

Step 1：
- Action：落地 `JsonRpcRequest { jsonrpc: String("2.0"), id: serde_json::Value, method: String, params: Option<serde_json::Value> }`、`JsonRpcResponse { jsonrpc, id, result: Option<serde_json::Value>, error: Option<JsonRpcError> }`、`JsonRpcNotification { jsonrpc, method, params }`、`JsonRpcError { code: i32, message: String, data: Option<serde_json::Value> }`。提供 `JsonRpcRequest::method / params` 等 getter；序列化遵循 JSON-RPC 2.0。
- Done when：`cargo test -p octopus-sdk-mcp jsonrpc::` 覆盖 round-trip，包含 (a) request with params、(b) response with error、(c) notification。
- Verify：`cargo test -p octopus-sdk-mcp jsonrpc::`
- Stop if：MCP 官方文档（2024-11-05 spec）对 `id` 允许 `null` / string / number → 本实现用 `serde_json::Value` 兼容所有形态；若需更窄类型保证 → 登记 `02 §5`。

Step 2：
- Action：落地 `McpTool { name, description, input_schema }`（与 `ToolSchema` 同构，但保持 MCP 原字段命名避免 wire 不兼容）；`McpPrompt / McpResource` 按 MCP spec 最小字段；`McpToolResult { content: Vec<ContentBlock>, is_error: bool }`（与 `ToolResult` 同构，便于 `SdkTransport` 折返）。提供 `From<ToolSchema> for McpTool` 与 `From<McpTool> for ToolSchema`。
- Done when：`cargo test -p octopus-sdk-mcp types::` round-trip + `ToolSpec ↔ McpTool` 互转测试通过。
- Verify：`cargo test -p octopus-sdk-mcp types::`
- Stop if：MCP `inputSchema`（驼峰）vs 本库 `input_schema`（蛇形） → 在 `#[serde(rename = "inputSchema")]` 保持 wire 驼峰，Rust 侧用 snake_case。

Step 3：
- Action：落地 `McpError` 7 变体（修订清单 #18）；派生 `thiserror::Error + Debug + Send + Sync`。**不**在 `octopus-sdk-mcp` 内提供任何会反向触碰 tools 的错误映射 helper；若工具侧需要把 MCP 错误映射回 `ToolError`，统一由 `octopus-sdk-tools` 侧定义 `impl From<McpError> for ToolError` 或等价 adapter。
- Done when：`cargo test -p octopus-sdk-mcp error::` 通过；`McpError::ServerCrashed` 序列化含 `server_id + exit_code`。
- Verify：`cargo test -p octopus-sdk-mcp error::`
- Stop if：为了在 mcp crate 内复用 tools 错误语义而抽出 `octopus-sdk-contracts::CommonToolError` → 若抽出将触发跨 crate 枚举迁移 → Stop #1；选 `impl From<McpError> for ToolError` 在 tools crate 内定义。

Step 4：
- Action：在 `octopus-sdk-mcp::directory.rs` 落地 `#[async_trait] trait ToolDirectory: Send + Sync { fn list_tools(&self) -> Vec<McpTool>; async fn call_tool(&self, name: &str, input: serde_json::Value) -> Result<McpToolResult, McpError>; }`。该 trait 保持**全 MCP 原生类型**，不触 `octopus-sdk-tools` 的任何符号；`octopus-sdk-mcp` 只消费这个 trait，本身不反向依赖 tools。`octopus-sdk-tools::ToolRegistry` 在 Task 6 Step 3 / Task 8 Step 4 通过 `impl ToolDirectory for ToolRegistry` + `registry.as_directory()` adapter 接入 `SdkTransport::from_directory(...)`。
- Done when：`cargo check -p octopus-sdk-mcp` 通过；`trait ToolDirectory` 对象安全；`octopus-sdk-mcp/src/lib.rs` ≤ 80 行。
- Verify：`cargo check -p octopus-sdk-mcp && wc -l crates/octopus-sdk-mcp/src/lib.rs`
- Stop if：`ToolDirectory::call_tool` 的 `Result<McpToolResult, McpError>` 不够表达 `AskApproval`（需要与 PermissionGate 交互） → 本 Step 只约定"权限检查由 ToolRegistry 在实现 `call_tool` 时完成"，MCP trait 不感知权限；若 E2E 需要 MCP 客户端代入权限 → Stop #1（跨 crate 边界变更，进 W4 决策）。

Notes：
- Task 4 Step 4 的高风险点是依赖方向：本 Plan 只允许 **`tools → mcp` 的单向窄依赖**，不允许 `mcp → tools` 反向回指；`ToolDirectory` 保持 MCP-native 签名就是为此服务。
- `octopus-sdk-mcp` 对 `ContentBlock` 的引用经 `octopus-sdk-contracts::ContentBlock`（W1 已有）。

---

### Task 5：`trait McpTransport` + `McpClient` + `McpServerManager`

Status: `done`

Files:
- Create: `crates/octopus-sdk-mcp/src/transport/mod.rs`（`trait McpTransport` + `enum TransportKind`）
- Create: `crates/octopus-sdk-mcp/src/client.rs`（`McpClient`）
- Create: `crates/octopus-sdk-mcp/src/manager.rs`（`McpServerManager` + `ServerHandle`）
- Create: `crates/octopus-sdk-mcp/src/lifecycle.rs`（`McpLifecyclePhase`，轻量级）
- Modify: `crates/octopus-sdk-mcp/src/lib.rs`

Preconditions：Task 4 完成。

Step 1：
- Action：落地 `#[async_trait] trait McpTransport: Send + Sync { async fn call(&self, req: JsonRpcRequest) -> Result<JsonRpcResponse, McpError>; async fn notify(&self, msg: JsonRpcNotification) -> Result<(), McpError>; }`；`enum TransportKind { Stdio, Http, Sdk }` 供上层识别。
- Done when：`cargo check -p octopus-sdk-mcp` 通过；trait 对象安全（`Arc<dyn McpTransport>` 构造）。
- Verify：`cargo check -p octopus-sdk-mcp`
- Stop if：部分 transport 只单向 notify（例如 HTTP POST 无响应） → `notify` 默认实现调用 `call` 并 drop 响应。

Step 2：
- Action：落地 `McpClient { transport: Arc<dyn McpTransport>, server_id: String, initialized: AtomicBool }`；方法：
  - `async fn initialize(&self) -> Result<InitializeResult, McpError>`（发送 MCP `initialize` 请求，记录 protocol version）；
  - `async fn list_tools(&self) -> Result<Vec<McpTool>, McpError>`；
  - `async fn call_tool(&self, name: &str, input: serde_json::Value) -> Result<McpToolResult, McpError>`；
  - `async fn list_prompts(&self) -> Result<Vec<McpPrompt>, McpError>`、`async fn list_resources(&self) -> Result<Vec<McpResource>, McpError>`（可先返回空列表占位，`notimpl` 时不报错）。
  - 未 `initialize` 前调用 list_tools 自动触发一次；`initialized` flag 避免重复。
- Done when：`cargo test -p octopus-sdk-mcp client::` 用 `MockTransport`（回放 JSON fixture）跑通 `initialize → list_tools → call_tool` 三步链路。
- Verify：`cargo test -p octopus-sdk-mcp client::`
- Stop if：MCP 最新协议要求 `initialize` 携带 `clientInfo.name / version` → 在 `McpClient::initialize()` 读 `env!("CARGO_PKG_VERSION") / CARGO_PKG_NAME`；不外传业务侧信息。

Step 3：
- Action：落地 `McpServerManager { servers: Arc<Mutex<HashMap<String, ServerHandle>>> }`；方法：
  - `async fn spawn(&self, spec: McpServerSpec) -> Result<String, McpError>`：按 `spec.transport`（Stdio / Http / Sdk）建立 transport + client + initialize；注册到 map；返回 `server_id`。
  - `async fn shutdown(&self, server_id: &str) -> Result<(), McpError>`：查 `ServerHandle`，对 stdio 类调 `start_kill + wait`；移除 map。
  - `fn get_client(&self, server_id: &str) -> Option<Arc<McpClient>>`；
  - `fn list_servers(&self) -> Vec<String>`。
  - `Drop for McpServerManager`：同步 kill 所有 stdio 子进程（`tokio::task::block_in_place` + `rt.block_on`；若没在 tokio runtime 中 → 直接 `std::process::Child` fallback + `kill`）。**测试必须断言 drop 后无残留进程。**
- Done when：`cargo test -p octopus-sdk-mcp manager::` 覆盖 (a) spawn stdio → get_client → shutdown 顺利；(b) drop 管理器后子进程回收；(c) 重复 shutdown 幂等。
- Verify：`cargo test -p octopus-sdk-mcp manager::`
- Stop if：`Drop` 内同步 `block_on` 在 tokio `current_thread` runtime 下会 panic → 用 `tokio::task::spawn_blocking` 或依赖 `atomic-waker` 的 "best-effort drop" 路径；若 W3 无法完美覆盖所有 runtime 组合 → Linux/macOS 先绿，Windows 与 multi-thread panic 场景登记到 `02 §5 + 00 §6`。

Notes：
- `ServerHandle` 内部结构：`{ client: Arc<McpClient>, kind: TransportKind, stdio_child: Option<Mutex<tokio::process::Child>> }`；HTTP/Sdk transport 无 `stdio_child`。
- 对 `McpLifecyclePhase`：W3 仅保留 `{ Starting, Ready, Degraded, Stopped }` 4 态；`docs/plans/sdk/03-legacy-retirement.md §6.1 mcp_lifecycle_hardened.rs` 的 "Phase 细分"（7 phase）延后迁到本 crate（W4/W5 plugin 侧处理）。

---

### Task 6：三 transport 实现 + 集成测试（含 process drop）

Status: `done`

Files:
- Create: `crates/octopus-sdk-mcp/src/transport/stdio.rs`
- Create: `crates/octopus-sdk-mcp/src/transport/http.rs`
- Create: `crates/octopus-sdk-mcp/src/transport/sdk.rs`
- Create: `crates/octopus-sdk-mcp/tests/mcp_stdio_transport.rs`
- Create: `crates/octopus-sdk-mcp/tests/mcp_http_transport.rs`
- Create: `crates/octopus-sdk-mcp/tests/mcp_sdk_transport.rs`
- Create: `crates/octopus-sdk-mcp/tests/mcp_process_lifecycle.rs`（R3 守护）
- Create: `crates/octopus-sdk-mcp/tests/fixtures/mcp_echo_server.py` 或 `.rs` binary（参考 server；优先 Rust bin 目标 `cargo build --bin mcp-echo-server` 避免 Python 依赖）
- Create: `crates/octopus-sdk-mcp/tests/fixtures/*.json`（请求/响应固定载荷）

Preconditions：Task 5 完成。

**PR 边界约定**（对应 R7 / S1 先例）：Task 6 按 "1 Step = 1 PR" 合入；串行：Step 1 → Step 2 → Step 3 → Step 4；每个 PR ≤ 300 行（不含 fixture）。

Step 1：
- Action：落地 `StdioTransport`。内部 `tokio::process::Command::new(cmd).args(args).envs(env).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?`；用 `LinesCodec` 按行读取 stdout（MCP stdio 每帧一行 JSON）；建立 `request_id → oneshot::Sender<JsonRpcResponse>` map；内置 `StdioProcessGuard`，`Drop` 同步 `start_kill + wait`（R3）。`call` 语义：分配 id → 写入 stdin → 挂起 oneshot → 超时 30s。
- Done when：`tests/mcp_stdio_transport.rs` 启动 fixture `mcp-echo-server` 跑 `initialize + list_tools + call_tool` 链路，全部返回预期 result。
- Verify：`cargo test -p octopus-sdk-mcp --test mcp_stdio_transport`
- Stop if：fixture echo server 实现难度超 80 行 → 用 `tokio` + `serde_json` 写 ~50 行回显逻辑（读 stdin JSON 判断 method 返回固定响应）即可；若仍超 → 引入 `rmcp` 依赖（Anthropic 的 Rust MCP SDK） → Stop #8（引入新依赖需审）。

Step 2：
- Action：落地 `HttpTransport`。内部持 `reqwest::Client` + `base_url` + `default_headers`。`call` 语义：POST `base_url/mcp`（MCP HTTP spec），body = `JsonRpcRequest`，反序列化 `JsonRpcResponse`；超时 30s。`notify` 同 POST 但 body 为 notification + 不等响应。支持 `Authorization: Bearer <token>` 头部（token 从 `SecretVault` 注入，不走 env）。
- Done when：`tests/mcp_http_transport.rs` 用 `wiremock` 模拟 MCP 服务器返回固定 `tools/list` + `tools/call` JSON，断言 parsing 正确；`header("Authorization")` 断言存在。
- Verify：`cargo test -p octopus-sdk-mcp --test mcp_http_transport`
- Stop if：MCP HTTP 最新 spec 使用 SSE 双向流（`streamable-http`） → W3 先支持"标准 request/response" 单轮模式；SSE 流式（long-running）登记 `02 §5` 作 W4/W5 增量。

Step 3：
- Action：落地 `SdkTransport { directory: Arc<dyn ToolDirectory> }`；`call` 语义：解析 `JsonRpcRequest.method`（`tools/list / tools/call / initialize`）→ 路由到 `ToolDirectory` 方法 → 封回 `JsonRpcResponse`。`notify` 无 side-effect（或记日志）。构造器 `SdkTransport::from_directory(dir: Arc<dyn ToolDirectory>) -> Self`；同时追加 `octopus-sdk-tools::ToolRegistry::as_directory(&self) -> Arc<dyn ToolDirectory>` helper（在 `octopus-sdk-tools` crate 里，因为 mcp 是 trait 定义方不能反向；此 helper 会随 Task 7/Task 8 落在 tools crate）。
- Done when：`tests/mcp_sdk_transport.rs` 用最小 `DummyDirectory`（手写 1 个 tool）构造 `SdkTransport + McpClient`，跑 `list_tools → call_tool` 返回预期结果。
- Verify：`cargo test -p octopus-sdk-mcp --test mcp_sdk_transport`
- Stop if：`ToolDirectory::call_tool` 期望 `McpToolResult`，而 `ToolRegistry` 返回 `ToolResult` → 在 tools 侧 `as_directory()` adapter 做 `ToolResult → McpToolResult` 的 `From` 转换。

Step 4（R3 硬门禁）：
- Action：落地 `tests/mcp_process_lifecycle.rs`：`#[tokio::test] async fn stdio_process_cleanup_on_drop()` —— 启动 50 次 `McpServerManager::spawn(stdio)`，每次 shutdown；在循环后检查 `/proc/<parent_pid>/task/` 或 `ps --ppid $$` 无 `mcp-echo-server` 残留。macOS 用 `pgrep -P $$ mcp-echo-server` 等价命令。Linux / macOS 都覆盖；Windows 用 `#[cfg(not(windows))]` skip，`TODO(W8)` 登记。
- Done when：测试绿；跑 `cargo test -p octopus-sdk-mcp --test mcp_process_lifecycle` 无残留；跑完后 `ps -eo pid,comm | rg mcp-echo-server | wc -l` 为 0。
- Verify：`cargo test -p octopus-sdk-mcp --test mcp_process_lifecycle && (ps -eo pid,comm | rg -c mcp-echo-server || true)`（第二段应为 0）
- Stop if：`Drop` 内同步 kill 与 tokio `current_thread` runtime 冲突 → 引入 `shutdown(&self)` 显式接口作为主路径，`Drop` 作为"best-effort 兜底 + warn 日志"；放弃"强制同步 kill"的 Drop 保证，仅在 `shutdown` 完成后才声明 R3 已覆盖。

Notes：
- fixture `mcp-echo-server` 作为 `[[bin]]` 定义在 `crates/octopus-sdk-mcp/Cargo.toml`，测试时通过 `env!("CARGO_BIN_EXE_mcp-echo-server")` 拿路径，避免路径硬编码。
- Task 6 总代码量预估 600–800 行分布在 4 个 PR（含 fixture）；任一 PR > 300 行必须再拆。

---

### Task 7：15 个内置工具（按 4 个子 PR 合入）

Status: `done`

Files：
- **7a：FS 读族**（3 工具 · Read / Glob / Grep）
  - Create: `crates/octopus-sdk-tools/src/builtin/fs_read.rs`
  - Create: `crates/octopus-sdk-tools/src/builtin/fs_glob.rs`
  - Create: `crates/octopus-sdk-tools/src/builtin/fs_grep.rs`
  - Create: `crates/octopus-sdk-tools/tests/builtin_fs_read.rs`
- **7b：FS 写族 + Shell**（3 工具 · Write / Edit / Bash）
  - Create: `crates/octopus-sdk-tools/src/builtin/fs_write.rs`
  - Create: `crates/octopus-sdk-tools/src/builtin/fs_edit.rs`
  - Create: `crates/octopus-sdk-tools/src/builtin/shell_bash.rs`
  - Create: `crates/octopus-sdk-tools/tests/builtin_fs_write.rs`
  - Create: `crates/octopus-sdk-tools/tests/builtin_bash.rs`
- **7c：Web + 交互 + 调度**（4 工具 · WebSearch / WebFetch / AskUserQuestion / TodoWrite / Sleep）
  - Create: `crates/octopus-sdk-tools/src/builtin/web_search.rs`
  - Create: `crates/octopus-sdk-tools/src/builtin/web_fetch.rs`
  - Create: `crates/octopus-sdk-tools/src/builtin/ask_user_question.rs`
  - Create: `crates/octopus-sdk-tools/src/builtin/todo_write.rs`
  - Create: `crates/octopus-sdk-tools/src/builtin/sleep.rs`
  - Create: `crates/octopus-sdk-tools/tests/builtin_web.rs`
  - Create: `crates/octopus-sdk-tools/tests/builtin_ask.rs`
- **7d：W5 stub 族**（4 工具 · Agent / Skill / TaskList / TaskGet）
  - Create: `crates/octopus-sdk-tools/src/builtin/w5_stubs.rs`
  - Create: `crates/octopus-sdk-tools/tests/builtin_stubs.rs`

Preconditions：Task 2 / Task 3 完成；Task 6 Step 3 完成（`SdkTransport` 存在，但本 Task 不依赖它，直接注册进 `ToolRegistry`）。

**PR 边界约定**：Task 7 按 4 个 sub-PR 串行合入；7a → 7b → 7c → 7d；每 PR ≤ 300 行（不含 fixture 与 tests）。

Step 7a（FS 读族）：
- Action：落地 `FileReadTool`（区间读 + 行前缀 `NNNNNN|`，遵循本 Plan 系统信息 §inline_line_numbers）、`GlobTool`（`globset` 匹配）、`GrepTool`（`ignore::WalkBuilder` + `regex`，参数遵循 `docs/sdk/03 §3.4` 条目与 Claude Code 源里 GrepTool 签名）。`is_concurrency_safe = true` 所有 3 个。默认截断：Read 输出 ≤ 2 000 行或 ≤ 500 KB（`docs/sdk/03 §3.3.2` 的 "List-style 工具 50 items" 不适用，文件截断用字节+行双约束）；Grep ≤ 100 matches；Glob ≤ 500 paths。
- Done when：单元测试覆盖 (a) 小文件读全文、(b) offset+limit 区间读、(c) glob 匹配 workspace tmp、(d) grep 正则大小写、(e) 超限截断提示文本。
- Verify：`cargo test -p octopus-sdk-tools --test builtin_fs_read`
- Stop if：`globset` 版本与 workspace 不兼容 → 尝试 `glob` crate 兜底；若无可用方案 → Stop #8。

Step 7b（FS 写族 + Shell）：
- Action：
  - `FileWriteTool`：`is_concurrency_safe = false`；写前 `permissions.check(ToolCallRequest { name: "fs_write", .. })`；原子写（`tempfile::NamedTempFile + persist`）。
  - `FileEditTool`：`is_concurrency_safe = false`；字符串替换（支持 `replace_all: bool`），失败归 `ToolError::Validation`。
  - `BashTool`：`is_concurrency_safe = false`；`tokio::process::Command::new("bash").arg("-c").arg(input.command).current_dir(&ctx.sandbox.cwd).envs(filtered_env).spawn()`；**最大输出** `std::env::var("BASH_MAX_OUTPUT_LENGTH").unwrap_or(BASH_MAX_OUTPUT_DEFAULT).min(BASH_MAX_OUTPUT_UPPER_LIMIT)`；超过则截断 + 附加"已截断，用 `grep`/`head`"提示（`docs/sdk/03 §3.3.2`）。超时默认 120s。
- Done when：`tests/builtin_fs_write.rs` 原子写 + 路径注入（`ctx.permissions` 拒绝 `/etc/passwd`）；`tests/builtin_bash.rs` 覆盖 (a) 成功 echo、(b) 非零退出 `is_error = true`、(c) 输出超 30 000 字符被截断、(d) `BASH_MAX_OUTPUT_LENGTH=100` 环境变量生效。
- Verify：`cargo test -p octopus-sdk-tools --test builtin_fs_write --test builtin_bash`
- Stop if：Windows CI 无 `bash` → `BashTool` 在 `#[cfg(not(windows))]` 路径全功能，Windows 留 `TODO(W8) PowerShellTool`；测试用 `#[cfg(unix)]` 守护。

Step 7c（Web + 交互 + 调度）：
- Action：
  - `WebFetchTool`：`is_concurrency_safe = true`；`reqwest::get(url).timeout(30s)`；html → markdown（`html2md` 或最小正则剥 script/style + 30 000 字符截断）。
  - `WebSearchTool`（**W6-stub**，与 Scope 的 "10 full + 5 stub" 分类一致）：`is_concurrency_safe = true`；`execute()` 直接返回 `ToolError::NotYetImplemented { crate_name: "octopus-sdk-tools::web_search", week: "W6" }`；`ToolSpec` 填入完整 `input_schema`（`query: string, count?: u32`）供 prompt cache 稳定性参与 fingerprint；`ToolSpec.description` 前 80 字符以 `"[STUB · W6]"` 开头。**不** 在 W3 内引入 `SearchProvider` trait（该 trait 归 `octopus-sdk-plugin` 在 W5 定义，`WebSearchTool` 在 W6 Brain Loop 装配时注入 provider）。
  - `AskUserQuestionTool`：`is_concurrency_safe = false`；构造 `AskPrompt`（`docs/sdk/14`） + 调用 `ctx.ask_resolver.resolve(prompt_id, &prompt)` → 返回 `AskAnswer` 填入 `ToolResult`；超时 300s；同时发出 `SessionEvent::Ask`。
  - `TodoWriteTool`：`is_concurrency_safe = false`；W3 contracts 里还没有 `SessionEvent::TodoUpdated`，因此改用现有 `SessionEvent::Render { block: RenderBlock { kind: Record, ... }, lifecycle: OnToolResult }` 承载结构化 todo 更新。
  - `SleepTool`：`is_concurrency_safe = true`；`tokio::time::sleep(Duration::from_millis(input.ms))`；上限 60 000ms。
- Done when：`tests/builtin_web.rs` 用 `wiremock` 覆盖 fetch（含截断断言）；`WebSearchTool` 单测断言 `execute()` 返回 `ToolError::NotYetImplemented { week: "W6" }` + `ToolSpec.description.starts_with("[STUB · W6]")`；`tests/builtin_ask.rs` 用 `MockAskResolver` 返回预设 `AskAnswer`，断言 `ToolResult.content` 含答案文本。
- Verify：`cargo test -p octopus-sdk-tools --test builtin_web --test builtin_ask`
- Stop if：`html2md` 输出不稳定 → 用最小剥离（`regex` 去 tag）；`SearchProvider` trait 若被误抽到 tools crate → 立刻回退（归属 `octopus-sdk-plugin` W5）。

Step 7d（W5 stub 族）：
- Action：落地 4 个 stub：`AgentTool / SkillTool / TaskListTool / TaskGetTool`。所有 `execute` 返回 `ToolError::NotYetImplemented { crate_name: "octopus-sdk-subagent" (或 "octopus-sdk-tools::task_registry"), week: "W5" }`；`is_concurrency_safe = false`；`ToolSpec.description` 前 80 字符以 `"[STUB · W5]"` 开头。
- Done when：`tests/builtin_stubs.rs` 覆盖 4 个 stub 均返回 `NotYetImplemented` + `ToolResult.is_error = true`；`ToolSpec.description.starts_with("[STUB · W5]")` 断言通过。
- Verify：`cargo test -p octopus-sdk-tools --test builtin_stubs`
- Stop if：`AgentTool / TaskListTool` 等某工具在 W3 出口前被 Brain Loop 或其他 crate 需要"真能用" → Stop #8（跨周契约）。

Notes：
- 每个 builtin 文件 ≤ 150 行；若 `BashTool` 超 150 → 抽 `shell_bash/output_limit.rs` 独立子模块。
- 所有 builtin 工具通过 `ToolRegistry::register_builtins()` 辅助函数一次性注册（在 `registry.rs` 或 `builtin/mod.rs` 提供），固定注册顺序 + 确定性排序（依赖 Task 2 Step 3）。
- 构成：7a (FileRead/Glob/Grep) + 7b (FileWrite/FileEdit/Bash) + 7c (WebFetch full · WebSearch W6-stub · AskUserQuestion · TodoWrite · Sleep) + 7d (Agent/Skill/TaskList/TaskGet 四 W5-stub) = **3 + 3 + 5 + 4 = 15**；满足 "15 个内置工具以 `trait Tool` 形式存在" 的 W3 exit state（10 full / 5 stub 分布见 Scope）。

---

### Task 8：Prompt Cache 稳定性 + partition_tool_calls 契约 + Bash 截断硬门禁

Status: `done`

Files:
- Create: `crates/octopus-sdk-tools/tests/registry_stability.rs`
- Create: `crates/octopus-sdk-tools/tests/partition_concurrency.rs`
- Create: `crates/octopus-sdk-tools/tests/bash_output_limit.rs`
- Create: `crates/octopus-sdk-tools/tests/mcp_sdk_shim_roundtrip.rs`

Preconditions：Task 3 / Task 6 / Task 7 完成。

Step 1：
- Action：落地 `registry_stability.rs`：
  - `registry_schemas_sorted_is_byte_stable`：构造 15 工具注册（`register_builtins`），调用 `schemas_sorted()` 3 次 → `serde_json::to_string(&specs)` 字节一致；
  - `registry_tools_fingerprint_is_deterministic`：`tools_fingerprint()` 3 次调用一致；
  - `registry_fingerprint_diff_after_new_tool`：再注册一个自定义 tool → 指纹变化；
  - `registry_fingerprint_stable_across_order`：两个 Registry 实例用不同注册顺序，相同工具集 → 指纹相等。
- Done when：4 个测试全绿；失败 panic 信息指向不稳定字段（用 `pretty_assertions::assert_eq`）。
- Verify：`cargo test -p octopus-sdk-tools --test registry_stability`
- Stop if：`input_schema` 内 object key 顺序依赖 `serde_json::Map` 的 `preserve_order` feature → workspace 默认开 `preserve_order` 时需额外 canonical_json 处理；本 Task 显式用 `BTreeMap<String, Value>` 或自写 canonical_json。

Step 2：
- Action：落地 `partition_concurrency.rs`：
  - `partition_respects_max_concurrency`：11 个只读 call → `[Concurrent(10), Concurrent(1)]`；
  - `partition_serializes_writes`：`[read, write, write, read]` → `[Concurrent([read]), Serial([write]), Serial([write]), Concurrent([read])]`；
  - `partition_unknown_tool_serial`：未注册 tool 直接 `Serial(vec![call])`；
  - `partition_respects_input_concurrency_hint`：对同一 `FileWriteTool` 用"不同 input"时 `is_concurrency_safe(input)` 仍返回 false（写工具始终 false）。
- Done when：4 测试全绿。
- Verify：`cargo test -p octopus-sdk-tools --test partition_concurrency`
- Stop if：规范后续要求"同文件并发写入走 serial bucket"（细粒度）→ 本 Task 不实现，登记到 `02 §5` `W4 增强项 · partition_tool_calls.resource_bucket`；本 Task 的测试只覆盖 W3 承诺的工具级粒度。

Step 3（Bash 截断硬门禁）：
- Action：落地 `bash_output_limit.rs`：
  - `bash_output_default_is_30_000_chars`：执行 `yes | head -c 60000` → ToolResult text 长度 = 30 000 + 截断提示；
  - `bash_output_upper_limit_via_env`：`std::env::set_var("BASH_MAX_OUTPUT_LENGTH", "150000")` → 50 000 字符输出完整保留；
  - `bash_output_upper_limit_cap`：设置 `BASH_MAX_OUTPUT_LENGTH=200000` → 实际 cap 回 `BASH_MAX_OUTPUT_UPPER_LIMIT = 150_000`。
  - 静态断言：`const _: () = assert!(BASH_MAX_OUTPUT_DEFAULT == 30_000); const _: () = assert!(BASH_MAX_OUTPUT_UPPER_LIMIT == 150_000);`
- Done when：3 测试 + 静态断言通过。
- Verify：`cargo test -p octopus-sdk-tools --test bash_output_limit`
- Stop if：Windows / CI 无 `yes`/`head` → 用 Rust stdout 打印等效循环实现；测试限 `#[cfg(unix)]`。

Step 4（R5 MCP in-process shim 闭环）：
- Action：落地 `mcp_sdk_shim_roundtrip.rs`：构造 `ToolRegistry`（仅注册 `SleepTool`），调 `registry.as_directory()` → 包 `SdkTransport::from_directory(...)` → 建 `McpClient` → 调 `initialize → list_tools → call_tool("sleep", { ms: 10 })` → 返回 `McpToolResult { is_error: false, .. }`；断言 round-trip 端到端成功。
- Done when：测试绿；断言验证 `McpTool.name == "sleep"` 且 `call_tool` 耗时 ≥ 10ms。
- Verify：`cargo test -p octopus-sdk-tools --test mcp_sdk_shim_roundtrip`
- Stop if：`ToolRegistry::as_directory()` 辅助尚未落（Task 6 Step 3 已指定其归属 tools crate） → 回到 Task 6 Step 3 补 adapter；若 adapter 的 `From<ToolResult> for McpToolResult` 有损 → 取 `ToolResult.content + is_error` 两字段直映射即可。

Notes：
- Task 8 总约 300 行测试代码；若超 → 按 4 个独立 `tests/*.rs` 文件拆分（当前已按此）。
- 本 Task 是 **W3 硬门禁**的直接承载；Weekly Gate 勾选 `Prompt cache 稳定性 + Bash 截断 + MCP in-process shim` 三项。

---

### Task 9：Capability 退役（4 个 sub-Step 串行合入）

Status: `done`

Files（总表；sub-Step 分割见下）：
- Delete: `crates/tools/src/capability_runtime/mod.rs`
- Delete: `crates/tools/src/capability_runtime/events.rs`
- Delete: `crates/tools/src/capability_runtime/executor.rs`
- Delete: `crates/tools/src/capability_runtime/exposure.rs`
- Delete: `crates/tools/src/capability_runtime/planner.rs`
- Delete: `crates/tools/src/capability_runtime/provider.rs`
- Delete: `crates/tools/src/capability_runtime/state.rs`
- Delete: `crates/octopus-runtime-adapter/src/capability_executor_bridge.rs`
- Delete: `crates/octopus-runtime-adapter/src/capability_planner_bridge.rs`
- Delete: `crates/octopus-runtime-adapter/src/capability_state.rs`
- Delete: `crates/octopus-runtime-adapter/src/capability_runtime_tests.rs`
- Modify: `crates/tools/src/lib.rs`、`crates/tools/src/tool_registry.rs`、`crates/tools/src/builtin_catalog.rs`、`crates/tools/src/skill_runtime.rs`、`crates/tools/src/subagent_runtime.rs`、`crates/tools/src/split_module_tests.rs`（去除 capability 相关 `use` 与函数体）
- Modify: `crates/octopus-runtime-adapter/src/{lib.rs, run_context.rs, agent_runtime_core.rs, persistence.rs, session_service.rs, execution_events.rs, subrun_orchestrator.rs, team_runtime.rs, memory_selector.rs, adapter_state.rs, runtime_persistence_tests.rs, approval_runtime_tests.rs}`（含仅持 `capability_state_ref / load_capability_store / persist_capability_store` 的活调用点：`execution_events.rs`、`persistence.rs`、`approval_runtime_tests.rs`、`adapter_state.rs`、`memory_selector.rs`、`subrun_orchestrator.rs`、`team_runtime.rs`、`runtime_persistence_tests.rs`）
- Modify: `crates/octopus-server/src/workspace_runtime.rs`、`crates/octopus-platform/src/runtime.rs`、`crates/octopus-core/src/lib.rs`、`crates/octopus-infra/src/infra_state.rs`、`crates/rusty-claude-cli/src/main.rs`、`crates/runtime/src/session/session_tests.rs`（最终清单以 Task 9a 超集扫描输出为准）
- Modify: `docs/plans/sdk/03-legacy-retirement.md §3.1 + §6.1`（对应行 `状态：pending → done`；限 "capability_runtime/** / capability_*_bridge.rs / capability_state.rs" 3 行）

Preconditions：Task 2 / Task 3 / Task 4 / Task 5 / Task 6 完成（SDK 侧替代面存在）；当前工作树干净；`cargo build --workspace` 基线绿。

Step 9a（审计与分类）：
- Action：用 **Capability Scan Superset**（见 R7）`<SUPER>` 正则执行 `rg -l --type rust -e '<SUPER>' crates/` 得到全量文件清单（预估 35+ 个）；在 `docs/plans/sdk/06-week-3-tools-mcp.md` §附录 A 填入 "capability call-site 分类表"，逐行分类为以下 3 类：
  1. **dead**：删 `use` + 删对应代码段（无下游调用）；
  2. **redirect-to-sdk**：W3 改写为 `octopus_sdk_tools::ToolRegistry / octopus_sdk_mcp::McpClient` 的等价调用；
  3. **panic-stub**：改为 `todo!("TODO(W7-RETIRE): capability → octopus-sdk replacement")`（仅对 "W7 整个 crate 会删、现在暂不真切" 的业务路径，例如 `rusty-claude-cli::main`、`crates/runtime/src/session/session_tests.rs`）。
- 对比基线：同时运行 **3-token 对外口径** `rg -l --type rust -e 'capability_runtime|CapabilityPlanner|CapabilitySurface' crates/` 与 **附加 token 补集** `rg -l --type rust -e 'capability_state_ref|load_capability_store|persist_capability_store' crates/`；**前者 ⊆ <SUPER>**、**后者 ⊆ <SUPER>**，且两者并集 = `<SUPER>` 全集（用于单测抛异常防止日后再漏扫）。
- Done when：分类表覆盖 **<SUPER> 全量命中**；每行标明 `(文件, 命中 token 列表, 替代类别, 预估行数, 所属 sub-Step 9b/9c)`；附录 A 落地；分类表总行数 = `rg -c --type rust -e '<SUPER>' crates/ | awk -F: '$2>0' | wc -l`。
- Verify：`rg --type rust --files-with-matches -e '<SUPER>' crates/ | sort > /tmp/w3-cap-hits.txt && diff /tmp/w3-cap-hits.txt <(awk -F'|' 'NR>2 {gsub(/ /,"",$2); print $2}' docs/plans/sdk/06-week-3-tools-mcp.md | sort -u)` 无 diff。
- Stop if：`<SUPER>` 命中 > 3-token 命中 + "补集 token" 命中（说明仍有漏扫 token）→ Stop #1（审计规则本身不自洽），补齐 `<SUPER>` 再继续。分类后 "redirect-to-sdk" 条目 > 10 项 → Stop #8，评估是否把部分 `redirect-to-sdk` 降级为 `panic-stub(TODO W6-WIRE)` 推迟到 W6。

Step 9b（adapter 内部 call-site 清理）：
- Action：按附录 A 的分类，**仅处理** `crates/octopus-runtime-adapter/` 下的文件（adapter 主力 ~16 文件：`adapter_state.rs / agent_runtime_core.rs / approval_runtime_tests.rs / execution_events.rs / memory_selector.rs / persistence.rs / run_context.rs / runtime_persistence_tests.rs / session_service.rs / subrun_orchestrator.rs / team_runtime.rs` + 4 个待删文件）。`redirect-to-sdk` 条目：在 `agent_runtime_core.rs / run_context.rs / session_service.rs / persistence.rs` 的 capability 调用点改写为 `octopus_sdk_tools::ToolRegistry::get + Tool::execute`；`dead` 条目：删 `use` + 相关函数体；`panic-stub` 条目：替换为 `todo!("TODO(W7-RETIRE)")`。保持其他行不动；`cargo build -p octopus-runtime-adapter` 绿。
- Done when：对 adapter 目录跑 `<SUPER>` 扫描，只剩 `capability_executor_bridge.rs / capability_planner_bridge.rs / capability_state.rs / capability_runtime_tests.rs` 4 个待删文件自身（仍未删）；`cargo build -p octopus-runtime-adapter` 绿；本 sub-PR ≤ 800 行。
- Verify：`cargo build -p octopus-runtime-adapter && rg --type rust --files-with-matches -e '<SUPER>' crates/octopus-runtime-adapter/ | rg -v '(capability_executor_bridge|capability_planner_bridge|capability_state\.rs|capability_runtime_tests)' | wc -l` 为 0。
- Stop if：某函数签名因去 capability 而改变 → 若函数是公共面（`pub fn`） → 先保留旧签名 + 内部 `todo!()`；若仍无法避免签名变更且调用方在 `crates/octopus-server` / `crates/octopus-platform` → 合到 Step 9c 一起改。

Step 9c（外圈 legacy crate 清理）：
- Action：按附录 A 的分类，处理 adapter 之外的命中文件：`crates/tools/` 自身（去掉对 `capability_runtime` 的 `use`，它的实现就是要被删；这是"自己清理自己"）、`crates/octopus-server/src/workspace_runtime.rs`、`crates/octopus-platform/src/runtime.rs`、`crates/octopus-core/src/lib.rs`、`crates/octopus-infra/src/infra_state.rs`、`crates/rusty-claude-cli/src/main.rs`、`crates/runtime/src/session/session_tests.rs`。多数为 `dead` / `panic-stub` 类型；仅 `crates/tools/src/lib.rs` 的 `pub mod capability_runtime` 行是 `dead`（删该行）。
- Done when：用 `<SUPER>` 扫描 `crates/`，除 `crates/tools/src/capability_runtime/` 与 adapter 的 4 个待删文件外，全部 0；`cargo build --workspace` 绿；本 sub-PR ≤ 800 行。
- Verify：`cargo build --workspace && rg --type rust --files-with-matches -e '<SUPER>' crates/ | rg -v '(crates/tools/src/capability_runtime/|crates/octopus-runtime-adapter/src/capability_executor_bridge|crates/octopus-runtime-adapter/src/capability_planner_bridge|crates/octopus-runtime-adapter/src/capability_state\.rs|crates/octopus-runtime-adapter/src/capability_runtime_tests)' | wc -l` 为 0。
- Stop if：`crates/octopus-server::workspace_runtime.rs`（9 890 行，跨多 W 的拆分目标）含深度 capability 调用链，仅改此单文件即超 800 行 → 拆两个 sub-PR（9c-1 / 9c-2）。

Step 9d（文件删除 + 守护扫描）：
- Action：`git rm -r crates/tools/src/capability_runtime/`；`git rm crates/octopus-runtime-adapter/src/capability_{executor_bridge,planner_bridge,state,runtime_tests}.rs`；更新 `crates/octopus-runtime-adapter/src/lib.rs` 与 `crates/tools/src/lib.rs` 删除对应 `mod` 声明；在 `03-legacy-retirement.md §3.1 + §6.1` 对应 3 行把 `pending → done`。
- Done when：
  - `cargo build --workspace` 绿；
  - `cargo clippy --workspace -- -D warnings` 绿；
  - **3-token 对外硬门禁**（`00-overview.md §3 W3`）：`rg --type rust -e 'capability_runtime|CapabilityPlanner|CapabilitySurface' crates/` 0 命中；
  - **Capability Scan Superset 守护扫描**（W3 内部保险）：`rg --type rust -e '<SUPER>' crates/` 0 命中；
  - `ls crates/tools/src/capability_runtime/ 2>/dev/null` 空；`ls crates/octopus-runtime-adapter/src/capability_*.rs 2>/dev/null` 空；
  - `03-legacy-retirement.md §3.1` 的 `capability_runtime/**` 行状态 = `done`；`§6.1` 的 `Capability Bridge` 行 = `done`。
- Verify：`cargo build --workspace && cargo clippy --workspace -- -D warnings && rg --type rust -e 'capability_runtime|CapabilityPlanner|CapabilitySurface' crates/ ; rg --type rust -e '<SUPER>' crates/`（后两个 rg 均期望 exit code 1 = 0 hits）。
- Stop if：删除后仍有 warning（例如 `unused_imports` 残留） → 同 PR 一并清理；不能延到下周。`<SUPER>` 仍有残留但 3-token 为 0 → Stop #1（扫描口径自洽断裂）。

Notes：
- Task 9 总净删预估 **4 500 – 5 500 行**（capability_runtime/** ≈ 3 092 + bridges ≈ 1 800 + tests ≈ 850 − 少量 redirect/stub 新增），拆 4 sub-PR 后各自 ≤ 800 行改动（删除为主，净添加少）。
- Task 9 是 W3 的最大单项风险（R7）；若 9a 审计后判定不可 4-PR 完成，追加 9e 并在 README.md §文档索引把 W3 行状态保留 `in_progress` 直到 9e 落地。
- `03 §8 守护扫描` 的命令在 Step 9d 必须全绿，作为 Weekly Gate 的前置。

---

### Task 10：`02 §2.1 / §2.4 / §2.5` 公共面冻结 + Weekly Gate 收尾

Status: `done`

Files:
- Modify: `docs/plans/sdk/02-crate-topology.md §2.1 / §2.4 / §2.5`（若实际 `pub` 符号与登记仍有 diff；Task 1/2/3/4/5/6/7 已做同批次回填，此处仅"补漏"）
- Modify: `docs/plans/sdk/02-crate-topology.md §5 契约差异清单`（登记本周实际发现的差异）
- Modify: `docs/plans/sdk/README.md`（W3 行状态 → `done`；追加 `2026-04-2X` 最后更新日期）
- Modify: `docs/plans/sdk/00-overview.md §10`（追加 W3 Weekly Gate 结果到总控变更日志）
- Modify: `docs/sdk/README.md ## Fact-Fix 勘误`（**必改**：至少追加 `task_output` 条目；若本周另有规范矛盾同批追加）
- Modify: `docs/plans/sdk/06-week-3-tools-mcp.md`（本文件；Task 全 `done`、追加 Checkpoint、变更日志）

Preconditions：Task 1–Task 9 全部 `done`；`cargo test -p octopus-sdk-contracts -p octopus-sdk-tools -p octopus-sdk-mcp` 全绿；`cargo clippy --workspace -- -D warnings` 全绿；`cargo build --workspace` 全绿。

Step 1：
- Action：核对 `crates/octopus-sdk-tools + crates/octopus-sdk-mcp + crates/octopus-sdk-contracts`（含 W3 新增的 `PermissionGate / AskResolver / ToolCallRequest / PermissionMode / PermissionOutcome`）的所有 `pub (struct|enum|trait|fn|type|const) ` 符号 ⊆ `02 §2.1 / §2.4 / §2.5` 登记集合；若出现未登记的 `pub` → 要么回到本 Plan 内收回 `pub`，要么补登 §2.1 / §2.4 / §2.5；**二选一**必须在本 Task 内闭环。
- Done when：`rg 'pub (struct|enum|trait|fn|type|const) ' crates/octopus-sdk-{tools,mcp}/src crates/octopus-sdk-contracts/src/{permission.rs,ask_resolver.rs}` 输出逐项能在 `02 §2.1` 或 `§2.4` 或 `§2.5` 中找到对应行；`crates/octopus-sdk-tools/src/lib.rs` ≤ 80 行；`crates/octopus-sdk-mcp/src/lib.rs` ≤ 80 行。
- Verify：作者自核对（附在 Checkpoint 的 `Completed` 列表中）。
- Stop if：实际代码有 `02` 未登记的 `pub` 且有生产使用方 → 必须补登，不可回收。

Step 2：
- Action：
  1. 登记与 `contracts/openapi/src/**` 的契约差异到 `02 §5`（预期差异：`ToolSpec` vs OpenAPI `RuntimeToolDefinition` 的 `category` 枚举；`McpTool` vs OpenAPI 当前是否存在；`PermissionOutcome` 变体与 OpenAPI `RuntimePermissionDecision` 比对）；
  2. 追加 **W4 增强项 · `partition_tool_calls.resource_bucket`** 到 `02 §5`（状态 `open`，责任窗口 `W4`，说明 "W3 仅工具级粒度，资源级串行延 W4 由 `HookRunner / PermissionPolicy` 兜底"）；
  3. 追加 **Fact-Fix 勘误** 到 `docs/sdk/README.md ## Fact-Fix 勘误`：条目 `[Fact-Fix · W3] 03-tool-system.md §3.4 line 290 · task_output` ——SDK 首版 15 工具不包含独立 `TaskOutputTool`；W3 当前 contracts 也未定义等价 retrieval 契约，本周只记录 defer；若后续要折叠进 `task_get` 或新增 session 事件，必须先登记到 `02 §2.1 / §2.4`，再由 W5 `octopus-sdk-subagent` 决策；
  4. 若本周发现 `docs/sdk/03` 与实际实现存在其他矛盾 → 同文件 `## Fact-Fix 勘误` 追加条目。
- Done when：`02 §5` 至少新增 **4 行** 非占位数据（含 `partition_tool_calls.resource_bucket`）；每行 `状态 = open`；`docs/sdk/README.md ## Fact-Fix 勘误` 至少新增 1 条（`task_output` 条目）。
- Verify：`rg -c 'align-openapi|align-sdk|dual-carry|no-op|resource_bucket' docs/plans/sdk/02-crate-topology.md` ≥ `W2 结束计数 + 4`；`rg -n 'task_output' docs/sdk/README.md` 命中 Fact-Fix 条目。
- Stop if：OpenAPI 侧某字段命名需要更新而非登记 → 走 `contracts/openapi/src/**` 正规路径（本 Task 不做），只在 `02 §5` 标注 "W7 adapter 下线时 platform 决定"；Fact-Fix 条目若与 `docs/sdk/03` 正文强冲突（例如 03 明文要求 15+1 工具）→ Stop #1。

Step 3：
- Action：执行 `01-ai-execution-protocol.md §4 Weekly Gate` 全部勾选；与 `00-overview.md §3 W3` 的出口状态 / 硬门禁逐条对齐；更新本文件"任务状态 + Checkpoint + 变更日志"；把 `README.md` 的 W3 行状态从 `in_progress` 改为 `done`；并在 `00-overview.md §10` 追加一行 W3 Weekly Gate 结果。
- Done when：
  - `cargo test -p octopus-sdk-contracts -p octopus-sdk-tools -p octopus-sdk-mcp` 全绿；
  - `cargo clippy -p octopus-sdk-contracts -p octopus-sdk-tools -p octopus-sdk-mcp -- -D warnings` 全绿；
  - `cargo clippy --workspace -- -D warnings` 全绿；
  - `cargo build --workspace` 全绿；
  - `cargo test --workspace` 全绿（含 capability 删除后的 legacy 回归）；
  - `find crates/octopus-sdk-tools crates/octopus-sdk-mcp -type f -name '*.rs' -exec wc -l {} \; | awk '$1 > 800'` 无输出（单文件行数硬约束）；
  - `cargo test -p octopus-sdk-tools --test registry_stability` 绿（W3 硬门禁 · Prompt Cache 稳定性）；
  - `cargo test -p octopus-sdk-tools --test partition_concurrency` 绿（W3 硬门禁 · 并发分区）；
  - `cargo test -p octopus-sdk-tools --test bash_output_limit` 绿（W3 硬门禁 · Bash 截断）；
  - `cargo test -p octopus-sdk-mcp --test mcp_process_lifecycle` 绿（W3 硬门禁 · MCP 进程回收 · R3）；
  - `cargo test -p octopus-sdk-tools --test mcp_sdk_shim_roundtrip` 绿（W3 硬门禁 · MCP in-process shim · R5）；
  - `rg --type rust -e 'capability_runtime|CapabilityPlanner|CapabilitySurface' crates/` **0 命中**（W3 对外硬门禁 · Capability 下线，与 `00-overview.md §3 W3` 完全一致）；
  - `rg --type rust -e '<SUPER>' crates/` **0 命中**（W3 内部保险 · Capability Scan Superset，见 R7）；
  - `rg '^\| `06-week-3-tools-mcp\.md` \| .* \| `done` \|' docs/plans/sdk/README.md` 命中；
  - `rg 'W3|06-week-3-tools-mcp|octopus-sdk-tools|octopus-sdk-mcp' docs/plans/sdk/00-overview.md` 在 `§10` 新增一条当周收尾记录。
- Verify：执行上述 14 条命令。
- Stop if：任一硬门禁失败 → Weekly Gate 未通过；W3 保持 `in_progress`，不切 W4；在 `00-overview.md §6 风险登记簿` 追加阻塞条。

Notes：
- Step 3 是本 Plan 的唯一出口；未执行不得声明本周完成。
- 本 Task 以文档收尾为主；若 Weekly Gate 暴露仓库级既有门禁（类似 W2 Tauri sidecar 阻塞），允许同批提交最小 unblocker 代码修复后再完成收尾。
- 本 Task 不改 OpenAPI / schema 生成物；不改 `docs/sdk/01–14` 主体文字（`## Fact-Fix 勘误` 除外）。

---

## Exit State 对齐表（与 `00-overview.md §3 W3` 逐条对齐）

| `00 §3 W3` 出口状态 | 本 Plan 对应任务 | 验证命令 |
|---|---|---|
| 15 个内置工具以 `trait Tool` 形式存在 | Task 7（4 sub-Step） | `cargo test -p octopus-sdk-tools --test builtin_fs_read --test builtin_fs_write --test builtin_bash --test builtin_web --test builtin_ask --test builtin_stubs` |
| `partition_tool_calls` 严格实现只读并发 / 写串行；默认 `max_concurrency = 10` | Task 3 Step 3 + Task 8 Step 2 | `cargo test -p octopus-sdk-tools --test partition_concurrency` |
| MCP stdio / http / sdk 三 transport 通过集成测试 | Task 6 Step 1 / Step 2 / Step 3 | `cargo test -p octopus-sdk-mcp --test mcp_stdio_transport --test mcp_http_transport --test mcp_sdk_transport` |
| `crates/tools/src/capability_runtime/*` 与 `adapter::capability_*_bridge.rs` 本周末全部删除 | Task 9（9a–9d） | `ls crates/tools/src/capability_runtime/ 2>/dev/null && ls crates/octopus-runtime-adapter/src/capability_*.rs 2>/dev/null`（均空） |
| `rg "capability_runtime\|CapabilityPlanner\|CapabilitySurface" crates/` 无结果（对外硬门禁 · 3-token） | Task 9 Step 9d | `rg --type rust -e 'capability_runtime|CapabilityPlanner|CapabilitySurface' crates/`（期望 0 hits） |
| `<SUPER>` 无结果（W3 内部保险 · 14-token，含 `capability_state_ref / load_capability_store / persist_capability_store` 等补全口径） | Task 9 Step 9a / 9b / 9c / 9d | `rg --type rust -e '<SUPER>' crates/`（期望 0 hits） |
| `BASH_MAX_OUTPUT_DEFAULT = 30_000` 字符（硬门禁） | Task 3 Step 3（常量）+ Task 8 Step 3（测试） | `cargo test -p octopus-sdk-tools --test bash_output_limit` |
| MCP 进程 drop 安全（R3） | Task 6 Step 4 | `cargo test -p octopus-sdk-mcp --test mcp_process_lifecycle` |
| Prompt Cache 稳定性（R1；隐含于 "本周出口状态" 的工具顺序不变承诺） | Task 2 Step 3 + Task 8 Step 1 | `cargo test -p octopus-sdk-tools --test registry_stability` |
| `cargo test -p octopus-sdk-tools -p octopus-sdk-mcp` 全绿 | Task 10 Step 3 | `cargo test -p octopus-sdk-tools -p octopus-sdk-mcp` |

---

## 公共面登记回链（必填）

本 Plan 所有对外 `pub` 符号集中登记在 `02-crate-topology.md §2.1 / §2.4 / §2.5`；对任一节的签名修正必须在 Task 1 / Task 2 / Task 3 / Task 4 / Task 5 / Task 6 的同批 PR 内完成，不得延后。

## Legacy 退役登记回链（必填）

| 本 Plan 任务 | `03-legacy-retirement.md` 条目 | 状态变化 |
|---|---|---|
| Task 9（9d） | §3.1 `capability_runtime/**`（6 文件） | W3 末 `pending → done`（本周真删） |
| Task 9（9d） | §6.1 `Capability Bridge`（3 文件：`capability_executor_bridge.rs / capability_planner_bridge.rs / capability_state.rs`） | W3 末 `pending → done`（本周真删） |
| Task 7b（BashTool）替代 `runtime::bash` 的能力面（但不删代码） | §2 `crates/runtime` 行 `bash.rs + bash_validation.rs` | W3 期间保持 `pending`（本周仅新 SDK 覆盖能力，旧代码 W7 删） |
| Task 7a/b（FS 工具族）替代 `runtime::file_ops` 的能力面（但不删代码） | §2 `crates/runtime` 行 `file_ops.rs` | W3 期间保持 `pending` |
| Task 4/5/6（MCP）替代 `runtime::mcp*` 的能力面（但不删代码） | §2 `crates/runtime` 行 `mcp*.rs / mcp_stdio / mcp_lifecycle_hardened.rs` | W3 期间保持 `pending` |
| Task 2/3/7（Tools）替代 `crates/tools` 大部分模块（但不删文件） | §3 `tool_registry / builtin_catalog / builtin_exec / fs_shell / skill_runtime / web_external / split_module_tests` | W3 期间保持 `pending`（仅 `capability_runtime/**` 真删） |

> 说明：W3 的"真删"仅覆盖 `capability_runtime/** + 三个 capability bridge + 1 个 capability_runtime_tests.rs`。其他 `crates/runtime / crates/tools / crates/api / crates/plugins / crates/octopus-runtime-adapter` 的删除动作统一延到 W7。

---

## Batch Checkpoint Format

按 `01-ai-execution-protocol.md §6.1` 追加（本文档末尾）。

```md
## Checkpoint YYYY-MM-DD HH:MM

- Week: W3
- Batch: Task <i> Step <j> → Task <i+1> Step <j>
- Completed:
  - <item>
- Files changed:
  - `path` (+added / -deleted / modified)
- Verification:
  - `cargo test -p octopus-sdk-tools -p octopus-sdk-mcp` → pass
  - `cargo clippy --workspace -- -D warnings` → pass
  - `rg 'capability_runtime|CapabilityPlanner|CapabilitySurface' crates/` → 0 hits
- Exit state vs plan:
  - matches / partial / blocked
- Blockers:
  - <none | 具体问题 + 待人判断点>
- Next:
  - <Task i+1 Step j+1 | Task 10 | Weekly Gate>
```

---

## 附录 A：capability call-site 分类表（Task 9a 已填充）

> `Capability Scan Superset` 实扫结果共 34 个 Rust 文件；口径：
>
> ```
> capability_runtime|CapabilityPlanner|CapabilitySurface|CapabilityExecutor|CapabilityExposure|CapabilityState|CapabilityStore|CapabilityExecutionEvent|CapabilityExecutionPhase|CapabilityExecutionRequest|CapabilityMediationDecision|capability_executor_bridge|capability_planner_bridge|capability_state|capability_state_ref|load_capability_store|persist_capability_store
> ```
>
> 分类结果：`redirect-to-sdk = 4`，未超过 Step 9a 的 Stop 条件。

| 文件 | 命中 token 列表 | 命中行数 | 分类 | 所属 sub-Step | 预估改动行 | 备注 |
|---|---|---|---|---|---|---|
| crates/octopus-core/src/lib.rs | CapabilityState, CapabilitySurface, capability_state_ref | 7 | dead | 9c | 90 | 删 `RuntimeCapability*` DTO 与 `capability_state_ref` 字段 |
| crates/octopus-infra/src/infra_state.rs | capability_state_ref | 4 | dead | 9c | 36 | 删持久化列与迁移断言里的 `capability_state_ref` |
| crates/octopus-platform/src/runtime.rs | capability_state_ref | 3 | dead | 9c | 18 | 测试/fixture 里的旧字段字面量 |
| crates/octopus-runtime-adapter/src/adapter_state.rs | capability_state_ref | 1 | dead | 9b | 6 | 聚合 summary 的残留字段透传 |
| crates/octopus-runtime-adapter/src/agent_runtime_core.rs | capability_executor_bridge, capability_planner_bridge, capability_runtime, capability_state, capability_state_ref, load_capability_store, persist_capability_store | 96 | redirect-to-sdk | 9b | 280 | 运行时主调用链，替到 `octopus_sdk_tools` / `octopus_sdk_mcp` |
| crates/octopus-runtime-adapter/src/approval_runtime_tests.rs | capability_state_ref, load_capability_store, persist_capability_store | 13 | dead | 9b | 48 | capability 持久化测试壳，跟随 adapter 清理 |
| crates/octopus-runtime-adapter/src/capability_executor_bridge.rs | CapabilityPlanner, CapabilityStore, capability_runtime | 12 | dead | 9b | 298 | 整文件删除 |
| crates/octopus-runtime-adapter/src/capability_planner_bridge.rs | CapabilityPlanner, CapabilityState, CapabilityStore, capability_runtime, capability_state, capability_state_ref, persist_capability_store | 39 | dead | 9b | 593 | 整文件删除 |
| crates/octopus-runtime-adapter/src/capability_runtime_tests.rs | capability_runtime, capability_state, capability_state_ref | 16 | dead | 9b | 1994 | 整文件删除 |
| crates/octopus-runtime-adapter/src/capability_state.rs | CapabilityState, CapabilityStore, capability_state, load_capability_store, persist_capability_store | 25 | dead | 9b | 91 | 整文件删除 |
| crates/octopus-runtime-adapter/src/execution_events.rs | capability_state_ref | 25 | dead | 9b | 38 | 事件投影里的旧字段搬运 |
| crates/octopus-runtime-adapter/src/lib.rs | CapabilityState, capability_executor_bridge, capability_planner_bridge, capability_runtime, capability_state | 5 | dead | 9b | 8 | 删 `mod capability_*` 与相关 re-export |
| crates/octopus-runtime-adapter/src/memory_selector.rs | capability_state_ref | 1 | dead | 9b | 4 | 默认 summary 里的空字段 |
| crates/octopus-runtime-adapter/src/persistence.rs | capability_state, capability_state_ref | 6 | redirect-to-sdk | 9b | 120 | 持久化路径需改到 SDK 后的新快照/结果面 |
| crates/octopus-runtime-adapter/src/run_context.rs | capability_planner_bridge, capability_state, capability_state_ref, load_capability_store | 10 | redirect-to-sdk | 9b | 60 | submit 上下文仍在拼 capability runtime 输入 |
| crates/octopus-runtime-adapter/src/runtime_compatibility_tests.rs | CapabilityState | 1 | dead | 9b | 14 | 兼容测试只剩旧快照类型引用 |
| crates/octopus-runtime-adapter/src/runtime_persistence_tests.rs | capability_state, capability_state_ref, load_capability_store | 6 | dead | 9b | 34 | legacy capability 文件回退测试 |
| crates/octopus-runtime-adapter/src/session_service.rs | CapabilityStore, capability_state, capability_state_ref | 8 | redirect-to-sdk | 9b | 90 | 建 session 时仍走 capability projection |
| crates/octopus-runtime-adapter/src/subrun_orchestrator.rs | capability_state_ref | 5 | dead | 9b | 20 | 子运行默认 checkpoint 里的旧字段 |
| crates/octopus-runtime-adapter/src/team_runtime.rs | CapabilityStore, capability_state, capability_state_ref | 6 | dead | 9b | 40 | worker subrun 的旧 capability 预热与摘要透传 |
| crates/octopus-server/src/workspace_runtime.rs | capability_state_ref | 3 | dead | 9c | 18 | server 侧 fixture / 断言字段 |
| crates/runtime/src/session/session_tests.rs | capability_runtime | 3 | panic-stub | 9c | 24 | legacy session extension 测试；W7 runtime 清退前改成占位 |
| crates/rusty-claude-cli/src/main.rs | CapabilityExecutor, CapabilityPlanner, CapabilityState, CapabilityStore, capability_runtime, capability_state | 163 | panic-stub | 9c | 320 | CLI 仍整块依赖 legacy capability runtime；先打 `TODO(W7-RETIRE)` |
| crates/tools/src/builtin_catalog.rs | capability_runtime | 1 | dead | 9c | 12 | capability 可见性桥接类型删掉 |
| crates/tools/src/capability_runtime/executor.rs | CapabilityExecutor, CapabilityPlanner, CapabilityStore | 17 | dead | 9c | 645 | 整文件删除 |
| crates/tools/src/capability_runtime/exposure.rs | CapabilityExposure | 2 | dead | 9c | 60 | 整文件删除 |
| crates/tools/src/capability_runtime/planner.rs | CapabilityPlanner, CapabilityState, CapabilitySurface | 32 | dead | 9c | 449 | 整文件删除 |
| crates/tools/src/capability_runtime/provider.rs | CapabilityExecutor, CapabilityPlanner, CapabilityState, CapabilityStore, CapabilitySurface | 40 | dead | 9c | 1506 | 整文件删除 |
| crates/tools/src/capability_runtime/state.rs | CapabilityExposure, CapabilityState, CapabilityStore, CapabilitySurface, capability_runtime | 18 | dead | 9c | 437 | 整文件删除 |
| crates/tools/src/lib.rs | CapabilityExecutor, CapabilityExposure, CapabilityPlanner, CapabilityState, CapabilityStore, CapabilitySurface, capability_runtime | 13 | dead | 9c | 30 | 删 `mod capability_runtime` 与全部导出 |
| crates/tools/src/skill_runtime.rs | CapabilityExecutor, CapabilityState, capability_runtime | 9 | dead | 9c | 60 | skill capability 描述与 executor 接口一起退掉 |
| crates/tools/src/split_module_tests.rs | CapabilityPlanner, CapabilityState, CapabilityStore, capability_runtime, capability_state | 167 | dead | 9c | 280 | legacy capability 回归测试批量删除 |
| crates/tools/src/subagent_runtime.rs | CapabilityPlanner, CapabilityState, CapabilityStore, capability_runtime, capability_state | 31 | panic-stub | 9c | 160 | W5 `AgentTool` 前不再维护 legacy subagent capability wiring |
| crates/tools/src/tool_registry.rs | CapabilityPlanner, CapabilityState, capability_runtime | 10 | dead | 9c | 52 | 旧 tool search / deferred surface 帮助函数删掉 |

---

## 变更日志

| 日期 | 变更 | 责任人 |
|---|---|---|
| 2026-04-21 | 首稿（10 Task Ledger + Exit State 对齐表 + Legacy 退役回链 + 附录 A 占位）。核心决策：① 沿用 W2 B3 先例，在 `octopus-sdk-contracts` 下沉 `PermissionGate / PermissionOutcome / PermissionMode / ToolCallRequest / AskResolver`；② `SandboxHandle` 占位于 tools crate，W4 由 `octopus-sdk-sandbox` 升级；③ MCP in-process shim 以 `ToolDirectory` trait（定义于 mcp）+ `ToolRegistry::as_directory()` 实现的方向一致化方案落地；④ Task 9 的 capability 退役拆 4 sub-PR，每 PR ≤ 800 行；⑤ Task 7 的 15 工具分 11 full + 4 W5-stub；⑥ W3 硬门禁加入 `mcp_process_lifecycle` + `mcp_sdk_shim_roundtrip` + `registry_stability` 三项契约测试。 | AI Agent |
| 2026-04-21 | P1/P2 审计修订：① **P1-a** 修订清单 #22 的 `ToolDirectory` 签名改为全 MCP 原生类型（`fn list_tools(&self) -> Vec<McpTool>; async fn call_tool(..) -> Result<McpToolResult, McpError>`），与 Architecture `tools → mcp` 方向与 Task 4 Step 4 决策对齐；② **P1-b** 引入 `Capability Scan Superset`（14-token 正则），Step 9a 审计、9b/9c/9d 守护扫描、Exit Alignment、Task 10 Weekly Gate 全部同步为"对外 3-token + 内部 14-token 双口径"；Task 9 Files 清单补入 `adapter_state.rs / memory_selector.rs / subrun_orchestrator.rs / team_runtime.rs / runtime_persistence_tests.rs / execution_events.rs / approval_runtime_tests.rs` 等仅持 `capability_state_ref / load_capability_store / persist_capability_store` 的调用点；③ **P2-a** WebSearchTool 由 11 full 降为 5 个 stub 中的 W6-stub（W3 不引入 `SearchProvider` trait，归 `octopus-sdk-plugin` W5 定义、`octopus-sdk-core` W6 装配），Scope/R4/Task 7c/Exit Alignment 统一为 "10 full + 5 stub (4 W5 + 1 W6)"；④ **P2-b** `partition_tool_calls` 语义统一为工具级粒度（`is_concurrency_safe` 为准），`ResourceKey` 级 serial bucket 明确延 W4，Scope/Task 3 Step 3/Task 8 Step 2 同批对齐，`02 §5` 预登 `W4 增强项 · partition_tool_calls.resource_bucket`；⑤ **Open Q1** Scope Out of scope 追加 `task_output` defer 条款 + Task 10 Step 2 强制 `docs/sdk/README.md ## Fact-Fix 勘误` 追加 `[Fact-Fix · W3] 03-tool-system.md §3.4 task_output`；⑥ **Open Q2** Execution Rules 第 "default-members" 条补阶段性偏离说明 + 回收点（W7 legacy 整合同批收敛回 `02 §8` 目标口径）。 | AI Agent |
| 2026-04-21 | 复审收口修订：① Architecture / Execution Rules 统一为 **`tools → mcp` 单向窄依赖**，删除 W3 中会导致 `mcp → tools` 反向依赖的旧表述；② Task 4 改成唯一的 MCP-native `ToolDirectory` 路径，去掉旧版 `ToolSpec / ToolResult / ToolError` 签名与伪 precondition；③ 修订清单 #15 改成 `4 个 W5-stub + 1 个 W6-stub`；④ `task_output` defer/Fact-Fix 只引用仓内已登记事实，不再引用任何未落 retrieval 契约名；⑤ R5 的 shim 构造器口径统一成 `SdkTransport::from_directory(registry.as_directory())`。 | AI Agent |
| 2026-04-21 | W3 Weekly Gate 收口：① `cargo build --workspace`、`cargo clippy --workspace -- -D warnings`、`cargo test --workspace` 全绿；② 显式补跑 `registry_stability / partition_concurrency / bash_output_limit / mcp_sdk_shim_roundtrip / mcp_process_lifecycle` 硬门禁；③ `02-crate-topology.md §2.4` 补登记 `ToolError::as_tool_result`；④ Task 9 / Task 10 切为 `done` 并补齐最终 Checkpoint。 | AI Agent |
| 2026-04-21 | W3 审计补完：① 删除遗漏的 `crates/tools/src/capability_runtime/{mod.rs,events.rs}` 两个残留文件；② `Capability Scan Superset` 从 14-token 扩为 18-token，补上 `CapabilityExecutionEvent / CapabilityExecutionPhase / CapabilityExecutionRequest / CapabilityMediationDecision`，避免 residual 类型绕过门禁；③ 重跑 capability 守护扫描与 W3 关键门禁，确认 Task 9 真正闭环。 | AI Agent |
| 2026-04-21 | W3 最终审计收口：删除空目录 `crates/tools/src/capability_runtime/`，使 capability runtime 目录级残留也归零；补记最终 Checkpoint，确认 Task 9/10 与仓库物理状态一致。 | AI Agent |

## Checkpoint 2026-04-21 11:45

- Week: W3
- Batch: Task 1 Step 1 → Step 3
- Completed:
  - 新建 `octopus-sdk-tools` 与 `octopus-sdk-mcp` crate 骨架，并把两者加入 workspace `default-members`
  - 在 `octopus-sdk-contracts` 下沉 `ToolCallRequest / PermissionMode / PermissionOutcome / PermissionGate / AskResolver` 契约
  - 回填 `docs/plans/sdk/02-crate-topology.md §2.1` 的 W3 公共面修订清单，并把 W3 plan 状态切到 `in_progress`
- Files changed:
  - `Cargo.toml` / `Cargo.lock`
  - `crates/octopus-sdk-contracts/src/{lib.rs,permission.rs,ask_resolver.rs}`
  - `crates/octopus-sdk-tools/Cargo.toml`
  - `crates/octopus-sdk-tools/src/{lib.rs,stub.rs}`
  - `crates/octopus-sdk-mcp/Cargo.toml`
  - `crates/octopus-sdk-mcp/src/{lib.rs,stub.rs}`
  - `docs/plans/sdk/{README.md,02-crate-topology.md,06-week-3-tools-mcp.md}`
- Verification:
  - `cargo build -p octopus-sdk-tools -p octopus-sdk-mcp` → pass
  - `cargo test -p octopus-sdk-contracts permission::` → pass
  - `cargo test -p octopus-sdk-contracts ask_resolver::` → pass
  - `cargo clippy -p octopus-sdk-contracts -p octopus-sdk-tools -p octopus-sdk-mcp -- -D warnings` → pass
  - `cargo metadata --format-version=1 --no-deps | jq -r '.workspace_default_members[]' | rg 'octopus-sdk-(tools|mcp)'` → pass
  - `rg -n '^(rusqlite|tauri|axum|octopus-core|octopus-platform)\\s*=|^octopus-sdk-mcp\\s*=|^octopus-sdk-tools\\s*=' crates/octopus-sdk-tools/Cargo.toml crates/octopus-sdk-mcp/Cargo.toml` → 0 hits
- Exit state vs plan:
  - matches
- Blockers:
  - none
- Next:
  - Task 2 Step 1

## Checkpoint 2026-04-21 11:48

- Week: W3
- Batch: Task 2 Step 1 → Step 3
- Completed:
  - 落地 `ToolCategory / ToolSpec / Tool::spec()` 基础契约，并复用 `octopus_sdk_contracts::ToolSchema`
  - 落地 `ToolRegistry` 的确定性排序、重复名防护、稳定指纹和 canonical JSON 排序
  - 预置 `ToolContext / ToolResult / ToolError / RegistryError` 最小骨架，满足 `Tool` trait 对象安全与注册表测试
- Files changed:
  - `crates/octopus-sdk-tools/src/{lib.rs,context.rs,error.rs,result.rs,spec.rs,tool.rs,registry.rs}`
- Verification:
  - `cargo test -p octopus-sdk-tools spec::` → pass
  - `cargo test -p octopus-sdk-tools registry::` → pass
  - `cargo clippy -p octopus-sdk-tools -- -D warnings` → pass
- Exit state vs plan:
  - matches
- Blockers:
  - none
- Next:
  - Task 3 Step 1

## Checkpoint 2026-04-21 12:00

- Week: W3
- Batch: Task 3 Step 1 → Step 3
- Completed:
  - 落地 `ToolContext` / `SandboxHandle` 真字段，并用 mock services 补齐可构造测试
  - 落地 `ToolResult` / `ToolError` / `RegistryError` 与 `as_tool_result()` 边界转换
  - 落地 `ExecBatch` / `partition_tool_calls` / 三个常量，并把 `EventSink`、`AskResolver(prompt_id, ...)`、`ToolError::NotYetImplemented { crate_name, week }` 同步回计划与拓扑文档
- Files changed:
  - `crates/octopus-sdk-contracts/src/event.rs`
  - `crates/octopus-sdk-tools/src/{constants.rs,context.rs,error.rs,lib.rs,partition.rs,result.rs}`
  - `docs/plans/sdk/{02-crate-topology.md,06-week-3-tools-mcp.md}`
- Verification:
  - `cargo check -p octopus-sdk-tools --tests` → pass
  - `cargo test -p octopus-sdk-tools error::` → pass
  - `cargo test -p octopus-sdk-tools result::` → pass
  - `cargo test -p octopus-sdk-tools partition::` → pass
  - `cargo clippy -p octopus-sdk-tools -- -D warnings` → pass
- Exit state vs plan:
  - matches
- Blockers:
  - none
- Next:
  - Task 4 Step 1

## Checkpoint 2026-04-21 12:04

- Week: W3
- Batch: Task 4 Step 1 → Step 4
- Completed:
  - 落地 `JsonRpcRequest / JsonRpcResponse / JsonRpcNotification / JsonRpcError`，补齐 round-trip 测试
  - 落地 `McpTool / McpPrompt / McpResource / McpToolResult / McpError / ToolDirectory`
  - 把计划里的 Task 4 互转口径从 `ToolSpec` 收口为 `ToolSchema`，同步回 `§2.5` 公共面文档
- Files changed:
  - `crates/octopus-sdk-mcp/src/{directory.rs,error.rs,jsonrpc.rs,lib.rs,types.rs}`
  - `docs/plans/sdk/{02-crate-topology.md,06-week-3-tools-mcp.md}`
- Verification:
  - `cargo test -p octopus-sdk-mcp jsonrpc::` → pass
  - `cargo test -p octopus-sdk-mcp types::` → pass
  - `cargo test -p octopus-sdk-mcp error::` → pass
  - `cargo check -p octopus-sdk-mcp` → pass
  - `wc -l crates/octopus-sdk-mcp/src/lib.rs` → `13`
  - `cargo clippy -p octopus-sdk-mcp -- -D warnings` → pass
- Exit state vs plan:
  - matches
- Blockers:
  - none
- Next:
  - Task 5 Step 1

## Checkpoint 2026-04-21 12:08

- Week: W3
- Batch: Task 5 Step 1 → Step 3
- Completed:
  - 落地 `McpTransport` / `TransportKind` 与默认 `notify()` 路径
  - 落地 `McpClient` 自动初始化链路、`InitializeResult`、prompt/resource 的空列表降级
  - 落地 `McpServerManager` / `McpServerSpec` / `McpServerTransport` / `McpLifecyclePhase`，补齐 spawn-shutdown-drop 基础测试
- Files changed:
  - `crates/octopus-sdk-mcp/src/{client.rs,lib.rs,lifecycle.rs,manager.rs,transport/mod.rs}`
  - `docs/plans/sdk/{02-crate-topology.md,06-week-3-tools-mcp.md}`
- Verification:
  - `cargo check -p octopus-sdk-mcp` → pass
  - `cargo test -p octopus-sdk-mcp client::` → pass
  - `cargo test -p octopus-sdk-mcp manager::` → pass
  - `cargo clippy -p octopus-sdk-mcp -- -D warnings` → pass
- Exit state vs plan:
  - matches
- Blockers:
  - none
- Next:
  - Task 6 Step 1

## Checkpoint 2026-04-21 12:31

- Week: W3
- Batch: Task 6 Step 1 → Step 4
- Completed:
  - 落地 `StdioTransport / HttpTransport / SdkTransport` 三个 transport，并补齐 `mcp-echo-server` fixture binary
  - 修正 stdio drop 在 `tokio current_thread` runtime 下的回收路径，改成独立线程同步 `start_kill + wait`
  - 补齐 HTTP method 精确匹配与 lifecycle 进程标记，避免并行测试互相污染
- Files changed:
  - `crates/octopus-sdk-mcp/Cargo.toml`
  - `crates/octopus-sdk-mcp/src/transport/{http.rs,sdk.rs,stdio.rs}`
  - `crates/octopus-sdk-mcp/src/bin/mcp-echo-server.rs`
  - `crates/octopus-sdk-mcp/tests/{mcp_http_transport.rs,mcp_process_lifecycle.rs,mcp_sdk_transport.rs,mcp_stdio_transport.rs}`
  - `crates/octopus-sdk-mcp/tests/fixtures/{initialize_response.json,tools_call_response.json,tools_list_response.json}`
  - `docs/plans/sdk/06-week-3-tools-mcp.md`
- Verification:
  - `cargo test -p octopus-sdk-mcp --test mcp_stdio_transport` → pass
  - `cargo test -p octopus-sdk-mcp --test mcp_http_transport` → pass
  - `cargo test -p octopus-sdk-mcp --test mcp_sdk_transport` → pass
  - `cargo test -p octopus-sdk-mcp --test mcp_process_lifecycle` → pass
  - `cargo check -p octopus-sdk-mcp` → pass
  - `cargo test -p octopus-sdk-mcp` → pass
  - `cargo clippy -p octopus-sdk-mcp --tests -- -D warnings` → pass
- Exit state vs plan:
  - matches
- Blockers:
  - none
- Next:
  - Task 7 Step 7a

## Checkpoint 2026-04-21 12:43

- Week: W3
- Batch: Task 7 Step 7a
- Completed:
  - 新增 `builtin` 模块并落地 `FileReadTool / GlobTool / GrepTool`
  - `FileReadTool` 支持 `offset + limit` 区间读、`NNNNNN|` 行号格式和 2000 行 / 500 KB 双阈值截断
  - `GlobTool` 与 `GrepTool` 统一按 workspace 相对路径回显，避免 canonical path 泄露到工具输出
- Files changed:
  - `crates/octopus-sdk-tools/src/{lib.rs,builtin/mod.rs,builtin/fs_read.rs,builtin/fs_glob.rs,builtin/fs_grep.rs}`
  - `crates/octopus-sdk-tools/tests/builtin_fs_read.rs`
  - `docs/plans/sdk/06-week-3-tools-mcp.md`
- Verification:
  - `cargo test -p octopus-sdk-tools --test builtin_fs_read` → pass
  - `cargo check -p octopus-sdk-tools` → pass
  - `cargo clippy -p octopus-sdk-tools --test builtin_fs_read -- -D warnings` → pass
- Exit state vs plan:
  - partial（Task 7 持续中；7a 完成，切到 7b）
- Blockers:
  - none
- Next:
  - Task 7 Step 7b

## Checkpoint 2026-04-21 13:02

- Week: W3
- Batch: Task 7 Step 7b
- Completed:
  - 落地 `FileWriteTool / FileEditTool / BashTool`
  - 写工具统一走 `permissions.check()` 和原子落盘；`edit_file` 支持 `replace_all`
  - `BashTool` 接上 `sandbox.cwd`、环境白名单、120s 默认超时和输出截断提示
- Files changed:
  - `crates/octopus-sdk-tools/Cargo.toml`
  - `crates/octopus-sdk-tools/src/builtin/{mod.rs,fs_write.rs,fs_edit.rs,shell_bash.rs}`
  - `crates/octopus-sdk-tools/tests/{builtin_fs_write.rs,builtin_bash.rs}`
  - `docs/plans/sdk/06-week-3-tools-mcp.md`
- Verification:
  - `cargo test -p octopus-sdk-tools --test builtin_fs_write` → pass
  - `cargo test -p octopus-sdk-tools --test builtin_bash` → pass
  - `cargo check -p octopus-sdk-tools` → pass
  - `cargo clippy -p octopus-sdk-tools --test builtin_fs_write --test builtin_bash -- -D warnings` → pass
- Exit state vs plan:
  - partial（Task 7 持续中；7a/7b 完成，切到 7c）
- Blockers:
  - none
- Next:
  - Task 7 Step 7c

## Checkpoint 2026-04-21 13:18

- Week: W3
- Batch: Task 7 Step 7c
- Completed:
  - 落地 `WebFetchTool / WebSearchTool / AskUserQuestionTool / TodoWriteTool / SleepTool`
  - `WebFetchTool` 走 `reqwest` + 最小 HTML 剥离和 30_000 字符截断；`WebSearchTool` 固定为 W6 stub
  - `AskUserQuestionTool` 接上 `AskResolver`、300s 超时和 `SessionEvent::Ask`；`TodoWriteTool` 因 contracts 无 `TodoUpdated` 改为发 `RenderKind::Record`
- Files changed:
  - `crates/octopus-sdk-tools/Cargo.toml`
  - `crates/octopus-sdk-tools/src/builtin/{mod.rs,web_fetch.rs,web_search.rs,ask_user_question.rs,todo_write.rs,sleep.rs}`
  - `crates/octopus-sdk-tools/tests/{builtin_web.rs,builtin_ask.rs,support/mod.rs}`
  - `docs/plans/sdk/{02-crate-topology.md,06-week-3-tools-mcp.md}`
- Verification:
  - `cargo test -p octopus-sdk-tools --test builtin_web --test builtin_ask` → pass
  - `cargo check -p octopus-sdk-tools` → pass
  - `cargo clippy -p octopus-sdk-tools --test builtin_web --test builtin_ask -- -D warnings` → pass
- Exit state vs plan:
  - partial（Task 7 持续中；7c 完成，切到 7d）
- Blockers:
  - none
- Next:
  - Task 7 Step 7d

## Checkpoint 2026-04-21 13:19

- Week: W3
- Batch: Task 7 Step 7d
- Completed:
  - 落地 `AgentTool / SkillTool / TaskListTool / TaskGetTool` 四个 W5 stub
  - 在 `builtin/mod.rs` 增加 `register_builtins()`，固定 15 个内置工具注册入口
  - 补齐 W5 stub 集成测试，统一断言 `NotYetImplemented` 和 `ToolResult.is_error = true`
- Files changed:
  - `crates/octopus-sdk-tools/src/builtin/{mod.rs,w5_stubs.rs}`
  - `crates/octopus-sdk-tools/tests/builtin_stubs.rs`
  - `docs/plans/sdk/{02-crate-topology.md,06-week-3-tools-mcp.md}`
- Verification:
  - `cargo test -p octopus-sdk-tools --test builtin_stubs` → pass
  - `cargo test -p octopus-sdk-tools --test builtin_web --test builtin_ask --test builtin_stubs` → pass
  - `cargo check -p octopus-sdk-tools` → pass
  - `cargo clippy -p octopus-sdk-tools --test builtin_web --test builtin_ask --test builtin_stubs -- -D warnings` → pass
- Exit state vs plan:
  - matches（Task 7 完成，切到 Task 8）
- Blockers:
  - none
- Next:
  - Task 8 Step 1

## Checkpoint 2026-04-21 13:42

- Week: W3
- Batch: Task 8 Step 1 → Step 4
- Completed:
  - 新增 `registry_stability / partition_concurrency / bash_output_limit / mcp_sdk_shim_roundtrip` 四组硬门禁测试
  - `ToolRegistry` 增加 `as_directory()`，并在 tools crate 内实现 `ToolDirectory` shim adapter，收口 `ToolResult -> McpToolResult`
  - 追加 `octopus-sdk-tools -> octopus-sdk-mcp` 的窄依赖，只用于 in-process MCP shim
- Files changed:
  - `crates/octopus-sdk-tools/Cargo.toml`
  - `crates/octopus-sdk-tools/src/registry.rs`
  - `crates/octopus-sdk-tools/tests/{registry_stability.rs,partition_concurrency.rs,bash_output_limit.rs,mcp_sdk_shim_roundtrip.rs}`
  - `docs/plans/sdk/{02-crate-topology.md,06-week-3-tools-mcp.md}`
- Verification:
  - `cargo test -p octopus-sdk-tools --test registry_stability --test partition_concurrency --test bash_output_limit --test mcp_sdk_shim_roundtrip` → pass
  - `cargo check -p octopus-sdk-tools` → pass
  - `cargo clippy -p octopus-sdk-tools --test registry_stability --test partition_concurrency --test bash_output_limit --test mcp_sdk_shim_roundtrip -- -D warnings` → pass
- Exit state vs plan:
  - matches（Task 8 完成，切到 Task 9）
- Blockers:
  - none
- Next:
  - Task 9 Step 9a

## Checkpoint 2026-04-21 14:41

- Week: W3
- Batch: Task 9 Step 9a → Step 9d
- Completed:
  - 删除 `crates/tools/src/capability_runtime/**` 与 `crates/octopus-runtime-adapter/src/capability_{executor_bridge,planner_bridge,state,runtime_tests}.rs`
  - 对齐 `octopus-runtime-adapter` 相关测试到当前 runtime 投影边界，收口 approval / tool events / runtime turn loop 断言
  - 将 `rusty-claude-cli` 主入口降级为 `TODO(W7-RETIRE)` panic-stub，并禁用依赖 legacy `claw` binary 的集成测试
- Files changed:
  - `crates/tools/src/lib.rs` + `crates/tools/src/capability_runtime/**`
  - `crates/octopus-runtime-adapter/src/{approval_runtime_tests.rs,actor_runtime_tests.rs,lib.rs,mcp_runtime_tests.rs,runtime_persistence_tests.rs}`
  - `crates/octopus-runtime-adapter/tests/runtime_turn_loop.rs`
  - `crates/rusty-claude-cli/src/main.rs`
  - `crates/rusty-claude-cli/tests/{cli_flags_and_config_defaults.rs,resume_slash_commands.rs,mock_parity_harness.rs,output_format_contract.rs}`
  - `docs/plans/sdk/03-legacy-retirement.md`
- Verification:
  - `cargo build --workspace` → pass
  - `cargo clippy --workspace -- -D warnings` → pass
  - `cargo test -p octopus-runtime-adapter --lib` → pass
  - `cargo test -p octopus-runtime-adapter --test runtime_turn_loop` → pass
  - `cargo test -p rusty-claude-cli` → pass
  - `rg --type rust -e 'capability_runtime|CapabilityPlanner|CapabilitySurface' crates/` → 0 hits
  - `rg --type rust -e 'capability_runtime|CapabilityPlanner|CapabilitySurface|CapabilityExecutor|CapabilityExposure|CapabilityState|CapabilityStore|capability_executor_bridge|capability_planner_bridge|capability_state|capability_state_ref|load_capability_store|persist_capability_store' crates/` → 0 hits
- Exit state vs plan:
  - matches
- Blockers:
  - none
- Next:
  - Task 10 Step 1

## Checkpoint 2026-04-21 14:42

- Week: W3
- Batch: Task 10 Step 1 → Step 3
- Completed:
  - 完成 `octopus-sdk-contracts / octopus-sdk-tools / octopus-sdk-mcp` 公共面自核对，并在 `02 §2.4` 补登记 `ToolError::as_tool_result`
  - 确认 `02 §5` 的 W3 契约差异和 `docs/sdk/README.md` 的 `task_output` Fact-Fix 已落地
  - 完成 W3 Weekly Gate，Task 9 / Task 10 与 Active Work 全部切到完成态
- Files changed:
  - `docs/plans/sdk/02-crate-topology.md`
  - `docs/plans/sdk/06-week-3-tools-mcp.md`
- Verification:
  - `cargo build --workspace` → pass
  - `cargo clippy --workspace -- -D warnings` → pass
  - `cargo test --workspace` → pass
  - `cargo test -p octopus-sdk-contracts -p octopus-sdk-tools -p octopus-sdk-mcp` → pass
  - `cargo clippy -p octopus-sdk-contracts -p octopus-sdk-tools -p octopus-sdk-mcp -- -D warnings` → pass
  - `cargo test -p octopus-sdk-tools --test registry_stability --test partition_concurrency --test bash_output_limit --test mcp_sdk_shim_roundtrip` → pass
  - `cargo test -p octopus-sdk-mcp --test mcp_process_lifecycle` → pass
  - `find crates/octopus-sdk-tools crates/octopus-sdk-mcp -type f -name '*.rs' -exec wc -l {} \; | awk '$1 > 800'` → 0 hits
  - `rg --type rust -e 'capability_runtime|CapabilityPlanner|CapabilitySurface' crates/` → 0 hits
  - `rg --type rust -e 'capability_runtime|CapabilityPlanner|CapabilitySurface|CapabilityExecutor|CapabilityExposure|CapabilityState|CapabilityStore|capability_executor_bridge|capability_planner_bridge|capability_state|capability_state_ref|load_capability_store|persist_capability_store' crates/` → 0 hits
- Exit state vs plan:
  - matches
- Blockers:
  - none
- Next:
  - W4 kick-off

## Checkpoint 2026-04-21 16:05

- Week: W3
- Batch: Task 9 residual closeout
- Completed:
  - 删除遗漏的 `crates/tools/src/capability_runtime/{mod.rs,events.rs}`
  - 扩大 `Capability Scan Superset` 守护口径，补上 4 个 legacy execution token
  - 复核 W3 收尾门禁，确认 Task 9/Task 10 的 `done` 状态现在与磁盘真实状态一致
- Files changed:
  - `crates/tools/src/capability_runtime/{mod.rs,events.rs}` (-deleted)
  - `docs/plans/sdk/06-week-3-tools-mcp.md`
- Verification:
  - `find crates/tools/src/capability_runtime -maxdepth 1 -type f` → 0 hits
  - `rg -n 'CapabilityExecution(Request|Event|Phase|MediationDecision)' crates/` → 0 hits
  - `rg --type rust -e 'capability_runtime|CapabilityPlanner|CapabilitySurface|CapabilityExecutor|CapabilityExposure|CapabilityState|CapabilityStore|CapabilityExecutionEvent|CapabilityExecutionPhase|CapabilityExecutionRequest|CapabilityMediationDecision|capability_executor_bridge|capability_planner_bridge|capability_state|capability_state_ref|load_capability_store|persist_capability_store' crates/` → 0 hits
  - `cargo test -p octopus-sdk-contracts -p octopus-sdk-tools -p octopus-sdk-mcp` → pass
- Exit state vs plan:
  - matches
- Blockers:
  - none
- Next:
  - W4 kick-off

## Checkpoint 2026-04-21 16:10

- Week: W3
- Batch: Final audit closeout
- Completed:
  - 删除空目录 `crates/tools/src/capability_runtime/`
  - 复核 W3 计划、SDK 实现、legacy 删除状态和全仓门禁结果
  - 确认 Task 1–10 的 `done` 状态与当前代码仓库一致
- Files changed:
  - `docs/plans/sdk/06-week-3-tools-mcp.md`
  - `crates/tools/src/capability_runtime/` (-removed directory)
- Verification:
  - `test ! -e crates/tools/src/capability_runtime && echo removed` → pass
  - `rg -n 'CapabilityExecution(Request|Event|Phase|MediationDecision)' crates/` → 0 hits
  - `rg --type rust -e 'capability_runtime|CapabilityPlanner|CapabilitySurface|CapabilityExecutor|CapabilityExposure|CapabilityState|CapabilityStore|CapabilityExecutionEvent|CapabilityExecutionPhase|CapabilityExecutionRequest|CapabilityMediationDecision|capability_executor_bridge|capability_planner_bridge|capability_state|capability_state_ref|load_capability_store|persist_capability_store' crates/` → 0 hits
  - `cargo build --workspace` → pass
  - `cargo clippy --workspace -- -D warnings` → pass
  - `cargo test --workspace` → pass
- Exit state vs plan:
  - matches
- Blockers:
  - none
- Next:
  - W4 kick-off
