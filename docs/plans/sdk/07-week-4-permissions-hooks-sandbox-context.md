# W4 · `octopus-sdk-permissions` + `octopus-sdk-sandbox` + `octopus-sdk-hooks` + `octopus-sdk-context`

> 本文档遵循 `docs/plans/sdk/AGENTS.md` 与 `docs/plans/PLAN_TEMPLATE.md`；执行规约见 `docs/plans/sdk/01-ai-execution-protocol.md`。
>
> 阅读顺序：**本文件 →** `docs/sdk/06-permissions-sandbox.md` → `docs/sdk/07-hooks-lifecycle.md` → `docs/sdk/02-context-engineering.md` → `docs/sdk/08-long-horizon.md` → `02-crate-topology.md §2.6 / §2.7 / §2.8 / §2.9 / §5 / §8` → `03-legacy-retirement.md §3`（`runtime::{permissions, hooks, sandbox, prompt, compact, summary_compression}` 行 + adapter Approval/Policy/Memory 行）。

## Status

状态：`draft`（Plan 起稿 + 关键决策审核通过；进入 Task 1 前切 `in_progress`）。

### 已确认的审核决策（2026-04-21）

下列 3 项跨周 / 结构性决策已由 owner 在 Plan 审核时 **明确接受**，不再回翻。执行期若需变更，必须新开 `docs/sdk/README.md ## Fact-Fix 勘误` 或本文件 §变更日志追加专项决策条目。

| # | 决策点 | 确认结论 | 关联章节 |
|---|---|---|---|
| D1 | `ToolCategory` 反向下沉到 Level 0 contracts | **接受**。W3 `sdk-tools` 通过 `pub use octopus_sdk_contracts::ToolCategory` 保持源兼容；`02 §2.4` 与 §10 变更日志同批登记 "W4 反向修订" 条目，以闭合 W3 公共面修改。 | R2 / Task 1 Step 4 / Task 1 Step 5 |
| D2 | Compactor 策略只落 `ClearToolResults` + `Summarize`，`Hybrid` 占位延 W5/W6 | **接受**。W4 的 Compactor 不承担调度决策，`Hybrid` 语义涉及"先清→再压→再回退"的管理背景，归 Orchestrator 层。`CompactionStrategyTag::Hybrid` 变体保留；`Compactor::summarize` 遇 `Hybrid` 返回 `CompactionError::Aborted { reason: "hybrid not implemented in W4" }`。 | Architecture / Task 10 Step 1 |
| D3 | Sandbox 落 3 后端（Noop / Seatbelt / Bubblewrap），Windows 回退 Noop | **接受**。`SeatbeltBackend` / `BubblewrapBackend` 的 smoke 测试统一标 `#[cfg_attr(..., ignore)]`，只在本地 / 专用 runner 跑；合同测试（凭据零暴露）走 `NoopBackend` 即可覆盖。真实 Bubblewrap seccomp profile 与 Windows AppContainer 留 `TODO(W8)`。 | Architecture / Task 6 |

## Goal

产出 **4 个零业务语义的新 SDK crate** —— `crates/octopus-sdk-permissions`（Level 2）/ `crates/octopus-sdk-sandbox`（Level 2）/ `crates/octopus-sdk-hooks`（Level 2）/ `crates/octopus-sdk-context`（Level 3），落地 `docs/sdk/06 / 07 / 02 / 08` 的：

1. **权限层** · `PermissionMode` 四态（`Default / AcceptEdits / BypassPermissions / Plan`） + `PermissionPolicy`（按 `source` 分组的 allow/deny/ask 规则） + `PermissionGate`（真实实现，替换 W3 的浅 shim）+ `ApprovalBroker`（对接 `AskPrompt`）+ **`canUseTool` 决策链**（对齐 `docs/sdk/06 §6.4`）。
2. **沙箱层** · `trait SandboxBackend` + `SandboxSpec` / `SandboxHandle`（从 W3 占位升级为真实类型） + `NoopBackend`（跨平台，test-only） + `SeatbeltBackend`（macOS） + `BubblewrapBackend`（Linux）；Windows 保留 `TODO(W8)`。
3. **钩子层** · `Hook` trait + `HookRunner`（确定性顺序）+ **8 种 `HookEvent`**：`PreToolUse / PostToolUse / Stop / SessionStart / SessionEnd / UserPromptSubmit / PreCompact / PostCompact`；`HookDecision::{Continue, Abort, InjectMessage, Rewrite}`。
4. **上下文层** · `SystemPromptBuilder`（确定性工具顺序 + `tools_fingerprint` 稳定性延续 W3）+ `Compactor`（`Summarize` / `ClearToolResults` 两策略；`Hybrid` 延 W5/W6）+ `trait MemoryBackend` + `DurableScratchpad`（`runtime/notes/<session>.md`）。

本周 **不** 实现 Brain Loop dispatch 流水线（归 W6），也 **不** 把旧 `runtime::permissions / runtime::hooks / runtime::sandbox / runtime::prompt / runtime::compact` 从 legacy crate 中删除（归 W7）；W4 仅做"新 SDK 实现到位 + legacy 仍可保留引用"。

## Architecture

- **Level 2 并行、不互依赖**：`permissions / sandbox / hooks` 三 crate 在 `02 §1.2` 的依赖图内**同层**，互不引用（不得出现 `hooks → permissions` 或 `permissions → sandbox` 的 `use` 语句）。它们的协作点由 `octopus-sdk-tools::ToolContext` 与 W6 `octopus-sdk-core::BrainLoop` 在上层拼装：`ToolContext` 已在 W3 持有 `Arc<dyn PermissionGate>`，W4 只是把 `PermissionGate` 的真实实现注入；`SandboxHandle` 从 W3 的"值类型占位"升级为"由 `SandboxBackend::provision` 产出的带 lifetime 的 handle"；`HookRunner` 由 W6 Brain Loop 在 tool dispatch 的 `pre / post` 阶段调用，W4 不碰 Brain Loop。

- **Level 3 · `octopus-sdk-context`**：可依赖 Level 0–2 全部，但本周**只消费** Level 0 `contracts`（`Message / ContentBlock / Usage / SessionId`）与 Level 2 `sdk-tools::ToolRegistry`（仅为 `SystemPromptBuilder` 从 registry 拉工具指南用，**只读 schema**，不触发执行）。**禁止** `context → permissions / sandbox / hooks` 的直接引用；Hook 的 `PreCompact / PostCompact` 由 W6 Brain Loop 在 `Compactor::maybe_compact` 前后夹层调用，不由 `Compactor` 自身发射。

- **Contracts 补丁面（Level 0 下沉，承 W1/W2/W3 先例）**：W3 已在 `octopus-sdk-contracts` 下沉 `ToolCallRequest / PermissionMode / PermissionOutcome::{Allow, Deny, AskApproval} / PermissionGate / AskResolver / AskAnswer / AskError / EventSink`。W4 追加：
  - `PermissionOutcome::RequireAuth { prompt: AskPrompt }`（完成四变体）；
  - `HookEvent` 枚举（8 类）+ `HookToolResult`（`PostToolUse` 的 Level 0 薄镜像，字段与 `sdk-tools::ToolResult` 对齐，由 hooks/core 边界互转）+ `HookDecision` 薄形状 + `EndReason`；
  - `CompactionResult`（`summary: String / folded_turn_ids: Vec<EventId> / tool_results_cleared: u32 / tokens_before: u32 / tokens_after: u32 / strategy: CompactionStrategyTag`）；
  - `MemoryItem / MemoryError`（最小签名，Level 0 只定义值形状，trait 留在 `sdk-context` Level 3）。
  - 下沉原因：这些是 Level 2/3 之间、以及未来 Level 4 Brain Loop 与 Level 2 hooks 之间的跨层数据契约；与 W3 `ToolCallRequest` / W2 `ToolSchema` 完全对称。

- **Prompt Cache 稳定（延续 C1 / 从 W3 接棒）**：`SystemPromptBuilder::build()` 产出的**段序列**必须 **在同一 `PromptCtx` 输入下字节一致**；工具指南段必须调用 `ToolRegistry::schemas_sorted()`（W3 已保证双键稳定排序），不得自己重排。**本周硬门禁**：`SystemPromptBuilder::fingerprint()` 新增 SHA-256，作为 W6 Brain Loop 缓存键的底层构件；契约测试 3 次构建字节一致。

- **Compaction 策略（Goldilocks）**：W4 首版 **只落两个策略**：
  - `CompactionStrategy::ClearToolResults`：对齐 Anthropic API beta "context management"，**不改消息结构**、只清空旧 `ToolResult.content` 字段；损失最小，不触发 cache 重建。
  - `CompactionStrategy::Summarize`：触发一轮**独立模型调用**（按 `ModelRole::Compact` 路由，使用 W2 已落地的 `FallbackPolicy`）把前半段历史压缩成 summary turn；从 summary 起建立新 cache 前缀。
  - `Hybrid`（先 ClearToolResults，超过阈值再 Summarize）在 `CompactionStrategy` 枚举中留占位，W4 只加枚举变体 + `todo!()` stub，**真实实现延 W5/W6**（归属 Orchestrator 决策层）。

- **Permission 四态语义（对齐 `docs/sdk/06 §6.2` Octopus 首版子集）**：
  - `Default`：写/破坏性工具弹窗审批；只读工具放行。
  - `AcceptEdits`：文件写入自动允许；执行类（Bash）仍审批。
  - `BypassPermissions`：全部放行（典型：外层已沙箱化 / CI）。
  - `Plan`：**所有**有副作用工具 deny；只放行 `Read / Glob / Grep / WebFetch / WebSearch`。
  - 不落地：`dontAsk`（非交互场景占位留到 W6 Brain Loop 的 session config）、`auto`（分类器模式留到 Phase 2）、`bubble`（父子代理冒泡，留 W5 `subagent`）。

- **`canUseTool` 决策链（`docs/sdk/06 §6.4`）**：
  1. `alwaysDenyRules` 命中 → `Deny`（硬红线，不可被 hook 覆盖）。
  2. `alwaysAllowRules` 命中 → `Allow`。
  3. `tool.permission_hint(input, mode)`（W3 `Tool` trait 可选方法，W4 若确需新增，则追加到 `sdk-tools` 公共面并在 `02 §2.4` 登记）——**W4 决策：不新增**。`canUseTool` 只消费 `ToolRegistry::get(name).category` + 工具名自描述的 `is_write: bool`（由 `ToolCategory` 推导：`Write / Shell / Subagent` 视为写）。理由：新增 `permission_hint` 会在 `sdk-tools` 公共面裸增 trait 方法，牵动 15 个 builtin + W3 契约测试；`ToolCategory` 推导足以覆盖首版 4 个 mode 的决策。
  4. `mode == BypassPermissions` → `Allow`。
  5. `mode == Plan && is_write` → `Deny { reason: "plan mode blocks writes" }`。
  6. `alwaysAskRules` 命中 → `AskApproval { prompt }`。
  7. 默认：`is_write && mode == Default` → `AskApproval`；`is_write && mode == AcceptEdits && category == Write` → `Allow`；其余 → `Allow`。
  - 需要 OAuth / vendor key 握手的工具（未来的 MCP OAuth 场景）→ `RequireAuth { prompt }`；W4 只把变体加入枚举，真实 OAuth 流接线延 W6（Brain Loop 的 `SecretVault` 注入点）。

- **Sandbox 后端选择策略**：
  - `NoopBackend`：`execute` 直接 `tokio::process::Command` 在 `spec.fs_whitelist[0]` 下跑，不做隔离；**仅限单元测试与 dev macOS 默认**。
  - `SeatbeltBackend`：`sandbox-exec -f <profile> /bin/sh -c <cmd>`；profile 由 `SandboxSpec.fs_whitelist / env_allowlist / network_proxy` 生成；macOS 默认真实后端。
  - `BubblewrapBackend`：`bwrap --ro-bind / --bind <whitelist> --unshare-all --die-with-parent <cmd>`；seccomp profile 暂未接入（W8 增强）。
  - Windows：`SandboxBackend` 返回 `SandboxError::UnsupportedPlatform`，业务侧回退到 `NoopBackend`；真实 AppContainer/Job Object 留 `TODO(W8)`。
  - **凭据零暴露**（`00-overview.md §关键不变量 #2`）：`SandboxSpec.env_allowlist` 只允许 **白名单** 环境变量进入沙箱；`SecretVault` 凭据通过 Hook `PreToolUse` 的 rewrite 路径注入到工具 input，而非环境；合同测试扫描沙箱 `ps` 输出与事件日志无 `API_KEY / TOKEN / BEARER / Bearer`。

- **Hook 执行顺序（确定性）**：
  - 配置源优先级：`session > project > workspace > defaults`（与 Permissions 对齐，`docs/sdk/07 §7.6`）。
  - 同源内：`priority: i32`（默认 `100`），数字小的先执行。
  - 同 source + 同 priority：按 `Hook::name()` 字典序。
  - Hook 链：`Continue → 下一 Hook`；`Rewrite(payload) → 下一 Hook 消费新 payload`；`Abort(reason) → 短路 + 写事件`；`InjectMessage(msg) → 仅 UserPromptSubmit / Stop 两个事件允许，其它事件返回此变体视为 runtime error`。
  - `HookRunner::run()` 不自己做超时；超时由 W6 Brain Loop 包裹（`10s` 默认）。W4 只在 `HookRunner::run()` 签名里预留 `cancellation: CancellationToken` 通路。

- **MemoryBackend 最小面**：W4 **只** 落 `trait MemoryBackend`（Level 3）+ `DurableScratchpad`（`runtime/notes/<session>.md` 的读写封装）。SQLite / FTS 记忆后端归业务侧 `octopus-persistence`（W8）；MCP `memory` tool 归 W5 `sdk-plugin`。

## Scope

- In scope：
  - 新建 4 个 crate 骨架（`Cargo.toml` / `src/lib.rs` / `tests/`），同批更新顶层 `Cargo.toml` `members`。
  - `02 §2.1` 追加 Level 0 contracts 补丁（`PermissionOutcome::RequireAuth / HookEvent / HookDecision / EndReason / CompactionResult / MemoryItem / MemoryError`）。
  - `02 §2.6 / §2.7 / §2.8 / §2.9` 全部数据符号落地 + 本周签名回填。
  - `canUseTool` 决策链最小实现（`PermissionGate::check` 内部）；`ApprovalBroker` 通过 `EventSink` 发射 `AskPrompt` 并通过 `AskResolver` 等待回答。
  - `SandboxBackend` 三后端 + 跨平台选择函数 `default_backend_for_host()` + 集成测试（macOS: `seatbelt_smoke`；Linux: `bubblewrap_smoke`，CI 不必跑但测试文件存在）。
  - `HookRunner` 注册 / 运行 / 优先级排序 + 8 个 `HookEvent` 构造测试。
  - `SystemPromptBuilder::{new, with_section, build, fingerprint}` + 延续 W3 `ToolRegistry::schemas_sorted()` 的字节稳定性。
  - `Compactor::maybe_compact(session)` 返回 `Result<Option<CompactionResult>, CompactionError>`；`ClearToolResults` 策略单测；`Summarize` 策略接 W2 `ModelProvider`（用 `MockModelProvider` 注入）。
  - `DurableScratchpad` 对 `runtime/notes/<session>.md` 的 atomic write（临时文件 + rename）；`read / write` 一致性测试。
  - **凭据零暴露合同测试**（硬门禁）：跑一个包含 `API_KEY=xxx-secret` env 的完整 HookRunner + SandboxBackend + PermissionGate 流水线，扫描所有 emit 的 SessionEvent JSON 与沙箱 `stdout/stderr` 无 `xxx-secret` / `API_KEY=` / `Bearer` 字样。
  - **Prompt cache 稳定性守护**：`SystemPromptBuilder` 的 fingerprint 与 W3 `ToolRegistry::tools_fingerprint` 组合哈希，3 次构建字节一致。
  - `02 §5 契约差异清单` 追加 W4 新增条目：至少覆盖 `PermissionMode` 四态与现有 UI/runtime 三态映射的迁移路径（`dual-carry` 或 `align-openapi`，不得记 `no-op`）、`PermissionOutcome::RequireAuth`、`HookEvent`、`CompactionResult`。
  - `README.md §文档索引` 同批次切 `W4: pending → draft / in_progress`；Weekly Gate 通过后切 `done`。

- Out of scope：
  - **Brain Loop dispatch 流水线**（`docs/sdk/03 §3.5.1` 的 8 阶段）→ 归 W6 `octopus-sdk-core`。
  - **W4 不删除** `crates/runtime/src/{permissions.rs, permission_enforcer.rs, policy_engine.rs, hooks.rs, sandbox.rs, prompt.rs, compact.rs, summary_compression.rs}` 以及 `crates/plugins/src/hooks.rs` 与 `crates/octopus-runtime-adapter/src/{approval_broker.rs, approval_flow.rs, policy_compiler.rs, memory_runtime.rs, memory_selector.rs, memory_writer.rs}`。这些 **归 W7** 整体下线；W4 只保证"新 SDK 实现到位后 legacy 不再被 SDK 新代码引用"。
  - `MCP OAuth` 真实握手 → 归 W6 `SecretVault` 注入点；W4 只加 `PermissionOutcome::RequireAuth` 变体。
  - `CompactionStrategy::Hybrid` 真实实现 → 归 W5/W6 Orchestrator。
  - 业务侧 `LaneContext / LaneBlocker / GreenLevel / DiffScope / ReviewStatus / ReconcileReason` 等业务域治理概念（当前在 `runtime::permissions`）→ 归 `octopus-platform::governance`（W7 切换时由业务侧实现，不进 SDK）。
  - 真实 Bubblewrap seccomp profile / Windows AppContainer → 归 W8。
  - `MemoryBackend` 的 SQLite / FTS 实现 → 归 `octopus-persistence`（W8）。
  - `PluginLifecycle` 对 Hook 的注册 → 归 W5 `sdk-plugin`。
  - `auto` / `bubble` / `dontAsk` 三种 `PermissionMode` → 不在 W4 实现。
  - Git proxy / 网络代理凭据注入 / Egress Proxy JWT（`docs/sdk/06 §6.8–§6.9`）→ 归 W8 增强（本周 Sandbox 只做 FS + env allowlist 二层）。

## Risks Or Open Questions

| # | 风险 / 问题 | 决策建议 | 触发 Stop Condition |
|---|---|---|---|
| R1 | **Compaction 对 Prompt Cache 的破坏**：`Compactor::Summarize` 会触发一轮独立模型调用，把前半段历史压缩；若 summary 生成不稳定（同样输入不同输出），会让 cache 前缀在下一轮完全失效。 | `Summarize` 策略用 `ModelRole::Compact` 路由 + `temperature: 0` + 固定 system prompt；测试用 `MockModelProvider` 注入确定性输出，验证 `Compactor::maybe_compact` 连续跑 3 次 summary 字节一致。真实命中率守护留 W6 E2E。 | 命中率基线 < 80% → Stop #4 |
| R2 | **Permission `canUseTool` 决策链对 `sdk-tools` 的反向依赖**：如果 `PermissionGate` 需要查 `ToolCategory` 或 `is_concurrency_safe`，意味着 `permissions` 要 `use octopus_sdk_tools`，而 Level 2 同层禁止横向依赖。 | **公开面保持窄接口**：`PermissionGate::check(&ToolCallRequest)` 继续只接收 `call`，由 `DefaultPermissionGate` 内部持有 `category_resolver: Arc<dyn Fn(&str) -> ToolCategory + Send + Sync>`，先用 `call.name` 解析类别，再组装内部 `PermissionContext { call, mode, category }` 走 `PermissionPolicy::evaluate`。`ToolCategory` 仍按本周决策下沉到 `sdk-contracts`，但不把 `PermissionContext` 暴露成跨 crate trait 参数。 | 若 review 认为 `category_resolver` 仍泄漏 `sdk-tools` 语义 → Stop #2（层越权） |
| R3 | **Hook 执行顺序的源优先级语义**：Hook 来自 4 个 source，同源内还有 `priority` 字段；`docs/sdk/07 §7.6` 规定 `session > project > workspace > defaults` 但对"跨 source 时 priority 是否全局可比"未定。 | **明确为 source 优先级先于 priority**：跨 source 按 source 枚举顺序，同 source 内按 `priority` + `name`。这与 `docs/sdk/06 §6.3` 的权限规则合并语义对称。若未来插件 source（`docs/sdk/07 §7.11`）加入，需决定插件 priority 的归属 source——W4 决策：插件 hook 默认 source = `workspace`，可被 plugin manifest override 为 `project`；未来细化延 W5。 | 若 docs/sdk/07 与本文件规定冲突 → 写入 `docs/sdk/README.md ## Fact-Fix 勘误`，不改 07 本体 |
| R4 | **Sandbox 在 CI 无权启用 seatbelt / bwrap**：macOS CI `sandbox-exec` 需要 `sudo` 或特定 entitlement；Linux CI 容器内 `bwrap --unshare-user` 可能被 docker 默认安全策略拒绝。 | **测试分级**：`NoopBackend` 测试必跑（所有平台）；`SeatbeltBackend` / `BubblewrapBackend` 的**烟测**只在本地/专用 CI runner 跑，用 `#[cfg_attr(not(all(target_os = "macos", feature = "sandbox-smoke")), ignore)]` 标记。合同测试（凭据零暴露）用 `NoopBackend` + 显式 env 过滤即可覆盖。 | 若 CI 拒绝跑 smoke → 登记到 `00-overview.md §6 风险登记簿`，不阻断 Weekly Gate |
| R5 | **`DurableScratchpad` 并发写**：两个 Hook 同时写 `runtime/notes/<session>.md` 会丢失更新。 | atomic write：写入 `runtime/notes/<session>.md.tmp.<pid>.<ts>` → `fs::rename`（POSIX rename atomic）。Windows 用 `MoveFileEx`（`std::fs::rename` 在 Windows 会失败，需 fallback；W4 先 Linux/macOS，Windows TODO(W8)）。 | 若 Windows 测试环境失败 → 文件级 `std::sync::Mutex` 兜底 |
| R6 | **`CompactionResult` 在 Level 0 contracts 中能否只持有 `String` summary？** 未来若需要保留 "折叠的 turn IDs" 做审计，会破坏性扩字段。 | `CompactionResult` 字段：`summary: String / folded_turn_ids: Vec<EventId> / tool_results_cleared: u32 / tokens_before: u32 / tokens_after: u32 / strategy: CompactionStrategyTag`。一次性预留充分字段；后续追加为 append-only。 | 若字段过于宽，`sdk-contracts` Level 0 行数预算（100 行 lib.rs）吃紧 → 在本文件登记并评估子拆 `compaction.rs` 模块 |
| R7 | **`SystemPromptBuilder` 与 W3 `ToolRegistry::schemas_sorted()` 的边界**：`SystemPromptBuilder` 要不要把工具 schema 自己 JSON 序列化进 system prompt？ | **不要**。system prompt 只放"工具使用指南"文本段（human-readable），并且固定走 `ToolRegistry::schemas_sorted()` 的稳定顺序，只消费 `spec.name / spec.description`，不序列化 `input_schema`。schema 序列化由 `ModelProvider` 在 `ModelRequest.tools` 字段单独承载（W2 已落地）。这样 W3 `tools_fingerprint` 的字节稳定性只作用于 `ModelRequest.tools`，`SystemPromptBuilder::fingerprint` 单独计算"指南文本段"的哈希。 | 若 W6 E2E 发现 cache 失效 → 合并两份 fingerprint 为 `PromptCtx.fingerprint()` |
| R8 | **凭据泄漏合同测试的口径**：扫描什么字段算"明文凭据"？ | 白名单反向：扫描所有 `emit` 的 `SessionEvent` 的 JSON 序列化结果，断言不含 `API_KEY=` / `Bearer ` / `x-api-key` / `xxx-secret`（测试用预置 token）/ `Authorization:` / `-----BEGIN` / `sk-` 前缀。沙箱 stdout/stderr 同样扫描。**W4 明确规则**：审批与工具执行事件只能发射当前 contracts 已有的安全摘要事件（`SessionEvent::Ask` / `SessionEvent::ToolExecuted`），不得把原始 `ToolCallRequest.input` 或 bearer material 序列化进事件。 | 若 W4 需要新增会携带原始 input 的事件载荷 → Stop #5，先补独立 redaction contract 再实现 |
| R9 | **W3 反向回填 `ToolCategory` 下沉到 Level 0**：W3 Checkpoint 已锁在 `sdk-tools`，现在要把 `ToolCategory` 移到 `sdk-contracts`，是"修订 W3 公共面"。 | 在 `02 §2.4` 标注"W4 把 `ToolCategory` 下沉到 `§2.1`"，`sdk-tools` 通过 `pub use octopus_sdk_contracts::ToolCategory` re-export 保持源兼容；不破坏 W3 call-site。在 `02 §10 变更日志` 追加 `W4 反向修订` 条目；不走 Fact-Fix（这是计划文档间的一致性，不是规范层 vs 实现的矛盾）。 | 若下沉触发 `sdk-tools` 测试失败 → Stop #10 |
| R10 | **`MemoryItem` 字段**：为了让 Level 0 不做 I/O，`MemoryItem` 是否要含 `embedding: Vec<f32>`？ | **不含**。Level 0 `MemoryItem { id: String, kind: MemoryKind, payload: serde_json::Value, created_at_ms: i64 }`；embedding 是后端内部细节（SQLite/FTS/Vector），由 `MemoryBackend` 实现者自行管理。 | 若 W5/W6 发现 `MemoryItem` 字段不够 → 走 `02 §5` 登记而非直接加字段 |
| R11 | **Hook 对"即将发生"数据的修改语义**：`HookDecision::Rewrite(payload)` 的 `payload` 是 `serde_json::Value` 还是强类型？ | 强类型：每个 `HookEvent` 的 `Rewrite` 载荷形状固定 —— `PreToolUse → ToolCallRequest`；`PostToolUse → HookToolResult`；`UserPromptSubmit → Message`；`PreCompact → CompactionCtx`；其他事件（含 `PostCompact`）不允许 `Rewrite`，返回 `Rewrite` 视为 `HookError::RewriteNotAllowed`。契约通过 `enum RewritePayload` 表达。**不直接复用 `sdk-tools::ToolResult`**，避免 `contracts` 反向依赖 Level 2。 | 若后续确需让 `PostCompact` 可改写结果 → 追加独立 `RewritePayload::CompactionResult(CompactionResult)`，不复用 `CompactionCtx` |
| R12 | **四态权限与现有 UI/runtime 三态映射的割裂**：当前 `packages/schema` / runtime config 仍暴露 `auto / readonly / danger-full-access` 与 `read-only / workspace-write / danger-full-access`，W4 若直接切四态会让 adapter/UI 无法对齐。 | **本周必须显式登记兼容层**：在 `02 §5` 记录 `PermissionMode` 迁移项，至少说明 `Default ↔ readonly(read-only)`、`AcceptEdits ↔ auto(workspace-write)`、`BypassPermissions ↔ danger-full-access` 的暂时映射，以及 `Plan` 只在 SDK 内部生效、对外未公开前不得替换现有 UI/runtime 枚举。若要对外公开四态，先走 OpenAPI/schema 生成链。 | 若 Task 4/12 需要改现有 UI/runtime 枚举才能继续 → Stop #1，先开契约迁移专项 |

## 承 W3 / 启 W5-W6 的契约链

- **承 W3**：
  - `ToolContext.permissions: Arc<dyn PermissionGate>` / `ToolContext.ask_resolver: Arc<dyn AskResolver>` / `ToolContext.event_sink: Arc<dyn EventSink>` 在 W3 已预留注入槽；W4 的 `PermissionGate` / `ApprovalBroker` 真实实现直接满足。
  - `ToolContext.sandbox: SandboxHandle`（W3 占位值类型 `{ cwd, env_allowlist }`）升级为 `SandboxHandle { inner: Arc<dyn SandboxHandleInner> }`；旧字段保持可访问（`handle.cwd() / handle.env_allowlist()`），避免 W3 测试破坏。
  - `ToolRegistry::schemas_sorted()` / `ToolRegistry::tools_fingerprint()` 继续承担 cache 稳定性；`SystemPromptBuilder` 消费其 `spec.name / spec.description` 作为工具指南段输入。
- **启 W5**：
  - `SubagentSpec.permission_mode: PermissionMode` 在 W5 直接复用 W4 枚举；
  - `PluginLifecycle` 在 W5 注册 Hook 时必须经过 `HookRunner::register(name, hook, Source::Plugin, priority)`；
  - `SubagentSpec.allowed_tools` 过滤由 W4 `PermissionPolicy` 的 `alwaysAllowRules / alwaysDenyRules` 在子代理独立上下文内复用。
- **启 W6**：
  - Brain Loop 的 **8 阶段 dispatch 流水线**（`docs/sdk/03 §3.5.1`）严格按 `PreToolUse Hook → PermissionGate::check → partition_tool_calls → SandboxBackend::execute → Tool::execute → PostToolUse Hook → emit events → Compactor::maybe_compact` 切入；W4 的 trait 形状必须与这 8 个槽位一一对应。
  - `Compactor::maybe_compact` 的触发条件（token 阈值）在 W4 只暴露 `threshold: f32 (0.0..=1.0)` 配置，真实读取当前 session token 水位由 W6 Brain Loop 传入。

## 本周 `02 §2.1 / §2.4 / §2.6 / §2.7 / §2.8 / §2.9` 公共面修订清单（同批次回填）

> 以下修订必须在 Task 1 / Task 2 / Task 4 / Task 6 / Task 8 / Task 10 合入批次内**同 PR** 回填到 `02-crate-topology.md`，否则视为 `Stop Condition #1`（公共面裸增）。

### `02 §2.1 octopus-sdk-contracts`（W4 补丁 · Level 0 下沉）

| # | 位置 | 修订类型 | 内容 |
|---|---|---|---|
| 1 | `§2.1` `PermissionOutcome` | 变体新增 | 追加 `RequireAuth { prompt: AskPrompt }`，变体数量 `3 → 4` |
| 2 | `§2.1` `ToolCategory`（W3 在 `§2.4`）| 反向下沉 | 从 `§2.4` 迁到 `§2.1`；`sdk-tools` 保持 `pub use octopus_sdk_contracts::ToolCategory` 源兼容；登记到 `§2.4` "W4 反向修订" 小节 |
| 3 | `§2.1` 新增类型 | 类型新增 | `HookEvent { PreToolUse { call: ToolCallRequest, category: ToolCategory }, PostToolUse { call: ToolCallRequest, result: HookToolResult }, Stop { session: SessionId }, SessionStart { session: SessionId }, SessionEnd { session: SessionId, reason: EndReason }, UserPromptSubmit { message: Message }, PreCompact { session: SessionId, ctx: CompactionCtx }, PostCompact { session: SessionId, result: CompactionResult } }` |
| 4 | `§2.1` 新增类型 | 类型新增 | `EndReason { Normal, MaxTurns, UserCancelled, Error(String), Compaction }` |
| 5 | `§2.1` 新增类型 | 类型新增 | `HookDecision { Continue, Rewrite(RewritePayload), Abort { reason: String }, InjectMessage(Message) }` |
| 6 | `§2.1` 新增类型 | 类型新增 | `HookToolResult { content: Vec<ContentBlock>, is_error: bool, duration_ms: u64, render: Option<RenderBlock> }`、`RewritePayload { ToolCall(ToolCallRequest), ToolResult(HookToolResult), UserPrompt(Message), Compaction(CompactionCtx) }` |
| 7 | `§2.1` 新增类型 | 类型新增 | `CompactionCtx { session: SessionId, strategy: CompactionStrategyTag, threshold: f32, tokens_current: u32, tokens_budget: u32 }` |
| 8 | `§2.1` 新增类型 | 类型新增 | `CompactionResult { summary: String, folded_turn_ids: Vec<EventId>, tool_results_cleared: u32, tokens_before: u32, tokens_after: u32, strategy: CompactionStrategyTag }` |
| 9 | `§2.1` 新增类型 | 类型新增 | `CompactionStrategyTag { Summarize, ClearToolResults, Hybrid }` |
| 10 | `§2.1` 新增类型 | 类型新增 | `MemoryItem { id: String, kind: MemoryKind, payload: serde_json::Value, created_at_ms: i64 }`、`MemoryKind { Note, Decision, Todo, SkillLog, Custom(String) }`、`MemoryError { NotFound, Backend { reason: String }, Serialization(#[from] serde_json::Error) }` |

### `02 §2.6 octopus-sdk-context`（Level 3）

| # | 位置 | 修订类型 | 内容 |
|---|---|---|---|
| 11 | `§2.6` `SystemPromptBuilder` | 方法补齐 | `new / with_section / build / fingerprint`；`PromptCtx { session: SessionId, mode: PermissionMode, project_root: PathBuf, tools: &ToolRegistry }` |
| 12 | `§2.6` 新增类型 | 类型新增 | `SystemPromptSection { id: &'static str, order: u32, body: String }`（`order` 为段排序键；与 `docs/sdk/02 §2.3.2` XML 结构对齐，但以纯文本段承载） |
| 13 | `§2.6` `Compactor` | 方法补齐 | `new(threshold: f32, strategy: CompactionStrategyTag, provider: Arc<dyn ModelProvider>) -> Self` / `async fn maybe_compact(&self, session: &mut SessionView) -> Result<Option<CompactionResult>, CompactionError>` / `async fn clear_tool_results(&self, session: &mut SessionView) -> u32` / `async fn summarize(&self, session: &mut SessionView) -> Result<CompactionResult, CompactionError>` |
| 14 | `§2.6` 新增类型 | 类型新增 | `SessionView<'a> { messages: &'a mut Vec<Message>, tokens: u32 }`（W4 薄壳；真实消息游标由 W6 Brain Loop 传入） |
| 15 | `§2.6` 新增类型 | 类型新增 | `CompactionError { ModelUnavailable, SummaryTooLarge, Aborted, Provider(#[from] ModelError) }` |
| 16 | `§2.6` `MemoryBackend` trait | 方法保持 | `async fn recall / async fn commit`；方法返回 `Result<_, MemoryError>`（MemoryError 从 contracts re-export） |
| 17 | `§2.6` `DurableScratchpad` | 方法补齐 | `new(base: PathBuf) -> Self / async fn read(&self, session: &SessionId) -> Result<Option<String>, MemoryError> / async fn write(&self, session: &SessionId, content: &str) -> Result<(), MemoryError>`；路径为 `<base>/runtime/notes/<session_id>.md`，atomic rename |

### `02 §2.7 octopus-sdk-permissions`（Level 2）

| # | 位置 | 修订类型 | 内容 |
|---|---|---|---|
| 18 | `§2.7` `PermissionMode` | 保持 | 4 变体（与 `§2.1` `PermissionMode` 一致；`sdk-permissions` 通过 `pub use octopus_sdk_contracts::PermissionMode`） |
| 19 | `§2.7` `PermissionPolicy` | 方法补齐 | `new / from_sources / match_rules / evaluate(ctx: &PermissionContext) -> Option<PermissionOutcome>` |
| 20 | `§2.7` 新增类型 | 类型新增 | `PermissionRule { source: PermissionRuleSource, behavior: PermissionBehavior, tool_name: String, rule_content: Option<String> }`、`PermissionRuleSource { UserSettings, ProjectSettings, LocalSettings, FlagSettings, PolicySettings, CliArg, Command, Session }`、`PermissionBehavior { Allow, Deny, Ask }` |
| 21 | `§2.7` `PermissionGate` | 真实实现 | 替换 W3 浅 shim；`DefaultPermissionGate { policy: PermissionPolicy, mode: PermissionMode, broker: Arc<ApprovalBroker>, category_resolver: Arc<dyn Fn(&str) -> ToolCategory + Send + Sync> }`；`async fn check(&self, call: &ToolCallRequest) -> PermissionOutcome` 内部先解析 `ToolCategory` 再实现 `canUseTool` 决策链（7 步） |
| 22 | `§2.7` 新增类型 | 类型新增 | `PermissionContext { call: ToolCallRequest, mode: PermissionMode, category: ToolCategory }`（内部结构；不跨 crate） |
| 23 | `§2.7` `ApprovalBroker` | 方法补齐 | `new(event_sink: Arc<dyn EventSink>, ask_resolver: Arc<dyn AskResolver>) -> Self / async fn request_approval(&self, call: &ToolCallRequest, prompt: AskPrompt) -> PermissionOutcome`（内部生成稳定 `prompt_id = format!("approval:{}", call.id)`；`emit(SessionEvent::Ask { prompt: prompt.clone() })`；`ask_resolver.resolve(&prompt_id, &prompt).await`；prompt 文案只含安全摘要，不含原始 input） |

### `02 §2.8 octopus-sdk-sandbox`（Level 2）

| # | 位置 | 修订类型 | 内容 |
|---|---|---|---|
| 24 | `§2.8` `SandboxBackend` trait | 方法保持 | `async fn provision / execute / terminate`；`SandboxError` 追加 `UnsupportedPlatform` 变体 |
| 25 | `§2.8` `SandboxSpec` | 字段保持 | `fs_whitelist / network_proxy / env_allowlist`；新增 `cpu_time_limit_ms: Option<u64>` / `wall_time_limit_ms: Option<u64>` / `memory_limit_bytes: Option<u64>`（与 `docs/sdk/06 §6.10` 资源限额表对齐；首版只传递，不强制执行） |
| 26 | `§2.8` 新增类型 | 类型新增 | `SandboxCommand { cmd: String, args: Vec<String>, stdin: Option<Vec<u8>> }`、`SandboxOutput { exit_code: i32, stdout: Vec<u8>, stderr: Vec<u8>, truncated: bool, timed_out: bool }` |
| 27 | `§2.8` `SandboxHandle` | 类型升级 | 从 W3 `{ cwd, env_allowlist }` 值类型升级为 `SandboxHandle { inner: Arc<dyn SandboxHandleInner + Send + Sync> }`；`trait SandboxHandleInner { fn cwd(&self) -> &Path; fn env_allowlist(&self) -> &[String]; fn backend_name(&self) -> &'static str; }`；W3 字段通过方法保持源兼容 |
| 28 | `§2.8` 新增符号 | 结构新增 | `NoopBackend`、`SeatbeltBackend`（macOS，`#[cfg(target_os = "macos")]`）、`BubblewrapBackend`（Linux，`#[cfg(target_os = "linux")]`） |
| 29 | `§2.8` 新增函数 | 函数新增 | `pub fn default_backend_for_host() -> Arc<dyn SandboxBackend>`（运行时根据 `cfg!(target_os)` 选择；Windows 返回 `NoopBackend` + tracing warn） |
| 30 | `§2.8` `SandboxError` | 变体补齐 | `Provision { reason: String } / Execute { reason: String } / Terminate { reason: String } / UnsupportedPlatform / ResourceExhausted { kind: String } / Timeout` |

### `02 §2.9 octopus-sdk-hooks`（Level 2）

| # | 位置 | 修订类型 | 内容 |
|---|---|---|---|
| 31 | `§2.9` `Hook` trait | 方法保持 | `fn name(&self) -> &str` / `async fn on_event(&self, event: &HookEvent) -> HookDecision` |
| 32 | `§2.9` 新增类型 | 类型新增 | `HookRegistration { hook: Arc<dyn Hook>, source: HookSource, priority: i32 }`、`HookSource { Session, Project, Workspace, Defaults, Plugin { plugin_id: String } }` |
| 33 | `§2.9` `HookRunner` | 方法补齐 | `new / register(name: &str, hook: Arc<dyn Hook>, source: HookSource, priority: i32) / unregister_by_source / async fn run(&self, event: HookEvent) -> Result<HookRunOutcome, HookError>` |
| 34 | `§2.9` 新增类型 | 类型新增 | `HookRunOutcome { decisions: Vec<(String, HookDecision)>, final_payload: Option<RewritePayload>, aborted: Option<String> }` |
| 35 | `§2.9` 新增类型 | 类型新增 | `HookError { RewriteNotAllowed { event_kind: &'static str }, InjectNotAllowed { event_kind: &'static str }, HookPanic { name: String }, Serialization(#[from] serde_json::Error) }` |
| 36 | `§2.9` 执行顺序契约 | 文本登记 | `source` 枚举顺序（`Plugin < Workspace < Defaults < Project < Session`，数值小的先执行）；同 source 内按 `priority` 升序；再按 `name` 字典序。**本登记为本文件与 `docs/sdk/07 §7.6` 的一致性锚点**；若未来规范变更需走 Fact-Fix。 |

## Task Ledger

> Task 粒度：10 个原子 Task。严格按依赖链执行；前置 Task `Done when` 未勾选 → 后置 Task 禁止开工。

### Task 1: Contracts Level 0 补丁（W4 前置）

Status: `pending`

Files:
- Modify: `crates/octopus-sdk-contracts/src/lib.rs`
- Modify: `crates/octopus-sdk-contracts/src/permissions.rs`（新 mod，从 W3 `permissions.rs` 拆出 / 或在 `lib.rs` 内拓展）
- Create: `crates/octopus-sdk-contracts/src/hooks.rs`
- Create: `crates/octopus-sdk-contracts/src/compaction.rs`
- Create: `crates/octopus-sdk-contracts/src/memory.rs`
- Create: `crates/octopus-sdk-contracts/tests/w4_contracts.rs`
- Modify: `docs/plans/sdk/02-crate-topology.md`（回填 §2.1 第 1–10 项）

Preconditions:
- W3 `octopus-sdk-contracts` 公共面稳定（W3 Weekly Gate 已通过）。
- `02 §2.1 §9 行数预算`：`sdk-contracts` `lib.rs` 100 行上限；本 Task 将增加约 30–50 行；拆子模块（`permissions / hooks / compaction / memory`）后 `lib.rs` 仅作 `pub use` re-export，保持 < 100。

Step 1:
- Action: 拆 `permissions` 子模块，追加 `PermissionOutcome::RequireAuth { prompt: AskPrompt }` 变体；把 W3 已定义的 `ToolCallRequest / PermissionMode / PermissionOutcome / PermissionGate / AskResolver / AskAnswer / AskError / EventSink` 迁到 `permissions.rs`；`lib.rs` `pub use` re-export 维持源兼容。
- Done when: `cargo check -p octopus-sdk-contracts` 通过；`rg 'pub enum PermissionOutcome' crates/octopus-sdk-contracts/src` 命中 1 处且含 4 变体。
- Verify: `cargo test -p octopus-sdk-contracts`（覆盖当前 `serialization_golden` 与模块内既有测试）依然 pass（源兼容验证）。
- Stop if: W3 测试失败（说明拆模块破坏了 re-export）→ 回滚到单文件。

Step 2:
- Action: 新增 `hooks.rs`，定义 `HookToolResult` / `HookEvent`（8 变体）/ `HookDecision` / `RewritePayload` / `EndReason` / `CompactionCtx`；其中 `CompactionCtx` 跨文件引用 `CompactionStrategyTag`（见 Step 3）。
- Done when: 所有类型 `Serialize + Deserialize + Debug + Clone`；`HookEvent::variant_count() == 8`（自写 const fn 或单测枚举）。
- Verify: `cargo test -p octopus-sdk-contracts --test w4_contracts` 含 "HookEvent 8 variants 稳定" 断言，pass。
- Stop if: `CompactionCtx` 字段引用形成 `hooks → compaction → hooks` 循环 → 把共享字段提到 `lib.rs` 或第三方 mod。

Step 3:
- Action: 新增 `compaction.rs`（`CompactionStrategyTag / CompactionResult`）与 `memory.rs`（`MemoryItem / MemoryKind / MemoryError`）。`CompactionResult.folded_turn_ids: Vec<EventId>` 引用现有 `EventId` 类型。
- Done when: 新类型全部 `Serialize + Deserialize`；`MemoryError::Serialization` 自动派生 `#[from] serde_json::Error`。
- Verify: `cargo test -p octopus-sdk-contracts -- --include-ignored` 全绿；`cargo clippy -p octopus-sdk-contracts -- -D warnings` 通过。
- Stop if: Level 0 新增依赖（除已有 `serde / serde_json / thiserror / uuid`）→ Stop #1 违反 Level 0 规则。

Step 4:
- Action: 反向下沉 `ToolCategory` 从 `sdk-tools::lib.rs` 迁到 `sdk-contracts::tools.rs`；`sdk-tools` 通过 `pub use octopus_sdk_contracts::ToolCategory` re-export。
- Done when: `cargo build -p octopus-sdk-tools -p octopus-sdk-mcp -p octopus-sdk-session -p octopus-sdk-model` 全绿；W3 `tools/registry_stability.rs` 测试 pass。
- Verify: `cargo test -p octopus-sdk-tools --test registry_stability --test partition_concurrency`；`rg 'pub enum ToolCategory' crates/octopus-sdk-tools` 无命中（已迁出）；`rg 'pub use octopus_sdk_contracts::ToolCategory' crates/octopus-sdk-tools/src` 命中 1 处。
- Stop if: W3 call-site 出现 `ToolCategory` 字段访问顺序依赖（enum discriminant 变化）→ 固定枚举顺序 + 补 `#[repr(u8)]`。

Step 5:
- Action: 回填 `02 §2.1 W4 补丁清单` 第 1–10 项（一次性完整粘贴表格内容）；在 `02 §2.4` 追加 "W4 反向修订：`ToolCategory` 下沉到 §2.1，`sdk-tools` 以 re-export 保持源兼容" 注记。
- Done when: `02-crate-topology.md` 同 PR diff 含上述条目；`docs/plans/sdk/02-crate-topology.md §10` 变更日志追加 "2026-04-xx · W4 Task 1：Level 0 contracts 补丁 + `ToolCategory` 下沉"。
- Verify: 阅读 diff 确认无遗漏；跑 `grep -n 'W4' docs/plans/sdk/02-crate-topology.md | wc -l` ≥ 11（10 行条目 + 1 行日志）。
- Stop if: 发现 `§2.1` 原本无 W4 占位小节 → 在 §2.1 顶部加 "**本周公共面修订清单（W4）**" 子节（与 W3 对称）。

Notes:
- 这是 W4 启动任务；其余所有 Task 都依赖本 Task 的 Level 0 补丁存在。

---

### Task 2: `octopus-sdk-permissions` 骨架 + `PermissionMode` 语义

Status: `pending`

Files:
- Create: `crates/octopus-sdk-permissions/Cargo.toml`
- Create: `crates/octopus-sdk-permissions/src/lib.rs`
- Create: `crates/octopus-sdk-permissions/src/policy.rs`
- Create: `crates/octopus-sdk-permissions/src/mode.rs`（薄 re-export + 文档）
- Create: `crates/octopus-sdk-permissions/tests/mode_semantics.rs`
- Modify: `Cargo.toml`（workspace members 追加）
- Modify: `docs/plans/sdk/02-crate-topology.md`（回填 §2.7 第 18–19 项）

Preconditions:
- Task 1 完成（`PermissionOutcome::RequireAuth` / `ToolCategory` 已在 Level 0）。

Step 1:
- Action: 创建 crate 骨架；`Cargo.toml` 仅依赖 `octopus-sdk-contracts / tokio / async-trait / serde / serde_json / thiserror / tracing`；`lib.rs` 声明 `pub mod policy; pub mod mode; pub mod gate; pub mod broker;`（后三 mod 由 Task 3-4 填充）。
- Done when: `cargo build -p octopus-sdk-permissions` 通过；`cargo metadata` 能识别新 crate。
- Verify: `rg 'octopus-sdk-permissions' Cargo.toml` 命中 1 次；`cargo check --workspace` 通过。
- Stop if: `Cargo.toml` 意外引入 `octopus-sdk-tools / octopus-sdk-sandbox / octopus-sdk-hooks` → Stop #2（同层反向依赖）。

Step 2:
- Action: 在 `mode.rs` 写 `pub use octopus_sdk_contracts::PermissionMode;` + 模式语义文档注释（Rustdoc，引用 `docs/sdk/06 §6.2` 链接）。
- Done when: `cargo doc -p octopus-sdk-permissions --no-deps` 生成含 `PermissionMode` 章节。
- Verify: `cargo doc -p octopus-sdk-permissions --no-deps 2>&1 | grep -i 'warning: missing documentation'` 为空。
- Stop if: rustdoc 警告 → 补全文档后再推进。

Step 3:
- Action: `tests/mode_semantics.rs` 覆盖 4 个 mode × 4 类工具（`Read/Write/Shell/Subagent`）= 16 组决策期望的 snapshot（不依赖真实 `PermissionGate`，只验证 `PermissionMode` + `ToolCategory` 组合的**期望决策**表格在编译期固定，以便 Task 4 `canUseTool` 对照）。
- Done when: 16 组断言 pass；表格作为 `const EXPECTED_DECISIONS: [(PermissionMode, ToolCategory, &'static str); 16]`。
- Verify: `cargo test -p octopus-sdk-permissions --test mode_semantics`。
- Stop if: 表格期望与 `docs/sdk/06 §6.4` 决策链冲突 → 先改文档（Fact-Fix），不改测试期望。

Notes:
- 本 Task 不触碰 `PermissionGate` / `PermissionPolicy` 的具体实现；只产出"骨架 + 契约 + 决策期望表"。

---

### Task 3: `octopus-sdk-permissions::PermissionPolicy`（规则 + source 合并）

Status: `pending`

Files:
- Modify: `crates/octopus-sdk-permissions/src/policy.rs`
- Create: `crates/octopus-sdk-permissions/tests/policy_merge.rs`
- Modify: `docs/plans/sdk/02-crate-topology.md`（回填 §2.7 第 19–20 项）

Preconditions:
- Task 2 完成。

Step 1:
- Action: 定义 `PermissionRule / PermissionRuleSource (8 变体) / PermissionBehavior (Allow/Deny/Ask)`；`PermissionPolicy::new() / from_sources(Vec<PermissionRule>) / match_rules(call: &ToolCallRequest) -> (Vec<&PermissionRule>, Vec<&PermissionRule>, Vec<&PermissionRule>)`（三个 vec 分别是 allow/deny/ask 命中规则）。
- Done when: `from_sources` 内部按 source 优先级排序；`match_rules` 的返回按规则匹配先后稳定。
- Verify: `cargo test -p octopus-sdk-permissions --test policy_merge -- test_source_priority_ordering` pass。
- Stop if: `from_sources` 需引用 `tokio::sync::Mutex` 做热刷新 → 留作内部细节，不暴露 pub。

Step 2:
- Action: 实现 `evaluate(ctx: &PermissionContext) -> Option<PermissionOutcome>`（纯函数，不调用 `AskResolver` / `EventSink`；后者由 `PermissionGate::check` 的 Task 4 完成）；`evaluate` 只处理 Step 1 的"规则命中"部分。
- Done when: `evaluate` 对 `alwaysDeny` 命中返回 `Some(Deny)`；对 `alwaysAllow` 命中返回 `Some(Allow)`；对 `alwaysAsk` 命中返回 `Some(AskApproval(placeholder_prompt))`；未命中返回 `None`（由 Task 4 继续走 mode 默认链）。
- Verify: `cargo test -p octopus-sdk-permissions --test policy_merge` 覆盖 deny/allow/ask/未命中 4 条路径。
- Stop if: `evaluate` 需要异步 → 说明设计漏了"规则匹配应纯同步"。

Step 3:
- Action: 回填 `02 §2.7` 条目 18–20（含 `PermissionRule / PermissionRuleSource / PermissionBehavior`）。
- Done when: `02-crate-topology.md` 同 PR 回填；`rg 'PermissionRuleSource' docs/plans/sdk/02-crate-topology.md` ≥ 2 处。
- Verify: diff 审读。
- Stop if: `§2.7` 已被 W3 或先前 PR 锁定 → 走 §10 变更日志追加而非直接改文本。

---

### Task 4: `octopus-sdk-permissions::PermissionGate` 真实实现（`canUseTool` 决策链）

Status: `pending`

Files:
- Create: `crates/octopus-sdk-permissions/src/gate.rs`
- Create: `crates/octopus-sdk-permissions/src/broker.rs`
- Create: `crates/octopus-sdk-permissions/tests/can_use_tool_chain.rs`
- Create: `crates/octopus-sdk-permissions/tests/approval_flow.rs`
- Modify: `docs/plans/sdk/02-crate-topology.md`（回填 §2.7 第 21–23 项）

Preconditions:
- Task 3 完成（`PermissionPolicy::evaluate` 可用）。

Step 1:
- Action: `DefaultPermissionGate { policy, mode, broker, category_resolver }`；`category_resolver: Arc<dyn Fn(&str) -> ToolCategory + Send + Sync>` 由调用方（W6 Brain Loop）注入，用于把 `ToolCallRequest.name` 解析为 `ToolCategory`（避免 `permissions → tools` 横向依赖）。
- Done when: `impl PermissionGate for DefaultPermissionGate` 覆盖 `check`。
- Verify: `cargo check -p octopus-sdk-permissions` 通过。
- Stop if: `category_resolver` 设计发现泄漏 `Tool` trait → 改成 `ToolCategory` 值传入（继续封闭 `sdk-tools` 边界）。

Step 2:
- Action: 在 `check` 内实现 7 步 `canUseTool` 决策链（见 Architecture）；`RequireAuth` 暂时走 `AskApproval`（`prompt.kind = "require_auth"`），W6 Brain Loop 接入 `SecretVault` 后再回补。
- Done when: `tests/can_use_tool_chain.rs` 覆盖 7 步的每一步分支（至少 12 条断言：4 mode × 3 类规则命中 + 兜底）。
- Verify: `cargo test -p octopus-sdk-permissions --test can_use_tool_chain`。
- Stop if: 某步需要读取工具 input 的具体字段（例如 Bash 命令前缀）→ 登记到 `02 §5` "W4 Bash 命令级审批延迟到 W8 增强"。

Step 3:
- Action: `ApprovalBroker { event_sink: Arc<dyn EventSink>, ask_resolver: Arc<dyn AskResolver> }`；`request_approval(call, prompt)` 内部：(1) 生成 `prompt_id = format!("approval:{}", call.id)`；(2) `event_sink.emit(SessionEvent::Ask { prompt: prompt.clone() })`；(3) `ask_resolver.resolve(&prompt_id, &prompt).await`；(4) 根据 `AskAnswer.option_id` 映射回 `PermissionOutcome::Allow / Deny`。`AskPrompt` 文案只允许携带 tool name / category / cwd 等安全摘要，不得带原始 `input`。
- Done when: `tests/approval_flow.rs` 用 `MockEventSink + MockAskResolver` 跑 "deny → AskPrompt → approve → Allow" 全路径（`01-ai-execution-protocol.md §3` 特殊校验钩子要求）。
- Verify: `cargo test -p octopus-sdk-permissions --test approval_flow`。
- Stop if: `AskResolver::resolve` 的 `AskAnswer` 形状需扩字段 → 走 `02 §2.1` 补丁而非本 Task 直接改。

Step 4:
- Action: 回填 `02 §2.7` 条目 21–23；追加 Rustdoc 指向 `docs/sdk/06 §6.4`。
- Done when: 同 PR 完成。
- Verify: 同 Task 3 Step 3。

---

### Task 5: `octopus-sdk-sandbox` 骨架 + `SandboxSpec/Handle` 升级

Status: `pending`

Files:
- Create: `crates/octopus-sdk-sandbox/Cargo.toml`
- Create: `crates/octopus-sdk-sandbox/src/lib.rs`
- Create: `crates/octopus-sdk-sandbox/src/spec.rs`
- Create: `crates/octopus-sdk-sandbox/src/handle.rs`
- Create: `crates/octopus-sdk-sandbox/tests/spec_handle.rs`
- Modify: `crates/octopus-sdk-tools/src/context.rs`（把 `SandboxHandle` 类型从值类型升级为 trait object 封装；通过 `pub use octopus_sdk_sandbox::SandboxHandle` 替换当前定义）
- Modify: `Cargo.toml`（workspace members 追加）
- Modify: `docs/plans/sdk/02-crate-topology.md`（回填 §2.8 第 24–30 项）

Preconditions:
- Task 1 完成。

Step 1:
- Action: 创建 `octopus-sdk-sandbox` crate；依赖 `tokio / async-trait / thiserror / tracing / serde`；不依赖 `sdk-contracts / sdk-permissions / sdk-hooks`（Sandbox 是纯 OS 原语层，不引 Level 0 以外的 SDK 符号，避免不必要耦合）。
- Done when: `cargo build -p octopus-sdk-sandbox` 通过。
- Verify: `rg '^octopus-sdk-sandbox' Cargo.toml` 命中。
- Stop if: `Cargo.toml` 出现 `octopus-sdk-contracts` 之外的 SDK 依赖 → 回滚。

Step 2:
- Action: 定义 `SandboxSpec / SandboxCommand / SandboxOutput / SandboxError`（含 `UnsupportedPlatform`）；`SandboxHandle { inner: Arc<dyn SandboxHandleInner> }` + `trait SandboxHandleInner { fn cwd() -> &Path; fn env_allowlist() -> &[String]; fn backend_name() -> &'static str; }`。
- Done when: `cargo test -p octopus-sdk-sandbox --test spec_handle` 覆盖 `SandboxHandle` 的方法访问（替换 W3 值类型后源兼容）。
- Verify: `cargo test -p octopus-sdk-sandbox`；`cargo test -p octopus-sdk-tools --test registry_stability --test partition_concurrency`（W3 测试依然通过，证明升级源兼容）。
- Stop if: W3 `ToolContext.sandbox` 字段访问失败 → 检查 `sdk-tools` 对 `SandboxHandle` 的使用路径。

Step 3:
- Action: `sdk-tools` 通过 `pub use octopus_sdk_sandbox::SandboxHandle` 替换原先的值类型定义；`ToolContext.sandbox` 字段类型不变（外部看名字不变）。
- Done when: `cargo build -p octopus-sdk-tools -p octopus-sdk-mcp` 通过；W3 15 个 builtin tool 编译通过。
- Verify: `cargo test -p octopus-sdk-tools --all-targets`。
- Stop if: W3 `BashTool::execute` 的 cwd/env 访问方式变化过大 → 在 `SandboxHandle` 上暴露旧字段的 getter（`cwd() / env_allowlist()`）保持 API 稳定。

Step 4:
- Action: 回填 `02 §2.8` 条目 24–30。
- Done when: 同 PR 完成。
- Verify: diff 审读。

---

### Task 6: `octopus-sdk-sandbox` 三后端实现

Status: `pending`

Files:
- Create: `crates/octopus-sdk-sandbox/src/backend/mod.rs`
- Create: `crates/octopus-sdk-sandbox/src/backend/noop.rs`
- Create: `crates/octopus-sdk-sandbox/src/backend/seatbelt.rs`（`#[cfg(target_os = "macos")]`）
- Create: `crates/octopus-sdk-sandbox/src/backend/bubblewrap.rs`（`#[cfg(target_os = "linux")]`）
- Create: `crates/octopus-sdk-sandbox/tests/noop_smoke.rs`
- Create: `crates/octopus-sdk-sandbox/tests/seatbelt_smoke.rs`（`#[cfg_attr(not(all(target_os = "macos", feature = "sandbox-smoke")), ignore)]`）
- Create: `crates/octopus-sdk-sandbox/tests/bubblewrap_smoke.rs`（`#[cfg_attr(not(all(target_os = "linux", feature = "sandbox-smoke")), ignore)]`）
- Create: `crates/octopus-sdk-sandbox/tests/no_credentials_leak.rs`

Preconditions:
- Task 5 完成。

Step 1:
- Action: `NoopBackend` 实现：`provision` 直接构造 `SandboxHandle`（inner 持 `cwd` / `env_allowlist`）；`execute` 用 `tokio::process::Command::new(cmd).args(args).env_clear().envs(&env_allowlist).current_dir(cwd).output()`；`terminate` no-op。
- Done when: `noop_smoke.rs` 跑 `echo hello` 返回 `exit_code=0, stdout="hello\n"`。
- Verify: `cargo test -p octopus-sdk-sandbox --test noop_smoke`。
- Stop if: `env_clear() + envs()` 在某些子 shell 下行为不一致 → 固定测试命令为 `/bin/sh -c 'echo $PATH'` 断言只见 `env_allowlist` 里的 `PATH`。

Step 2:
- Action: `SeatbeltBackend::provision` 生成 seatbelt profile（`(version 1) (deny default) (allow file-read* ...) (allow network-outbound ...)`）写入 temp 文件；`execute` 用 `sandbox-exec -f <profile> /bin/sh -c <cmd>`。
- Done when: profile 模板字符串化（`const SEATBELT_PROFILE_TEMPLATE: &str = ...`）；生成过程可单测。
- Verify: `cargo test -p octopus-sdk-sandbox --test seatbelt_smoke -- --ignored`（本地 macOS 运行）。
- Stop if: `sandbox-exec` macOS 12+ deprecated 告警 → 接受 warning，tracing 记录；Apple 未来删除时走 Fact-Fix。

Step 3:
- Action: `BubblewrapBackend::provision` 生成 `bwrap --ro-bind / --bind <whitelist> --unshare-all --die-with-parent --new-session` 参数；`execute` 用 `bwrap ... -- /bin/sh -c <cmd>`。
- Done when: 参数序列生成可单测；`bubblewrap_smoke.rs` 在 Linux CI 专用 runner 可跑（非必需）。
- Verify: `cargo test -p octopus-sdk-sandbox --test bubblewrap_smoke -- --ignored`。
- Stop if: Linux docker 默认 profile 拒绝 `--unshare-user` → 在 `SandboxError::Provision` 里提示 "CI 需启用 `--privileged` 或使用 sysbox"，不强制在默认 CI 跑。

Step 4:
- Action: `default_backend_for_host() -> Arc<dyn SandboxBackend>`：macOS → `SeatbeltBackend`；Linux → `BubblewrapBackend`；Windows → `NoopBackend` + `tracing::warn!("sandbox fallback to Noop; Windows support TODO(W8)")`。
- Done when: 跨平台编译通过；函数单测覆盖。
- Verify: `cargo build --target x86_64-unknown-linux-gnu -p octopus-sdk-sandbox`（若 CI 支持交叉编译）；否则本地 `cargo build -p octopus-sdk-sandbox`。
- Stop if: macOS + Linux cfg 分叉导致 `default_backend_for_host` 返回类型歧义 → 统一返回 `Arc<dyn SandboxBackend>`。

Step 5:
- Action: `no_credentials_leak.rs` 合同测试：用 `NoopBackend + env=["API_KEY=secret-xyz"]` + `env_allowlist=["PATH"]` 跑一条 `env | grep API_KEY` 命令；断言 `stdout` 不含 `secret-xyz`（因 `env_clear` 过滤）；同时序列化 `SandboxOutput` 到 JSON，扫描 `"API_KEY"` 无命中。
- Done when: 合同测试 pass。
- Verify: `cargo test -p octopus-sdk-sandbox --test no_credentials_leak`。
- Stop if: `NoopBackend` 没过滤 env → 修实现。

Step 6:
- Action: 回填 `02 §2.8` 条目 28–30（已在 Task 5 预登，本步补具体实现的确认）；在 `02 §5` 追加 "Sandbox Windows AppContainer 延 W8"。
- Done when: 同 PR 完成。

---

### Task 7: `octopus-sdk-hooks` 骨架 + `HookRunner` 优先级

Status: `pending`

Files:
- Create: `crates/octopus-sdk-hooks/Cargo.toml`
- Create: `crates/octopus-sdk-hooks/src/lib.rs`
- Create: `crates/octopus-sdk-hooks/src/runner.rs`
- Create: `crates/octopus-sdk-hooks/tests/priority.rs`
- Modify: `Cargo.toml`（workspace members 追加）
- Modify: `docs/plans/sdk/02-crate-topology.md`（回填 §2.9 第 31–36 项）

Preconditions:
- Task 1 完成（`HookEvent / HookDecision / RewritePayload` 已在 Level 0）。

Step 1:
- Action: 创建 crate 骨架；仅依赖 `octopus-sdk-contracts / tokio / async-trait / tracing / thiserror`；`lib.rs` 声明 `pub mod runner;` + `pub use octopus_sdk_contracts::{HookEvent, HookDecision, RewritePayload};`。
- Done when: `cargo build -p octopus-sdk-hooks` 通过。
- Verify: `rg '^octopus-sdk-hooks' Cargo.toml` 命中。
- Stop if: `Cargo.toml` 引入 `sdk-permissions / sdk-sandbox / sdk-tools` → Stop #2。

Step 2:
- Action: 定义 `trait Hook { fn name(&self) -> &str; async fn on_event(&self, event: &HookEvent) -> HookDecision; }` + `HookRegistration / HookSource / HookRunOutcome / HookError`；`HookRunner::new() / register / unregister_by_source / run`。
- Done when: `HookSource` 变体顺序 = `[Plugin, Workspace, Defaults, Project, Session]`（数字小的先执行；对应"自底向上叠加、session 最后覆盖"的语义）。
- Verify: `cargo test -p octopus-sdk-hooks --test priority -- test_source_ordering` pass。
- Stop if: 顺序与 `docs/sdk/07 §7.6` 的 "session > project > workspace > defaults" 描述冲突 → 写 Fact-Fix（两者口径不同：§7.6 是"谁胜出"，本处是"谁先执行"；**最后执行的胜出**，因此 `Session` 在最后）。

Step 3:
- Action: `HookRunner::run` 实现：排序（source → priority → name）→ 依次 `await hook.on_event(event)` → `match decision { Continue: next; Rewrite(p): ctx.payload = p, next; Abort(r): break Err; InjectMessage(m): 仅 UserPromptSubmit/Stop 事件允许 }`。
- Done when: `tests/priority.rs` 覆盖 `Continue / Rewrite chain / Abort short-circuit / InjectMessage only on allowed events` 4 条路径。
- Verify: `cargo test -p octopus-sdk-hooks --test priority`。
- Stop if: `Rewrite` 的 `RewritePayload` 变体与 `HookEvent` 变体不匹配 → `HookError::RewriteNotAllowed { event_kind }` 返回；单测覆盖。

Step 4:
- Action: 回填 `02 §2.9` 条目 31–36。
- Done when: 同 PR 完成。

---

### Task 8: `octopus-sdk-hooks` 8 种 `HookEvent` 的契约覆盖

Status: `pending`

Files:
- Create: `crates/octopus-sdk-hooks/tests/events_matrix.rs`
- Create: `crates/octopus-sdk-hooks/tests/credential_scrub.rs`
- Modify: `crates/octopus-sdk-hooks/src/runner.rs`（补充 `InjectMessage` 事件过滤）

Preconditions:
- Task 7 完成。

Step 1:
- Action: `events_matrix.rs` 枚举 8 种 `HookEvent`，对每种验证：(1) `HookRunner::run` 不 panic；(2) `Continue` 可达；(3) `Abort` 生效；(4) `Rewrite` 的载荷 enum 变体与事件种类强绑定；(5) `InjectMessage` 仅允许 `UserPromptSubmit / Stop`，其余返回 `HookError::InjectNotAllowed`。
- Done when: 8 × 5 = 40 条断言 pass。
- Verify: `cargo test -p octopus-sdk-hooks --test events_matrix`。
- Stop if: `RewritePayload` 对 `PreCompact` 的形状不明 → 先固化 `CompactionCtx` 为 Rewrite 目标，W4 Compactor 读取 `CompactionCtx.threshold`。

Step 2:
- Action: `credential_scrub.rs` 合同测试：注册一个 "故意泄漏凭据" 的恶意 Hook（在 `PreToolUse.Rewrite` 里把 `API_KEY=xxx-secret` 塞进 `ToolCallRequest.input`），然后只发射当前 contracts 已有的安全摘要事件（`SessionEvent::Ask` / `SessionEvent::ToolExecuted`），扫描事件 JSON 与沙箱 `stdout/stderr`，断言 `xxx-secret` 不出现。该测试验证的是"事件构造边界不序列化原始 input"，不是 HookRunner 主动清洗 rewrite payload。
- Done when: 审批分支与执行分支都覆盖；两条路径都证明当前事件面不会把 rewrite 后的原始凭据带出进 JSON。
- Verify: `cargo test -p octopus-sdk-hooks --test credential_scrub`。
- Stop if: 测试需要新增能直接序列化原始 `ToolCallRequest.input` 的 `SessionEvent` 变体 → 登记 R8 阻塞，先补 redaction contract 再继续。

---

### Task 9: `octopus-sdk-context::SystemPromptBuilder`

Status: `pending`

Files:
- Create: `crates/octopus-sdk-context/Cargo.toml`
- Create: `crates/octopus-sdk-context/src/lib.rs`
- Create: `crates/octopus-sdk-context/src/compact.rs`
- Create: `crates/octopus-sdk-context/src/prompt.rs`
- Create: `crates/octopus-sdk-context/src/memory.rs`
- Create: `crates/octopus-sdk-context/src/scratchpad.rs`
- Create: `crates/octopus-sdk-context/tests/prompt_stability.rs`
- Create: `crates/octopus-sdk-context/tests/scratchpad_atomic.rs`
- Modify: `Cargo.toml`（workspace members 追加）
- Modify: `docs/plans/sdk/02-crate-topology.md`（回填 §2.6 第 11–17 项）

Preconditions:
- Task 1 完成；Task 2–4 可并行（`sdk-context` 不依赖 `sdk-permissions`）；W3 `ToolRegistry::schemas_sorted()` 已稳定。

Step 1:
- Action: 创建 `octopus-sdk-context` crate（Level 3）；依赖 `octopus-sdk-contracts / octopus-sdk-model / octopus-sdk-session / octopus-sdk-tools / tokio / async-trait / sha2 / serde / serde_json / thiserror / tracing`。`lib.rs` 声明 `pub mod prompt; pub mod compact; pub mod memory; pub mod scratchpad;`。
- Done when: `cargo build -p octopus-sdk-context` 通过。
- Verify: `rg '^octopus-sdk-context' Cargo.toml` 命中。
- Stop if: `Cargo.toml` 引入 `sdk-permissions / sdk-sandbox / sdk-hooks` → Stop #2（Level 3 允许，但本周明确禁止这三项以避免 Compactor 内部偷引钩子执行）。

Step 2:
- Action: `SystemPromptBuilder::{new, with_section(SystemPromptSection), build(&PromptCtx) -> Vec<String>, fingerprint(&PromptCtx) -> [u8; 32]}`；`build` 按 `section.order` 升序输出；`fingerprint` = `sha256(build(ctx).join("\n"))`。
- Done when: 单测 3 次 build 字节一致；fingerprint SHA 固定。
- Verify: `cargo test -p octopus-sdk-context --test prompt_stability`。
- Stop if: 段内含随机 / 时间戳（如 `"now: {}".format(now)`）→ 设计错误，移除或冻结在 `PromptCtx` 输入里。

Step 3:
- Action: 内置段生成器：`role_section / tools_guidance_section(&ToolRegistry) / process_section / safety_section / output_section`；`tools_guidance_section` 固定遍历 `registry.schemas_sorted()`，对每个 `schema` 输出 `"- {name}: {description}\n"` 的确定性段（不序列化 `input_schema`）。
- Done when: `tools_guidance_section` 同一 registry 输入 3 次输出字节一致。
- Verify: `cargo test -p octopus-sdk-context --test prompt_stability -- test_tools_guidance_stability`。
- Stop if: `schemas_sorted()` 不能暴露 `description` → 回 W3 补只读访问面，不在 `SystemPromptBuilder` 内部自建排序。

Step 4:
- Action: 回填 `02 §2.6` 条目 11–12。

---

### Task 10: `octopus-sdk-context::Compactor` + `MemoryBackend` + `DurableScratchpad`

Status: `pending`

Files:
- Modify: `crates/octopus-sdk-context/src/compact.rs`
- Modify: `crates/octopus-sdk-context/src/memory.rs`
- Modify: `crates/octopus-sdk-context/src/scratchpad.rs`
- Create: `crates/octopus-sdk-context/tests/compactor_clear_tool_results.rs`
- Create: `crates/octopus-sdk-context/tests/compactor_summarize.rs`
- Create: `crates/octopus-sdk-context/tests/scratchpad_atomic.rs`（Task 9 已建文件，本步填内容）
- Modify: `docs/plans/sdk/02-crate-topology.md`（回填 §2.6 第 13–17 项）

Preconditions:
- Task 9 完成。

Step 1:
- Action: `Compactor::new(threshold, strategy, provider)`；`maybe_compact(&self, session: &mut SessionView)`：若 `session.tokens / tokens_budget < threshold` 返回 `Ok(None)`；否则根据 strategy 分派到 `clear_tool_results` / `summarize` / `Hybrid -> Err(CompactionError::Aborted { reason: "hybrid not implemented in W4" })`。
- Done when: 阈值分支单测覆盖；`Hybrid` 返回 `Err(CompactionError::Aborted { reason: "hybrid not implemented in W4" })`。
- Verify: `cargo test -p octopus-sdk-context --test compactor_clear_tool_results -- test_below_threshold_noop`。
- Stop if: `SessionView` 需要持有 `Arc<Mutex<Session>>` → W4 保持 `&mut Vec<Message>`，真实游标延 W6。

Step 2:
- Action: `Compactor::clear_tool_results`：遍历 `session.messages`，对 `ContentBlock::ToolResult` 的 `content` 字段置空并返回清理数量；不改消息顺序，不改 `message.role`。
- Done when: 单测覆盖"3 个 ToolResult 消息 → 清空 → `tokens_before - tokens_after == sum(prev content len)`"（tokens 估算用简单字符数，避免引入 tokenizer）。
- Verify: `cargo test -p octopus-sdk-context --test compactor_clear_tool_results`。
- Stop if: tokens 估算漂移 → 固化估算函数为 `chars / 4`（近似 GPT 系列），登记到 `02 §5`。

Step 3:
- Action: `Compactor::summarize`：用 `MockModelProvider` 注入确定性 summary 输出（`"SUMMARY: turns [0..n-1] condensed"`）；构造 `CompactionResult.summary / folded_turn_ids / tokens_before / tokens_after / strategy: Summarize`；替换前半段 `messages` 为单条 `Message { role: System, content: vec![Text { text: summary }] }`。
- Done when: 单测 3 次 summarize 输出字节一致；`folded_turn_ids` 正确记录被折叠的 EventId。
- Verify: `cargo test -p octopus-sdk-context --test compactor_summarize`。
- Stop if: `ModelProvider::complete` 签名与 W2 不一致 → 走 `sdk-model` 补丁而非本 Task 改。

Step 4:
- Action: `trait MemoryBackend { async fn recall / async fn commit }`；`DurableScratchpad::{new, read, write}`；`write` 用 `tempfile::NamedTempFile` + `persist` 实现 atomic rename。
- Done when: `scratchpad_atomic.rs` 覆盖：并发 10 次 `write` 不丢失最后一次；读取内容与最后一次 write 一致。
- Verify: `cargo test -p octopus-sdk-context --test scratchpad_atomic`。
- Stop if: Windows `rename` 失败 → `#[cfg(windows)]` fallback 用 `std::sync::Mutex` 序列化写。

Step 5:
- Action: 回填 `02 §2.6` 条目 13–17。

---

### Task 11: W4 合同测试 + Prompt Cache 稳定性守护

Status: `pending`

Files:
- Create: `crates/octopus-sdk-context/tests/prompt_cache_fingerprint.rs`
- Create: `crates/octopus-sdk-permissions/tests/no_credentials_in_events.rs`
- Create: `crates/octopus-sdk-hooks/tests/no_credentials_in_events.rs`

Preconditions:
- Task 4 / Task 6 Step 5 / Task 8 / Task 9 完成。

Step 1:
- Action: `prompt_cache_fingerprint.rs` 跑 `SystemPromptBuilder + ToolRegistry` 组合：3 次 `build + fingerprint` 字节一致；然后 `register` 一个新 `Tool`，`fingerprint` 必须变（正向守护：新增工具应失效 cache，这是预期行为）；最后 `unregister`，`fingerprint` 回到初始值（证明排序稳定不依赖注册顺序）。
- Done when: 3 条断言 pass。
- Verify: `cargo test -p octopus-sdk-context --test prompt_cache_fingerprint`。
- Stop if: 指纹不回到初始值 → W3 `schemas_sorted` 真实存在顺序泄漏，回 W3 补丁。

Step 2:
- Action: `no_credentials_in_events.rs`（跨 permissions + hooks）：构造一条 `ToolCallRequest { input: { "api_key": "secret-xyz" } }`，走 `PermissionGate::check` + `HookRunner::run(PreToolUse)`，然后只发射当前安全摘要事件（审批分支 `SessionEvent::Ask`，执行分支 `SessionEvent::ToolExecuted`）；扫描所有 emit 的 JSON 序列化结果无 `secret-xyz`。
- Done when: `rg -F 'secret-xyz'` 对事件 JSON 无命中。
- Verify: `cargo test -p octopus-sdk-permissions --test no_credentials_in_events` + `cargo test -p octopus-sdk-hooks --test no_credentials_in_events`。
- Stop if: 命中泄漏 → 检查是否有新事件在 W4 直接序列化原始 `ToolCallRequest.input`；若是，先补 redaction contract 再继续。

Step 3:
- Action: 在 `00-overview.md §3 W4 硬门禁` 的"凭据零暴露合同测试"项下追加具体命令：`cargo test -p octopus-sdk-permissions -p octopus-sdk-hooks -p octopus-sdk-sandbox --test no_credentials_in_events --test no_credentials_leak`。
- Done when: `00-overview.md` 同 PR 更新。

---

### Task 12: W4 Weekly Gate 收尾

Status: `pending`

Files:
- Modify: `docs/plans/sdk/README.md`（W4 状态切 `in_progress → done`）
- Modify: `docs/plans/sdk/00-overview.md §10 变更日志`（追加 W4 收尾）
- Modify: `docs/plans/sdk/02-crate-topology.md §10`（追加 W4 日志 / §8 `default-members` 如需更新）
- Modify: `docs/plans/sdk/03-legacy-retirement.md`（把 `runtime::{permissions, hooks, sandbox, prompt, compact}` 与 adapter approval/memory 行状态从 `pending` 切 `replaced`，`W` 列保持 W4/W7 语义）
- Modify: `docs/plans/sdk/07-week-4-permissions-hooks-sandbox-context.md`（追加最终 Checkpoint）

Preconditions:
- Task 1–11 全部 `done`。

Step 1:
- Action: 执行 `01-ai-execution-protocol.md §4` Weekly Gate Checklist 全量核对。
- Done when: 12 项全 pass。
- Verify: 各命令输出贴到本文件 Checkpoint 节。

Step 2:
- Action: 跑 `cargo build --workspace / cargo clippy --workspace -- -D warnings / cargo test --workspace`；跑 `01 §7.4` 守护扫描：
  ```bash
  find docs/plans/sdk -type f -name '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]-*.md'
  find crates -type f -name '*.rs' -size +800
  rg "use (runtime|tools|plugins|api)::" crates/octopus-sdk-{permissions,sandbox,hooks,context}
  ```
- Done when: 三项扫描全部 0 命中；workspace 三项 pass。
- Verify: 同上。

Step 3:
- Action: 把 W4 的 12 个 Task `Status` 全部切 `done`；`README.md §文档索引` 的 W4 状态切 `done`；`00-overview.md §10` 追加 "2026-04-xx | W4 Weekly Gate 收尾：`07-week-4-*.md` 由 `in_progress` 切为 `done`。4 个 Level 2/3 crate 落地；Level 0 contracts W4 补丁完成；凭据零暴露合同测试 + prompt cache fingerprint 守护通过。| Codex"。
- Done when: 三处同批次完成。
- Verify: diff 审读。

## Exit State 对齐表（与 `00-overview.md §3 W4` 逐条）

| `00-overview.md §3 W4 出口状态` | 本 Plan 交付点 | 验证命令 |
|---|---|---|
| `PermissionMode` 四态 + `PermissionGate` + `ApprovalPrompt`；对接 `AskPrompt` UI Intent | Task 2 / 3 / 4 | `cargo test -p octopus-sdk-permissions` |
| `HookRunner` 支持 `PreToolUse / PostToolUse / Stop / SessionStart / SessionEnd / UserPromptSubmit / PreCompact / PostCompact` | Task 7 / 8 | `cargo test -p octopus-sdk-hooks --test events_matrix` |
| Bubblewrap / Seatbelt 两个 `SandboxBackend` 最小实现；容器内零凭据 | Task 5 / 6 | `cargo test -p octopus-sdk-sandbox --test no_credentials_leak` + smoke 标 ignored |
| `Compactor`（compaction + tool-result clearing）与 `SystemPromptBuilder` 完成，工具顺序由确定性排序保证 | Task 9 / 10 / 11 | `cargo test -p octopus-sdk-context --test prompt_stability --test compactor_clear_tool_results --test compactor_summarize --test prompt_cache_fingerprint` |
| **硬门禁**：凭据零暴露合同测试；事件日志扫描不得含 `API_KEY / TOKEN / BEARER` 明文 | Task 6 Step 5 / Task 8 Step 2 / Task 11 Step 2 | `cargo test -p octopus-sdk-permissions -p octopus-sdk-hooks -p octopus-sdk-sandbox --test no_credentials_in_events --test no_credentials_leak` |
| **硬门禁**：`cargo test -p octopus-sdk-permissions -p octopus-sdk-hooks -p octopus-sdk-sandbox -p octopus-sdk-context` 全绿 | Task 12 Step 2 | 命令同左 |

## Weekly Gate Checklist · W4（执行时勾选）

```md
- [ ] 本周 12 个 Task 状态 = `done` 或 明确 `blocked`（带原因）。
- [ ] `00-overview.md §3 W4 出口状态` 逐条勾选通过（上表 6 行）。
- [ ] `00-overview.md §3 W4 硬门禁` 命令实际执行过并 pass。
- [ ] 当周 Checkpoint 无缺失（每批次一条，本文件末尾累积）。
- [ ] 本周 PR 总 diff 行数分布记录完成（用于预警 R5）。
- [ ] 未引入 `docs/sdk/*` 与实现的新矛盾；如有 → 追加到 `docs/sdk/README.md ## Fact-Fix 勘误`。
- [ ] 新公共面符号 = `02-crate-topology.md` 登记；`03-legacy-retirement.md` 同步切状态。
- [ ] Prompt Cache 相关：`prompt_cache_fingerprint.rs` 绿。
- [ ] 凭据零暴露：三处合同测试绿。
- [ ] `cargo build --workspace` 全绿；`cargo clippy --workspace -- -D warnings` 全绿；`cargo test --workspace` 全绿。
- [ ] 完成本周"变更日志"追加到 `00-overview.md §10`。
- [ ] `README.md §文档索引` W4 行状态 = `done`。
```

## 变更日志

| 日期 | 变更 | 责任人 |
|---|---|---|
| 2026-04-21 | 首稿：W4 完整 Plan（Goal / Architecture / Scope / Risks R1–R11 / 公共面修订清单 36 条 / Task Ledger 12 个 / Exit State 对齐表 / Weekly Gate）。按"层级从低到高"排序（contracts 补丁 → permissions → sandbox → hooks → context → 合同测试 → 收尾）。 | Architect (Claude) |
| 2026-04-21 | 审核通过：3 项关键决策（D1 `ToolCategory` 下沉 / D2 Compactor 只落 `ClearToolResults` + `Summarize` / D3 Sandbox 3 后端 + Windows Noop fallback）已由 owner 确认，写入 Status §审核决策表。Plan 保持 `draft`，进入 Task 1 前切 `in_progress`。 | Architect (Claude) |
