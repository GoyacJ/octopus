# D4 · Event Schema 与 Replay 语义

> 依赖 ADR：ADR-001（Event Sourcing）, ADR-007（权限决策事件化）
> 状态：Accepted · Event 结构变更必须视为破坏性修改

## 1. 设计原则

1. **Append-Only**：所有 Event 只追加，**不得修改已写入的 Event**
2. **单调序号**：每个 `session_id` 的 Event 按 `JournalOffset` 严格单增
3. **自洽性**：给定 Event 序列，可以**重建完整 Session Projection**
4. **因果可追溯**：每个 Event 携带 `causation_id`（触发它的上游事件）和 `correlation_id`（同一 Turn 的一组事件）
5. **幂等性**：同一 Event 多次应用到 Projection，结果**相同**

## 2. Event 总览

所有 Event 统一为一个 enum（`harness-contracts::Event`）。本表必须与
`crates/harness-contracts.md §3.3` 的 Accepted `Event` enum 一一对齐。

| 分类 | 完整变体 |
|---|---|
| **Session 生命周期** | `SessionCreated / SessionForked / SessionEnded / SessionReloadRequested / SessionReloadApplied` |
| **Run 执行** | `RunStarted / RunEnded / GraceCallTriggered` |
| **消息流** | `UserMessageAppended / AssistantDeltaProduced / AssistantMessageCompleted` |
| **工具调用** | `ToolUseRequested / ToolUseApproved / ToolUseDenied / ToolUseCompleted / ToolUseFailed / ToolUseHeartbeat / ToolResultOffloaded / ToolRegistrationShadowed` |
| **权限审批** | `PermissionRequested / PermissionResolved / PermissionPersistenceTampered / PermissionRequestSuppressed` |
| **Hook** | `HookTriggered / HookRewroteInput / HookReturnedAdditionalContext / HookFailed / HookOutcomeInconsistent / HookReturnedUnsupported / HookPanicked / HookPermissionConflict` |
| **Steering 软引导** | `SteeringMessageQueued / SteeringMessageApplied / SteeringMessageDropped` |
| **ExecuteCode（PTC）** | `ExecuteCodeStepInvoked / ExecuteCodeWhitelistExtended` |
| **上下文** | `CompactionApplied / ContextBudgetExceeded / ContextStageTransitioned` |
| **MCP** | `McpToolInjected / McpConnectionLost / McpConnectionRecovered / McpElicitationRequested / McpElicitationResolved / McpToolsListChanged / McpResourceUpdated / McpSamplingRequested` |
| **Tool Search** | `ToolDeferredPoolChanged / ToolSearchQueried / ToolSchemaMaterialized` |
| **Subagent** | `SubagentSpawned / SubagentAnnounced / SubagentTerminated / SubagentSpawnPaused / SubagentPermissionForwarded / SubagentPermissionResolved` |
| **Team** | `TeamCreated / TeamMemberJoined / TeamMemberLeft / TeamMemberStalled / AgentMessageSent / AgentMessageRouted / TeamTurnCompleted / TeamTerminated` |
| **Memory** | `MemoryUpserted / MemoryRecalled / MemoryRecallDegraded / MemoryRecallSkipped / MemoryThreatDetected / MemdirOverflow / MemoryConsolidationRan` |
| **Sandbox** | `SandboxExecutionStarted / SandboxExecutionCompleted / SandboxActivityHeartbeat / SandboxActivityTimeoutFired / SandboxOutputSpilled / SandboxBackpressureApplied / SandboxSnapshotCreated / SandboxContainerLifecycleTransition` |
| **Plugin 生命周期** | `PluginLoaded / PluginRejected / ManifestValidationFailed` |
| **可观测性** | `UsageAccumulated / TraceSpanCompleted` |
| **凭证池审计** | `CredentialPoolSharedAcrossTenants` |
| **错误** | `EngineFailed / UnexpectedError` |

## 3. 核心 Event 详表

### 3.0 Session 生命周期（SessionCreated / SessionForked / SessionEnded）

```rust
pub struct SessionCreatedEvent {
    pub session_id: SessionId,
    pub tenant_id: TenantId,
    pub options_hash: [u8; 32],
    pub snapshot_id: SnapshotId,
    pub effective_config_hash: ConfigHash,
    pub created_at: DateTime<Utc>,
}

pub struct SessionForkedEvent {
    pub parent_session_id: SessionId,
    pub child_session_id: SessionId,
    pub tenant_id: TenantId,
    pub fork_reason: ForkReason,
    pub from_offset: JournalOffset,
    pub config_delta_hash: Option<DeltaHash>,
    pub cache_impact: CacheImpact,
    pub at: DateTime<Utc>,
}

/// 会话血缘原因（fork & compaction 共用）。
///
/// **重要**：本枚举同时被 `harness-journal::CompactionLineage.reason` 引用——
/// 父→子 session 关联与 fork 的语义在 octopus 中是同一棵树（HER-023），
/// 不再为 `CompactionLineage` 单独维护 `CompactReason`。
///
/// 变体语义：
/// - `UserRequested`：用户显式发起的 fork / compaction（`/compact`、UI Fork 按钮）。
/// - `Compaction`：自动 compaction（context budget 触发）。判断"是否压缩 fork"
///   还应看父 session 同 envelope 的 `Event::RunEnded { reason: Compacted }`。
/// - `HotReload`：`ConfigDelta` 破坏性变更触发的 fork（ADR-003）。
/// - `Isolation`：主动隔离（subagent / 危险操作旁路）。
/// - `RetryFromCheckpoint(offset)`：从快照点重放生成新分支。
pub enum ForkReason {
    UserRequested,
    Compaction,
    HotReload,
    Isolation,
    RetryFromCheckpoint(JournalOffset),
}

pub struct SessionEndedEvent {
    pub session_id: SessionId,
    pub tenant_id: TenantId,
    pub reason: EndReason,
    pub final_usage: UsageSnapshot,
    pub at: DateTime<Utc>,
}
```

`CacheImpact` 定义见 `crates/harness-session.md` §2.4。

`EndReason` / `CancelInitiator` 见 `crates/harness-contracts.md §3`。
**审计契约**：`EndReason::Cancelled { initiator }`（主动取消，有发起方）与 `EndReason::Interrupted`（被动终止，无发起方）在 SDK 层面不可互替；业务层 UI / 复盘报表按 `initiator` 维度（User / Parent / System）区分。`harness-engine.md §5`（中断节末尾的"`RunEnded.reason` 触发源选择表"）给出完整映射。

### 3.1 RunStarted

```rust
pub struct RunStartedEvent {
    pub run_id: RunId,
    pub session_id: SessionId,
    pub tenant_id: TenantId,
    pub parent_run_id: Option<RunId>,
    pub input: TurnInput,
    pub snapshot_id: SnapshotId,
    pub effective_config_hash: ConfigHash,
    pub started_at: DateTime<Utc>,
    pub correlation_id: CorrelationId,
}
```

### 3.1.1 GraceCallTriggered

迭代预算耗尽前一轮（`current_iteration == max_iterations - 1` 且 `grace_enabled = true`）由 Engine 主动写入；标志 Engine 进入"最后一次让 Assistant 收尾、不再允许发新 tool_call"的特殊态。

```rust
pub struct GraceCallTriggeredEvent {
    pub run_id: RunId,
    pub session_id: SessionId,
    pub tenant_id: TenantId,
    /// 当前迭代序号（1-indexed），等于 `max_iterations - 1`
    pub current_iteration: u32,
    pub max_iterations: u32,
    /// 触发时已累计的 token / cost 用量快照（用于复盘）
    pub usage_snapshot: UsageSnapshot,
    pub at: DateTime<Utc>,
    pub correlation_id: CorrelationId,
}
```

**Replay 行为**：纯审计事件；`SessionProjection::apply` 不改变状态，仅追加到 `grace_history` 列表（业务复盘用）。

**与 `RunEnded` 的关系**：`GraceCallTriggered` 永远 **早于** 同 Run 的 `RunEnded`；若 grace call 触发后下一轮 Assistant 仍发起 tool_call，Engine 写 `RunEnded { reason: MaxIterationsReached }`；若 Assistant 正常收尾，写 `RunEnded { reason: Completed }`。两者通过同一 `correlation_id` 串联。

### 3.2 UserMessageAppended

```rust
pub struct UserMessageAppendedEvent {
    pub run_id: RunId,
    pub message_id: MessageId,
    pub content: MessageContent,
    pub metadata: MessageMetadata,
    pub at: DateTime<Utc>,
}

pub enum MessageContent {
    Text(String),
    Multimodal(Vec<ContentPart>),
}

pub enum ContentPart {
    Text(String),
    Image { mime: String, data: BlobRef },
    Audio { mime: String, data: BlobRef },
    File { name: String, mime: String, data: BlobRef },
}
```

### 3.3 AssistantDeltaProduced

```rust
pub struct AssistantDeltaProducedEvent {
    pub run_id: RunId,
    pub message_id: MessageId,
    pub delta: DeltaChunk,
    pub at: DateTime<Utc>,
}

pub enum DeltaChunk {
    Text(String),
    /// Thinking / Reasoning 增量。结构化字段保留 provider-native 数据，
    /// 用于下一轮 cache-safe 回灌（详见 `crates/harness-model.md` §2.2 `ThinkingDelta`）。
    Thought(ThoughtChunk),
    ToolUseStart { tool_use_id: ToolUseId, tool_name: String },
    ToolUseInputDelta { tool_use_id: ToolUseId, delta: String },
    ToolUseEnd { tool_use_id: ToolUseId },
}

pub struct ThoughtChunk {
    pub text: Option<String>,
    pub provider_id: String,
    pub provider_native: Option<Value>,
    pub signature: Option<String>,
}

impl From<String> for ThoughtChunk {
    /// 兼容旧 `Thought(String)` 事件的反序列化路径
    fn from(text: String) -> Self {
        Self {
            text: Some(text),
            provider_id: "legacy".into(),
            provider_native: None,
            signature: None,
        }
    }
}
```

### 3.4 AssistantMessageCompleted

```rust
pub struct AssistantMessageCompletedEvent {
    pub run_id: RunId,
    pub message_id: MessageId,
    pub content: MessageContent,
    pub tool_uses: Vec<ToolUseSummary>,
    pub usage: UsageSnapshot,
    /// 当次推理结算所用的不可变定价快照。`None` 表示 Pricing 不可用
    /// （本地 Provider / Mock）；详见 `crates/harness-model.md` §2.1.1 R-P1。
    pub pricing_snapshot_id: Option<PricingSnapshotId>,
    pub stop_reason: StopReason,
    pub at: DateTime<Utc>,
}

pub enum StopReason {
    EndTurn,
    ToolUse,
    MaxIterations,
    Interrupt,
    Error(String),
}
```

> **关于 thinking 块的持久化（cache-safe replay）**
>
> `AssistantMessageCompletedEvent` 本身**不直接**携带 `ThinkingBlock`，而是通过：
>
> 1. 流式期间所有 `AssistantDeltaProduced.delta = DeltaChunk::Thought(ThoughtChunk)` 事件按序写入 Journal；
> 2. `MessageContent::Multimodal` 中可包含 `ContentPart::Thinking(ThinkingBlock)`（`harness-contracts` §3.5）。
>
> Replay/重建时，`SessionProjection` 在收到 `AssistantMessageCompleted` 后，回扫该 `message_id` 期间的所有 `Thought` deltas，按 `provider_id` 聚合为 `ThinkingBlock` 序列，附加到 `Message.parts`，使 cache-safe 续接成为可能（见 `crates/harness-model.md` §2.2 `ThinkingDelta`）。
>
> 该重建步骤是**纯函数**，不引入新的事件类型，也不破坏"events are the source of truth"原则。

### 3.5 ToolUseRequested / Approved / Denied / Completed / Failed

```rust
pub struct ToolUseRequestedEvent {
    pub run_id: RunId,
    pub tool_use_id: ToolUseId,
    pub tool_name: String,
    pub input: Value,
    pub properties: ToolProperties,
    pub causation_id: EventId,
    pub at: DateTime<Utc>,
}

pub struct ToolUseApprovedEvent {
    pub tool_use_id: ToolUseId,
    pub decision_id: DecisionId,
    pub scope: DecisionScope,
    pub at: DateTime<Utc>,
}

pub struct ToolUseDeniedEvent {
    pub tool_use_id: ToolUseId,
    pub reason: DenyReason,
    pub at: DateTime<Utc>,
}

pub struct ToolUseCompletedEvent {
    pub tool_use_id: ToolUseId,
    pub result: ToolResult,
    pub usage: Option<UsageSnapshot>,
    pub duration_ms: u64,
    pub at: DateTime<Utc>,
}

pub struct ToolUseFailedEvent {
    pub tool_use_id: ToolUseId,
    pub error: ToolErrorPayload,
    pub at: DateTime<Utc>,
}

/// 长任务存活信号（ADR-010 §2.7 + harness-tool §2.7）。
///
/// 来源：`ToolOrchestrator` 在 `LongRunningPolicy.stall_threshold` 触发时
/// 自动注入；不计入 `ResultBudget`，仅做 UI/监控用途。
pub struct ToolUseHeartbeatEvent {
    pub tool_use_id: ToolUseId,
    pub run_id: RunId,
    /// 工具自报的进度文本（自动注入时为 "still running…"）
    pub message: String,
    /// 进度百分比，工具未给出时为 None
    pub fraction: Option<f32>,
    /// 自上一次 ToolEvent（任意类型）以来的静默时长
    pub silent_for_ms: u64,
    pub at: DateTime<Utc>,
}

/// 工具结果命中预算并落盘（ADR-010 §2.4）。
///
/// 与 `ToolUseCompletedEvent` 同时写入：前者承载 envelope（含 head/tail 预览），
/// 本事件提供"完整原文已落盘"的元数据，供审计与 `read_blob` 查询。
pub struct ToolResultOffloadedEvent {
    pub tool_use_id: ToolUseId,
    pub run_id: RunId,
    pub blob_ref: BlobRef,
    pub original_metric: BudgetMetric,
    pub original_size: u64,
    pub effective_limit: u64,
    pub head_chars: u32,
    pub tail_chars: u32,
    pub at: DateTime<Utc>,
}

/// Registry 同名工具裁决遮蔽（harness-tool §2.5.1）。
///
/// 任一同名注册被裁决拒绝时写入；`reason` 取自
/// `harness-tool::ShadowReason`（`BuiltinWins / HigherTrust / Duplicate`）。
pub struct ToolRegistrationShadowedEvent {
    pub tool_name: String,
    /// 当前保留的来源（决策赢家）
    pub kept: ToolOrigin,
    /// 被遮蔽的来源（决策输家）
    pub rejected: ToolOrigin,
    pub reason: ShadowReason,
    /// 触发本次裁决的注册请求的内部因果 ID（来自 PluginRegistry / McpRegistry / Skill 等）
    pub causation_id: Option<EventId>,
    pub at: DateTime<Utc>,
}
```

> `BudgetMetric` / `ToolOrigin` / `ShadowReason` 定义在 `harness-contracts` §3.4；
> `BlobRef` 定义在 `harness-contracts` §3.6。

### 3.5.1 Steering 事件族（ADR-0017）

> 软引导队列三件套：`SteeringMessageQueued / Applied / Dropped`，与
> `Session::push_steering(...)` 入队、Engine 主循环 Safe Merge Point drain 一一对应。
> `SteeringId / SteeringKind / SteeringPriority / SteeringSource / SteeringBody`
> 定义在 `harness-contracts §3.4.3`。

```rust
/// 软引导消息已成功入队（push_steering 成功）。
pub struct SteeringMessageQueuedEvent {
    pub id: SteeringId,
    pub session_id: SessionId,
    pub run_id: Option<RunId>,
    pub kind: SteeringKind,
    pub priority: SteeringPriority,
    pub source: SteeringSource,
    /// `body` 的稳定指纹（blake3）；用于 dedup 与 Replay 比对。
    /// 完整 `body` 不进事件——超过 `SteeringPolicy` 上限时由 BlobStore 落盘。
    pub body_hash: [u8; 32],
    /// `body` 字节长度；UI 据此渲染 "x KB"。
    pub body_size: u32,
    /// `body_size > inline_threshold`（默认 8 KiB）时，原文落盘的 BlobRef。
    pub body_blob: Option<BlobRef>,
    pub correlation_id: Option<CorrelationId>,
    pub at: DateTime<Utc>,
}

/// 软引导消息已在 Safe Merge Point 合并到下一轮 user 消息（或写入 EventStore）。
pub struct SteeringMessageAppliedEvent {
    /// 本次 drain 一并合并的消息 Id 列表（保持 FIFO 顺序）。
    pub ids: Vec<SteeringId>,
    pub session_id: SessionId,
    pub run_id: RunId,
    /// `Append / Replace` 命中时的合成 user 消息 Id；`NudgeOnly` 全集为 `None`。
    pub merged_into_message_id: Option<MessageId>,
    /// 各 `SteeringKind` 的命中分布（便于审计与 metric label）。
    pub kind_distribution: BTreeMap<SteeringKind, u32>,
    pub at: DateTime<Utc>,
}

/// 软引导消息被丢弃；reason 与 `SteeringPolicy` / RunEnded / SessionEnd 路径对应。
pub struct SteeringMessageDroppedEvent {
    pub id: SteeringId,
    pub session_id: SessionId,
    pub run_id: Option<RunId>,
    pub reason: SteeringDropReason,
    pub at: DateTime<Utc>,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum SteeringDropReason {
    /// 容量已满，按 `SteeringOverflow::DropOldest / DropNewest` 丢弃。
    Capacity,
    /// 单条消息停留时间超过 `SteeringPolicy.ttl`。
    TtlExpired,
    /// `dedup_window` 内命中相同 `body_hash`（priority != High）。
    DedupHit,
    /// `Event::RunEnded` 后队列残留消息因 ttl 自然过期或 priority == NudgeOnly。
    RunEnded,
    /// `Session::end(...)` 触发的强制清空。
    SessionEnded,
    /// `SteeringSource::Plugin` 未声明 capability 或非 AdminTrusted 而被拒绝。
    PluginDenied,
}
```

> **设计要点**：
>
> 1. `body` 不直接进事件 —— 即便短文本也仅落 `body_hash + body_size`，长文本走
>    `body_blob`，与 ADR-0010 ResultBudget 同模式
> 2. `kind_distribution` 仅在 `Applied` 写出；`Queued` 与 `Dropped` 都只携带单条
>    消息身份，便于 metric 按 reason 分桶
> 3. 三件套与 `ADR-0007 Permission Events` 同观测面 —— UI 可同时消费、按 `session_id`
>    时间线渲染软引导与权限审批

### 3.5.2 ExecuteCode 事件族（ADR-0016）

> `execute_code` 元工具的内嵌步骤事件 + 白名单扩展事件；
> `parent_tool_use_id` 指向外层 `Event::ToolUseRequested.tool_use_id`，与
> `Event::ToolUseCompleted` 形成可追溯链。

```rust
/// `execute_code` 脚本一次嵌入式工具调用的审计事件。
///
/// 与外层 `ToolUseRequested / ToolUseCompleted` 通过 `parent_tool_use_id` 串联；
/// 每次内嵌调用本身仍走 ToolOrchestrator 五介入点（PreToolUse / Permission /
/// TransformToolResult / PostToolUse），各介入点会写出独立的 Hook / Permission 事件。
pub struct ExecuteCodeStepInvokedEvent {
    pub parent_tool_use_id: ToolUseId,
    pub run_id: RunId,
    pub session_id: SessionId,
    /// 内嵌工具名（白名单内）。
    pub embedded_tool: String,
    /// 内嵌调用的稳定指纹（blake3 of canonical args），用于 dedup 关联。
    pub args_hash: [u8; 32],
    /// 步骤序号（同一 `parent_tool_use_id` 内单调递增，1-based）。
    pub step_seq: u32,
    /// 本步耗时（仅嵌入工具自身；不含 dispatcher 校验开销）。
    pub duration_ms: u64,
    /// 嵌入工具的 budget 命中状态（同 ADR-0010）；overflow 时由父 envelope 收敛。
    pub overflow: Option<OverflowMetadata>,
    /// 步骤被 dispatcher 拒绝时的原因（白名单不命中 / 自递归 / capability 拒绝）。
    /// `None` 表示步骤被正常分发并执行。
    pub refused_reason: Option<EmbeddedRefusedReason>,
    pub at: DateTime<Utc>,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum EmbeddedRefusedReason {
    /// 工具名不在 `EmbeddedToolWhitelist`。
    NotWhitelisted,
    /// 脚本试图反射调用 `execute_code` 自身。
    SelfReentrant,
    /// `EmbeddedToolDispatcher` capability 缺失（装配漏配置）。
    CapabilityDenied,
    /// 工具属性违反 ADR-0016 §2.6（`is_destructive == true` / 信任级不足）。
    PropertyViolation,
    /// PermissionBroker 拒绝。
    PermissionDenied,
}

/// 业务在 `team_config.toml` 中显式扩展 `execute_code` 嵌入工具白名单。
///
/// `source` 始终是配置来源（M0 仅支持 `"team_config"`）；运行期不可经 API 修改，
/// 任何热更新都必须通过 `Session::reload_with(ConfigDelta::ReloadTeamConfig)`。
pub struct ExecuteCodeWhitelistExtendedEvent {
    pub session_id: SessionId,
    /// 本次 reload 后新增的工具名（与默认 7 个 read-only 不重叠）。
    pub added: Vec<String>,
    pub source: String,
    pub at: DateTime<Utc>,
}
```

> **设计要点**：
>
> 1. 步骤事件**只**携带摘要（`args_hash` / `duration_ms` / `overflow`），完整
>    内容由内嵌工具自身的 `ToolUseCompleted` 承担，避免双写
> 2. `step_seq` 由 dispatcher 维护，与脚本 `for` 循环执行顺序一致
> 3. `ExecuteCodeWhitelistExtendedEvent` 只在配置加载/reload 时写一次，**不**
>    随每次脚本执行重复写出；与 PluginLoaded 事件同模式
> 4. 当脚本被外层 `Event::ToolUseDenied { reason: SubagentBlocked }` 阻拦时，
>    本节事件**不**写出（脚本根本未启动）

### 3.6 PermissionRequested / Resolved

> 字段定义权威来源：ADR-007 §2.4 / §2.6 / §6.1；`Decision` / `DecisionScope` / `DecidedBy` / `Severity` /
> `PermissionSubject` / `InteractivityLevel` 定义在 `harness-contracts §3.4 / §3.4.1`。

```rust
pub struct PermissionRequestedEvent {
    pub request_id: RequestId,
    pub run_id: RunId,
    pub session_id: SessionId,
    pub tenant_id: TenantId,
    pub tool_use_id: ToolUseId,
    pub tool_name: String,
    /// 语义化主体；`Display for PermissionSubject` 提供 UI/审计派生字符串，
    /// 不再单独存 `subject: String + detail: Option<String>`。
    pub subject: PermissionSubject,
    pub severity: Severity,
    pub scope_hint: DecisionScope,
    /// 当 `scope_hint == DecisionScope::ExactCommand { .. }` 时必填。
    /// 由 `ExecSpec::canonical_fingerprint(&base)` 计算（`harness-sandbox.md` §2.2）。
    /// AllowList 在 `replay` 阶段需依赖本字段重建 fingerprint 索引；缺失则按 ADR-007
    /// 的 schema 迁移路径补算，不可静默忽略。
    pub fingerprint: Option<ExecFingerprint>,
    /// UI 可见的候选项；**空 vec 标识"未真正发往 UI"**——
    /// 即规则预检命中、`PermissionMode` 默认模式、Hook OverridePermission 等
    /// 由内部链路即可定决策的路径（详见 ADR-007 §2.6）。
    pub presented_options: Vec<Decision>,
    /// 由调用方按上下文设置；订阅外部 UI 的 EventStream 过滤层依据
    /// `presented_options.is_empty() && interactivity == NoInteractive`
    /// 决定是否丢弃事件、还是仅供审计落库。
    pub interactivity: InteractivityLevel,
    pub causation_id: EventId,               // 触发该请求的上游事件 ID
    pub at: DateTime<Utc>,
}

pub struct PermissionResolvedEvent {
    pub request_id: RequestId,
    pub decision: Decision,
    pub decided_by: DecidedBy,
    pub scope: DecisionScope,                // 实际落盘的 scope（可能与 scope_hint 不同）
    /// 与 `PermissionRequestedEvent.fingerprint` 同源；
    /// `Decision::AllowSession` / `AllowPermanent` 命中 `ExactCommand` 时必填。
    pub fingerprint: Option<ExecFingerprint>,
    pub rationale: Option<String>,           // Broker/User 给出的理由
    pub at: DateTime<Utc>,
}
```

> `DecidedBy` 枚举定义在 `harness-contracts §3.4`：
>
> ```rust
> pub enum DecidedBy {
>     User,
>     Rule { rule_id: String },
>     DefaultMode,
>     Broker { broker_id: String },
>     Hook { handler_id: String },
>     Timeout { default: Decision },
>     ParentForwarded { parent_session_id: SessionId, original_decided_by: Box<DecidedBy> },
> }
> ```
>
> 本 crate 不再重复声明，避免多处漂移（对应审计 H-6 / P2-5）。
> `ParentForwarded` 由子代理 `DeferredInteractive` 路径产生，详见
> `crates/harness-subagent.md §6.2` 与本文档 §3.9.1。

#### 3.6.2 PermissionPersistenceTampered

```rust
pub struct PermissionPersistenceTamperedEvent {
    pub tenant_id: TenantId,
    /// 文件路径的 BLAKE3 哈希（不直接落原路径，避免在 Journal 中暴露磁盘结构）。
    pub file_path_hash: [u8; 32],
    /// 受影响记录的 `ExecFingerprint`；非 `ExactCommand` scope 时为 None。
    pub fingerprint: Option<ExecFingerprint>,
    /// 触发原因：MAC 不匹配 / 算法降级 / key_id 未知 / 缺签名。
    pub reason: PersistenceTamperReason,
    pub key_id: String,
    pub at: DateTime<Utc>,
}

#[non_exhaustive]
pub enum PersistenceTamperReason {
    SignatureMismatch,
    AlgorithmDowngrade,
    UnknownKeyId,
    MissingSignature,
}
```

**约束**：

- 该事件**必记且不可禁用**（与 `MemoryThreatDetected` / `PermissionRequested` 同等审计权重）。
- Journal 写入后，原文件被重命名为 `<file>.tampered.<ts>` 备份；SDK 不自动恢复或删除备份。
- 业务监控 / SIEM 可订阅该事件类型触发告警。详见 `security-trust.md §6.4`。

#### 3.6.1 事件配对约束（对齐 ADR-007 §2.6）

| 路径 | 是否写 `PermissionRequested` | `presented_options` | `decided_by` |
|---|---|---|---|
| 规则引擎命中 Allow/Deny | 是 | 空 vec | `Rule { rule_id }` |
| `PermissionMode::AcceptEdits` / `BypassPermissions` 自动放行 | 是 | 空 vec | `DefaultMode` |
| `PermissionMode::Plan` / `DontAsk` 自动拒绝 | 是 | 空 vec | `DefaultMode` |
| Hook `OverridePermission` 覆盖 | 是 | 空 vec | `Hook { handler_id }` |
| 用户 UI 答复 | 是 | 完整候选项 | `User` 或 `Broker { broker_id }` |
| `TimeoutPolicy` 兜底 | 是（先发） | 完整候选项 | `Timeout { default }` |

**所有路径都成对发出 `PermissionRequested + PermissionResolved`**，确保：

- 审计 API 通过 `PermissionResolved.causation_id` 总能找到同 `request_id` 的 `PermissionRequested`。
- replay 工具用同一份代码处理"询问/未询问"两类决策。
- EventStream 过滤层用 `presented_options.is_empty() || interactivity != FullyInteractive` 判断是否对外推送。

`Decision::Escalate` **不写** `PermissionResolved`：它是链内信号，由 `ChainedBroker` 内部消费（详见 `crates/harness-permission.md §3.6`）。Journal 看到的 Resolved 永远是确定性决策（Allow* / Deny* / 终结后转换的最终值）。

**被 `DedupGate` 命中的请求**（详见 §3.6.3）是上述配对原则的**唯一例外**：被合流 / 复用的请求不写 `PermissionRequested + PermissionResolved`，改写一条 `PermissionRequestSuppressed`，并通过 `original_request_id` 反查原决策。审计 API 与 replay 工具按"看到 Suppressed 即跳到原决策"统一处理。

#### 3.6.3 PermissionRequestSuppressed

```rust
pub struct PermissionRequestSuppressedEvent {
    /// 被去重命中的新请求 ID。该 `request_id` **不**对应任何
    /// `PermissionRequested` / `PermissionResolved`——它在链路前就被
    /// `DedupGate` 拦下，仅在本事件中存在。
    pub request_id: RequestId,
    pub run_id: RunId,
    pub session_id: SessionId,
    pub tenant_id: TenantId,
    pub tool_use_id: ToolUseId,
    pub tool_name: String,
    /// 与 `PermissionRequestedEvent.subject` 同构，便于审计 / 监控直接读出。
    pub subject: PermissionSubject,
    pub severity: Severity,
    pub scope_hint: DecisionScope,

    /// 复用的原 `PermissionRequested.event_id`；replay 由此跳到原决策。
    pub original_request_id: RequestId,
    /// 复用的原 `PermissionResolved.decision_id`（`recent` 命中时存在；
    /// `JoinedInFlight` 时为 `None`，等原 Resolved 写出后再由审计 API 补关联）。
    pub original_decision_id: Option<DecisionId>,
    /// 复用决策的语义；`JoinedInFlight` 时为 `None`。
    pub reused_decision: Option<Decision>,
    pub reason: SuppressionReason,
    pub causation_id: EventId,                // 触发该被压制请求的 `ToolUseRequested`
    pub at: DateTime<Utc>,
}

#[non_exhaustive]
pub enum SuppressionReason {
    /// 在 `DedupGate` 的 `in_flight` 表里命中尚未答复的同语义请求。
    JoinedInFlight,
    /// `recent_window` 内同语义请求最近一次答复为 Allow*。
    RecentlyAllowed,
    /// `recent_window` 内同语义请求最近一次答复为 Deny*。
    RecentlyDenied,
    /// `recent_window` 内同语义请求被 `TimeoutPolicy` 兜底。
    RecentlyTimedOut,
}
```

> `SuppressionReason` 是 `harness-contracts §3.4.1` 的扩展，跟 `Decision` / `DecidedBy` 同等级别的契约枚举；新增 variant 视为破坏性升级。
> **窗口刷屏防护**：`DedupGate` 在同 `recent_window` 内连续命中超过 `suppression_max_events_per_window` 时，仅递增 `permission_dedup_suppressed_total{reason}` 指标（见 `crates/harness-permission.md §12`），不再写新事件。审计需要"窗口内被压制次数"时按指标查询，**不**回填到 Journal。

**约束**：

- `original_request_id` 在 `JoinedInFlight` 时是已经在写出过程中的 `PermissionRequested.event_id`；`recent` 命中时来自 `RecentDecisionCache`。
- `causation_id` 必须指向触发被压制请求的 `ToolUseRequested.event_id`，而不是原决策的 `PermissionRequested`，避免因果链跳跃。
- 审计 API 在重建"某 ToolUse 的审批结果"时，先查 `PermissionRequestSuppressed`：若存在则跳到 `original_request_id` 取最终决策；与"成对原则"等价。

### 3.7 HookTriggered

```rust
pub struct HookTriggeredEvent {
    pub hook_event_kind: HookEventKind,
    pub handler_id: HandlerId,
    pub outcome_summary: HookOutcomeSummary,
    pub duration_ms: u64,
    pub at: DateTime<Utc>,
}

/// **审计专用摘要**——`HookOutcome` 本身可能携带不适合落 Journal 的大体积载荷
/// （rewritten tool input / additional context 文本），此结构只保留"做了什么"的
/// 维度信息，便于审计 / 指标聚合。具体载荷以独立事件落盘：
/// - rewrite_input → `HookRewroteInputEvent`
/// - additional_context → `HookContextPatchEvent`
/// - override_permission → 合并到对应 `PermissionResolved.decided_by = Hook { .. }`
pub struct HookOutcomeSummary {
    pub continued: bool,
    pub blocked_reason: Option<String>,
    pub rewrote_input: bool,
    pub overrode_permission: Option<Decision>,
    pub added_context_bytes: Option<u64>,
    pub transformed: bool,
}
```

> `HookOutcome` 与 `PreToolUseOutcome` 的实际定义在 `crates/harness-hook.md §2.3`，
> 本节仅描述"事件层落盘的派生摘要"。
> Dispatcher 在 PreToolUse 三件套场景下，可能为同一个 `HookTriggered` 事件
> 同时联动多条 `HookRewroteInput` / `HookReturnedAdditionalContext`，
> 由 `causation_id = HookTriggeredEvent.id` 串成因果链。

#### 3.7.1 PreToolUse 三件套的事件配对

| 字段 | 触发的事件 | 备注 |
|---|---|---|
| `rewrite_input: Some(_)` | `HookRewroteInputEvent { tool_use_id, before_hash, after_hash }` | 实际 input 不入 Journal，仅落 hash 与 BlobRef |
| `override_permission: Some(_)` | `PermissionResolvedEvent { decided_by: Hook { handler_id }, .. }` | 复用现有审批事件，与 `PermissionRequested` 配对 |
| `additional_context: Some(_)` | `HookReturnedAdditionalContext / HookContextPatchEvent` | 大文本走 BlobRef |
| `block: Some(reason)` | `HookTriggeredEvent.outcome_summary.blocked_reason = Some(_)` + `ToolUseDenied { reason: HookBlocked, handler_id }` | 不引入新事件类型，复用现有 Deny 信号 |

#### 3.7.2 Hook 失败链事件

下列事件覆盖 dispatcher 在 `harness-hook.md §2.6.1` 中记录的失败语义；**所有失败必记**，与 `failure_mode = FailOpen | FailClosed` 是否影响最终 outcome 无关。

```rust
/// Hook 链路任一 handler 失败时落盘的总账事件——
/// `cause` 同 `harness-hook.md §2.6` 的 `HookFailureCause` 判别量。
pub struct HookFailedEvent {
    pub hook_event_kind: HookEventKind,
    pub handler_id: HandlerId,
    pub failure_mode: HookFailureMode,
    pub cause_kind: HookFailureCauseKind,
    /// 文本化原因（含 redacted）；超过 4 KiB 时由 `harness-redactor` 做 head/tail 截断。
    pub cause_detail: String,
    pub duration_ms: u64,
    /// 命中 `FailClosed` 时关联的 `ToolUseDenied.event_id`；`FailOpen` 为 None。
    pub fail_closed_denied: Option<EventId>,
    pub at: DateTime<Utc>,
}

#[non_exhaustive]
pub enum HookFailureCauseKind {
    Unsupported,
    Inconsistent,
    Panicked,
    Timeout,
    Transport,
    Unauthorized,
}

/// 显式记录"返回了不在能力矩阵内的 outcome variant"——
/// 与 `HookFailed { cause_kind: Unsupported }` 一一对应，独立成型便于审计聚合。
pub struct HookReturnedUnsupportedEvent {
    pub hook_event_kind: HookEventKind,
    pub handler_id: HandlerId,
    /// `HookOutcome` 的判别量；接收端可据此还原"handler 想返回什么"。
    pub returned_kind: HookOutcomeDiscriminant,
    pub causation_id: EventId,
    pub at: DateTime<Utc>,
}

/// `PreToolUseOutcome` 字段互斥违规 / `RewriteInput` schema 违规 / `ContextPatch` 体积超限 等。
pub struct HookOutcomeInconsistentEvent {
    pub hook_event_kind: HookEventKind,
    pub handler_id: HandlerId,
    pub reason: InconsistentReason,
    pub causation_id: EventId,
    pub at: DateTime<Utc>,
}

#[non_exhaustive]
pub enum InconsistentReason {
    PreToolUseBlockExclusive,
    PromptCacheViolation,
    SchemaInvalid {
        schema_id: SchemaId,
        /// `harness-redactor` 处理后的错误片段（不含原始 input/result 内容）。
        message: String,
    },
    ContextPatchTooLarge {
        limit_bytes: u64,
        actual_bytes: u64,
    },
}

/// 仅在 in-process transport 路径下产生（Exec/HTTP 表现为 NonZeroExit / 5xx，
/// 走 `HookFailedEvent { cause_kind: Transport }` 而非本事件）。
pub struct HookPanickedEvent {
    pub hook_event_kind: HookEventKind,
    pub handler_id: HandlerId,
    /// panic message 的尾部 1 KiB 摘要（用于排错；不含完整 backtrace 以避免泄露路径）。
    pub message_snippet: String,
    pub causation_id: EventId,
    pub at: DateTime<Utc>,
}

/// 同 priority handler 在 `OverridePermission` 上给出冲突决策（如 A: Allow / B: Deny），
/// 由 dispatcher 按"Deny 压过 Allow"裁决后写出。
pub struct HookPermissionConflictEvent {
    pub hook_event_kind: HookEventKind,
    pub priority: i32,
    /// 写出 conflict 时所有参与的 handler 与各自决策；**不**包含被裁决放弃的 handler 之外的副作用。
    pub participants: Vec<HookPermissionConflictParticipant>,
    /// 最终采用的决策与 handler。
    pub winner: HookPermissionConflictParticipant,
    /// 关联的 `PermissionResolved.event_id`，便于审计跳转。
    pub resolved_event_id: EventId,
    pub at: DateTime<Utc>,
}

pub struct HookPermissionConflictParticipant {
    pub handler_id: HandlerId,
    pub decision: Decision,
}
```

> `HookFailureMode` / `HookOutcomeDiscriminant` / `InconsistentReason` 的语义与
> `harness-hook.md §2.6 / §2.6.1` 的 Rust 定义保持一一对应；事件层只承载"已发生"的事实，不重复定义这些枚举的字段含义。

**replay 期望**：

- 这些事件全部 append-only，**不允许后台清理**——审计与 `ReplayMode::Audit` 重建时依赖它们还原 `HookFailureRecord`。
- `HookFailedEvent.cause_detail` 大于 4 KiB 时由 dispatcher 改写为 `BlobRef`，原文落 BlobStore；事件本体只保留 `BlobRef + size`。

### 3.8 CompactionApplied

```rust
pub struct CompactionAppliedEvent {
    pub session_id: SessionId,
    /// 触发本次 compaction 的 strategy 引用（trait 定义见 `api-contracts.md §11.2`）。
    pub strategy: CompactStrategyId,
    /// 触发来源：主动（soft/hard）/ 被动（provider 报告）/ 用户命令。
    pub trigger: CompactTrigger,
    /// 执行结果：成功 / 降级 / reactive 失败（详见 `context-engineering.md §3.7–§3.8`）。
    pub outcome: CompactOutcome,
    pub before_tokens: u64,
    pub after_tokens: u64,
    /// 对话主体的压缩摘要 blob；reactive 失败路径下可能为空 blob。
    pub summary_ref: BlobRef,
    /// 仅当本次 compaction 触发 fork 时写入；与 `Event::SessionForked.child_session_id` 保持一致。
    pub child_session_id: Option<SessionId>,
    /// 子 session 续接所需的全部上下文；仅 fork 路径填。
    pub handoff: Option<CompactionHandoff>,
    pub at: DateTime<Utc>,
}

pub enum CompactTrigger {
    /// `estimated_tokens` 越过 `soft_budget_ratio`
    SoftBudget,
    /// `estimated_tokens` 越过 `hard_budget_ratio`
    HardBudget,
    /// model 服务端返回 context-window-exceeded（Reactive 路径）
    ProviderReport { reported_tokens: u64 },
    /// 用户显式 `/compact` 命令或 SDK `Session::compact()` 调用
    UserCommand,
}

pub enum CompactOutcome {
    /// 正常完成（aux 摘要 + token 落到目标区间）
    Succeeded,
    /// aux_provider 缺失或处于 cooldown，跳过该阶段（`context-engineering §3.7.1`）
    DegradedNoAuxProvider,
    /// aux 调用失败累计触发 cooldown（`context-engineering §3.7.2`）
    DegradedAuxFailure { failure_count: u32 },
    /// Reactive 重试也失败 → 当前 turn 终止（`context-engineering §3.8`）
    ReactiveAttemptFailed,
}

/// 子 session fork 续接的全部"必需重发"信息（HER-023 / `context-engineering §3.5`）。
/// Replay 重建子 session 时不重算压缩，直接从 `summary_ref` + 本结构恢复语义。
pub struct CompactionHandoff {
    /// 指向 BlobStore 的 "## Active Task" 段（自然语言陈述未完成意图）。
    pub active_task_ref: BlobRef,
    /// 跨 compaction 必须重发的预算（不重发会导致子 session 无限制运行）。
    pub remaining_budget: RemainingBudget,
    /// 父 session 中尚未拿到 tool_result 的 tool_use 列表；
    /// 子 session 第一轮必须明确处置（取消 / 重发 / 转报告）。
    pub pending_tool_uses: Vec<ToolUseId>,
    /// 父 session 中未决的权限审批；子 session 视情况重发或视为 cancelled。
    pub outstanding_permissions: Vec<PermissionRequestId>,
}

pub struct RemainingBudget {
    pub iterations_remaining: u32,
    pub tokens_remaining_in_session: u64,
    pub wall_clock_deadline: Option<DateTime<Utc>>,
}
```

> **写入约束**：
> - `CompactionApplied` 写入**父 session**；`SessionForked` 与 `RunEnded { reason: Compacted }` 紧随其后，三者通过同一 `correlation_id` 串联。
> - `outcome != Succeeded` 但 `child_session_id.is_some()` 表示降级 fork 路径（aux 缺失/失败但仍继承）；`handoff.summary_ref` 此时仍可消费，但内容可能仅是兜底模板。
> - Replay（`harness-journal::ReplayContext`）重建子 session 时**禁止**重新调用 aux LLM；必须严格基于 `summary_ref + handoff` 还原（保 Replay 确定性，对齐 `context-engineering §10`）。

### 3.8.1 ContextStageTransitioned

```rust
pub struct ContextStageTransitionedEvent {
    pub session_id: SessionId,
    /// 阶段身份；详见 `crates/harness-context.md §2.2`。
    /// 注意与 `api-contracts.md §11.1` 的 trait `ContextStage`（阶段执行接口）
    /// 区分——本字段是 enum `ContextStageId`，仅用于事件分桶与 replay 标识。
    pub stage: ContextStageId,
    pub provider_id: String,
    pub outcome: ContextStageOutcome,
    pub before_tokens: u64,
    pub after_tokens: u64,
    pub bytes_saved: u64,
    pub duration_ms: u32,
    pub at: DateTime<Utc>,
}

pub enum ContextStageOutcome {
    NoChange,
    Modified,
    Forked { child: SessionId },
    SkippedNoAuxProvider,
    SkippedAuxCooldown { until_turn: u32 },
    Failed { reason: String },
}
```

> `ContextStageId` 的 enum 定义在 `crates/harness-context.md §2.2`（与 `ContextProvider::stage()` 共用），事件 schema 不重复定义，避免漂移。

> Compact pipeline 每一阶段执行结束都写一条本事件，是 §3.8 `CompactionApplied` 的细粒度补充。`provider_id` 让多个同阶段 provider 可被区分（默认实现 + 业务自定义 provider 同存时）。

### 3.8.2 ContextBudgetExceeded

```rust
pub struct ContextBudgetExceededEvent {
    pub session_id: SessionId,
    /// 详见 `crates/harness-contracts.md §3.8.1`，与 `HarnessError::BudgetExhausted` 共享
    pub budget_kind: BudgetKind,
    pub source: BudgetExceedanceSource,
    pub requested: u64,
    pub max: u64,
    pub at: DateTime<Utc>,
}

pub enum BudgetExceedanceSource {
    /// ContextEngine 本地估算（assemble 阶段或 stage 末尾的 `estimated_tokens` 检查）
    LocalEstimate,
    /// Model 服务端返回 context-window-exceeded（Reactive Compact 入口；
    /// 与 `CompactTrigger::ProviderReport` 共享 `reported_tokens` 语义）
    ProviderReport { reported_tokens: u64 },
}
```

> 该事件**不**直接终止 turn；它是 ContextEngine 进入下一动作（启动 compact pipeline / 发起 reactive retry / 抛 `StopReason`）的触发信号。

### 3.9 Subagent 事件族

> 本节字段定义权威来源：`crates/harness-subagent.md §2 / §3.2 / §6.1 / §7.1`。
> 任何字段调整必须先改 subagent crate SPEC 再同步本节，避免双轨漂移。

#### 3.9.1 `SubagentSpawnedEvent`

```rust
pub struct SubagentSpawnedEvent {
    pub subagent_id: SubagentId,
    pub parent_session_id: SessionId,
    /// 触发本次 spawn 所属的父 RunId；与父 `RunStartedEvent.run_id` 严格相等。
    /// Replay 与审计据此把"父→子"关系串到同一 turn 内。
    pub parent_run_id: RunId,
    pub agent_ref: AgentRef,
    /// `SubagentSpec` 的不可变快照引用（仍走 BlobStore，避免事件膨胀）。
    pub spec_snapshot_id: SnapshotId,
    /// `SubagentSpec` 经 BLAKE3 派生的稳定指纹；用于审计跨 Session 比对
    /// 同一 spec 的子代理（无须读 BlobStore）。
    pub spec_hash: [u8; 32],
    /// 子代理在递归链中的深度；与 `harness-subagent.md §2.2 ParentContext.depth`
    /// 字面相等。`SubagentRunner::spawn` 在 depth >= effective_max_depth 时
    /// **不**写本事件，直接 fail-closed。
    pub depth: u8,
    /// 触发本次 spawn 的 `ToolUseRequested.tool_use_id`（建议 9 因果链锚点）。
    /// 由 `AgentTool::execute` 注入；非工具触发的内部 spawn（如 Hook 内部委派）
    /// 为 None。
    pub trigger_tool_use_id: Option<ToolUseId>,
    /// 触发工具的 canonical 名（如 `agent` / `delegate`）；与 `trigger_tool_use_id`
    /// 同时 None / 同时 Some。冗余于 `causation_id` 链路，但便于面板按工具分桶。
    pub trigger_tool_name: Option<String>,
    pub at: DateTime<Utc>,
}
```

#### 3.9.2 `SubagentAnnouncedEvent`

```rust
pub struct SubagentAnnouncedEvent {
    pub subagent_id: SubagentId,
    pub parent_session_id: SessionId,
    /// 子代理终态语义；定义见 `harness-subagent.md §2.4 SubagentStatus`。
    /// 与 `SubagentTerminatedEvent.reason` 互补：status 描述"为何不再继续"
    /// （由子代理自身决定），reason 描述"如何被终结"（由外部 / 兜底决定）。
    pub status: SubagentStatus,
    pub summary: String,
    pub result: Option<Value>,
    pub usage: UsageSnapshot,
    /// 完整 transcript 落 BlobStore 的引用；仅 `AnnounceMode::FullTranscript`
    /// 路径填充，其余路径为 None（避免审计存储放大）。
    pub transcript_ref: Option<TranscriptRef>,
    /// 渲染本次 announcement 的 `AnnouncementRenderer` 标识（见
    /// `harness-subagent.md §7.1`）；用于审计"切换 renderer 是否影响父 cache"。
    /// 与 `subagent_announce_rendered_total{renderer_id}` 指标 label 同源。
    pub renderer_id: String,
    pub at: DateTime<Utc>,
}
```

#### 3.9.3 `SubagentTerminatedEvent`

子代理生命周期收尾事件；与 `SubagentAnnounced` 通过 `subagent_id` 配对。**任一 spawned 的子代理最终必须有一条 terminated**，否则视为 Journal 损坏并触发 `Event::EngineFailed { kind: SubagentBridgeBroken }`。

```rust
pub struct SubagentTerminatedEvent {
    pub subagent_id: SubagentId,
    pub parent_session_id: SessionId,
    pub reason: SubagentTerminationReason,
    /// 子代理 Session 的最终 usage 快照（含 token / tool_calls / wall-clock）；
    /// 与 `SubagentAnnounced.usage` 字面相等（announce 后无新工作）。
    pub final_usage: UsageSnapshot,
    pub at: DateTime<Utc>,
}

#[non_exhaustive]
pub enum SubagentTerminationReason {
    /// 自然结束：`SubagentStatus::Completed` / `MaxIterationsReached` /
    /// `MaxBudget(_)` 路径都进 NaturalCompletion，区分仍由 announce.status 提供。
    NaturalCompletion,
    /// 父 Session cancel 级联（`SubagentHandle::cancel` / 父 RunEnded.Cancelled）。
    ParentCancelled,
    /// `SubagentAdmin::interrupt` 主动中断（与 `subagent_admin_interrupted_total` 同源）。
    AdminInterrupted { admin_id: String },
    /// Watchdog 判定卡死（详见 `harness-subagent.md §4.3`）。
    Stalled { silent_for_ms: u64 },
    /// `SubagentBridge` 与父 Session 失联超过 `acquire_timeout`。
    BridgeBroken,
    /// 其它非典型失败；`detail` 经 `harness-redactor` 截断 ≤ 4 KiB。
    Failed { detail: String },
}
```

> **与 `SubagentStatus` 的关系**：`SubagentAnnounced.status` 是子代理对自己结局的"自我陈述"；`SubagentTerminated.reason` 是 SDK 对外部终结路径的"客观记录"。Replay 优先以 reason 推动状态机，status 仅参与 announcement 渲染。

#### 3.9.4 `SubagentSpawnPausedEvent`

`SubagentAdmin::pause_spawning(true)` 与 `pause_spawning(false)` 都写一条本事件；面板基于此事件 + `subagent_admin_paused` gauge 反推全租户 spawn 暂停时段。

```rust
pub struct SubagentSpawnPausedEvent {
    pub tenant_id: TenantId,
    pub paused: bool,
    /// 触发本次切换的 admin 标识（如 CLI 用户 / Bugbot 钩子 / Server 健康端点）。
    pub by: String,
    /// 可选：业务侧给出的人类可读原因（如灰度回退说明）。
    pub reason: Option<String>,
    pub at: DateTime<Utc>,
}
```

**必记**：列入 `SessionEventSinkPolicy::DEFAULT_NEVER_DROP_KINDS`（与 §9.3 列表对齐——见本节末尾的更新提示）。

#### 3.9.5 `SubagentPermissionForwardedEvent` / `SubagentPermissionResolvedEvent`

子代理 `DeferredInteractive` 路径下，由 `SubagentBridge` 在父 Session 写入的"审批转发"镜像事件；与子 Session 的 `PermissionRequested` 通过 `original_request_id` 配对（详见 `crates/harness-subagent.md §6.2`）。

```rust
pub struct SubagentPermissionForwardedEvent {
    pub parent_session_id: SessionId,
    pub subagent_id: SubagentId,
    pub original_request_id: PermissionRequestId,
    pub subject: PermissionSubject,
    pub presented_options: Vec<Decision>,
    pub timeout_policy: Option<TimeoutPolicy>,
    pub forwarded_at: DateTime<Utc>,
}

/// 父 Session 决策完成后写入的镜像事件；
/// 子 Session 收到 Bridge 回写后再写一条 `PermissionResolved`，
/// `decided_by = ParentForwarded { parent_session_id, original_decided_by }`。
pub struct SubagentPermissionResolvedEvent {
    pub parent_session_id: SessionId,
    pub subagent_id: SubagentId,
    pub original_request_id: PermissionRequestId,
    pub decision: Decision,
    pub decided_by: DecidedBy,
    pub at: DateTime<Utc>,
}
```

> **配对约束**：父 Session `Forwarded → Resolved`、子 Session `Requested → Resolved` 四条事件必须围绕同一个 `request_id`/`original_request_id` 形成闭环；任一缺失视为 Journal 损坏，`harness-engine` 写出 `Event::EngineFailed { kind: SubagentBridgeBroken }` 并把子 Session 转入失败态。

#### 3.9.6 与 `Event` 枚举的对接

`harness-contracts.md §3.3` 的 `Event` 枚举需新增以下变体（增量为 **1** 条；其余 4 条已声明）：

```rust
Event::SubagentSpawnPaused(SubagentSpawnPausedEvent),
```

按 §1.3 `#[non_exhaustive]` 规则升 minor。`TranscriptRef` 定义在 `harness-contracts.md §3.5`（与 `BlobRef` 同段，因为它内含 BlobRef）；`SubagentTerminationReason` 定义在 `harness-contracts.md §3.5` 末尾的事件附属枚举区。`SubagentStatus` 沿用 `harness-subagent.md §2.4` 的权威定义。

### 3.10 Team Lifecycle

```rust
pub struct TeamCreatedEvent {
    pub team_id: TeamId,
    pub tenant_id: TenantId,
    pub name: String,
    pub topology_kind: TopologyKind,
    /// 创建期 `TeamSpec.members` 列表的整体 BLAKE3 指纹；用于 replay 校验
    /// 与审计快照对账。**仅覆盖创建期成员**——运行期通过 `Team::add_member`
    /// 加入的成员由各自的 `TeamMemberJoinedEvent.spec_hash` 承载，不在此聚合。
    pub member_specs_hash: [u8; 32],
    pub created_at: DateTime<Utc>,
}

pub struct TeamMemberJoinedEvent {
    pub team_id: TeamId,
    pub agent_id: AgentId,
    pub role: String,
    pub session_id: SessionId,
    pub visibility: ContextVisibility,

    /// 加入时刻该成员 `TeamMemberSpec` 在 BlobStore 的不可变快照引用；
    /// replay 据此重建 `EngineConfig` / `quota` / 自定义字段。
    /// **创建期成员**与**运行期 `Team::add_member` 加入者**都必须填充该字段，
    /// 不允许仅靠 `TeamCreatedEvent.member_specs_hash` 还原后续加入者。
    pub spec_snapshot_id: BlobRef,
    /// `TeamMemberSpec` 规范化字节序列的 BLAKE3 指纹；用于
    /// `TeamProjection::replay` 与 `member_specs_hash` 交叉对账（创建期成员
    /// 的 `spec_hash` 顺序拼接的 BLAKE3 必须等于 `member_specs_hash`）。
    pub spec_hash: [u8; 32],

    pub joined_at: DateTime<Utc>,
}

pub struct TeamMemberLeftEvent {
    pub team_id: TeamId,
    pub agent_id: AgentId,
    pub reason: MemberLeaveReason,
    pub left_at: DateTime<Utc>,
}

pub enum MemberLeaveReason {
    GoalAchieved,
    QuotaExceeded,
    Interrupted,
    Error(String),
    /// 通过 `Team::remove_member` 主动移除（动态成员场景）。
    Removed,
    /// 通过 `Team::remove_member` 在 watchdog 判定 stalled 之后被移除
    /// （与 `TeamMemberStalledEvent.action == Removed` 对账）。
    StalledRemoved,
}

pub enum TopologyKind {
    CoordinatorWorker,
    PeerToPeer,
    RoleRouted,
    Custom(String),
}
```

#### 3.10.1 `TeamMemberStalledEvent`

Watchdog 判定成员长时间无心跳（与 `harness-subagent.md §4.3` 的
`SubagentStalled` 对称）。**不参与 `TeamProjection::replay`**，仅供审计与
可观测性。

```rust
pub struct TeamMemberStalledEvent {
    pub team_id: TeamId,
    pub agent_id: AgentId,
    pub session_id: SessionId,
    /// 最后一次活动时间（成员 Session 最近一条事件 / 心跳 timestamp）
    pub last_activity_at: DateTime<Utc>,
    /// 累计 stall 时长
    pub stalled_for: Duration,
    /// Watchdog 采取的处置；与 `team_member_stalled_total{action}` 指标对齐
    pub action: StalledAction,
    pub at: DateTime<Utc>,
}

pub enum StalledAction {
    /// 仅记录、不干预（指标 + 审计）
    Reported,
    /// 主动 `Team::pause(agent)` 中断当前 turn
    Interrupted,
    /// `Team::remove_member(agent, MemberLeaveReason::StalledRemoved)`
    Removed,
}
```

#### 3.10.2 与 `Event` 枚举的对接

`harness-contracts.md §3.3` 的 `Event` 枚举需新增以下变体：

```rust
Event::TeamMemberStalled(TeamMemberStalledEvent),
```

按 §1.3 `#[non_exhaustive]` 规则升 minor。`StalledAction` / 新增的
`MemberLeaveReason::Removed` / `MemberLeaveReason::StalledRemoved` 沿用
`harness-contracts.md §3.4`/`§3.5` 的事件附属枚举区放置。

> **不引入 `TeamMessageFiltered` 事件的决定**（与 `crates/harness-team.md §11` 对应）：
> `ContextVisibility` 过滤掉的消息已经能由 `AgentMessageRoutedEvent.resolved_recipients`
> 与原始 `to: Recipient` 的差集还原；可观测性靠 `team_context_visibility_blocked_total`
> 指标承载，无需独立事件变体——避免 `Event` 枚举膨胀。

### 3.11 AgentMessageSent / Routed

```rust
pub struct AgentMessageSentEvent {
    pub team_id: TeamId,
    pub from: AgentId,
    pub to: Recipient,
    pub payload: MessagePayload,
    pub message_id: MessageId,
    pub at: DateTime<Utc>,
}

pub struct AgentMessageRoutedEvent {
    pub team_id: TeamId,
    pub message_id: MessageId,
    pub resolved_recipients: Vec<AgentId>,
    pub routing_policy: RoutingPolicyKind,
    pub at: DateTime<Utc>,
}
```

#### 3.11.1 Team / Member / Subagent CorrelationId 全链不变量

`EventEnvelope.correlation_id`（§4）在多 Agent 路径上的传递规则；与
`crates/harness-team.md §6.1` 同款契约，本节给出权威字段角度的精确定义。

**贯穿规则**（fail-closed）：

1. `Team::dispatch(TeamInput)` 创建该次目标的 root `correlation_id` 并写入
   首条 `AgentMessageSentEvent` 的 envelope。
2. 成员 Engine 处理某条 `AgentMessage` 时启动的 `RunStarted`、`AssistantDelta`、
   `ToolUseRequested` ... 全部 envelope `correlation_id` **必须等于**入站
   `AgentMessage.correlation_id`，**禁止**再生成新值。
3. 该轮内成员 spawn 的 Subagent：`SubagentSpawnedEvent` 与子 Session
   全部事件的 envelope `correlation_id` 同样沿用——这与
   `crates/harness-subagent.md §6.1` 的"父子链"契约同根。
4. 成员通过 `Team::post` 发出回复时，新 `AgentMessageSentEvent` 的
   envelope `correlation_id` 仍为同一值；MessageBus 据此完成 §6.1 的
   `CyclicRouting` 计数。
5. **`SubagentPermissionForwarded` / `SubagentPermissionResolved`** 镜像事件
   （§3.9.5）必须保持 envelope `correlation_id` 与子 Session 原始
   `PermissionRequestedEvent` 一致，不得借父 Session 的 turn correlation
   覆盖——审计据此跨 Session 对账。

**违反时的处置**：

- Engine 写出事件前比对 envelope `correlation_id` 与当前 Run 的 root
  `correlation_id`，不等则 fail-closed 并写
  `Event::EngineFailed { kind: CorrelationIdBroken }`。
- 该 fail 与现有 `SubagentBridgeBroken`（§3.9.5）并列，列入
  `DEFAULT_NEVER_DROP_KINDS`，不可被 lossy 丢弃。

**与 `turn_id` 的边界**：

- `RunStartedEvent.run_id` / `TeamTurnCompletedEvent.turn_id` 标识"一次执行
  周期"；同一 `correlation_id` 在 `PeerToPeer` / 多轮 Coordinator-Worker 派
  发下可能跨 N 个 turn。审计与指标必须**同时**索引 `correlation_id` 与
  `turn_id`，不能用任一替代另一者。

### 3.12 SessionReloadRequested / SessionReloadApplied

```rust
pub struct SessionReloadRequestedEvent {
    pub session_id: SessionId,
    pub delta_hash: DeltaHash,
    pub at: DateTime<Utc>,
}

pub struct SessionReloadAppliedEvent {
    pub session_id: SessionId,
    pub mode: ReloadMode,
    pub delta_hash: DeltaHash,
    pub cache_impact: CacheImpact,
    pub effective_from: ReloadEffect,
    pub child_session_id: Option<SessionId>,
    pub at: DateTime<Utc>,
}

pub enum ReloadMode {
    AppliedInPlace,
    ForkedNewSession,
    Rejected { reason: String },
}

pub enum ReloadEffect {
    NextTurn,
    NextMessage,
    Immediate,
}
```

`SessionReloadRequestedEvent.delta_hash` 是 reload 请求快照。`SessionReloadAppliedEvent`
是 replay 权威结果，必须携带 `cache_impact`，不能只把 cache 影响藏在 API 返回值里。

### 3.13 ToolDeferredPoolChanged

Deferred 工具池增减通告（ADR-009 §2.9）。触发 `DeferredToolsDelta` attachment 拼装。

```rust
pub struct ToolDeferredPoolChangedEvent {
    pub session_id: SessionId,
    /// 本次进入 deferred 集的工具（含 hint）
    pub added: Vec<DeferredToolHint>,
    /// 本次从 deferred 集移除的工具（可能因 MCP server 断开或工具被删）
    pub removed: Vec<ToolName>,
    /// 触发来源
    pub source: ToolPoolChangeSource,
    /// 变更后 deferred 池的工具总数（便于审计与可观测性）
    pub deferred_total: u32,
    pub at: DateTime<Utc>,
}

pub struct DeferredToolHint {
    pub name: ToolName,
    pub hint: Option<String>,
}

pub enum ToolPoolChangeSource {
    /// Session 首次进入 deferred 模式，初始化 pool
    InitialClassification,
    /// MCP tools/list_changed 推送导致的增删
    McpListChanged { server_id: McpServerId },
    /// 插件动态注册/注销工具
    PluginRegistration { plugin_id: String },
    /// Skill 热更新
    SkillHotReload { skill_id: String },
}
```

**Projection 影响**：追加到 `DiscoveredToolProjection` 的 attachment 队列；下一轮 prompt 拼装时消费并清空。

### 3.14 ToolSearchQueried

`tool_search` 元工具被模型调用（ADR-009 §2.6）。

```rust
pub struct ToolSearchQueriedEvent {
    pub session_id: SessionId,
    pub run_id: RunId,
    pub tool_use_id: ToolUseId,
    /// 原始 query 字符串（审计用）
    pub query: String,
    pub query_kind: ToolSearchQueryKind,
    /// 所有被 scorer 评分的 (tool_name, score) —— 用于分析 scorer 效果
    pub scored: Vec<(ToolName, u32)>,
    /// 最终进入 backend.materialize 的工具列表
    pub matched: Vec<ToolName>,
    pub truncated_by_max_results: bool,
    pub at: DateTime<Utc>,
}
```

**Projection 影响**：仅用于可观测性与审计，不影响 `SessionProjection`。

### 3.15 ToolSchemaMaterialized

Backend 完成 materialize，deferred 工具 schema 进入模型可调用集合（ADR-009 §2.4 + §2.5）。

```rust
pub struct ToolSchemaMaterializedEvent {
    pub session_id: SessionId,
    pub run_id: RunId,
    /// 触发本次 materialize 的 ToolUse（即 tool_search 调用本身）
    pub tool_use_id: ToolUseId,
    /// 被 materialize 的工具
    pub names: Vec<ToolName>,
    /// 走哪条 backend：`anthropic_tool_reference` / `inline_reinjection` / 自定义
    pub backend: ToolLoadingBackendName,
    /// 本次 materialize 对 Prompt Cache 的影响
    pub cache_impact: CacheImpact,
    /// 是否附带触发了 session.reload_with（InlineReinjectionBackend 路径为 true）
    pub triggered_session_reload: bool,
    /// 若 InlineReinjectionBackend 合并了多次调用，这里记录合并数量（否则为 1）
    pub coalesced_count: u32,
    pub at: DateTime<Utc>,
}
```

**Projection 影响**：
- `SessionProjection.discovered_tools.materialized += names`
- 若 `triggered_session_reload == true`，后续必有配对的 `SessionReloadAppliedEvent`（`causation_id` 指向本事件）

### 3.16 UsageAccumulated

```rust
pub struct UsageAccumulatedEvent {
    pub session_id: SessionId,
    pub run_id: Option<RunId>,
    /// 累加增量（不是累计快照）；UsageAccumulator 自行做加和
    pub delta: UsageSnapshot,
    /// 本次增量的 model 来源（主对话 / Aux 都可能产生 UsageAccumulated）
    pub model_ref: Option<ModelRef>,
    /// 增量基于的不可变定价快照。详见 `crates/harness-model.md` §2.1.1 R-P1
    pub pricing_snapshot_id: Option<PricingSnapshotId>,
    pub at: DateTime<Utc>,
}
```

**与 `AssistantMessageCompleted.usage` 的关系**：

- `AssistantMessageCompleted.usage` 是该轮对话的最终聚合值（含 `pricing_snapshot_id`），用于 Replay 重算 cost
- `UsageAccumulated` 是流式增量，可被 `lossy_event_kinds` 丢弃（详见 §9）；`UsageAccumulator` 实时聚合用于 Prometheus / Cost dashboard
- 两者都基于**同一** snapshot ID；Engine 通过 `ModelCatalog::lock_pricing_for_session` 保证一致

**Replay 语义**：仅用于现场观测，重建 `SessionProjection` 时**不消费** `UsageAccumulated`（避免与 `AssistantMessageCompleted.usage` 双重累加）。

### 3.17 Memory Events（v1.4 完整化）

详细 SPEC：`crates/harness-memory.md`。本节给出事件层字段与 Replay 语义。

#### 3.17.1 MemoryUpserted

```rust
pub struct MemoryUpsertedEvent {
    pub session_id: SessionId,
    pub run_id: Option<RunId>,
    pub memory_id: MemoryId,
    pub kind: MemoryKind,
    pub visibility: MemoryVisibility,
    pub action: MemoryWriteAction,
    pub provider_id: String,
    pub source: MemorySource,
    pub content_hash: ContentHash,
    pub bytes_written: u64,
    pub takes_effect: TakesEffect,
    pub at: DateTime<Utc>,
}
```

**字段说明**：

- `provider_id` 取 `"builtin:memdir"` 或外部 provider 的 `provider_id()`。
- `content_hash` 是 SHA-256；Event **不携带** raw content，避免敏感信息落 Journal（对齐 `security-trust.md §8`）。
- `takes_effect` 标识此次写入对当前 / 下一 Session 的可见性（`NextSession` / `AfterReloadWith`）。
- `action: MemoryWriteAction` 区分 `AppendSection` / `ReplaceSection` / `DeleteSection` / `Upsert` / `Forget`，用于审计回放。

**Replay 语义**：是否参与 `MemoryProjection` 重建由实现侧决定；推荐 `Builtin` 路径**不重建**（因 Memdir 即文件本身），`External` 路径可选重建（需 provider 提供幂等 `upsert`）。

#### 3.17.2 MemoryRecalled

```rust
pub struct MemoryRecalledEvent {
    pub session_id: SessionId,
    pub run_id: RunId,
    pub turn: u32,
    pub provider_id: String,
    pub query_text_hash: ContentHash,
    pub returned_count: u32,
    pub kept_count: u32,
    pub injected_chars: u32,
    pub deadline_used_ms: u32,
    pub min_similarity: f32,
    pub kinds_returned: Vec<MemoryKind>,
    pub at: DateTime<Utc>,
}
```

**字段说明**：

- `returned_count` = provider 原始返回；`kept_count` = visibility + threat scan + RecallBudget 截断后实际注入条数。
- `query_text_hash` 不存原始 query 文本（用户消息可能含敏感信息）。
- `kinds_returned` 用于观测面统计召回成分（UserPreference / ProjectFact 比例）。

#### 3.17.3 MemoryRecallDegraded

```rust
pub struct MemoryRecallDegradedEvent {
    pub session_id: SessionId,
    pub run_id: RunId,
    pub turn: u32,
    pub provider_id: String,
    pub reason: MemoryRecallDegradedReason,
    pub at: DateTime<Utc>,
}
```

> 与 `MemoryRecalled` 互斥（一次 turn 至多落其中一个）。`MemoryRecallDegraded` 是 fail-open 模式下的可观测事件，业务侧可据此发告警。

#### 3.17.4 MemoryRecallSkipped

```rust
pub struct MemoryRecallSkippedEvent {
    pub session_id: SessionId,
    pub run_id: RunId,
    pub turn: u32,
    pub reason: RecallSkipReason,
    pub at: DateTime<Utc>,
}

#[non_exhaustive]
pub enum RecallSkipReason {
    NoExternalProvider,
    PolicyDecidedSkip,
    DeadlineZero,
    Cancelled,
}
```

> 用于把 "本轮没有 recall" 的因果显式落事件，便于 Replay / 故障排查。

#### 3.17.5 MemoryThreatDetected

```rust
pub struct MemoryThreatDetectedEvent {
    pub session_id: SessionId,
    pub run_id: Option<RunId>,
    pub pattern_id: String,
    pub category: ThreatCategory,
    pub severity: Severity,
    pub action: ThreatAction,
    pub direction: ThreatDirection,
    pub provider_id: Option<String>,
    pub content_hash: ContentHash,
    pub at: DateTime<Utc>,
}
```

**必记事件**：列入 `security-trust.md §8` 的"必记事件白名单"，`SessionEventSinkPolicy::DEFAULT_NEVER_DROP_KINDS` 默认包含本事件。

#### 3.17.6 MemdirOverflow

```rust
pub struct MemdirOverflowEvent {
    pub session_id: SessionId,
    pub tenant_id: TenantId,
    pub file: MemdirFileTag,
    pub current_chars: u64,
    pub threshold: u64,
    pub strategy_applied: OverflowStrategy,
    pub at: DateTime<Utc>,
}

#[non_exhaustive]
pub enum OverflowStrategy {
    SectionTruncated { kept_sections: u32, dropped_sections: u32 },
    HeadOnly { kept_chars: u32 },
}
```

#### 3.17.7 MemoryConsolidationRan（feature-gated）

```rust
pub struct MemoryConsolidationRanEvent {
    pub session_id: SessionId,
    pub hook_id: String,
    pub promoted: Vec<MemoryId>,
    pub demoted: Vec<MemoryId>,
    pub draft_dreams_chars: u32,
    pub duration_ms: u32,
    pub at: DateTime<Utc>,
}
```

仅在 `consolidation` feature 启用时产生（详见 `harness-memory.md §9`）。

#### 3.17.8 与 `Event` 枚举的对接

`harness-contracts.md §3.3` 的 `Event` 枚举需新增以下变体：

```rust
Event::MemoryRecallDegraded(MemoryRecallDegradedEvent),
Event::MemoryRecallSkipped(MemoryRecallSkippedEvent),
Event::MemdirOverflow(MemdirOverflowEvent),
Event::MemoryConsolidationRan(MemoryConsolidationRanEvent),
```

> 既有 `MemoryUpserted` / `MemoryRecalled` / `MemoryThreatDetected` 不变；新增的 4 个事件按 §1.3 `#[non_exhaustive]` 规则升 minor。

### 3.18 Sandbox Events

> 与 `crates/harness-sandbox.md` §9 一一对齐。Sandbox 仅产生**与权限/Tool 不可替代**的事件，
> 子进程 stdout/stderr 字节流不进 Journal（避免数据放大），由 Tool 层在 `ToolUseCompleted`
> 中以摘要 / `BlobRef` 形式收敛。

#### 3.18.1 SandboxExecutionStarted / Completed

```rust
pub struct SandboxExecutionStartedEvent {
    pub session_id: SessionId,
    pub run_id: RunId,
    pub tool_use_id: Option<ToolUseId>,
    pub backend_id: String,
    pub fingerprint: ExecFingerprint,
    pub policy: SandboxPolicySummary,        // 不含 secret，仅枚举/数值
    pub at: DateTime<Utc>,
}

pub struct SandboxExecutionCompletedEvent {
    pub session_id: SessionId,
    pub run_id: RunId,
    pub tool_use_id: Option<ToolUseId>,
    pub backend_id: String,
    pub fingerprint: ExecFingerprint,
    pub exit_status: SandboxExitStatus,
    pub stdout_bytes_observed: u64,
    pub stderr_bytes_observed: u64,
    pub duration_ms: u64,
    pub overflow: Option<SandboxOverflowSummary>,
    pub at: DateTime<Utc>,
}

#[non_exhaustive]
pub enum SandboxExitStatus {
    Code(i32),
    Signal(i32),
    Timeout,
    InactivityTimeout,
    OutputBudgetExceeded,
    Cancelled,
    BackendError,
}
```

`SandboxPolicySummary` 是 `SandboxPolicy` 的瘦投影（`mode` / `scope` / `network` 标签 + `resource_limits` 数值），不复制 `denied_host_paths` 等可能含敏感路径的字段；细节由 `Redactor` 决定，详见 `security-trust.md §6`。

**必记**：列入 `SessionEventSinkPolicy::DEFAULT_NEVER_DROP_KINDS`。

#### 3.18.2 SandboxActivityHeartbeat / TimeoutFired

```rust
pub struct SandboxActivityHeartbeatEvent {
    pub session_id: SessionId,
    pub run_id: RunId,
    pub tool_use_id: Option<ToolUseId>,
    pub backend_id: String,
    pub since_last_io_ms: u64,
    pub at: DateTime<Utc>,
}

pub struct SandboxActivityTimeoutFiredEvent {
    pub session_id: SessionId,
    pub run_id: RunId,
    pub tool_use_id: Option<ToolUseId>,
    pub backend_id: String,
    pub configured_timeout: Duration,
    pub kill_scope: KillScope,
    pub at: DateTime<Utc>,
}
```

`Heartbeat` **可被采样**（参见 `harness-journal.md` §EventStream 背压策略），`TimeoutFired` 必记。

#### 3.18.3 SandboxOutputSpilled

```rust
pub struct SandboxOutputSpilledEvent {
    pub session_id: SessionId,
    pub run_id: RunId,
    pub tool_use_id: Option<ToolUseId>,
    pub stream: SandboxOutputStream,
    pub blob_ref: BlobRef,
    pub head_bytes: u32,
    pub tail_bytes: u32,
    pub original_bytes: u64,
    pub at: DateTime<Utc>,
}

#[non_exhaustive]
pub enum SandboxOutputStream {
    Stdout,
    Stderr,
    Combined,
}
```

落盘内容通过 `BlobStore`（`harness-contracts.md §3.7`）持久化；本事件只记元数据。**必记**。

#### 3.18.4 SandboxBackpressureApplied

```rust
pub struct SandboxBackpressureAppliedEvent {
    pub session_id: SessionId,
    pub run_id: RunId,
    pub tool_use_id: Option<ToolUseId>,
    pub queued_bytes: u64,
    pub paused_for_ms: u64,
    pub at: DateTime<Utc>,
}
```

慢消费者背压触发；可采样。

#### 3.18.5 SandboxSnapshotCreated

```rust
pub struct SandboxSnapshotCreatedEvent {
    pub session_id: SessionId,
    pub backend_id: String,
    pub kind: SessionSnapshotKind,
    pub size_bytes: u64,
    pub content_hash: [u8; 32],
    pub at: DateTime<Utc>,
}
```

**必记**：跨重启恢复需依赖此事件定位 snapshot。

#### 3.18.6 SandboxContainerLifecycleTransition

```rust
pub struct SandboxContainerLifecycleTransitionEvent {
    pub session_id: SessionId,
    pub backend_id: String,
    pub container_ref: ContainerRef,
    pub from: ContainerLifecycleState,
    pub to: ContainerLifecycleState,
    pub reason: ContainerLifecycleReason,
    pub at: DateTime<Utc>,
}

#[non_exhaustive]
pub enum ContainerLifecycleState {
    Provisioning,
    Ready,
    InUse,
    Idle,
    Stopping,
    Stopped,
    Failed,
}

#[non_exhaustive]
pub enum ContainerLifecycleReason {
    SessionAttached,
    SessionDetached,
    PoolReused,
    PoolEvicted,
    HealthCheckFailed,
    SnapshotRestore,
    Manual,
}

pub struct ContainerRef {
    pub backend_kind: String,    // "docker" / "k8s" / "nomad" / ...
    pub container_id: String,
}
```

**必记**：用于审计"容器何时被复用 / 销毁"，避免长寿命容器被静默回收。

#### 3.18.7 与 `Event` 枚举的对接

`harness-contracts.md §3.3` 的 `Event` 枚举需新增以下变体：

```rust
Event::SandboxExecutionStarted(SandboxExecutionStartedEvent),
Event::SandboxExecutionCompleted(SandboxExecutionCompletedEvent),
Event::SandboxActivityHeartbeat(SandboxActivityHeartbeatEvent),
Event::SandboxActivityTimeoutFired(SandboxActivityTimeoutFiredEvent),
Event::SandboxOutputSpilled(SandboxOutputSpilledEvent),
Event::SandboxBackpressureApplied(SandboxBackpressureAppliedEvent),
Event::SandboxSnapshotCreated(SandboxSnapshotCreatedEvent),
Event::SandboxContainerLifecycleTransition(SandboxContainerLifecycleTransitionEvent),
```

按 §1.3 `#[non_exhaustive]` 规则升 minor。`KillScope` / `SessionSnapshotKind` / `ExecFingerprint` 复用 `harness-contracts.md §3.4` 已声明的共享类型。

### 3.19 MCP Events

> 与 `crates/harness-mcp.md` §2 / §3 / §6 一一对齐。本节列出全部 MCP 事件结构。
>
> **必记事件**：`McpConnectionLost{terminal:true}` / `McpSamplingRequested{outcome:Denied|BudgetExceeded}` / `McpToolInjected{filtered_out:true}` 应进入 `SessionEventSinkPolicy::DEFAULT_NEVER_DROP_KINDS`（与 ADR-001 §6 对齐），其余 MCP 事件可由 `SessionEventSinkPolicy` 采样。

#### 3.19.1 `McpToolInjectedEvent`

```rust
pub struct McpToolInjectedEvent {
    pub session_id: SessionId,
    pub server_id: McpServerId,
    /// canonical 工具名（`mcp__<server>__<tool>`）。
    pub tool_name: String,
    pub upstream_name: String,
    pub defer_policy: DeferPolicy,
    /// 是否被 `McpToolFilter` 过滤掉（详见 `harness-mcp.md §2.6`）。
    /// `true` 时 `tool_name` 仍记录便于审计；ToolRegistry 中无对应 entry。
    pub filtered_out: bool,
    pub filter_reason: Option<String>,
    pub at: DateTime<Utc>,
}
```

#### 3.19.2 `McpConnectionLostEvent`

```rust
pub struct McpConnectionLostEvent {
    pub session_id: Option<SessionId>,         // Global server 时为 None
    pub server_id: McpServerId,
    pub server_source: McpServerSource,
    pub reason: McpConnectionLostReason,
    /// 当前已尝试重连次数（首次断连为 0）。
    pub attempts_so_far: u32,
    /// 是否已达到 `ReconnectPolicy.max_attempts` 终态。
    pub terminal: bool,
    pub at: DateTime<Utc>,
}

#[non_exhaustive]
pub enum McpConnectionLostReason {
    Network(String),
    AuthFailure(String),
    HandshakeMismatch(String),
    /// stdio 子进程崩溃 / 退出码非 0
    StdioProcessExited { exit_code: Option<i32>, signal: Option<i32> },
    /// 被 SDK 主动 shutdown（不计 reconnect）
    Shutdown,
    Other(String),
}
```

`terminal = true` 时**必记**；其他情况可采样。

#### 3.19.3 `McpConnectionRecoveredEvent`

```rust
pub struct McpConnectionRecoveredEvent {
    pub session_id: Option<SessionId>,
    pub server_id: McpServerId,
    pub server_source: McpServerSource,
    /// 是否是 SDK 启动后第一次 Ready（首次连接成功）。
    pub was_first: bool,
    /// 自最近一次 `McpConnectionLost` 起的累计 down 时间（首次连接为 0）。
    pub total_downtime_ms: u64,
    /// 累计重连尝试次数（首次连接为 0）。
    pub attempts_used: u32,
    /// 重连后服务端工具列表是否发生变化（用于触发后续 `McpToolsListChanged`）。
    pub schema_changed: bool,
    pub at: DateTime<Utc>,
}
```

`schema_changed = true` 时下一刻必有 `McpToolsListChanged` 配对（业务方可据此跳过冗余 diff）。

#### 3.19.4 `McpElicitationRequestedEvent` / `McpElicitationResolvedEvent`

```rust
pub struct McpElicitationRequestedEvent {
    pub session_id: SessionId,
    pub run_id: Option<RunId>,
    pub server_id: McpServerId,
    pub request_id: RequestId,
    pub subject: String,
    /// 仅元数据；具体 schema 走 `Decision`/`Permission` 通道，避免事件流过宽。
    pub schema_summary: ElicitationSchemaSummary,
    pub timeout: Option<Duration>,
    pub at: DateTime<Utc>,
}

pub struct ElicitationSchemaSummary {
    pub field_count: u16,
    pub required_count: u16,
    pub has_secret_field: bool,
}

pub struct McpElicitationResolvedEvent {
    pub session_id: SessionId,
    pub run_id: Option<RunId>,
    pub server_id: McpServerId,
    pub request_id: RequestId,
    pub outcome: ElicitationOutcome,
    pub at: DateTime<Utc>,
}

#[non_exhaustive]
pub enum ElicitationOutcome {
    Provided { value_hash: [u8; 32] },
    UserDeclined,
    Timeout,
    Invalid { reason: String },
    NoHandlerRegistered,
}
```

`Provided` 不写入明文值（凭证可能存在），仅记 hash 便于事后审计。

#### 3.19.5 `McpToolsListChangedEvent`

```rust
pub struct McpToolsListChangedEvent {
    pub session_id: Option<SessionId>,
    pub server_id: McpServerId,
    pub received_at: DateTime<Utc>,
    /// 自最近一次 `tools/list` 与本次 push 的间隔（去重 + 调试用）
    pub pending_since: Option<DateTime<Utc>>,
    /// SDK 计算出的 delta 概要；详细列表落入 `ToolDeferredPoolChanged`
    pub added_count: u32,
    pub removed_count: u32,
    /// 是否触发 `ToolDeferredPoolChanged`（仅 deferred）或 pending（含 AlwaysLoad）
    pub disposition: ToolsListChangedDisposition,
}

#[non_exhaustive]
pub enum ToolsListChangedDisposition {
    DeferredApplied,
    PendingForReload,
    NoChange,
}
```

#### 3.19.6 `McpResourceUpdatedEvent`

```rust
pub struct McpResourceUpdatedEvent {
    pub session_id: Option<SessionId>,
    pub server_id: McpServerId,
    pub kind: McpResourceUpdateKind,
    pub at: DateTime<Utc>,
}

#[non_exhaustive]
pub enum McpResourceUpdateKind {
    /// `resources/list_changed`
    ListChanged { added: u32, removed: u32 },
    /// 单条 `resources/updated { uri }`
    ResourceUpdated { uri: String },
    /// `prompts/list_changed`
    PromptsListChanged { added: u32, removed: u32 },
}
```

资源/Prompt 推送密度高时，本事件可由 `SessionEventSinkPolicy` 按 server_id 采样（不属于 `DEFAULT_NEVER_DROP_KINDS`）。

#### 3.19.7 `McpSamplingRequestedEvent`

```rust
pub struct McpSamplingRequestedEvent {
    pub session_id: SessionId,
    pub run_id: Option<RunId>,
    pub server_id: McpServerId,
    pub request_id: RequestId,
    pub model_id: Option<String>,            // 命中 ModelAllowlist 后的最终 model；Denied 时为 None
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub latency_ms: u64,
    pub outcome: SamplingOutcome,
    /// `IsolatedNamespace` 时形如 `mcp::sampling::<server>::<session>`；
    /// `SharedWithMainSession` 时为主 Session 的 cache namespace。
    pub prompt_cache_namespace: String,
    pub at: DateTime<Utc>,
}

#[non_exhaustive]
pub enum SamplingOutcome {
    Completed,
    Denied { reason: SamplingDenyReason },
    BudgetExceeded { dimension: SamplingBudgetDimension },
    RateLimited,
    UpstreamError { code: i32, message: String },
    Cancelled,
}

#[non_exhaustive]
pub enum SamplingDenyReason {
    PolicyDenied,                   // SamplingAllow::Denied
    ApprovalDenied,                 // 经审批被拒
    ModelNotAllowed,
    PermissionModeBlocked,          // BypassPermissions / DontAsk 强制降级
    InlineUserSourceRefused,        // user-controlled server + AllowAuto
}

#[non_exhaustive]
pub enum SamplingBudgetDimension {
    PerRequestInputTokens,
    PerRequestOutputTokens,
    PerRequestTimeout,
    PerRequestToolRounds,
    PerServerSessionInput,
    PerServerSessionOutput,
    PerSessionInput,
    PerSessionOutput,
}
```

`Denied` / `BudgetExceeded` / `RateLimited` 必记；`Completed` 可采样（高频 server 启用 `SamplingLogLevel::Summary` 时）。

#### 3.19.8 与 `Event` 枚举的对接

`harness-contracts.md §3.3` 已声明所有 8 个 MCP Event 变体：

```rust
Event::McpToolInjected(McpToolInjectedEvent),
Event::McpConnectionLost(McpConnectionLostEvent),
Event::McpConnectionRecovered(McpConnectionRecoveredEvent),
Event::McpElicitationRequested(McpElicitationRequestedEvent),
Event::McpElicitationResolved(McpElicitationResolvedEvent),
Event::McpToolsListChanged(McpToolsListChangedEvent),
Event::McpResourceUpdated(McpResourceUpdatedEvent),
Event::McpSamplingRequested(McpSamplingRequestedEvent),
```

`McpServerSource` 复用 `harness-contracts.md §3.4`；`DeferPolicy` 复用 `harness-tool.md §2.2`；`RequestId` 复用 §3.1。

### 3.20 Plugin 生命周期事件

> 权威定义见 `crates/harness-plugin.md §7（生命周期状态机）` / §4（RejectionReason）/ §16.1（必记审计事件）。本节定义事件载荷字段。

#### 3.20.1 PluginLoaded

```rust
pub struct PluginLoadedEvent {
    pub tenant_id: TenantId,
    pub plugin_id: PluginId,
    pub plugin_name: String,
    pub plugin_version: SemverString,
    pub trust_level: TrustLevel,
    pub capabilities: PluginCapabilitiesSummary,
    pub manifest_origin: ManifestOriginRef,
    pub manifest_hash: [u8; 32],
    /// 触发本次 Activated 转换的状态来源；通常是 `Validated`，重试场景下可能是 `Failed`
    pub from_state: PluginLifecycleStateDiscriminant,
    pub at: DateTime<Utc>,
}

pub struct PluginCapabilitiesSummary {
    pub tools: u16,
    pub hooks: u16,
    pub mcp_servers: u16,
    pub skills: u16,
    pub memory_provider: bool,
    pub coordinator: bool,
}

#[non_exhaustive]
pub enum ManifestOriginRef {
    File { path: String },
    CargoExtension { binary: String },
    RemoteRegistry { endpoint: String },
}
```

> `manifest_hash` 与 ADR-0014 验签后的 canonical bytes 一致，可用于跨节点核对插件版本一致性。

#### 3.20.2 PluginRejected

```rust
pub struct PluginRejectedEvent {
    pub tenant_id: TenantId,
    /// Manifest 已成功解析，因此 `plugin_id` 必然有值
    pub plugin_id: PluginId,
    pub plugin_name: String,
    pub plugin_version: SemverString,
    pub trust_level: TrustLevel,
    pub manifest_origin: ManifestOriginRef,
    pub manifest_hash: [u8; 32],
    /// 状态机转入 Rejected 时持有的 RejectionReason；详见 `harness-plugin.md §4`
    pub reason: RejectionReason,
    pub at: DateTime<Utc>,
}
```

#### 3.20.3 ManifestValidationFailed

```rust
/// Discovery / Manifest 解析阶段失败：
/// - YAML / JSON 解析错误
/// - JSON schema 不通过 / 未知字段
/// - `manifest_schema_version` 不被支持（早于 SDK 最低版本 / 晚于已知最高版本）
/// - cargo extension 元数据子命令冷输出无法解释为 ManifestRecord
///
/// 此时尚未构造出合法 `PluginId`（manifest 未解析或解析得到的字段不通过基础校验），
/// 因此事件不持有 `plugin_id`，仅以 `manifest_origin` 标识来源。
pub struct ManifestValidationFailedEvent {
    pub tenant_id: TenantId,
    pub manifest_origin: ManifestOriginRef,
    /// 若 manifest 已部分解析（如能拿到 name 但 version 字段类型错），可填入；否则为 None
    pub partial_name: Option<String>,
    pub partial_version: Option<String>,
    /// 解析阶段计算的"原始字节哈希"（不是 canonical hash），用于把同一个坏文件的多次重试去重
    pub raw_bytes_hash: [u8; 32],
    pub failure: ManifestValidationFailure,
    pub at: DateTime<Utc>,
}

#[non_exhaustive]
pub enum ManifestValidationFailure {
    /// YAML / JSON 等格式层面错误
    SyntaxError { details: String },
    /// JSON schema 不通过（含必填字段缺失 / 类型错 / 未知字段）
    SchemaViolation { json_pointer: String, details: String },
    /// `manifest_schema_version` 不在 SDK 支持范围内
    UnsupportedSchemaVersion { found: u32, supported: SchemaVersionRange },
    /// cargo extension 元数据子命令冷输出无法解释
    CargoExtensionMetadataMalformed { details: String },
    /// 远端注册中心返回的 manifest 字节签名 ETag 与 cache 不一致
    RemoteIntegrityMismatch { expected_etag: String, got_etag: Option<String> },
}

pub struct SchemaVersionRange {
    pub min: u32,
    pub max: u32,
}
```

> **与 `PluginRejected` 的边界**：
> - `ManifestValidationFailed`：尚不能构造合法 `PluginId`，仅持有 `manifest_origin`；解析失败属于"上游分发链"问题
> - `PluginRejected`：已解析出合法 manifest，但因 trust / signature / namespace / dependency / slot / signer revocation 等业务规则被拒；持有完整 `plugin_id`
> 两者**不重叠**也**不替代**对方；`harness-plugin §16.1` 必记审计事件清单同时收录两者。

#### 3.20.4 与 `Event` 枚举的对接

`harness-contracts.md §3.3` 已声明三个 Plugin 生命周期 Event 变体：

```rust
Event::PluginLoaded(PluginLoadedEvent),
Event::PluginRejected(PluginRejectedEvent),
Event::ManifestValidationFailed(ManifestValidationFailedEvent),
```

`PluginId / TrustLevel / RejectionReason / ManifestOriginRef / SchemaVersionRange` 由 `crates/harness-plugin.md` §2 / §4 / §3.2 给出权威定义；`SemverString` 复用 `harness-contracts.md §3.4`。

## 4. Envelope（外壳）

每个 Event 写入 Journal 时都带一层 Envelope：

```rust
pub struct EventEnvelope {
    pub offset: JournalOffset,
    pub event_id: EventId,
    pub session_id: SessionId,
    pub tenant_id: TenantId,
    pub run_id: Option<RunId>,
    pub correlation_id: CorrelationId,
    pub causation_id: Option<EventId>,
    pub schema_version: SchemaVersion,
    pub recorded_at: DateTime<Utc>,
    pub payload: Event,
}
```

- `offset` 由 EventStore 实现保证单增
- `schema_version` 使 replay 能识别版本并走迁移
- `correlation_id` 串联一次 Turn 内所有 Event（便于分布式追踪）

## 5. Projection 规则

### 5.1 SessionProjection

```rust
pub struct SessionProjection {
    pub id: SessionId,
    pub tenant_id: TenantId,
    pub state: SessionState,
    pub messages: Vec<MessageSummary>,
    pub pending_tool_uses: Vec<PendingToolUse>,
    pub open_permission_requests: Vec<PendingPermission>,
    pub usage_aggregate: UsageAggregate,
    pub last_offset: JournalOffset,
    /// ADR-009 Deferred Tool Loading 投影
    pub discovered_tools: DiscoveredToolProjection,
}

pub enum SessionState {
    Idle,
    Running { since: DateTime<Utc>, current_run: RunId },
    AwaitingPermission,
    AwaitingElicitation,
    Ended { reason: EndReason, at: DateTime<Utc> },
}
```

Projection 规则：

- `SessionProjection::apply(event)` 为**纯函数**，不产生副作用
- 同一 Event 多次 apply，结果等价（幂等）
- Projection 仅依赖当前状态 + Event，不依赖外部 IO

### 5.2 UsageProjection / 审计 Projection / 性能 Projection

除 SessionProjection 外，同一 Event Stream 可派生多种 Projection（详见 crates/harness-journal.md §7）。

## 6. Replay 语义

### 6.1 从头重放

```rust
let store = harness.event_store();
let projection = harness
    .replay_engine()
    .reconstruct(session_id, ReplayCursor::Start)
    .await?;
```

**保证**：

1. 对于同一 Event Stream，Replay 结果**确定性**（deterministic）
2. Replay 过程中**不产生任何副作用**（不写 Journal、不调用模型、不执行工具）
3. Replay 可与活跃 session 并行（只读）

### 6.2 从 Snapshot 重放

```rust
let snapshot = store.snapshot(session_id).await?;
let projection = harness
    .replay_engine()
    .reconstruct_from_snapshot(snapshot, ReplayCursor::After(snapshot.offset))
    .await?;
```

Snapshot 策略：

- 默认每 500 Event 或每 5 分钟生成一次
- Snapshot 只是加速手段，**不修改 Event Stream 语义**
- 丢失 Snapshot 可从 Start 重建

### 6.3 Diff

```rust
let diff = harness
    .replay_engine()
    .diff(session_a, session_b)
    .await?;
```

用途：
- 非确定性调试（同 input 下对比两次 run 的分叉点）
- 配置变更评估（相同 session + 两套 config 对比效果）

## 7. Schema 演进

### 7.1 原则

- **新增字段**：默认值可兼容旧版 Event → Minor 版本
- **重命名字段**：提供迁移器 + Schema 版本跳跃 → Major 版本
- **删除字段**：绝不允许直接删除；先标记 `#[deprecated]`，一个 Major 周期后再移除

### 7.2 迁移器

```rust
pub trait EventMigrator: Send + Sync + 'static {
    fn from_version(&self) -> SchemaVersion;
    fn to_version(&self) -> SchemaVersion;
    fn migrate(&self, envelope: EventEnvelope) -> Result<EventEnvelope, MigrationError>;
}
```

读取旧 Schema 时链式应用 Migrator 直到当前版本。写入**总是当前版本**。

### 7.2.1 运行时应用路径（`VersionedEventStore` 装饰器）

迁移器**不由具体 Store 实现**自行应用（`SqliteEventStore` / `JsonlEventStore` 不需要知道迁移规则），而是通过 `harness-journal::VersionedEventStore` 装饰器在读侧透明应用：

```rust
pub struct VersionedEventStore<S: EventStore> {
    inner: S,
    migrators: Arc<MigratorChain>,
    strict: bool,
}

pub struct MigratorChain {
    migrators: Vec<Box<dyn EventMigrator>>,
    cache: DashMap<(SchemaVersion, SchemaVersion), Vec<usize>>,
}

impl MigratorChain {
    pub fn builder() -> MigratorChainBuilder;

    /// 找 from→to 的最短链；缺失时返回 None
    pub fn find_path(
        &self,
        from: SchemaVersion,
        to: SchemaVersion,
    ) -> Option<Vec<&dyn EventMigrator>>;
}

impl<S: EventStore> EventStore for VersionedEventStore<S> {
    async fn read(/* ... */) -> Result<BoxStream<Event>, JournalError> {
        let raw = self.inner.read(/* ... */).await?;
        Ok(Box::pin(raw.map(|envelope| {
            let envelope = envelope?;
            let target = SchemaVersion::CURRENT;
            if envelope.schema_version == target {
                return Ok(envelope.payload);
            }
            // 链式应用 migrators
            let path = self.migrators
                .find_path(envelope.schema_version, target)
                .ok_or(JournalError::MigrationPathMissing { .. })?;
            let mut env = envelope;
            for m in path {
                env = m.migrate(env).map_err(JournalError::from)?;
            }
            Ok(env.payload)
        })))
    }

    async fn append(/* ... */) -> Result<...> {
        // 写入 **永远** 使用当前版本 envelope
        self.inner.append(...)
    }
}
```

### 7.2.2 业务层使用

```rust
let base = JsonlEventStore::open("runtime/events").await?;
let store = VersionedEventStore::builder(base)
    .with_migrator(V0ToV1Migrator)
    .with_migrator(V1ToV2Migrator)
    .strict(true)   // 缺路径即报错；false 时跳过该 Event 并记 Event::MigrationFailed
    .build();

let harness = HarnessBuilder::new()
    .with_store(store)
    // ...
    .build()
    .await?;
```

### 7.2.3 约束

- Migrator 必须**幂等**（对已迁移到目标版本的 envelope 再跑一次 migrate 应保持不变或显式 noop）
- `SchemaVersion` 在 contracts 中定义，与 SDK 版本号解耦（允许 SDK patch 升级不触发 schema bump）
- 迁移失败事件不得丢弃：`JournalError::MigrationFailed { envelope, from, to, cause }` 向上传递，业务可降级到"只读历史"模式

### 7.3 Schema 文件

- `harness-contracts/schemas/event.v1.json`：当前版本 JSON Schema
- `harness-contracts/schemas/event.v0.json`：历史版本（可用于生成迁移器）

由 `schemars` 自动派生 + 手工检查维护。

## 8. 与 OpenAPI / 前端共享

- 通过 `schemars::JsonSchema` derive 导出到 `contracts/openapi/`
- 前端（`@octopus/schema`）消费，跨端事件结构一致
- 业务层如需定义**私有事件**，不应扩展 `harness-contracts::Event` enum，而应通过 `HookEvent` 或独立 Journal（如业务自有 `runtime/events/business-*.jsonl`）

## 9. 写入保障

### 9.1 一致性

- Journal `append` 必须**原子**（同一 batch 的 events 要么全写入，要么全失败）
- `SqliteEventStore` 使用 `BEGIN IMMEDIATE` + 20~150ms 抖动重试（对齐 HER-021）
- `JsonlEventStore` 使用文件锁 + fsync 策略（配置项）
- Octopus 产品基线使用 `JsonlEventStore` 写 `runtime/events/*.jsonl`。`SqliteEventStore` 是通用 SDK 后端，不得把 `data/main.db` 改造成事件真相源。

### 9.2 可用性

- 写入失败时，Engine 降级到**拒绝处理该 Turn**（不能丢 Event）
- 多存储后端组合（如 `SqliteEventStore + JsonlEventStore` 双写）由业务层通过 `CompositeEventStore` 实现；Octopus 产品内不得双写两套权威事件流，只能从 JSONL 派生 SQLite projection

### 9.3 EventStream 背压与丢弃

> 详细 API 见 `crates/harness-session.md` §2.3.1。本节约定**"丢"的边界**。

Journal 写入（上述 §9.1/§9.2）与 EventStream 投递（对消费者）是**两条独立路径**：

- **Journal 写入不可丢**：所有 Event 必须进 Journal，否则 Replay 发散
- **EventStream 投递允许按策略丢**：消费者慢时，按 `SessionEventSinkPolicy.overflow` 决定；仅 `lossy_event_kinds` 白名单内事件可丢

**默认不可丢的事件**（`DEFAULT_NEVER_DROP_KINDS` 常量）：

```text
SessionCreated / SessionForked / SessionEnded
SessionReloadRequested / SessionReloadApplied
RunStarted / RunEnded
ToolUseRequested / ToolUseApproved / ToolUseDenied
PermissionRequested / PermissionResolved
PluginLoaded / PluginRejected / ManifestValidationFailed
MemoryThreatDetected
SubagentSpawned / SubagentAnnounced / SubagentTerminated / SubagentSpawnPaused
TeamCreated / TeamTerminated
ExecuteCodeWhitelistExtended
EngineFailed / UnexpectedError
```

这些事件决定业务语义正确性（审批、生命周期、安全事件），任何 OverflowPolicy 都**不得**丢弃它们；若 buffer 满则 `BlockProducer`。

**交互 UI 可丢的事件候选**：`AssistantDeltaProduced / UsageAccumulated / TraceSpanCompleted / SteeringMessageQueued / SteeringMessageApplied / SteeringMessageDropped / ExecuteCodeStepInvoked`。

边界如下：

| 订阅面 | `ExecuteCodeStepInvoked` 规则 |
|---|---|
| Journal | 必写。丢失即 replay / audit 发散 |
| 普通交互 UI EventStream | 可显式加入 `lossy_event_kinds` 采样 |
| 合规 / SIEM / 审计订阅 | 必须提升为 never-drop；不得使用默认 UI lossy 策略 |

安全开关型 `ExecuteCodeWhitelistExtended` 与审批型
`SteeringMessageDropped { reason: PluginDeny }` 仍走 never-drop（后者由
`SessionEventSinkPolicy::sticky_filter` 选择性提升）。业务层按订阅面加入
`lossy_event_kinds`，不得把 UI 降级策略复用于合规订阅。

## 10. 隐私与合规

- Event 中**不得**包含完整密钥 / OAuth Token（由 Redactor 在写前剥离）
- PII 字段按 `tenant_id` 的 `RedactionPolicy` 脱敏
- Journal 支持按 `tenant_id` / `session_id` 物理删除（GDPR 删除权），但删除后 Replay 该 Session 会抛 `Gone` 错误

## 11. 性能预算

| 指标 | 目标值 |
|---|---|
| 单 Event append p99 | < 5ms（SQLite WAL） / < 1ms（In-Memory） |
| Replay 吞吐 | > 10k events/sec（SQLite） / > 100k events/sec（In-Memory） |
| Projection 构建 | O(n) with n = #events；Snapshot 后 O(incremental) |
| 单 Event 平均大小 | ≤ 4KB（超大字段走 BlobRef） |

## 12. 测试要求

| 测试类型 | 覆盖目标 |
|---|---|
| 单元测试 | 每个 Event variant 序列化/反序列化往返 |
| Projection 测试 | 给定 Event 序列，Projection 结果与 expected 相等 |
| 幂等测试 | 任一 Event 单独 apply 两次，Projection 相等 |
| 回放对比测试 | 录制真实 Session，重放后 Projection 与录制一致 |
| 版本迁移测试 | 旧 Schema Event → 迁移 → 新 Schema Event，Projection 相等 |

详见 `crates/harness-journal.md §测试策略`。
