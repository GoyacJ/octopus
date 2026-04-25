# Octopus Agent Harness SDK · 架构变更日志

> 状态：历史修订记录。当前权威基线见 `README.md`、D1-D10、ADR 与 crate SPEC。
> 本文件不承载新的规范要求；历史摘要与 Accepted 文档冲突时，以 Accepted 文档为准。

## 更新历史

- **1.8（2026-04-25）**：参照 `reference-analysis` 8 项剩余设计点的全量评估，固化 3 个新增 ADR：ADR-0016（Programmatic Tool Calling：`execute_code` 元工具，单轮编排多 tool calls，默认只读嵌入式白名单 7 项）/ ADR-0017（Steering Queue：会话级软引导队列，主循环安全检查点 drain-and-merge，默认 capacity 8 / TTL 60s / DropOldest）/ ADR-0018（反向决议：不引入 Loop-Intercepted Tools，维持"Tool 是一等公民"边界）；`harness-contracts` §3.2 / §3.3 / §3.4 / §3.5 增量：`SteeringId` / 5 个 Event variant / `DecisionScope::ExecuteCodeScript` / `ToolCapability::CodeRuntime` + `EmbeddedToolDispatcher` / 4 个 Steering 共享枚举 / `ToolResultPart` 正向白名单 8 个变体；ADR-0002 升级为"正向白名单 + 反向黑名单"双向定义；`harness-engine` 主循环新增 `steering.drain_and_merge()` 阶段；`harness-session` 新增 `push_steering / steering_snapshot` API + `SteeringQueue` 内部章节；`harness-tool` 新增 `ExecuteCodeTool` 章节；`harness-sandbox` 新增 `CodeSandbox` trait（独立于 `SandboxBackend`）；`event-schema.md` 新增 §3.5.1 Steering 事件族（3 个）+ §3.5.2 ExecuteCode 事件族（2 个）；feature flags 新增 `programmatic-tool-calling`（默认 off）+ `steering-queue`（默认 on）；comparison-matrix.md 回填 R-04~R-08 已吸收 + R-09/R-10/R-11 新增 + R-12 反向决议。变更摘要见 §12。
- **1.7（2026-04-25）**：深度修订 `harness-plugin` 治理面：新增 ADR-0014（Plugin Manifest Signer：`TrustedSignerStore` + 启用窗口 + 撤销 + 与 ADR-0013 IntegritySigner 完全独立的边界）、ADR-0015（Plugin Loader 二分 `PluginManifestLoader / PluginRuntimeLoader` + Capability-Scoped ActivationContext，把 Manifest-first 硬约束抬到类型层）；`harness-contracts` §3.3 新增 `Event::ManifestValidationFailed` variant，把"manifest 解析失败"与"业务规则拒绝"拆为互斥两路审计；`event-schema.md` 新增 §3.20 Plugin Lifecycle Events 整段；`PluginActivationContext` 重构为按 manifest 声明范围注入的 capability handle 集合，越权注册被类型 + 运行期双重拦截。变更摘要见 §11。
- **1.6（2026-04-25）**：参照 `reference-analysis` 修订 `harness-hook`。补 `PreToolSearch / PostToolSearchMaterialize` 至 20 类能力矩阵；`HookContext` 五元组结构定义 + `HookSessionView` 只读视图；`HookFailureMode = FailOpen / FailClosed` 与"User 域强制 FailOpen"；`HookExecSpec` 补 `working_dir / resource_limits / signal_policy`；`HookHttpSpec` 补 `allowlist / ssrf_guard / max_redirects / max_body_bytes / mtls`；`HookProtocolVersion` 协商 + replay 幂等契约 + `ReplayMode = Live / Audit`；新增 5 个 hook 事件 variant；多文档横向同步至 v1.6 一致状态。变更摘要见 §10。
- **1.5（2026-04-25）**：参照 `reference-analysis/{claude-code-sourcemap,openclaw,hermes-agent}.md` 深度修订 `harness-mcp`。`McpServerSpec` 引入三维正交字段 `source` × `trust` × `scope`（生命周期），废弃旧 `scope` 字段同时承担三种语义的歧义（对齐 CC-19/20/21）；新增 `McpToolFilter` allow/deny 工具预过滤、`SamplingPolicy` 七维 budget + cache 硬隔离 + PermissionMode 联动、`StdioPolicy/StdioEnv` 子进程环境变量沙化（默认屏蔽常见凭证）、`ReconnectPolicy` 指数退避重连 + 自重置、`TenantIsolationPolicy` Server Adapter 多租户隔离（默认 StrictTenant）、`McpTimeouts` 四档超时；明写 `McpServerRef::Required` 复用要求 + `SubagentSpec.required_mcp_servers` pattern 级依赖装配校验；Inline MCP 受 trust 限制（user-controlled agent 不得 inline 引入 user-source MCP）；新增 `McpConnectionState` 与 `schema_fingerprint` 字段；运行期方法表补 `shutdown / ping / resources/subscribe / resources/updated / prompts/list_changed / notifications/cancelled / notifications/progress`；canonical 工具命名 `mcp__<server>__<tool>` 与 `harness-permission` glob 共用；Event 集合扩 3 个 `McpConnectionRecovered / McpResourceUpdated / McpSamplingRequested`，既有 5 个 MCP 事件字段补全（详见 `event-schema.md §3.19`）。变更摘要见 §9。
- **1.4（2026-04-25）**：参照 `reference-analysis/{claude-code-sourcemap,openclaw,hermes-agent}.md` 深度修订 `harness-memory`。`MemoryProvider` 拆为 `MemoryStore` + `MemoryLifecycle`、补 7 个生命周期 Hook（`on_pre_compress` / `on_delegation` / `on_session_end` 等）；明写 Memdir「写磁盘立即生效 + 系统提示下一 Session 生效」契约 + 与 ADR-003 交叉；`MemoryScope` 一维拆为 `MemoryKind` × `MemoryVisibility` 二维（对齐 CC-31）；`MemoryMetadata` 补时效字段；威胁扫描多档动作 `Warn/Redact/Block`；栅栏新增 `escape_for_fence` 第一道闸门；Memdir 跨进程 advisory lock + atomic-rename SPEC；Recall 编排整段化（每轮至多 1 次 + fail-safe 默认）；Consolidation Hook 扩展点；Event 集合扩 4 个 `MemoryRecallDegraded / MemoryRecallSkipped / MemdirOverflow / MemoryConsolidationRan`。变更摘要见 §8。
- **1.3.1（2026-04-25）**：1.3 follow-ups 全部落地——`event-schema.md` 补三类 Event 字段、`harness-hook.md` 与 Tool 流水线协同节、`harness-subagent.md` 增加 `SubagentRunnerCap` 投影、`harness-engine.md` 装配 `CapabilityRegistry` 样板、`harness-tool-search.md` 与 ResultBudget 对接、`extensibility.md` §3 整段重写、ADR-010/011 Evidence 回归 `HER-/OC-/CC-` 编号体系；新增 ADR-012 固化 `MockCapabilityRegistry` 的 testing 边界。
- **1.3（2026-04-25）**：参照三大开源 Agent 框架（Claude Code / OpenClaw / Hermes）深度修订 `harness-tool`：流式 ToolStream、ResultBudget 流式预算、Capability Handle、Registry 裁决矩阵；新增 ADR-010 / ADR-011；新增 4 个内置工具（WebSearch / Clarify / SendMessage / ListDir）+ 1 个元工具 ReadBlob。变更摘要见 §7。
- **1.2（2026-04-25）**：引入 ADR-009 Deferred Tool Loading 与 Tool Search 元工具，新增 L2 crate `harness-tool-search`（crate 总数 18 → 19）。变更摘要见 §6。
- **1.1（2026-04-24）**：按"全方位架构审计"结果修订，覆盖 P0/P1/P2/P3 共 31 项变更。关键变更见 §5 变更摘要。
- **1.0（2026-04-24）**：初版定稿。

## 详细变更记录

## v1.1 变更摘要（2026-04-24）

### P0（3 项，核心一致性修复）

- **C-1** Prompt Cache 矛盾修复：`ReloadMode::AppliedInPlace` 新增 `CacheImpact` 字段；澄清"in-place ≠ 零成本"；四处文档（ADR-003 / overview / harness-session / context-engineering）语义对齐
- **H-1** `DecisionScope` 统一：下沉到 `harness-contracts` §3.4；permission-model / ADR-007 / harness-permission 三处对齐
- **H-2** 补齐 `BlobStore` trait：L0 契约 + L1 `harness-journal` 三种默认实现（File / Sqlite / InMemory）+ `RetentionEnforcer` + HarnessBuilder 入口

### P1（6 项，契约对齐 + 背压补齐）

- **H-3** `HookHandler` 统一为 `interested_events`
- **H-4** `HookEventKind` 统一 18 类 + `extensibility.md` §5.3 完整能力矩阵
- **H-5** 契约补 `TeamCreated / TeamMemberJoined / TeamMemberLeft` 事件
- **H-6** `PermissionRequested/Resolved` 以 ADR-007 为准，补全字段 + `DecidedBy` 契约化
- **H-7** `RuleProvider` + `RuleEngineBroker` watch 实现 + 四种内置 Provider
- **H-8** `EventStream` 背压/缓冲：`SessionEventSinkPolicy` + `DEFAULT_NEVER_DROP_KINDS` 白名单

### P2（16 项，一致性 + 抽象补齐）

- `TenantId` type alias 统一；所有语义 ID 强制使用 `TypedUlid<XxxScope>` 模式
- Subagent blocklist `memory_write` 统一
- `module-boundaries.md` 修正 `harness-tool` 依赖
- `MemoryProvider` 接口对齐（`recall / upsert / forget / list`）
- `DecidedBy` 枚举对齐契约
- `SessionCreatedEvent / SessionForkedEvent / SessionEndedEvent` 字段完整定义
- `ElicitationHandler` + 三种内置实现 + `harness.resolve_elicitation` API
- Aux Model 统一注入点 `ModelCatalog::aux_model` + `HarnessBuilder::with_aux_model`
- `ModelPricing` + `CostCalculator` trait 接入 `UsageAccumulator`
- `overview.md §1.3` 显式排除 Task/Scheduler/Dream + 跨进程 Team
- MCP `tools/list_changed` 运行期策略 + ADR-003 §7 关联
- Sandbox CWD marker 改为独立 FD 协议
- Windows/PowerShell 危险命令库 `default_windows()` + `default_all()`
- Event Schema 版本迁移运行时路径：`VersionedEventStore<S>` 装饰器 + `MigratorChain`
- 配置严格校验 + Last-Known-Good 回退（R-13）
- Session ≠ auth token 硬约束（R-16）

### P3（5 项，抽象加固）

- **Workspace** 抽象：`WorkspaceId` + `Harness::create_workspace` + 多级 Bootstrap 继承
- **Skill 预取**：`SkillPrefetchStrategy` 四档
- **ConcurrentSubagentPool** 死锁防御：per-parent × per-depth Semaphore + acquire_timeout + Watchdog
- **术语表 + 拓扑关系图**：`overview.md §1.4 / §1.5`
- **Feature flag default 理由**：`feature-flags.md §3.5`

### 触发的结构调整

- `harness-contracts.md` §3.6 新增 BlobStore trait；原错误码类型 → §3.7；原 Schema 导出 → §3.8
- `harness-journal.md` 新增 §3（BlobStore 实现）、§4.4（VersionedEventStore），原章节后移
- `harness-mcp.md` 新增 §6（list_changed），原章节后移
- `event-schema.md` 新增 §3.0（Session 生命周期）、§3.10（Team Lifecycle），原编号后移
- `security-trust.md` 新增 §7.2.1（Session ≠ auth）、§9.X（严格校验 + LKG）

### 向后兼容性

- v1.1 是**文档级**修订，**尚未进入实现期**（按仓库工程遗产迁移策略），因此无代码层向后兼容问题
- 对已在审阅 v1.0 文档的评审者：建议从 ADR-003 / api-contracts §5 / event-schema §3 / harness-session §2.3 开始回看

## v1.2 变更摘要（2026-04-25）

### 新增 ADR

- **ADR-009** · Deferred Tool Loading 与 Tool Search 元工具：引入 `DeferPolicy` / `ToolSearchMode` / `ToolLoadingBackend` / `ToolSearchScorer` 四大抽象；采纳 Claude Code 经 A/B 验证的 `tool_search` 设计，但按 Octopus 多 provider / 企业级诉求重新组织。

### 新增 crate

- **`octopus-harness-tool-search`**（L2 · 复合能力）：实现 `ToolSearchTool` / `AnthropicToolReferenceBackend` / `InlineReinjectionBackend`（含 50ms / max 32 合并窗口） / `DefaultScorer`。crate 总数 18 → 19（ADR-008 清单同步更新）。

### 关键契约扩展

- `ToolProperties`：`defer_load: bool` → `defer_policy: DeferPolicy`；新增 `search_hint: Option<String>`
- `ModelDescriptor`：新增 `capabilities: ModelCapabilities`，其中包含 `supports_tool_reference` + `tool_reference_beta_header`
- `SessionOptions`：新增 `tool_search: ToolSearchMode`（默认 `Auto { ratio: 0.10, min_absolute_tokens: 4_000 }`）
- `SessionProjection`：新增 `discovered_tools: DiscoveredToolProjection`
- `HookEvent`：新增 `PreToolSearch` / `PostToolSearchMaterialize`（能力矩阵 18 → 20）
- `ToolPool`：扩展为三分区（AlwaysLoad / Deferred / RuntimeAppended）

### 新增事件

- `Event::ToolDeferredPoolChanged`（`ToolPoolChangeSource` 四类来源）
- `Event::ToolSearchQueried`
- `Event::ToolSchemaMaterialized`

### 关键语义改动

- **MCP 默认 `DeferPolicy::AutoDefer`**：绝大多数 MCP `tools/list_changed` 路径从 `OneShotInvalidation` 降级为 `NoInvalidation`（ADR-003 冲突面显著缩小）
- **Session 内禁止切换 `ToolSearchMode`**：`reload_with` 尝试切换模式将被 `Rejected`（ADR-009 §2.3）
- **Feature flag 默认开启**：`tool-search` 进入 `harness-sdk` default 集合

### Security 新增

- `security-trust.md §10.X` · Tool Search 安全考量 + 三级 kill switch（全局 / Session / Feature）
- 必记事件：`ToolDeferredPoolChanged` + `ToolSchemaMaterialized`

### 受影响文档（本次同步修订）

- 新增：`crates/harness-tool-search.md`、`adr/0009-deferred-tool-loading.md`
- 修订：`README.md` · `crates/harness-contracts.md` · `crates/harness-tool.md` · `crates/harness-mcp.md` · `crates/harness-model.md` · `crates/harness-session.md` · `context-engineering.md` · `event-schema.md` · `extensibility.md` · `security-trust.md` · `feature-flags.md`

### 向后兼容性

- v1.2 仍是**文档级**修订，无代码层兼容问题
- 对已审阅 v1.1 的评审者：建议从 ADR-009 → `harness-tool-search.md` → `context-engineering.md §9 注入顺序表` 开始回看

## v1.3 变更摘要（2026-04-25）

### 触发动机

参照 `docs/architecture/reference-analysis/{claude-code-sourcemap,openclaw,hermes-agent}.md` 与新增的 `comparison-matrix.md`，发现 v1.2 的 `harness-tool` 在以下 13 个维度落后于业内最佳实践：缺少流式输出协议、`max_result_size_chars` 字段未消费、`ToolDescriptor`/`ToolProperties` 字段重复、Agent 级工具直接耦合 Engine 内部、Hook 介入点未文档化、Registry 同名裁决无矩阵、SubagentSpec 协议字段未约束、缺 ToolGroup、缺长任务心跳、缺 DenyPattern 库、缺 dynamic schema、缺 ProviderRestriction、缺 4 个常用内置工具。

### 新增 ADR

- **ADR-010** · Tool 结果预算与溢出自动落盘 BlobStore：`ResultBudget` / `BudgetMetric` / `OverflowAction` / `OverflowMetadata` + `ToolResultEnvelope` + `read_blob` 内置工具
- **ADR-011** · Tool Capability Handle：`ToolCapability` 枚举 + 7 个 capability traits + `CapabilityRegistry` + `default_locked()` 矩阵；让 `harness-tool` 不再反向依赖 `harness-subagent` / `harness-session` 内部

### 关键契约扩展（`harness-contracts.md`）

- §3.3 Event：新增 `ToolUseHeartbeat` / `ToolResultOffloaded` / `ToolRegistrationShadowed`
- §3.4 共享枚举：新增 `ToolGroup` / `ToolOrigin` / `ProviderRestriction` / `ResultBudget` / `BudgetMetric` / `OverflowAction` / `OverflowMetadata` / `ToolCapability`
- §3.5 消息：新增 `ToolResultEnvelope`（包裹现有 `ToolResult` enum，承载 overflow 元数据）
- 新增 `harness-contracts::capability` 模块，承载 7 个 capability traits 接口

### `harness-tool` 重大改动

- `Tool::invoke → execute` 返回 `ToolStream`（流式协议；`ToolEvent::Progress / Partial / Final / Error`）
- `ToolDescriptor` 收敛为单一权威信息体（合并 properties / capability / budget / provider / group / origin）
- `ToolProperties` 移除 `max_result_size_chars` / `mcp_origin` / `skill_origin`（迁移到 `ToolDescriptor`）
- `ToolPool::assemble` 增加 `ProviderRestriction` 过滤与 `dynamic_schema` 解析
- `ToolOrchestrator` 实装 `ResultBudget` 流式收集 + `LongRunningPolicy` 心跳 + 五个 Hook 介入点串联
- Registry 引入裁决矩阵：**built-in wins**，同名遮蔽产生 `ShadowedRegistration` 审计
- 新增 `DenyPatternLibrary`（bash 命令 / 路径 / URL 三类）+ `harness-tool/data/deny-patterns.toml`

### 新增内置工具（M0 全集）

- `WebSearchTool` · 网络只读检索（pluggable backend：brave / serpapi / 自建）
- `ClarifyTool` · 结构化向用户提问（依赖 `ClarifyChannel` capability）
- `SendMessageTool` · 异步对外发消息（依赖 `UserMessenger` capability）
- `ListDirTool` · 目录列表（FileSystem 组）
- `ReadBlobTool` · 取回 ToolResultOffloaded 落盘（ADR-010 配套）

### 受影响文档（本次同步修订）

- 新增：`adr/0010-tool-result-budget.md`、`adr/0011-tool-capability-handle.md`
- 重写：`crates/harness-tool.md`
- 增量：`crates/harness-contracts.md`（§3.3 / §3.4 / §3.5）、`README.md`（本节 + ADR 索引）

### 待跟进（Follow-ups）

- [x] `event-schema.md` §3.5：补 `ToolUseHeartbeatEvent` / `ToolResultOffloadedEvent` / `ToolRegistrationShadowedEvent` 字段定义（v1.3.1）
- [x] `harness-hook.md`：§2.4 能力许可表补 `PostToolUseFailure` / `SubagentStart-Stop` / `Elicitation`；新增 §2.7 与 Tool 流水线协同（含 ADR-010 / ADR-011 边界）（v1.3.1）
- [x] `harness-subagent.md` §3.1：`SubagentRunnerCap` 投影适配器与装配示例（v1.3.1）
- [x] `harness-engine.md`：`Engine` 字段加 `cap_registry`/`blob_store`；§3 主循环 `invoke→execute` + Hook 链；§6.2 `CapabilityRegistry` 装配样板（v1.3.1）
- [x] `harness-tool-search.md`：`ToolSearchTool::descriptor` 补 `budget/group/required_capabilities`；§2.2.4 与 ResultBudget 对接（v1.3.1）
- [x] `extensibility.md` §3：整段重写（流式 trait / `ToolDescriptor` 字段表 / Trust × Capability × Group 裁决矩阵 / 业务工具最小示例）（v1.3.1）
- [x] `evidence-index.md`：将 ADR-010 / ADR-011 中临时占位的 `EVID-*` 编号回归到既有 `HER-* / OC-* / CC-*` 体系（v1.3.1）

### 向后兼容性

- v1.3 仍是**文档级**修订，无代码层兼容问题
- 对已审阅 v1.2 的评审者：建议按 ADR-010 → ADR-011 → `harness-tool.md` 顺序回看；`harness-contracts.md` 仅看 §3.3 / §3.4 / §3.5 增量段

## v1.4 变更摘要（2026-04-25）

### 触发动机

参照 `docs/architecture/reference-analysis/{claude-code-sourcemap,openclaw,hermes-agent}.md`，发现 v1.3.1 的 `harness-memory` 在以下维度落后于业内最佳实践：

| 缺口 | 反映在 |
|---|---|
| `MemoryProvider` 单 trait 同时承载存储 + 生命周期，stub 实现需写一堆 `pass` | 对齐 Hermes `MemoryProvider` 七 hook 拆分需求 |
| Memdir 写入与 system message 生效语义未约束 | 与 ADR-003 Prompt Cache Locked 暗合但未明写 |
| `MemoryScope` 一维既表"类型"又表"范围" | CC-31 已用 type × scope 二维 |
| 栅栏未防 special-token 注入 | OC-34 早已用 `<<<EXTERNAL_UNTRUSTED_CONTENT...>>>` |
| Memdir 跨进程并发写无契约 | HER-021 已为 SQLite 引入 `BEGIN IMMEDIATE` + 抖动重试 |
| Recall 编排未文档化（触发条件 / 失败降级 / 预算） | Hermes / OpenClaw 在主循环里有清晰编排 |
| 威胁扫描只有 Block 单档 | HER-019 已列三档 |
| Dreaming 这种 OC-14 的合理扩展点未声明 | 业务侧无法接入 |

### 关键契约扩展（`harness-memory.md`）

- §2.1 `MemoryProvider` 拆分为 `MemoryStore` + `MemoryLifecycle`，blanket impl 合并；新增 7 个生命周期 Hook：`initialize / on_turn_start / on_pre_compress / on_memory_write / on_delegation / on_session_end / shutdown`
- §2.3 `MemoryScope` → `MemoryKind` × `MemoryVisibility` 二维拆分（5 + 4 个 variant）
- §2.4 `MemoryMetadata` 补 `access_count / last_accessed_at / recall_score / ttl / redacted_segments`
- §3.3 明写 Memdir 持久化生效语义（写磁盘立即 + 系统提示下一 Session 生效），通过 `TakesEffect` 字段下放到事件层
- §3.4 Memdir 跨进程并发写：advisory lock + atomic-rename + `MemdirConcurrencyPolicy`（含抖动重试）
- §3.5 Memdir 超限退化策略（Section 截断 / Head-only）+ `MemdirOverflow` 事件
- §4.2 Recall 编排 SPEC：触发条件 / Builtin 不参与 / visibility 过滤 / 威胁扫描 / 预算 / 失败降级（fail-safe 默认）
- §4.4 Consolidation Hook 扩展点（Dreaming 风格）+ `DREAMS.md` 草稿区
- §5 栅栏三道闸门：新增 `escape_for_fence`（清洗 `</memory-context>` / `<|im_end|>` / role markers / `<<<EXTERNAL_UNTRUSTED_CONTENT...>>>` 等）
- §6 威胁扫描多档动作 `Warn / Redact / Block` + 30 条默认模式
- §11 测试矩阵覆盖 1000 并发 + 跨 2 进程 + kill -9 持久化用例

### `harness-contracts` 增量

- §3.1 新增 `MemoryScope` PhantomData marker 与 `MemoryId = TypedUlid<MemoryScope>`
- §3.4 新增枚举：`MemoryKind / MemoryVisibility / MemoryWriteAction / MemorySource / ThreatCategory / ThreatAction / ThreatDirection / TakesEffect / MemoryRecallDegradedReason`
- §3.6 新增「Memory 共享类型」：`ContentHash / MemoryActor / MemoryWriteTarget / WriteDestination / MemdirFileTag`
- §3.7 BlobStore（原 §3.6）→ §3.7 重编号；§3.8 错误根类型 / §3.9 Schema 导出顺位下移

### 新增事件

- `Event::MemoryRecallDegraded`（reason: Timeout / ProviderError / RecordTooLarge / VisibilityViolation / ScannerBlocked）
- `Event::MemoryRecallSkipped`（reason: NoExternalProvider / PolicyDecidedSkip / DeadlineZero / Cancelled）
- `Event::MemdirOverflow`（含 `OverflowStrategy` 枚举）
- `Event::MemoryConsolidationRan`（feature-gated `consolidation`）
- 既有 `MemoryUpserted / MemoryRecalled / MemoryThreatDetected` 字段补全（详见 `event-schema.md §3.17`）

### Feature Flags

- `external-slot`（默认 ❌）：启用 `MemoryManager::set_external`
- `consolidation`（默认 ❌）：启用 `ConsolidationHook` 扩展点

### 受影响文档

- 重写：`crates/harness-memory.md`（v1.4 整体修订）
- 增量：`crates/harness-contracts.md`（§3.1 / §3.4 / §3.6 / §3.7 重编号）
- 增量：`event-schema.md`（§2 总览表 / §3.17 完整化 + 4 个新事件）
- 增量：`context-engineering.md`（§8 整段重写）
- 增量：`extensibility.md`（§9 整段重写）
- 增量：`api-contracts.md`（§3 章节号引用同步）
- 增量：`adr/0010-tool-result-budget.md`（章节号引用同步）

### 证据索引校验（建议2落地）

- 已核验 `evidence-index.md` 中 `HER-026` / `OC-33` / `CC-31` 三个编号均存在且与 v1.4 memory 设计主张一一对应。
- v1.4 文档引用面已统一：`harness-memory.md`、`context-engineering.md`、`harness-context.md`、`comparison-matrix.md` 均能回链到上述证据编号。

### 明确不引入

| 候选 | 来源 | 不引入原因 |
|---|---|---|
| `MemoryProvider::get_tool_schemas` | Hermes | 破坏 ADR-003 / ADR-009 工具面冻结 |
| Dreaming 算法本体（分数 / 频次门槛硬编码） | OpenClaw | L1 仅声明 `ConsolidationHook` 接口，业务自决算法 |
| 多 External Provider 路由器 | （无） | HER-016 已论证 1 个上限是产品决策 |
| `Memory::reload_partial`（仅刷新 Memdir 不 fork） | （无） | 与 ADR-003 不可调和；强制走 `Session::reload_with(ReloadMemdir)` |

### 向后兼容性

- v1.4 仍是**文档级**修订，无代码层兼容问题
- 对已经按 v1.3 接口实现 PoC 的下游：参考 `harness-memory.md §14 迁移说明` 获取字段映射；推荐使用 `derive(MemoryProvider)` 过程宏自动展开两 trait 的空 lifecycle 实现，迁移 1-line
- 对已审阅 v1.3 的评审者：建议按 `harness-memory.md` → `event-schema.md §3.17` → `context-engineering.md §8` 顺序回看；`harness-contracts.md` 仅看 §3.1 / §3.4 / §3.6 增量段

## v1.5 变更摘要（2026-04-25）

### 触发动机

参照 `reference-analysis/{claude-code-sourcemap,openclaw,hermes-agent}.md` 与 `comparison-matrix.md`，发现 v1.4 的 `harness-mcp` 在以下维度落后于业内最佳实践：

| 缺口 | 反映在 |
|---|---|
| `McpServerSpec.scope` 字段同时承担"来源 / 信任 / 生命周期"三种语义，引入歧义 | CC-19 已用三维正交（`source` / `trust` / `scope`） |
| 工具注入前缺少 allow/deny pre-filter，模型上下文一次性暴露 server 全部工具 | CC-20 用 ServerToolFilter 在装配期收敛工具面 |
| stdio 子进程默认继承父进程全部环境变量，凭证类变量易泄露 | Hermes / Claude Code 默认走 deny-list 沙化 |
| MCP Server 反向调用 `sampling/createMessage` 缺治理（无 budget / cache 隔离） | CC-21 SEP-990 已固化 budget + namespace |
| 连接断开后无统一重连策略，dynamic 工具状态丢失 | Hermes 已有指数退避 + 抖动 + 自重置 |
| `McpServerRef::Inline` 未约束 trust，user-controlled agent 可自行 inline 升权 | ADR-006 二分模型隐含但未 MCP 侧落点 |
| `SubagentSpec` 缺 `required_mcp_servers` pattern 级依赖声明，装配期无 fail-closed 校验 | 业务侧需写一堆 if-let 自检 |
| Server Adapter 多租户隔离未文档化，跨 tenant 列举/聚合无策略 | OpenClaw 在 Gateway 已有 isolation 模式枚举 |
| 既有 5 个 MCP 事件字段不全 + 缺连接恢复 / resource 更新 / sampling 三类事件 | 影响审计与排错 |
| 协议方法表未涵盖 `shutdown / ping / resources/subscribe / notifications/*` | 与 MCP spec 现行版本对齐缺口 |

### 关键契约扩展（`harness-mcp.md`）

- §1 职责扩展：补 stdio 沙化 / 重连 / 工具预过滤 / Sampling 治理 / Agent-scoped 注入五条核心能力
- §2.1 `McpServerSpec` 新增字段：`source / trust / timeouts / reconnect / tool_filter / sampling`
- §2.2 来源 / 信任推导表：`McpServerSource → TrustLevel` 7 行映射；`source` × `trust` × `scope`（生命周期）三维正交，废弃 1.4 之前 `scope` 同时承担三种语义的歧义
- §2.3 Registry：`ManagedMcpServer` 补 `connection_state` + `schema_fingerprint`；新增 `McpConnectionState` 五态枚举
- §2.4 `TransportChoice::Stdio` 加 `policy: StdioPolicy + env: StdioEnv`（Allowlist / InheritWithDeny / Empty 三档；默认屏蔽 `*_TOKEN / *_KEY / AWS_* / GCP_* / AZURE_*` 等）
- §2.5 `McpTimeouts` 四档超时（handshake / call / sampling / idle）
- §2.6 **新增** `McpToolFilter`：注入 `ToolRegistry` 前 allow/deny 过滤（与 `harness-permission.DenyRule` 共用 canonical glob 语法），`FilterConflict::DenyWins / AllowWins / Reject` 三档
- §2.7 **新增** 连接生命周期与 `ReconnectPolicy`（max_attempts / backoff / jitter / success_reset / keep_deferred_tools）
- §3.4 `TenantIsolationPolicy` + `RateLimit`：Server Adapter 默认 `StrictTenant + audit_severity=High`；`Delegated` 模式风险自负；RPS / burst 三层粒度
- §4.2 协议方法表补：`shutdown / ping / resources/subscribe / resources/unsubscribe / resources/updated / prompts/list_changed / notifications/cancelled / notifications/progress`
- §5.1 `McpServerRef` 加 `Required(id)` 变体；§5.2 Inline 受 trust 限制（user-controlled agent fail-closed 拒绝）；§5.3 **新增** `required_mcp_servers` pattern 级装配校验（`McpServerPattern + RequiredEvaluation`）
- §6.5 **新增** `sampling/createMessage` 反向调用治理：`SamplingPolicy` 七维（`allow / per_request / aggregate / rate_limit / allowed_models / log_level / cache_policy`）+ `SamplingCachePolicy::IsolatedNamespace { ttl }` 默认（不污染 Session cache key）+ PermissionMode 联动（`BypassPermissions / DontAsk` 强制 `AllowWithApproval` 降级 `Denied`）
- §6.6 `resources/updated / prompts/list_changed` 推送语义
- §11 可观测性指标补：连接状态 / 重连次数 / 工具过滤命中 / resource 更新 / sampling 请求数与 token / 租户隔离限流
- §12 反模式扩展：stdio 全继承环境变量 / 工具不过滤直注入 / sampling 默认放行 / cache 跨 namespace 共享 / `TransportChoice` 直接 fork 新变体

### `harness-contracts` 增量

- §3.4 `McpOrigin` 补 `server_source: McpServerSource + server_trust: TrustLevel`
- §3.4 新增枚举：`McpServerSource`（Workspace / User / Project / Policy / Plugin{trust} / Dynamic{registered_by} / Managed{registry_url}）
- §3.3 Event 加 3 个变体：`McpConnectionRecovered / McpResourceUpdated / McpSamplingRequested`

### 新增 / 补全事件

- 新增 `Event::McpConnectionRecoveredEvent`（`was_first / total_downtime_ms / schema_changed`）
- 新增 `Event::McpResourceUpdatedEvent`（`McpResourceUpdateKind`：list 变更 / 单 resource 更新 / prompt list 变更）
- 新增 `Event::McpSamplingRequestedEvent`（`model_id / input_tokens / output_tokens / latency_ms / SamplingOutcome / SamplingDenyReason / SamplingBudgetDimension / prompt_cache_namespace`）
- 既有 5 个 MCP 事件字段补全（`McpToolInjected` 加 `filtered_out / filter_reason`；`McpConnectionLost` 加 `terminal` 终态标记；`McpToolsListChanged` 加 `added_count / removed_count / disposition` 等）；详见 `event-schema.md §3.19`

### `harness-subagent` 增量

- `SubagentSpec` 新增 `required_mcp_servers: Vec<McpServerPattern>`
- §3 `spawn` 流程：装配期对 `Inline` server 做 trust 检查、对 `required_mcp_servers` pattern 做评估
- §9 `SubagentError` 加 `McpRequirementUnsatisfied / InlineMcpTrustViolation` 两个变体

### `extensibility` 增量

- §6.3 Agent-scoped 注入：补 `McpServerRef::Required` 与 `Inline` trust 限制说明
- §6.4 **新增** 注入前过滤与反向调用扩展点表（`McpToolFilter / SamplingPolicy / StdioPolicy / ReconnectPolicy / TenantIsolationPolicy`）
- §6.5 **新增** 自定义 Transport 扩展指引：实现 `McpTransport` trait，**禁止** fork `TransportChoice` 枚举

### ADR 修订

- ADR-005：补 §2.4 三维正交字段说明 / §6.3 工具预过滤 / §6.4 Sampling 反向调用与 cache 隔离 / §6.5 Inline MCP trust 限制 / §6.6 工具命名冲突重编号；交叉引用补 ADR-003 / ADR-006 / ADR-007

### 受影响文档

- 重写：`crates/harness-mcp.md`（v1.5 整体修订）
- 增量：`crates/harness-contracts.md`（§3.3 / §3.4）、`crates/harness-subagent.md`（§3 / §9）、`event-schema.md`（§2 总览表 / §3.19 整段）、`extensibility.md`（§6.3 / §6.4 / §6.5）、`adr/0005-mcp-bidirectional.md`（§2.3 / §2.4 / §6.3 / §6.4 / §6.5 / §6.6 / §7）

### 明确不引入

| 候选 | 来源 | 不引入原因 |
|---|---|---|
| 直接在 `TransportChoice` 加新变体（如 QUIC / WebTransport） | （无） | 通过实现 `McpTransport` trait 接入；枚举膨胀属反模式 |
| Sampling 默认 `AllowAuto` | （部分参考实现） | fail-closed 是默认；`UserControlled` server 永不允许 `AllowAuto` |
| Inline MCP 不限 trust 自由声明 | Hermes 早期 | 与 ADR-006 二分模型冲突；user-controlled agent 不得升权 |
| Cross-tenant 默认放行（`Delegated` 隔离） | （部分 Gateway 实现） | 默认 `StrictTenant`；`Delegated` 仅在显式声明 + 高严重度审计下可用 |
| MCP server 列表的"运行期热添加"短路 trust 推导 | （部分参考实现） | 任何 `Dynamic` 来源必须经 `registered_by` 审计标记 |

### 向后兼容性

- v1.5 仍是**文档级**修订，无代码层兼容问题
- 对已审阅 v1.4 的评审者：建议按 `crates/harness-mcp.md` → `event-schema.md §3.19` → `adr/0005-mcp-bidirectional.md` → `crates/harness-subagent.md §3 / §9` → `extensibility.md §6.3-§6.5` 顺序回看；`harness-contracts.md` 仅看 §3.3 / §3.4 增量段
- `McpServerScope` 名字保留兼容历史引用，语义即"生命周期范围"，与 `source / trust` 完全解耦

## v1.6 变更摘要（2026-04-25）

### 触发动机

参照 `reference-analysis/{claude-code-sourcemap,openclaw,hermes-agent}.md` 与 `comparison-matrix.md`，发现 v1.5 的 `harness-hook` 在以下维度存在文档漂移与契约缺口：

| 缺口 | 反映在 |
|---|---|
| `harness-hook.md §2.2` 缺 `PreToolSearch / PostToolSearchMaterialize`（v1.2 ADR-009 引入但未回填） | 与 `extensibility.md §5.3` 的 20 类能力矩阵不一致 |
| `extensibility.md §5.2 / §5.3` 仍沿用旧版 `HookOutcome` 单一形态，未同步 `PreToolUse(PreToolUseOutcome)` 三件套 | 与 `harness-hook.md §2.3` 不一致 |
| `HookContext` 在多文档反复出现，但**无任何结构定义** | handler 实现者无入参契约可依 |
| `Event` 枚举里没有 `HookFailed / HookOutcomeInconsistent / HookReturnedUnsupported / HookPanicked / HookPermissionConflict`，但被多处引用 | `harness-tool.md` / `harness-hook.md` 引用悬空 |
| Exec/HTTP transport 缺 `working_dir / resource_limits / signal_policy / SSRF guard / allowlist / max_redirects / max_body / 协议版本`，无法通过安全审计 | 多处文档假设它们存在 |
| 多 handler 串联的 tiebreaker / 冲突合成 / RewriteInput schema 校验 / AddContext 体积上限未明文化 | 实现层会出现确定性差异 |
| `failure_mode = FailOpen / FailClosed` 与"User 域强制 FailOpen"未文档化 | 安全 hook 无法保证 fail-closed |
| Replay 重建时 hook 行为契约未明示（in-process handler 是否会被重新调用、哪些副作用对 replay 可见） | 与 ADR-001 Event Sourcing 联动缺失 |
| `permission-model.md §9` 示例代码与新版 trait 不一致（`HookOutput` / `on_event` / 单一 `OverridePermission`） | 误导实现者 |

### 关键契约扩展（`harness-hook.md`）

- §1：核心能力描述同步为"20 类标准 HookEvent + 三种内置 transport + failure_mode + replay 幂等"
- §2.2：`HookEventKind` / `HookEvent` 补 `PreToolSearch` / `PostToolSearchMaterialize`，注释分组与 `extensibility.md §5.1` 对齐
- §2.2.1 **新增** `HookContext` 结构定义：tenant/session/run/turn/correlation/causation 五元组 + trust/permission_mode/interactivity 只读快照 + `HookSessionView` trait + `UpstreamOutcomeView` + `ReplayMode`
- §2.4：能力矩阵补 `PreToolSearch / PostToolSearchMaterialize / PreLlmCall / PostLlmCall / PreApiRequest / PostApiRequest` 行；显式标注 `PreLlmCall::RewriteInput` 必须保持 Prompt Cache 锁定字段
- §2.6：`DispatchResult` 增 `failures: Vec<HookFailureRecord>`；`HookFailureCause` / `InconsistentReason` / `TransportFailureKind` 三个判别枚举
- §2.6 串联规则补：同 priority `handler_id` 字典序 tiebreaker / RewriteInput 链尾 schema 校验 / AddContext 累计 64 KiB 上限 / OverridePermission "Deny 压过 Allow" 冲突合成
- §2.6.1 **新增** `HookFailureMode = FailOpen | FailClosed` 与默认值矩阵：UserControlled / User / Workspace 强制 `FailOpen`；FailClosed 路径串到 `ToolUseDenied { reason: HookFailClosed, handler_id }`
- §3.2 `HookExecSpec` 补 `working_dir: WorkingDir / resource_limits: HookExecResourceLimits / signal_policy: HookExecSignalPolicy / protocol_version: HookProtocolVersion`；shell metacharacter / env deny-list 约束明文化
- §3.3 `HookHttpSpec` 补 `security: HookHttpSecurityPolicy { allowlist, ssrf_guard, max_redirects, max_body_bytes, mtls } / protocol_version`；DNS rebinding 防护要求每次请求重新解析；UserControlled 必填非空 allowlist + ssrf_guard 全启用
- §3.4 **新增** 协议版本化：`HookProtocolVersion` 协商规则 + 与 ADR-001 schema 迁移流程对齐
- §5：错误类型枚举完整化（`Panicked / Inconsistent / Unsupported / Transport`）+ `TransportFailureKind`
- §8：可观测性指标补 `hook_failures_total{cause}` / `hook_chain_depth` / `hook_context_bytes_added` / `hook_permission_conflict_total`；replay 模式只前进不重演
- §11 **新增** Replay 与 Event Sourcing 语义专章：`ReplayMode = Live | Audit` + 幂等契约（纯函数化 / 不持久化外部副作用 / 不读易变全局态）+ Audit 模式不可见行为 + 与 ADR-003 Prompt Cache 协同

### 关键契约扩展（`extensibility.md`）

- §5.1 "18 类" → "20 类"，注释明示与 `harness-hook.md §2.2` 双向同步
- §5.2 `HookOutcome` enum 改为与 `harness-hook.md §2.3` 同款（含 `PreToolUse(PreToolUseOutcome)` / `Transform` 形态）；`HookHandler::priority` 显式声明
- §5.3 `PreToolUse` 行：`Continue / Block / PreToolUse(PreToolUseOutcome)`；独占改写通道补 `PreLlmCall::RewriteInput` 与 Prompt Cache 锁定要求
- §5.4 多 transport 字段同步至最新版 + `failure_mode` 矩阵

### 新增事件（`event-schema.md §3.7.2`）

- `Event::HookFailed`（cause_kind / failure_mode / fail_closed_denied）
- `Event::HookReturnedUnsupported`（returned_kind: HookOutcomeDiscriminant）
- `Event::HookOutcomeInconsistent`（reason: InconsistentReason）
- `Event::HookPanicked`（仅 in-process；含 message_snippet）
- `Event::HookPermissionConflict`（participants + winner + resolved_event_id）

### `harness-contracts` 增量

- §3.3 Event 枚举补 5 个 hook 事件 variant（`HookFailed / HookReturnedUnsupported / HookOutcomeInconsistent / HookPanicked / HookPermissionConflict`）
- §3.4 `DecidedBy::Hook` 注释同步：在 PreToolUse 事件下走 `PreToolUseOutcome { override_permission, .. }`，PermissionRequest 事件下走 `OverridePermission(Decision)`

### `api-contracts` 增量

- §10.1 `HookHandler` 输出能力描述同步为"PreToolUse 三件套唯一入口 + replay 幂等契约"；引用 §2.2.1 `HookContext` 定义
- §10.2 `HookTransport` 内置实现明确为 `in-process / exec / http`；`agent-bridge / wasm` 标为业务可叠加的扩展点

### 横向一致性补丁

- `harness-tool.md §2.8` 五个 hook 介入点的"允许动作"列同步为新版（PreToolUse 三件套 / PostToolUse 改 `Continue + AddContext`）；`failure_mode` 兜底引用补全
- `harness-permission.md §7.4` `OverridePermission` 引用按事件区分；补 `HookPermissionConflict` 冲突合成说明
- `permission-model.md §9` 示例代码完全重写：用 `#[async_trait] impl HookHandler { async fn handle(...) -> Result<HookOutcome, HookError> }` + 在 PreToolUse 用三件套、在 PermissionRequest 用单一 `OverridePermission`
- `adr/0006-plugin-trust-levels.md §2.2` 能力矩阵补 "声明 Hook `failure_mode = FailClosed`" 一行；HTTP Hook 行补"非空 allowlist + ssrf_guard 全启用"
- `overview.md` 18 类 → 20 类；F 槽位描述补三件套 / failure_mode / replay 幂等

### 受影响文档

- 重写局部：`crates/harness-hook.md`（§1 / §2 / §3 / §5 / §8 / §11；新增 §2.2.1 / §2.6.1 / §3.4 / §11）
- 增量：`extensibility.md §5.1-§5.4`、`event-schema.md §3.7 / §3.7.2`、`crates/harness-contracts.md §3.3 / §3.4 注释`、`api-contracts.md §10`、`crates/harness-tool.md §2.8`、`crates/harness-permission.md §7.4`、`permission-model.md §9`、`adr/0006-plugin-trust-levels.md §2.2`、`overview.md`

### 明确不引入（保持现状）

| 候选 | 不引入原因 |
|---|---|
| `HookOutcome::Block` 区分 `Hard / SoftFeedback` | 影响 contracts/event-schema/permission-model 多处 enum payload；`Block` 已表达硬终止，软反馈可由 `PostToolUseFailure + AddContext` 两步实现 |
| `OverridePermission { decision, rationale }` 双字段 | rationale 可经 `additional_context` 注入；先不破坏现有 enum 形状 |
| 新增 `PreCompact / PostCompact / PreRun / PostRun / PermissionDenied` 事件 | 跨 context-engineering / engine / permission-model / journal 多文档；非"漂移修复"范畴 |
| `PluginPreInstall / PluginInstalled` hook | 已有 `Event::PluginLoaded / PluginRejected` 审计；引入 hook 会带来"plugin 注册的 hook 不能审 plugin 自己"的循环依赖 |
| `HookTransport` 内置 `agent / wasm` | 维持作为开放扩展点；SDK 默认仅 in-process / exec / http 三类，避免依赖膨胀 |

### 向后兼容性

- v1.6 仍是**文档级**修订，无代码层兼容问题
- 对已按 v1.5 接口实现 PoC 的下游：`HookExecSpec` / `HookHttpSpec` 字段为新增字段（向后兼容；缺省值由 SDK 提供）；新事件 variant 用 `#[non_exhaustive]` 保护，消费者按 catch-all 即可
- 对已审阅 v1.5 的评审者：建议按 `harness-hook.md §2.2.1 / §2.6.1 / §3.4 / §11` → `event-schema.md §3.7.2` → `extensibility.md §5` → `permission-model.md §9` 顺序回看

## v1.7 变更摘要（2026-04-25）

### 触发动机

参照 `reference-analysis/{claude-code-sourcemap,openclaw,hermes-agent}.md` 与 `comparison-matrix.md`，对照已固化的 ADR-006 信任域二分与 v1.6 后的 `harness-plugin` SPEC，发现以下治理与契约缺口：

| 缺口 | 反映在 |
|---|---|
| `PluginActivationContext` 是大而全的 Registry 句柄包，与 manifest 声明的 capabilities 没有类型关联，插件可越权注册 | 与 ADR-006 / HER-035 反模式列表暗合但未在 SPEC 兜底 |
| Discovery / Runtime 加载耦合在 PluginRegistry 内部，"manifest-first 不得执行代码"硬约束只能靠 SPEC 文字保证，没有 trait / 类型层面的形态保护 | 与 §3.1 硬约束的语义同源 |
| `PluginManifest.signature` 指向"trusted_signers 列表"，但缺少启用窗口 / 撤销列表 / 来源 provenance 的运维语义；私钥泄露场景没有"立即生效"的撤销路径 | 与 ADR-006 § 2.4 的"AdminTrusted 必签"是同一治理面但未配套 |
| `security-trust.md §8.1` 的"Manifest validation failed" 必记事件在 contracts §3.3 没有对应 `Event` variant，造成 PluginRejected.reason 兜底膨胀（既要承载"业务规则拒"，又要兜底"manifest 没解析出来"） | 审计语义不清晰，且 `PluginId` 在解析失败时根本不存在 |

### 新增 ADR

- **ADR-014** · Plugin Manifest Signer 治理：`TrustedSignerStore` trait + `TrustedSigner` 启用窗口 / 撤销 / provenance；五步验签流程；与 ADR-013 IntegritySigner 完全独立的边界声明
- **ADR-015** · Plugin Loader 二分 + Capability-Scoped ActivationContext：`PluginManifestLoader` / `PluginRuntimeLoader` 类型层面把 §3.1 硬约束抬到 trait；`PluginActivationContext` 重构为 capability handle 集合，每个 handle 是 manifest 声明范围内的窄接口 trait

### 关键契约扩展（`harness-plugin.md`）

- §1：核心能力补"Loader 二分（§3.2 / ADR-0015）" / "Manifest signer 治理（§4.1 / ADR-0014）"；核心原则补 §5 "Manifest validation ≠ PluginRejected"
- §2.3 `PluginRegistry` 字段：`trusted_signers: Vec<PublicKey>` → `signer_store: Arc<dyn TrustedSignerStore>`；新增 `manifest_loaders` / `runtime_loaders` / `naming_policy` / `config` 字段
- §2.4 `PluginActivationContext` 整段重构为 capability handle 集合：`tools / hooks / mcp / skills / memory / coordinator` 均为 `Option<Arc<dyn *Registration>>`；6 个 `*Registration` 子 trait（窄接口）；类型层 + 运行期双向校验"声明 vs 实际注册"
- §3 重写为按 ManifestLoader 编排；保留 §3.1 硬约束并加注 trait 层保护
- §3.2 **新增** `PluginManifestLoader` / `PluginRuntimeLoader` trait + `ManifestRecord` / `ManifestOrigin` 数据结构 + 默认实现矩阵 6 行（File / CargoExtension / StaticLink / Dylib / CargoExtensionRuntime / Wasm）+ Builder 装配规则
- §4 `RejectionReason` 二分清理：移除 `ManifestValidationFailed` 子项（独立成 `Event::ManifestValidationFailed`），新增 `SignerRevoked`、`AdmissionDenied`；`ManifestValidator` 字段 `trusted_signers` → `signer_store`
- §4.1 **新增** `TrustedSignerStore` trait + `TrustedSigner` 结构 + 五步验签流程 + 与 ADR-013 边界声明 + `StaticTrustedSignerStore` 默认实现
- §5.5 **新增** `SignerId` 命名约定：`<provider>-<purpose>-<rev>` 三段式
- §13 错误根扩展：新增 `ManifestLoaderError / RuntimeLoaderError / SignerStoreError / RegistrationError`，补 `SignerRevoked / AdmissionDenied` variant
- §14 使用示例：新增"企业级"分支演示 `with_signer_store / with_manifest_loader / with_runtime_loader`；明示与 `with_trusted_signer` 的互斥规则
- §15 测试策略：补 "ManifestValidationFailed / Loader 二分 / CapabilityHandle 越权 / Signer 启用窗口 / Signer 撤销 / Signer 轮换"六类
- §16.1 必记审计事件：从 2 行扩为 3 行（新增 `Event::ManifestValidationFailed`）；§16.2 指标补 `plugin_manifest_validation_failed_total / plugin_signer_active_total / plugin_signer_revoked_total / plugin_capability_registration_rejected_total`
- §17 反模式扩展：新增 6 条（自定义 ManifestLoader 中执行子进程 / cargo extension 元数据子命令越界 / capability handle 越权注册 / 复用 IntegritySigner key 签 manifest / 撤销 signer 强制 deactivate / Builder 互斥规则违反）
- §18 相关引用：补 ADR-0013 / ADR-0014 / ADR-0015

### `harness-contracts` 增量

- §3.3 `Event` 枚举新增 1 个 variant：`ManifestValidationFailed(ManifestValidationFailedEvent)`，注释明示与 `PluginRejected` 的互斥边界

### 新增事件（`event-schema.md §3.20`）

- §3.20 **新增** "Plugin 生命周期事件" 整段：定义 `PluginLoadedEvent / PluginRejectedEvent / ManifestValidationFailedEvent` 三个事件载荷字段
- `ManifestValidationFailure` 五种判别枚举（`SyntaxError / SchemaViolation / UnsupportedSchemaVersion / CargoExtensionMetadataMalformed / RemoteIntegrityMismatch`）
- §2 总览补 "Plugin 生命周期" 类目
- §9.3 `DEFAULT_NEVER_DROP_KINDS` 补 `ManifestValidationFailed`

### 横向一致性补丁

- `extensibility.md §11.3` 流程描述按 ManifestLoader / Validator / RuntimeLoader 三段重写
- `security-trust.md §9.2` 加 TrustedSignerStore + 启用窗口 + 撤销 + 与 ADR-0013 边界声明；§8.1 必记事件补 `ManifestValidationFailed`
- `module-boundaries.md §6` 补"`harness-plugin ↔ Loader` 检查项"

### 受影响文档

- 新增：`adr/0014-plugin-manifest-signer.md`、`adr/0015-plugin-loader-capability-handles.md`
- 增量：`crates/harness-plugin.md`（§1 / §2.3 / §2.4 重构 / §3 重写 / §3.2 新增 / §4 / §4.1 新增 / §5.5 新增 / §13 / §14 / §15 / §16 / §17 / §18）、`crates/harness-contracts.md` §3.3、`event-schema.md` §2 / §3.20 新增 / §9.3、`security-trust.md` §8.1 / §9.2、`extensibility.md` §11.3、`module-boundaries.md` §6、`README.md`（本节 + ADR 索引 + 交叉引用矩阵）

### 明确不引入

| 候选 | 来源 | 不引入原因 |
|---|---|---|
| `PluginPreInstall / PluginInstalled` Hook | （部分参考实现） | 已有 `Event::PluginLoaded / Rejected / ManifestValidationFailed` 审计；引入 hook 会让"plugin 注册的 hook 不能审 plugin 自己"陷入循环依赖（与 v1.6 决议一致） |
| 把 `IntegritySigner` 与 `TrustedSignerStore` 合并 | （部分参考实现） | 一对称一非对称、一自签自验一上游签发；强行合并会让密钥泄露 blast radius 越界（ADR-0014 §2.1） |
| `Plugin` trait 拆 5 个 `activate_*` 方法 | （ADR-015 §3.2 替代方案） | Plugin trait 实现侧要写一堆 `default { Ok(...) }`；与 manifest "声明 → 检查"模型不一致 |
| 撤销 signer 时强制下线 Activated 插件 | （部分 PKI 实践） | 线上抖动；ADR-0014 §2.5 决议下次 Discovery 重判 |
| 多版本同名 Plugin 共存 | （部分参考实现） | 与 §9 现状一致：当前 SDK 拒绝；解决方式由业务层选择（升级 / 卸载 / 分离 Workspace） |

### 向后兼容性

- v1.7 仍是**文档级**修订，无代码层兼容问题
- 对已按 v1.6 接口理解 `PluginActivationContext` 的评审者：字段形态发生破坏性变化（Registry 句柄包 → capability handle 集合）；Plugin trait 签名不变；现有 `with_trusted_signer` API 保留，新 `with_signer_store / with_manifest_loader / with_runtime_loader` 平行存在
- 对已审阅 v1.6 的评审者：建议按 `adr/0014-plugin-manifest-signer.md` → `adr/0015-plugin-loader-capability-handles.md` → `crates/harness-plugin.md`（§1 / §2.3 / §2.4 / §3.2 / §4.1 / §13 / §14 / §16.1 / §17）→ `event-schema.md §3.20` → `security-trust.md §9.2` → `extensibility.md §11.3` 顺序回看；`harness-contracts.md` 仅看 §3.3 增量段

## v1.8 变更摘要（2026-04-25）

### 触发动机

参照 `reference-analysis/{claude-code-sourcemap,openclaw,hermes-agent}.md` 与
`comparison-matrix.md` 对 8 项剩余优秀设计点（A-H）的逐项评估，得出三类结论：

| 类别 | 来源点 | 处置 |
|---|---|---|
| 已吸收（v1.3-v1.7 落地） | C 流式 ToolResult Preview+Handle / D Skill Inline 模板 / G Provider Adapter Hook 网格 / H 审批粒度 | 仅在 `comparison-matrix.md` 标记 ✅，无新 ADR |
| 真实缺口 | A Programmatic Tool Calling / B Steering Queue / E ToolResult 结构化白名单 | 新增 ADR-0016 / ADR-0017 + ADR-0002 升级 |
| 反向决议 | F Loop-Intercepted Tools | 新增 ADR-0018 形式化拒绝 |

### 新增 ADR

- **ADR-0016** · Programmatic Tool Calling（`execute_code` 元工具）：让主 Agent 在
  单次推理中通过受限脚本（MiniLua 子集）编排多次工具调用，把 N 工具调用 = N 次推理
  的弊端压成 1 次。引入 `CodeRuntime` / `EmbeddedToolDispatcher` 两个新 capability、
  默认 7 个只读嵌入式工具白名单（read / read_blob / list_dir / grep / web_search /
  glob / find）、`Event::ExecuteCodeStepInvoked` / `Event::ExecuteCodeWhitelistExtended`
  两个事件、Subagent 双层闸门（blocklist + 调用链检查）、与 ADR-0010 ResultBudget
  叠加预算口径、与 ADR-0003 PromptCache 锁定字段不交叉。
- **ADR-0017** · Steering Queue / In-flight User Messages：会话级软引导队列；主循环
  在每轮 `model.infer` 前 `drain_and_merge()`，把用户的"补充想法 / 矫正"合并进下
  一轮 user prompt，避免硬中断当前推理。三种 `SteeringKind`（Append / Replace /
  NudgeOnly）+ `SteeringPolicy`（capacity 8 / ttl 60s / DropOldest / dedup_window 1500ms）
  + 三个事件（Queued / Applied / Dropped）+ Plugin 来源默认 deny 与 manifest
  capability gate。
- **ADR-0018** · No Loop-Intercepted Tools（反向决议）：明确**不**引入 Hermes 风格
  的"循环拦截工具"模式，理由是该模式与 Octopus "Tool 是一等公民" + ToolCapability
  Handle（ADR-0011） + 统一 ToolOrchestrator 流水线已完成的能力面**全程重叠**，
  再叠加会让循环控制权双轨化、审计语义混乱；现有 `BashTool` / `ListDir` / 五槽
  Hook + capability handle 已等价覆盖所有用例。

### 关键契约扩展（`harness-contracts.md`）

- §3.2 ID 类型：新增 `SteeringScope` + `SteeringId = TypedUlid<SteeringScope>`
- §3.3 Event：新增 5 个 variant：
  - `SteeringMessageQueued` / `SteeringMessageApplied` / `SteeringMessageDropped`
  - `ExecuteCodeStepInvoked` / `ExecuteCodeWhitelistExtended`
- §3.4 共享枚举：
  - `DecisionScope` 新增 `ExecuteCodeScript { script_hash: [u8; 32] }`
  - `ToolCapability` 新增 `CodeRuntime` / `EmbeddedToolDispatcher`
  - 新增 §3.4.3 Steering 软引导共享类型：`SteeringMessage` / `SteeringKind` /
    `SteeringBody` / `SteeringPriority` / `SteeringSource` / `SteeringPolicy` /
    `SteeringOverflow`
- §3.5 消息：`ToolResultPart` / `ReferenceKind` 整段正向白名单化（替代原"反向
  黑名单"单边定义）：
  - `Text / Structured / Blob / Code / Reference / Table / Progress / Error`
  - `ReferenceKind = Url / File / Transcript / ToolUse / Memory`

### `harness-engine` 主循环增量

主循环在 "iteration_budget / interrupt_token 检查" 与 "model.infer(prompt)" 之间
新增 `steering.drain_and_merge()` 阶段（ADR-0017 §2.5）。仅当本轮产生新的 user
消息时才触发 `Hook::UserPromptSubmit`；Hook 链尾仍按 ADR-0003 守住 prompt cache
锁定字段。

### `harness-session` 增量

- §2.1 `Session` 新增方法：
  ```rust
  pub async fn push_steering(&self, msg: SteeringRequest) -> Result<SteeringId, SessionError>;
  pub fn steering_snapshot(&self) -> SteeringSnapshot;
  #[cfg(feature = "testing")]
  pub fn cancel_steering(&self, id: SteeringId) -> Result<(), SessionError>;
  ```
- §2.7 **新增** `SteeringQueue`：数据结构 / `SessionBuilder::with_steering_policy` /
  `push_steering` 规则 / `drain_and_merge` 规则 / 与 `RunEnded / reload_with /
  compact / fork` 的交互
- §3 `SessionInner` 加 `steering_queue: Arc<SteeringQueue>` 字段

### `harness-tool` 增量

- §4.7 **新增** `ExecuteCodeTool`：descriptor / 执行流水线（脚本编译 → 嵌入式
  dispatcher 注入 → CodeSandbox 执行 → 步级事件落库 → 与 ResultBudget 累加） /
  `EmbeddedToolWhitelist`（7 个 read-only built-in 默认 + 业务扩展规则） / MiniLua
  受限语法 / 与 ToolRegistry 的注册规则

### `harness-sandbox` 增量

- §3.5 **新增** `CodeSandbox` trait：与 `SandboxBackend` 形成两条独立 trait（前者
  执行受限脚本无 OS syscall，后者执行进程命令）；`CodeSandboxCapabilities` 配额面
  （max_instructions / call_depth / wall_clock / deterministic）；`MiniLuaCodeSandbox`
  默认实现

### `harness-subagent` 增量

- §2.5 Default Policy 新增 "execute_code 的延续闸门" 节：明文写出双层保护
  （Subagent blocklist 静态拦截 + ToolOrchestrator 调用链动态检查）

### `adr/0002-tool-no-ui.md` 升级

- §6 实现指引升级为"正向白名单（§6.1）+ 反向黑名单（§6.2）+ 强制守卫（§6.3）"
  三段式；§6.1 引用 `harness-contracts §3.5` 作为唯一权威来源

### 新增 / 调整事件（`event-schema.md`）

- §3.5.1 **新增** Steering 事件族：
  - `SteeringMessageQueuedEvent`（id / source / kind / priority / scheduled_at /
    visible_at / ttl）
  - `SteeringMessageAppliedEvent`（ids / merged_into_message_id / applied_kind）
  - `SteeringMessageDroppedEvent { reason: SteeringDropReason }`
    - `SteeringDropReason = Capacity / TtlExpired / DedupHit / RunEnded /
      SessionEnded / PluginDenied`（权威定义在 `event-schema.md §3.5.1`）
- §3.5.2 **新增** ExecuteCode 事件族：
  - `ExecuteCodeStepInvokedEvent`（script_hash / step_index / embedded_tool /
    embedded_refused_reason / instructions_used）
  - `EmbeddedRefusedReason = NotInWhitelist / Destructive / RequiresHumanInLoop /
    SubagentCallerChain / TrustLevelInsufficient`
  - `ExecuteCodeWhitelistExtendedEvent`（added_tools / decided_by / reason）

### Feature Flags（`feature-flags.md`）

- `harness-sdk` features：
  - 新增 `programmatic-tool-calling`（默认 **off**，M1 期）
  - 新增 `steering-queue`（默认 **on**，M0 期）
- 各内部 crate features：
  - `harness-sandbox` 增 `code-runtime`
  - `harness-tool` 增 `programmatic-tool-calling`
  - `harness-session` / `harness-engine` 增 `steering`
- §3.5 default 集合理由：补 `steering-queue 默认开 / programmatic-tool-calling 默认关` 两条
- §4.2 依赖：补两个 feature 的内部 crate 依赖矩阵

### `api-contracts.md` 增量

- §12.1.1 **新增** "软引导 API（ADR-0017）"：`Session::push_steering /
  steering_snapshot` 签名 / observability / capability gate / 失败模式

### `agents-design.md` 增量

- §3.7 **新增** "主 Agent 与 execute_code 的协作（ADR-0016）"：何时该走 PTC
  / 系统提示模板示例 / 事件轨迹示例（ExecuteCodeStepInvoked × N 串联，与
  ToolUseStarted/Completed 互斥）

### `security-trust.md` 增量

- §10.Y **新增** "execute_code 安全考量（ADR-0016）"：风险矩阵 / 三层 kill switch
  （feature flag + capability deny + admin runtime kill）/ 审计保证
- §10.Z **新增** "Steering Queue 安全考量（ADR-0017）"：风险矩阵 / 双层 kill
  switch / 审计保证
- §13 默认值汇总表补四行（PTC feature / Steering policy / EmbeddedToolWhitelist /
  SteeringSource::Plugin 默认 deny）

### `extensibility.md` 增量

- §3.7 **新增** "让业务工具被 `execute_code` 调用（ADR-0016）"：5 项准入条件
- §7.1 **新增** "`CodeSandbox` 扩展（ADR-0016）"：trait 形态 / 业务扩展硬约束
  5 条 / 反模式（用 `LocalSandbox` 包装 `CodeSandbox`）

### `comparison-matrix.md` 回填

- R-04 / R-05 / R-06 / R-07 / R-08：标记为已在 v1.3-v1.7 中吸收
- R-09 / R-10 / R-11：分别对应 PTC / Steering / ToolResult 结构化，链回 ADR-0016 /
  ADR-0017 / ADR-0002 升级
- R-12：F 反向决议，链回 ADR-0018

### 受影响文档

- 新增：`adr/0016-programmatic-tool-calling.md` / `adr/0017-steering-queue.md` /
  `adr/0018-no-loop-intercepted-tools.md`
- 增量重写：`adr/0002-tool-no-ui.md`（§6 升级）
- 增量：`crates/harness-contracts.md`（§3.2 / §3.3 / §3.4 / §3.5）、
  `crates/harness-engine.md`（§3 主循环）、`crates/harness-session.md`
  （§2.1 / §2.7 / §3）、`crates/harness-tool.md`（§4.7）、`crates/harness-sandbox.md`
  （§3.5）、`crates/harness-subagent.md`（§2.5）、`event-schema.md`（§3.5.1 /
  §3.5.2）、`feature-flags.md`（§2.1 / §2.2 / §3.5 / §4.2）、`api-contracts.md`
  （§12.1.1）、`agents-design.md`（§3.7）、`security-trust.md`（§10.Y / §10.Z /
  §13）、`extensibility.md`（§3.7 / §7.1）、`comparison-matrix.md`、`README.md`
  （本节 + ADR 索引 + 交叉引用矩阵）

### 明确不引入

| 候选 | 来源 | 不引入原因 |
|---|---|---|
| Loop-Intercepted Tools（"循环拦截工具"） | Hermes / 部分 Claude Code 模式 | 与 ToolCapability Handle + 统一 Orchestrator + 五槽 Hook 全程重叠；引入会让循环控制权双轨化（ADR-0018） |
| 让 `execute_code` 默认放开写工具白名单 | （部分实现的便利做法） | 与 ADR-0007 审批边界冲突；任何写动作都必须走 ToolOrchestrator + Permission 全链路（ADR-0016 §2.6） |
| Subagent 也允许调用 `execute_code` | （部分实现） | 嵌入式 dispatcher 会让 subagent 单轮编排多写动作，与 ADR-0004 拓扑约束冲突；双层闸门拒绝 |
| 让 SteeringMessage 直接改 system prompt | （部分参考实现） | 破坏 ADR-0003 prompt cache 锁定字段；Steering 仅作 user 消息侧合并 |
| Steering 默认 BackPressure / DropNewest | （部分参考实现） | 默认策略偏向"留新弃旧"，DropOldest 在用户连续输入场景下语义最自然；其它策略可显式配置 |

### 向后兼容性

- v1.8 仍是**文档级**修订，无代码层兼容问题
- `programmatic-tool-calling` 默认 off：M1 期前业务方需显式启用 + 显式赋予
  `CodeRuntime` / `EmbeddedToolDispatcher` 两 capability，否则 `ExecuteCodeTool`
  根本不进 `ToolRegistry`
- `steering-queue` 默认 on：未声明 `with_steering_policy` 的 Session 自动获得
  `SteeringPolicy::default()`；Session API 表面**只新增方法**，不破坏现有签名
- `ToolResultPart` 现行 8 个变体均覆盖 v1.7 已隐含使用的子集，无 enum payload
  破坏；新增 `Code / Reference / Table / Progress / Error` 是显式化原"被嵌入
  Structured.value"的子结构
- 对已审阅 v1.7 的评审者：建议按 ADR-0016 → ADR-0017 → ADR-0018 → ADR-0002
  §6 升级 → `harness-contracts.md` §3.4 / §3.5 → `harness-engine.md §3` → `event-schema.md §3.5.1 / §3.5.2` → `feature-flags.md` 顺序回看
