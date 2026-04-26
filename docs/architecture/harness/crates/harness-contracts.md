# `octopus-harness-contracts` · L0 契约层 SPEC

> 层级：L0（最底层） · 状态：Accepted
> 版本：1.0 · 依赖：仅标准库 + `serde` + `ulid` + `schemars` + `thiserror` + `chrono` + `strum` + `async-trait` + `futures`

## 1. 职责

提供整个 SDK 的**公共类型真相源**。任何其他 crate 的公开 API 必须使用本 crate 定义的 ID / Event / Error / Schema。

- **包含**：ID 类型、Event 枚举、共享 enum（Decision / PermissionMode / Severity / ...）、错误根类型、JSON Schema 导出
- **不包含**：任何业务逻辑、任何 trait 方法实现、任何 IO

## 2. 依赖

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
ulid = { version = "1", features = ["serde"] }
schemars = { version = "0.8", features = ["chrono", "ulid", "derive"] }
thiserror = "1"
chrono = { version = "0.4", features = ["serde"] }
bytes = { version = "1", features = ["serde"] }
secrecy = { version = "0.8", features = ["serde"] }
strum = { version = "0.27", features = ["derive"] }
async-trait = "0.1"
futures = "0.3"
```

## 3. 对外 API

### 3.1 ID 类型（TypedUlid Pattern）

```rust
pub struct TypedUlid<Scope> {
    inner: Ulid,
    _marker: PhantomData<Scope>,
}

pub struct SessionScope;
pub struct RunScope;
pub struct MessageScope;
pub struct ToolUseScope;
pub struct SubagentScope;
pub struct TeamScope;
pub struct AgentScope;
pub struct TenantScope;
pub struct RequestScope;
pub struct DecisionScope;
pub struct WorkspaceScope;
pub struct MemoryScope;

pub type SessionId = TypedUlid<SessionScope>;
pub type RunId = TypedUlid<RunScope>;
pub type MessageId = TypedUlid<MessageScope>;
pub type ToolUseId = TypedUlid<ToolUseScope>;
pub type SubagentId = TypedUlid<SubagentScope>;
pub type TeamId = TypedUlid<TeamScope>;
pub type AgentId = TypedUlid<AgentScope>;
pub type TenantId = TypedUlid<TenantScope>;
pub type RequestId = TypedUlid<RequestScope>;
pub type DecisionId = TypedUlid<DecisionScope>;
pub type WorkspaceId = TypedUlid<WorkspaceScope>;
pub type MemoryId = TypedUlid<MemoryScope>;

impl<S> TypedUlid<S> {
    pub fn new() -> Self { /* ULID 生成 */ }
    pub fn parse(s: &str) -> Result<Self, IdParseError>;
    pub fn as_bytes(&self) -> [u8; 16];
    pub fn timestamp_ms(&self) -> u64;
}

impl TenantId {
    /// 单租户场景下的默认 tenant（`Default::default()`）。
    /// 多租户场景下不得使用本常量。
    pub const SINGLE: TenantId = /* 固定 ULID */;

    /// 跨租户**显式共享**资源的哨兵；**仅供凭证池等少数共享资源使用**。
    /// 出现 `SHARED` 必须由业务方在 builder 期主动声明（不得作为默认值），
    /// 且 SDK 必记一条 `Event::CredentialPoolSharedAcrossTenants`（或同等审计事件）。
    /// 反模式：把 `SHARED` 当作"无 tenant"占位符塞入业务字段——多租户隔离凭此识别破窗。
    pub const SHARED: TenantId = /* 固定 ULID，与 SINGLE 不同 */;
}
```

### 3.2 其他 Hash ID

```rust
pub struct CorrelationScope;
pub struct CausationScope;
pub struct SnapshotScope;
pub struct BlobScope;
pub struct TransactionScope;
pub struct EventScope;
pub struct DeltaScope;
pub struct BreakpointScope;
/// Steering 软引导消息的命名空间（ADR-0017 §2.2）。
pub struct SteeringScope;

pub type SnapshotId     = TypedUlid<SnapshotScope>;
pub type BlobId         = TypedUlid<BlobScope>;
pub type TransactionId  = TypedUlid<TransactionScope>;
pub type CorrelationId  = TypedUlid<CorrelationScope>;
pub type CausationId    = TypedUlid<CausationScope>;
pub type EventId        = TypedUlid<EventScope>;
pub type DeltaHash      = TypedUlid<DeltaScope>;
pub type BreakpointId   = TypedUlid<BreakpointScope>;
/// Steering 消息 Id；详见 ADR-0017 与 `crates/harness-session.md §2.7`。
pub type SteeringId     = TypedUlid<SteeringScope>;

pub struct JournalOffset(pub u64);
pub struct ConfigHash(pub [u8; 32]);
```

**命名规范**（所有 Id 必须遵守）：

1. 语义化 ID（会在日志/审计/API 中出现的强语义标识符）→ **一律使用 `TypedUlid<XxxScope>`**
2. 数值计数器（单调自增、无跨系统含义）→ 允许 `newtype struct Foo(pub u64)`（如 `JournalOffset`）
3. 哈希（固定字节数组）→ 允许 `struct Foo(pub [u8; N])`（如 `ConfigHash`）

> 禁止新增 `pub struct XxxId(pub Ulid)` 形式——这会打破 TypedUlid 的编译期类型安全；遇到要么迁移为 `TypedUlid<XxxScope>`，要么明确加 reason 注释（并在 ADR 登记例外）。

### 3.3 Event 枚举

> `Event` / `Decision` / `DecidedBy` / `PermissionSubject` 等大型 enum 统一通过 `strum::EnumDiscriminants` 派生
> 对应的"判别量 enum"——`EventKind` / `DecisionDiscriminant` / `DecidedByDiscriminant` / `PermissionSubjectDiscriminant`，
> 用于跨节点过滤、`AuditQuery` 入参（`crates/harness-journal.md §2.6`）以及指标 label。
> 业务方**不得**再为同一组语义重复定义判别枚举。

```rust
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, strum::EnumDiscriminants)]
#[strum_discriminants(name(EventKind), derive(Hash, Eq, PartialEq, Serialize, Deserialize))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    SessionCreated(SessionCreatedEvent),
    SessionForked(SessionForkedEvent),
    SessionEnded(SessionEndedEvent),
    SessionReloadRequested(SessionReloadRequestedEvent),
    SessionReloadApplied(SessionReloadAppliedEvent),

    RunStarted(RunStartedEvent),
    RunEnded(RunEndedEvent),

    UserMessageAppended(UserMessageAppendedEvent),
    AssistantDeltaProduced(AssistantDeltaProducedEvent),
    AssistantMessageCompleted(AssistantMessageCompletedEvent),

    ToolUseRequested(ToolUseRequestedEvent),
    ToolUseApproved(ToolUseApprovedEvent),
    ToolUseDenied(ToolUseDeniedEvent),
    ToolUseCompleted(ToolUseCompletedEvent),
    ToolUseFailed(ToolUseFailedEvent),
    /// Stream-level liveness signal for long-running tools (ADR-010 + harness-tool §2.7)
    ToolUseHeartbeat(ToolUseHeartbeatEvent),
    /// Tool result exceeded `ResultBudget`; payload offloaded to BlobStore (ADR-010)
    ToolResultOffloaded(ToolResultOffloadedEvent),
    /// Registry rejected a same-name tool registration; details in ShadowedRegistration (harness-tool §2.5.1)
    ToolRegistrationShadowed(ToolRegistrationShadowedEvent),

    PermissionRequested(PermissionRequestedEvent),
    PermissionResolved(PermissionResolvedEvent),
    /// `FilePersistence` 完整性签名验证失败；
    /// 详见 `crates/harness-permission.md §6.1`。
    PermissionPersistenceTampered(PermissionPersistenceTamperedEvent),
    /// 审批去重命中（in-flight 合流 / 短窗口决策复用），未实际触达 UI / 链尾 Broker；
    /// 详见 `permission-model.md §6.3` 与 `crates/harness-permission.md §3.8`。
    PermissionRequestSuppressed(PermissionRequestSuppressedEvent),

    HookTriggered(HookTriggeredEvent),
    HookRewroteInput(HookRewroteInputEvent),
    HookReturnedAdditionalContext(HookContextPatchEvent),
    /// Hook 链路任一 handler 失败的总账事件（含 timeout/panic/transport/unauthorized 等）。
    /// 详见 `event-schema.md §3.7.2` 与 `crates/harness-hook.md §2.6.1`。
    HookFailed(HookFailedEvent),
    /// handler 返回了 §2.4 能力矩阵不允许的 `HookOutcome` variant。
    HookReturnedUnsupported(HookReturnedUnsupportedEvent),
    /// `PreToolUseOutcome` 字段互斥违规 / RewriteInput schema 不通过 / ContextPatch 超限 等。
    HookOutcomeInconsistent(HookOutcomeInconsistentEvent),
    /// in-process handler panic（Exec/HTTP 路径走 `HookFailed { cause_kind: Transport }`）。
    HookPanicked(HookPanickedEvent),
    /// 同 priority handler 在 OverridePermission 上给出冲突决策；
    /// dispatcher 按"Deny 压过 Allow"裁决后写出。
    HookPermissionConflict(HookPermissionConflictEvent),

    CompactionApplied(CompactionAppliedEvent),
    ContextBudgetExceeded(ContextBudgetExceededEvent),
    ContextStageTransitioned(ContextStageTransitionedEvent),

    McpToolInjected(McpToolInjectedEvent),
    McpConnectionLost(McpConnectionLostEvent),
    /// 上一个 `McpConnectionLost` 之后连接重新就绪（重连成功）。
    /// 详见 `event-schema.md §3.19` 与 `crates/harness-mcp.md §2.7`。
    McpConnectionRecovered(McpConnectionRecoveredEvent),
    McpElicitationRequested(McpElicitationRequestedEvent),
    McpElicitationResolved(McpElicitationResolvedEvent),
    McpToolsListChanged(McpToolsListChangedEvent),
    /// MCP Server 通过 `resources/list_changed` 或 `resources/subscribe` 推送资源变更。
    /// 详见 `event-schema.md §3.19` 与 `crates/harness-mcp.md §4.2`。
    McpResourceUpdated(McpResourceUpdatedEvent),
    /// MCP Server 反向调用 `sampling/createMessage`；事件落 Sampling 限额命中、
    /// budget / 调用结果摘要。详见 `event-schema.md §3.19` 与 `crates/harness-mcp.md §6`。
    McpSamplingRequested(McpSamplingRequestedEvent),

    ToolDeferredPoolChanged(ToolDeferredPoolChangedEvent),
    ToolSearchQueried(ToolSearchQueriedEvent),
    ToolSchemaMaterialized(ToolSchemaMaterializedEvent),

    SubagentSpawned(SubagentSpawnedEvent),
    SubagentAnnounced(SubagentAnnouncedEvent),
    SubagentTerminated(SubagentTerminatedEvent),
    /// `SubagentAdmin::pause_spawning(true|false)` 切换的审计事件；
    /// 列入 `DEFAULT_NEVER_DROP_KINDS`。详见 `event-schema.md §3.9.4` 与
    /// `crates/harness-subagent.md §3.2`。
    SubagentSpawnPaused(SubagentSpawnPausedEvent),
    /// 子代理 `DeferredInteractive` 路径下父 Session 镜像的转发请求；
    /// 详见 `crates/harness-subagent.md §6.2`。
    SubagentPermissionForwarded(SubagentPermissionForwardedEvent),
    SubagentPermissionResolved(SubagentPermissionResolvedEvent),

    TeamCreated(TeamCreatedEvent),
    TeamMemberJoined(TeamMemberJoinedEvent),
    TeamMemberLeft(TeamMemberLeftEvent),
    /// Watchdog 上报的成员卡死事件；与 `harness-subagent.md §4.3` 的
    /// `SubagentStalled` 对称。详见 `event-schema.md §3.10.1`。
    TeamMemberStalled(TeamMemberStalledEvent),
    AgentMessageSent(AgentMessageSentEvent),
    AgentMessageRouted(AgentMessageRoutedEvent),
    TeamTurnCompleted(TeamTurnCompletedEvent),
    TeamTerminated(TeamTerminatedEvent),

    MemoryUpserted(MemoryUpsertedEvent),
    MemoryRecalled(MemoryRecalledEvent),
    MemoryRecallDegraded(MemoryRecallDegradedEvent),
    MemoryRecallSkipped(MemoryRecallSkippedEvent),
    MemoryThreatDetected(MemoryThreatDetectedEvent),
    MemdirOverflow(MemdirOverflowEvent),
    MemoryConsolidationRan(MemoryConsolidationRanEvent),

    UsageAccumulated(UsageAccumulatedEvent),
    TraceSpanCompleted(TraceSpanCompletedEvent),

    PluginLoaded(PluginLoadedEvent),
    PluginRejected(PluginRejectedEvent),
    /// Manifest 在 Discovery / 解析阶段就失败（YAML 错 / JSON schema 不通过 / 未知字段 /
    /// `manifest_schema_version` 不被支持），此时尚未构造出可信 `PluginId`，仅有 origin。
    /// 与 `PluginRejected` 互斥：前者已解析出合法 manifest 但被业务规则拒绝；
    /// 详见 `event-schema.md §3.20` 与 `crates/harness-plugin.md §4`。
    ManifestValidationFailed(ManifestValidationFailedEvent),

    /// Sandbox 执行流（详见 `event-schema.md §3.18`）。stdout/stderr 字节不入 Journal，
    /// 由 `ToolUseCompleted` / `SandboxOutputSpilled` 收敛。
    SandboxExecutionStarted(SandboxExecutionStartedEvent),
    SandboxExecutionCompleted(SandboxExecutionCompletedEvent),
    SandboxActivityHeartbeat(SandboxActivityHeartbeatEvent),
    SandboxActivityTimeoutFired(SandboxActivityTimeoutFiredEvent),
    SandboxOutputSpilled(SandboxOutputSpilledEvent),
    SandboxBackpressureApplied(SandboxBackpressureAppliedEvent),
    SandboxSnapshotCreated(SandboxSnapshotCreatedEvent),
    SandboxContainerLifecycleTransition(SandboxContainerLifecycleTransitionEvent),

    /// Steering 消息已入队，但尚未 drain（ADR-0017 §2.5）。
    /// 详见 `event-schema.md §3.5.1` 与 `crates/harness-session.md §2.7`。
    SteeringMessageQueued(SteeringMessageQueuedEvent),
    /// Steering 消息已在 Safe Merge Point 合并到下一轮 user 消息或写入 EventStore
    /// （`SteeringKind::NudgeOnly`）。`merged_into_message_id = None` 表示仅留痕。
    SteeringMessageApplied(SteeringMessageAppliedEvent),
    /// Steering 消息因容量 / TTL / dedup / RunEnded 被丢弃；`reason` 见 `event-schema.md`。
    SteeringMessageDropped(SteeringMessageDroppedEvent),

    /// `execute_code` 脚本内部一次嵌入式工具调用的审计事件（ADR-0016 §2.4）。
    /// 与外层 `ToolUseRequested / ToolUseCompleted` 通过 `parent_tool_use_id`
    /// 形成可追溯链。详见 `event-schema.md §3.5.2`。
    ExecuteCodeStepInvoked(ExecuteCodeStepInvokedEvent),
    /// 业务在 `team_config.toml` 中显式扩展 `execute_code` 嵌入工具白名单时
    /// 发出（ADR-0016 §2.6）；用户控制 / MCP 来源工具被尝试加入会被
    /// `ConfigError::EmbeddedToolNotPermitted` 拒绝，不会触发本事件。
    ExecuteCodeWhitelistExtended(ExecuteCodeWhitelistExtendedEvent),

    EngineFailed(EngineFailedEvent),
    UnexpectedError(UnexpectedErrorEvent),
}
```

每个具体 event 结构详见 D4 · `event-schema.md`。

### 3.4 共享枚举

```rust
#[non_exhaustive]
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, strum::EnumDiscriminants)]
#[strum_discriminants(name(DecisionDiscriminant), derive(Hash, Eq, PartialEq, Serialize, Deserialize))]
pub enum Decision {
    AllowOnce,
    AllowSession,
    AllowPermanent,
    DenyOnce,
    DenyPermanent,
    Escalate,
}

#[non_exhaustive]
pub enum PermissionMode {
    Default,
    Plan,
    AcceptEdits,
    BypassPermissions,
    DontAsk,
    Auto,
}

#[non_exhaustive]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

#[non_exhaustive]
pub enum TrustLevel {
    AdminTrusted,
    UserControlled,
}

#[non_exhaustive]
pub enum DecisionScope {
    ExactCommand { command: String, cwd: Option<PathBuf> },
    ExactArgs(Value),
    ToolName(String),
    Category(String),
    PathPrefix(PathBuf),
    GlobPattern(String),
    /// `execute_code`（ADR-0016 §2.2 / §2.9）整段脚本的稳定指纹。
    /// `script_hash = blake3(script_text)`；用于规则匹配 / dedup 命中
    /// 同一脚本多次执行（典型重试），不参与脚本内嵌工具的子级 scope 计算。
    ExecuteCodeScript { script_hash: [u8; 32] },
    Any,
}

#[non_exhaustive]
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, strum::EnumDiscriminants)]
#[strum_discriminants(name(DecidedByDiscriminant), derive(Hash, Eq, PartialEq, Serialize, Deserialize))]
pub enum DecidedBy {
    User,
    Rule { rule_id: String },
    DefaultMode,
    Broker { broker_id: String },
    /// Hook 通过 `HookOutcome::OverridePermission(Decision)`（PermissionRequest 事件）
    /// 或 `HookOutcome::PreToolUse(PreToolUseOutcome { override_permission, .. })`
    /// （PreToolUse 事件三件套）直接覆盖决策；
    /// 详见 ADR-007 §2.6 与 `crates/harness-hook.md §2.4 / §2.4.1`。
    Hook { handler_id: String },
    Timeout { default: Decision },
    /// 子代理 `DeferredInteractive` 路径下，由父 Session 决策后回写到子 Session
    /// 的 `PermissionResolved`。`original_decided_by` 是父侧真实决策者
    /// （递归保留祖父链信息）。详见 `crates/harness-subagent.md §6.2`。
    ParentForwarded {
        parent_session_id: SessionId,
        original_decided_by: Box<DecidedBy>,
    },
}

/// `GraceCallTriggered` 是独立事件，不复用 `EndReason`；`Run` 在 grace 态后仍可正常 `Completed`
/// 或在下一轮越界时 `MaxIterationsReached`（详见 `event-schema.md §3.1.1`）。
#[non_exhaustive]
pub enum EndReason {
    Completed,
    MaxIterationsReached,
    TokenBudgetExhausted,
    /// 系统级中断（区别于用户/父级显式取消）：
    /// - 网络/IO 异常导致的非正常终止
    /// - SIGTERM 等进程级信号
    /// - Engine 内部断言失败的 graceful shutdown
    /// 不携带 initiator——系统中断没有"发起方"语义。
    Interrupted,
    /// 显式取消（用户/父 Session/系统配额）。`initiator` 必填，承载审计语义。
    /// 与 `Interrupted` 的区别：`Cancelled` 是有"发起方"的主动取消，
    /// `Interrupted` 是无"发起方"的被动终止。
    Cancelled { initiator: CancelInitiator },
    Error(String),
    Compacted,
}

/// 取消请求的发起方。
///
/// **审计契约**：`EndReason::Cancelled { initiator }` 写入 `RunEndedEvent` 后，
/// 业务层可按 initiator 维度分类统计，如"用户取消率""配额取消率"。
#[non_exhaustive]
pub enum CancelInitiator {
    /// 用户显式取消（UI 取消按钮 / `Session::cancel()` API / Ctrl-C 信号）。
    User,
    /// 父 Session 取消级联（subagent 收到 `SubagentHandle::cancel`，或父 Run 因 Cancelled 终止时级联给所有 in-flight subagent）。
    Parent,
    /// 系统级配额/策略取消（租户限额、欠费、运营运维强制取消）。
    /// `reason` 为审计字符串，不参与控制流。
    System { reason: String },
}

#[non_exhaustive]
pub enum StopReason {
    ToolUseRequested,
    AssistantFinished,
    UserInterrupted,
    TokenBudgetExceeded,
    MaxIterations,
}

/// Tool 的加载策略（ADR-009 · §2.2）。
///
/// 由工具注册方决定，Session 运行期不可修改；
/// `ToolProperties.defer_policy` 引用本枚举。
#[non_exhaustive]
pub enum DeferPolicy {
    /// 总是进入 ToolPool 固定集，schema 随创建期注入。
    AlwaysLoad,
    /// 由 Session 级 `ToolSearchMode` 决定是否延迟。
    AutoDefer,
    /// 强制延迟；Session 关闭 Tool Search 时拒绝注册（`ToolError::DeferralRequired`）。
    ForceDefer,
}

/// Tool Search 的查询形态（ADR-009 · §2.6）。
#[non_exhaustive]
pub enum ToolSearchQueryKind {
    /// `select:A,B,C` —— 直接按名选择
    Select,
    /// 关键字（含 `+required` 语法）
    Keyword,
}

/// Tool Loading Backend 的标识（ADR-009 · §2.4）。
/// 字符串形态以便事件序列化；SDK 内部统一常量见
/// `harness-tool-search::backend::*::NAME`。
pub type ToolLoadingBackendName = std::borrow::Cow<'static, str>;

/// 工具分组（用于 toolset 选择 / Pool 装配过滤；详见 `harness-tool.md §2.2`）。
#[non_exhaustive]
pub enum ToolGroup {
    FileSystem,
    Search,
    Network,
    Shell,
    Agent,
    Coordinator,
    Memory,
    Clarification,
    Meta,
    Custom(String),
}

/// 工具来源元数据，参与 Registry 同名裁决矩阵（`harness-tool.md §2.5.1`）。
#[non_exhaustive]
pub enum ToolOrigin {
    Builtin,
    /// `plugin_id` 使用 newtype，与 `McpServerSource::Plugin(PluginId)` /
    /// `SkillSourceKind::Plugin(PluginId)` / `SteeringSource::Plugin { plugin_id: PluginId }`
    /// 形态对齐，禁止裸 `String`（`module-boundaries.md §4.2` 的 newtype 强制约束）。
    Plugin { plugin_id: PluginId, trust: TrustLevel },
    Mcp(McpOrigin),
    Skill(SkillOrigin),
}

/// MCP 来源元数据。canonical 命名规则见 §3.4.2。
pub struct McpOrigin {
    pub server_id: McpServerId,
    /// MCP 服务端原始 tool 名（未经 `mcp__<server>__<tool>` 包装），仅做审计；
    /// 不参与 ToolRegistry 同名裁决与 PermissionRule 匹配（详见 `harness-mcp.md §3`）。
    pub upstream_name: String,
    /// MCP 服务端在 `tool/_meta` 中携带的信任标记（如 `anthropic/alwaysLoad`），
    /// 仅作 hint，最终 `DeferPolicy` 由本端 `ToolDescriptor` 决定。
    pub server_meta: BTreeMap<String, Value>,
    /// MCP Server 在本端的来源类别（决定是否允许被用户 agent inline 引用、
    /// 是否参与 enterprise 强制策略等，详见 `harness-mcp.md §2.2`）。
    pub server_source: McpServerSource,
    /// MCP Server 的信任级别，与 `ToolOrigin::Plugin.trust` 同维度。
    /// 由 `harness-mcp` 在 `add_server` 阶段根据 `server_source` 推导，业务层不可直接覆盖。
    pub server_trust: TrustLevel,
}

/// MCP Server 的来源类别。与 `ADR-006 · 插件信任域二分` + `Subagent` 的 Inline 准入策略联动。
///
/// `Workspace` / `Policy` / `Plugin{ admin_trusted }` / `Managed` 推导为 `TrustLevel::AdminTrusted`；
/// `User` / `Project` / `Plugin{ user_trusted }` / `Dynamic { actor: !admin }` 推导为 `UserControlled`。
#[non_exhaustive]
pub enum McpServerSource {
    /// 仓库内置或部署清单声明（octopus.toml / 容器镜像构建期注入）
    Workspace,
    /// 用户级配置（`~/.octopus/mcp/*.toml`）
    User,
    /// 项目级配置（`./.octopus/mcp/*.toml`）
    Project,
    /// 企业策略下发，业务层不得绕过；用户 / 项目级配置不可覆盖。
    Policy,
    /// 由插件清单声明，`trust` 来自插件本身（详见 `harness-plugin.md §2`）。
    Plugin(PluginId),
    /// 运行期通过 `McpRegistry::add_server` 动态注册；`registered_by` 仅作审计 hint
    /// （形如 `"user:alice"` / `"agent:research-assistant"` / `"api:webhook-uuid"`），
    /// 真正的鉴权由调用 `add_server` 的上游栈在自身契约中保证。
    Dynamic { registered_by: String },
    /// 外部 MCP 目录服务/市场注册返回（CC-19 `managed`）；`registry_url` 是
    /// 字符串形态的 URL（实际类型在 `harness-mcp` 中转换为 `url::Url`），
    /// contracts 层不引入 `url` 依赖。
    Managed { registry_url: String },
}

/// Skill 来源元数据。
///
/// 当 Skill 的渲染面通过 `SkillsInvokeTool` 暴露给模型时，对应 ToolDescriptor 的
/// `origin = Skill(SkillOrigin)`，便于 Permission / Audit / Replay 一致追溯。
/// 完整 `Skill` 数据结构与生命周期见 `harness-skill.md §2`。
pub struct SkillOrigin {
    pub skill_id: SkillId,
    pub skill_name: String,
    /// Skill 的物理来源（Bundled / Workspace / User / Plugin / Mcp）。
    /// 与 `ToolOrigin::Plugin` 中的 `trust` 互不替代：Plugin 来源的 Skill 仍保留外层 `Skill(...)` 形态，
    /// 信任级别由本字段携带，避免 ToolOrigin 枚举层级过深。
    pub source_kind: SkillSourceKind,
    pub trust: TrustLevel,
}

/// Skill 类型 ID（详见 `harness-skill.md §2.1`）。
pub struct SkillId(pub Ulid);

#[non_exhaustive]
pub enum SkillSourceKind {
    Bundled,
    Workspace,
    User,
    Plugin(PluginId),
    Mcp(McpServerId),
}

/// 同名注册裁决的拒绝原因（harness-tool §2.5.1 矩阵）。
#[non_exhaustive]
pub enum ShadowReason {
    /// 内置工具不可被覆盖
    BuiltinWins,
    /// 已存在更高 Trust 的同名工具
    HigherTrust,
    /// 完全重复定义（来源相同 / 内容相同）
    Duplicate,
}

/// Provider 适用范围；用于 ToolPool 按 `ModelCapabilities.provider` 过滤。
#[non_exhaustive]
pub enum ProviderRestriction {
    All,
    Allowlist(BTreeSet<ModelProvider>),
    Denylist(BTreeSet<ModelProvider>),
}

/// 单个 Tool 实例的输出预算（ADR-010 · §2.2）。
pub struct ResultBudget {
    pub metric: BudgetMetric,
    pub limit: u64,
    pub on_overflow: OverflowAction,
    pub preview_head_chars: u32,
    pub preview_tail_chars: u32,
}

#[non_exhaustive]
pub enum BudgetMetric {
    Chars,
    Bytes,
    Lines,
}

#[non_exhaustive]
pub enum OverflowAction {
    Truncate,
    Offload,
    Reject,
}

/// 命中预算后的元数据（ADR-010 · §2.3）。
pub struct OverflowMetadata {
    pub blob_ref: BlobRef,
    pub head_chars: u32,
    pub tail_chars: u32,
    pub original_size: u64,
    pub original_metric: BudgetMetric,
    pub effective_limit: u64,
}

/// Tool 在执行期请求的高权限子系统能力（ADR-011 · §2.2）。
/// 注册时由 `ToolDescriptor.required_capabilities` 声明，
/// 运行期由 `ToolContext::capability::<T>()` 借用。
#[non_exhaustive]
#[derive(Clone, Eq, PartialEq, Hash)]
pub enum ToolCapability {
    SubagentRunner,
    TodoStore,
    RunCanceller,
    ClarifyChannel,
    UserMessenger,
    BlobReader,
    HookEmitter,
    SkillRegistry,
    /// 调度本 Run 内已注册的、白名单内的 read-only 工具（ADR-0016 §2.7）。
    /// `default_locked()` 矩阵默认仅对 `ToolOrigin::Builtin` 工具开放。
    EmbeddedToolDispatcher,
    /// 在 `CodeSandbox`（`harness-sandbox §3.5`）中执行受限脚本（ADR-0016 §2.7）。
    /// 默认装配仅给 `ExecuteCodeTool` 一家工具使用；其他工具想申请此 capability
    /// 必须显式走 ADR + plugin manifest declares 双闸门。
    CodeRuntime,
    Custom(&'static str),
}
```

> **Capability traits 的位置**：`SubagentRunnerCap` / `TodoStoreCap` / `RunCancellerCap` / `ClarifyChannelCap` / `UserMessengerCap` / `BlobReaderCap` / `EmbeddedToolDispatcherCap` / `CodeRuntimeCap` 等接口 trait 定义在 `harness-contracts::capability` 模块；具体实现由对应 L2/L3 crate 提供。详见 ADR-011 §2.2 / §2.5 与 ADR-0016 §2.7。

```rust
/// 记忆条目的语义类型（对齐 CC-31）。
/// 与 `MemoryVisibility` 是正交维度：`MemoryKind` 描述「这是什么记忆」，
/// `MemoryVisibility` 描述「谁能看到」。详见 `harness-memory.md` §2.3。
#[non_exhaustive]
pub enum MemoryKind {
    UserPreference,
    Feedback,
    ProjectFact,
    Reference,
    AgentSelfNote,
    Custom(String),
}

/// 记忆的共享范围（对齐 CC-31 / OpenClaw `plugins.slots` 模型）。
#[non_exhaustive]
pub enum MemoryVisibility {
    Private { session_id: SessionId },
    User { user_id: String },
    Team { team_id: TeamId },
    Tenant,
}

/// Memory 写入动作（事件 / `MemoryLifecycle::on_memory_write` 共用）。
#[non_exhaustive]
pub enum MemoryWriteAction {
    AppendSection { section: String },
    ReplaceSection { section: String },
    DeleteSection { section: String },
    Upsert,
    Forget,
}

/// 记忆写入路径的来源标签（对齐 HER-018 双层模型）。
#[non_exhaustive]
pub enum MemorySource {
    UserInput,
    AgentDerived,
    SubagentDerived { child_session: SessionId },
    ExternalRetrieval,
    Imported,
    Consolidated { from: Vec<MemoryId> },
}

/// 威胁扫描分类（对齐 HER-019）。
#[non_exhaustive]
pub enum ThreatCategory {
    PromptInjection,
    Exfiltration,
    Backdoor,
    Credential,
    Malicious,
    SpecialToken,
}

/// 威胁扫描命中后的动作分级（对齐 HER-019 + OC-34）。
#[non_exhaustive]
pub enum ThreatAction {
    Warn,
    Redact,
    Block,
}

/// 威胁扫描的方向（写入前 / 召回后），用于事件分类。
#[non_exhaustive]
pub enum ThreatDirection {
    OnWrite,
    OnRecall,
}

/// Memdir 写入对当前 / 下一 Session 的生效时机（ADR-003 配套）。
/// 由 `BuiltinMemory::*_section` 返回；详见 `harness-memory.md` §3.3。
#[non_exhaustive]
pub enum TakesEffect {
    /// 写磁盘已生效；当前 Session 的 system message 不变；下一 Session 创建时读到新内容。
    NextSession,
    /// 通过 `Session::reload_with(ConfigDelta::ReloadMemdir)` 显式 fork；当前会话句柄
    /// 升级为新 Session（含新 system 提示）。
    AfterReloadWith,
}

/// Recall 失败的降级原因（事件 `MemoryRecallDegraded` 共用）。
#[non_exhaustive]
pub enum MemoryRecallDegradedReason {
    Timeout,
    ProviderError(String),
    RecordTooLarge,
    VisibilityViolation,
    ScannerBlocked,
}

/// 跨 crate 共享的沙箱策略与决策范围类型。
///
/// 具体语义、各 backend 的实现约束、`ExecFingerprint` 的 canonical 算法
/// 以及 `SandboxCapabilities` 的协商规则，详见 `crates/harness-sandbox.md` §2。
/// 本节只暴露**被其他 crate 引用**的共享形状，避免 `harness-session` /
/// `harness-subagent` / `harness-team` / `harness-permission` 反向依赖
/// `harness-sandbox`。
#[non_exhaustive]
pub enum SandboxMode {
    None,
    OsLevel(LocalIsolationTag),
    Container,
    Remote,
}

/// `LocalIsolation` 在契约层的影子标签；`harness-sandbox::LocalIsolation` 与之 1:1 对应。
/// 具体配置（bwrap profile、seatbelt scope、JobObject 限制）在 sandbox crate。
#[non_exhaustive]
pub enum LocalIsolationTag {
    None,
    Bubblewrap,
    Seatbelt,
    JobObject,
}

#[non_exhaustive]
pub enum SandboxScope {
    WorkspaceOnly,
    WorkspacePlus(Vec<PathBuf>),
    Unrestricted,
}

#[non_exhaustive]
pub enum NetworkAccess {
    None,
    LoopbackOnly,
    AllowList(Vec<HostRule>),
    Unrestricted,
}

pub struct HostRule {
    pub pattern: String,           // 域名 / CIDR / IP
    pub ports: Option<Vec<u16>>,
}

pub struct ResourceLimits {
    pub max_memory_bytes: Option<u64>,
    pub max_cpu_cores: Option<f32>,
    pub max_pids: Option<u32>,
    pub max_wall_clock: Option<Duration>,
    pub max_open_files: Option<u32>,
}

#[non_exhaustive]
pub enum WorkspaceAccess {
    None,
    ReadOnly,
    ReadWrite { allowed_writable_subpaths: Vec<PathBuf> },
}

pub struct SandboxPolicy {
    pub mode: SandboxMode,
    pub scope: SandboxScope,
    pub network: NetworkAccess,
    pub resource_limits: ResourceLimits,
    pub denied_host_paths: Vec<PathBuf>,
}

/// 命令稳定指纹；权限决策范围 `DecisionScope::ExactCommand` 命中时，
/// `Decision::AllowSession` / `AllowPermanent` 存储与比对的就是这个值。
/// canonical 算法详见 `crates/harness-sandbox.md` §2.2。
pub struct ExecFingerprint(pub [u8; 32]);

/// 信号送达范围；与 `ActivityHandle::kill` 对接。
#[non_exhaustive]
pub enum KillScope {
    Process,
    ProcessGroup,
    SessionLeader,
}

/// Session snapshot 的分层种类；不同 backend 仅支持其子集，
/// 由 `SandboxCapabilities.snapshot_kinds` 声明。
#[non_exhaustive]
#[derive(Eq, PartialEq, Ord, PartialOrd)]
pub enum SessionSnapshotKind {
    FilesystemImage,
    ShellState,
    ContainerImage,
}

/// Shell 类型；同时被 `harness-sandbox::LocalSandbox`、
/// `harness-permission::DangerousPatternLibrary::with_platform_*` 使用。
///
/// 放在契约层，避免 `harness-permission` 反向依赖 `harness-sandbox`
/// 形成循环（`LocalIsolationTag` 已采用相同模式）。
/// 具体的 shell 路径解析、PowerShell 版本探测等业务逻辑仍在 sandbox 实现。
#[non_exhaustive]
pub enum ShellKind {
    System,
    Bash(PathBuf),
    Zsh(PathBuf),
    PowerShell,
}
```

### 3.4.1 权限决策共享类型

> 本节聚合权限审批模型在契约层暴露的共享类型；具体决策流程见 `permission-model.md`，
> Broker 实现细节见 `crates/harness-permission.md`，事件流见 `event-schema.md §3.6`。

```rust
/// 规则源（对齐 CC-14 的 9 级分层 + Octopus 扩展的多租户 `Policy` 源）。
///
/// 合并顺序由低到高：`User < Workspace < Project < Local < Flag <
/// Policy < CliArg < Command < Session`。`Policy` 源由租户/运营方下发，
/// 其 `Decision::Deny*` 不可被任何低优先级源覆盖；其 `Decision::Allow*`
/// 仍可被更高优先级源（CliArg/Command/Session）按规则引擎流程覆盖。
///
/// 详见 `permission-model.md §4.1` 与 `crates/harness-permission.md §3.4`。
#[non_exhaustive]
pub enum RuleSource {
    User,
    Workspace,
    Project,
    Local,
    Flag,
    Policy,
    CliArg,
    Command,
    Session,
}

/// `PermissionRequest` 的语义化主体；UI 据此差异化渲染、Broker 据此索引历史决策。
///
/// 规则预检命中（`DecidedBy::Rule`）与正常 Broker 决策走同一份 subject，
/// 保证审计字段一致。详见 `permission-model.md §3.1`。
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, strum::EnumDiscriminants)]
#[strum_discriminants(name(PermissionSubjectDiscriminant), derive(Hash, Eq, PartialEq, Serialize, Deserialize))]
pub enum PermissionSubject {
    /// 通用工具调用（fallback 主体；适合非 shell 类业务工具）。
    ToolInvocation {
        tool: String,
        input: serde_json::Value,
    },
    /// Shell / Bash 类工具的命令执行（带规范化指纹）。
    CommandExec {
        command: String,
        argv: Vec<String>,
        cwd: Option<PathBuf>,
        fingerprint: Option<ExecFingerprint>,
    },
    /// 文件写入（含创建/覆盖）。
    FileWrite {
        path: PathBuf,
        bytes_preview: Vec<u8>,
    },
    /// 文件删除。
    FileDelete { path: PathBuf },
    /// 网络出站访问。
    NetworkAccess {
        host: String,
        port: Option<u16>,
    },
    /// 危险命令命中（来自 `DangerousPatternLibrary`）。
    DangerousCommand {
        command: String,
        pattern_id: String,
        severity: Severity,
    },
    /// MCP 工具调用（保留服务器/工具双层命名空间）。
    McpToolCall {
        server: String,
        tool: String,
        input: serde_json::Value,
    },
    /// 业务自定义类型；`kind` 由调用方声明，Broker 按 `Custom` 通配处理。
    Custom {
        kind: String,
        payload: serde_json::Value,
    },
}

/// 调用上下文的可交互性。Broker 据此决定是否能阻塞等待用户决策。
///
/// 与 CC-15 的 `shouldAvoidPermissionPrompts` / `awaitAutomatedChecksBeforeDialog`
/// 对齐。详见 `permission-model.md §3.2`。
#[non_exhaustive]
pub enum InteractivityLevel {
    /// 前台 CLI / Desktop / Web UI 等可阻塞等待用户答复的上下文。
    FullyInteractive,
    /// Subagent / Cron / 后台 worker 等无 UI 上下文：
    /// 命中无明确规则时按 `FallbackPolicy` 默认决策；
    /// **不得**写出 `PermissionRequested` 触发外部 UI。
    NoInteractive,
    /// Gateway / 多平台消息等异步审批：
    /// 决策请求挂入父 Session 的 EventStream，由父级处置；
    /// 同时受 `TimeoutPolicy` 兜底。
    DeferredInteractive,
}

/// 无规则命中且 Broker 也不能给出明确决策时的兜底策略。
///
/// 由 `RuleEngineBroker` / `ChainedBroker` / `DirectBroker` 在
/// `Decision::Escalate` 链尾消费。详见 `crates/harness-permission.md §3.7`。
#[non_exhaustive]
pub enum FallbackPolicy {
    /// 触发交互式询问（仅 `InteractivityLevel::FullyInteractive` 有效）。
    AskUser,
    /// 默认拒绝（fail-closed；与 `PermissionMode::DontAsk` 语义一致）。
    DenyAll,
    /// 仅自动放行只读工具（`ToolProperties::is_read_only == true`）；
    /// 其余拒绝。
    AllowReadOnly,
    /// 在历史决策（`AllowList` projection）中寻找最接近的同 scope 决策；
    /// 找不到时按 `DenyAll` 处理。
    ClosestMatchingRule,
}

/// 审批超时与异步保活策略。
///
/// 适用于 `StreamBasedBroker` 与 `ChainedBroker` 的等待环节，
/// 配合 `DecidedBy::Timeout` 形成可审计的兜底决策。
pub struct TimeoutPolicy {
    /// 等待用户/外部决策的最长时长。
    pub deadline: Duration,
    /// 超时时落实的决策。生产环境推荐 `Decision::DenyOnce`。
    pub default_on_timeout: Decision,
    /// 异步审批的保活心跳间隔（如 Gateway 队列、桌面通知）；
    /// `None` 时不发送 heartbeat。
    pub heartbeat_interval: Option<Duration>,
}
```
> **设计要点**：
>
> 1. `PermissionSubject` 取代旧版 `PermissionRequest.subject: String + detail: Option<String>`
>    平铺字段。原 `subject` / `detail` 由 UI/审计派生（`Display for PermissionSubject`），
>    避免两份事实源漂移。
> 2. `RuleSource::Policy` 是企业级硬闸门；`harness-permission` 在合并 RuleSnapshot 时
>    必须**先**应用 Policy 的 Deny，再处理低源 Allow。
> 3. `InteractivityLevel` 与 `TimeoutPolicy` 是 Broker 的输入，**不是** Broker 实现自己的
>    内部状态——它们由调用方（Engine / Subagent Runner / Cron Driver）按上下文设置。

### 3.4.2 工具命名 canonical 规则

> 所有面向 LLM 的 Tool Name（`ToolDescriptor.name`、`MessagePart::ToolUse.name`、
> `Rule.scope` 文本表示）必须使用本节定义的 canonical 形态。
> Tool Name 出现在 OpenAI / Anthropic Function Calling 协议里时，受
> `^[a-zA-Z0-9_-]{1,64}$` 字符集限制——**冒号 `:` 不在允许集**——故下文一律以
> 双下划线 `__` 作为命名空间分隔符。

```rust
/// SDK 全局 Tool Name 字符集（与上游 LLM 工具协议一致）。
///
/// `Display for PermissionSubject` 与 `Rule.scope` 的字符串形态都基于本约束生成；
/// 任何 `register` 路径都必须先 `validate_tool_name`，命中即拒绝。
pub const TOOL_NAME_PATTERN: &str = r"^[a-zA-Z0-9_-]{1,64}$";

#[derive(Debug, thiserror::Error)]
pub enum ToolNameError {
    #[error("tool name `{0}` violates `{TOOL_NAME_PATTERN}`")]
    Invalid(String),
    #[error("mcp namespace separator `__` is reserved; got `{0}`")]
    ReservedSeparator(String),
}

pub fn validate_tool_name(name: &str) -> Result<(), ToolNameError>;

/// MCP 工具的 canonical 命名：`mcp__<server>__<tool>`。
///
/// `server` / `tool` 内部不允许出现 `__`；若上游 MCP 服务器自身工具名包含 `__`，
/// `harness-mcp` 在 `inject_tools_into` 阶段必须按 `_` 折叠或拒绝注册
/// （见 `crates/harness-mcp.md §2.4`）。
pub fn canonical_mcp_tool_name(server: &str, tool: &str)
    -> Result<String, ToolNameError>;

/// MCP 工具命名的反向解析；用于规则引擎、UI 渲染、审计。
///
/// 输入 `mcp__slack__post_message` → `Some(("slack", "post_message"))`；
/// 非 MCP 工具或格式异常 → `None`。
pub fn parse_canonical_mcp_tool_name(name: &str) -> Option<(&str, &str)>;
```

**约束**：

1. `ToolRegistry::register` 调用 `validate_tool_name`；不合规的工具名 fail-closed
   返回 `RegistrationError::InvalidToolName`，**不**做静默改名。
2. `harness-mcp` 注入时统一调用 `canonical_mcp_tool_name`，禁止业务层自行
   `format!("mcp:{}:{}", ...)`（旧的冒号形态属于反模式，详见 ADR-0005 §6.3 修订）。
3. 规则配置文件里的 glob 模式同样要求 canonical 形态——例如
   `mcp__slack__*` 而非 `mcp__slack/*` 或 `mcp:slack:*`。
4. `Display for PermissionSubject::McpToolCall` 输出
   `mcp__<server>__<tool>`，与规则匹配键完全一致，避免审计/UI 与匹配键漂移。

### 3.4.3 Steering 软引导共享类型

> 本节聚合 Steering Queue（ADR-0017）在契约层暴露的共享类型。
> 队列容器（`SteeringQueue` 实例 / drain 行为）实现细节在 `crates/harness-session.md §2.7`，
> Engine 主循环 Safe Merge Point 见 `crates/harness-engine.md §3`，
> 事件 schema 见 `event-schema.md §3.5.1`。

```rust
/// 用户在运行期"软引导"主 Agent 的一条消息（ADR-0017 §2.2）。
///
/// 与 `Session::interrupt()`（硬中断）正交：本类型从不终止 Run，
/// 仅在下一个 Safe Merge Point 合并到将要发出的 user 消息中。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SteeringMessage {
    pub id: SteeringId,
    pub session_id: SessionId,
    /// `None` 表示在当前 idle 期入队；下一次 `Session::run_turn` 启动时随首轮 drain。
    pub run_id: Option<RunId>,
    pub kind: SteeringKind,
    pub priority: SteeringPriority,
    pub body: SteeringBody,
    pub queued_at: SystemTime,
    /// 与 ADR-0001 Event Envelope 共用的关联 Id；Replay / Audit 用。
    pub correlation_id: Option<CorrelationId>,
    pub source: SteeringSource,
}

/// Steering 合并语义（ADR-0017 §2.2）。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum SteeringKind {
    /// 追加补充意图（默认）：本条与已入队的 Append 顺序拼接，写入下一轮 user 消息尾部。
    Append,
    /// 替换最近一条尚未 drain 的 Append（典型："改主意了"）；同时清掉队列中其他 Append。
    Replace,
    /// 仅写入 EventStore 留痕，不进入 prompt（不消耗 token）。
    NudgeOnly,
}

/// Steering 消息的载荷（ADR-0017 §2.2）。
///
/// 注意：`Structured` 不会以 JSON Schema 形式注入 prompt——它在 Safe Merge Point
/// 由 `harness-engine` 展开为人类可读文本片段，避免 prompt 中混入半结构化噪声。
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum SteeringBody {
    Text(String),
    Structured {
        instruction: String,
        addenda: BTreeMap<String, Value>,
    },
}

/// Steering 优先级（ADR-0017 §2.2）。
///
/// `High` 命中下一个 Safe Merge Point 必定 drain，**不**受 `dedup_window` 折叠。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum SteeringPriority {
    Normal,
    High,
}

/// Steering 消息来源（ADR-0017 §2.7）。
///
/// `Plugin` 与 `AutoMonitor` 走单独的 `SteeringRegistration` capability handle，
/// 见 `crates/harness-plugin.md §2.4`；user-controlled 插件 fail-closed。
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum SteeringSource {
    User,
    Plugin { plugin_id: PluginId },
    AutoMonitor { rule_id: String },
}

/// Steering Queue 的运行期策略（ADR-0017 §2.3）。
///
/// `harness-session` 在 `SessionInner.steering_queue` 中持有；
/// 业务可在 `SessionBuilder::with_steering_policy` 显式覆盖默认。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SteeringPolicy {
    /// 队列容量上限（默认 8）。
    pub capacity: usize,
    /// 单条消息从 queued 到 applied 的最大停留时间（默认 60s）。
    /// 超时后 emit `Event::SteeringMessageDropped { reason: TtlExpired }`。
    pub ttl: Duration,
    /// 容量已满时的处理策略（ADR-0017 q5：默认 `DropOldest`）。
    pub overflow: SteeringOverflow,
    /// 同 Run 内对相同 `body_hash` 的去重窗口（默认 1500ms）；
    /// `SteeringPriority::High` 不受此窗口约束。
    pub dedup_window: Duration,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum SteeringOverflow {
    /// 默认（ADR-0017 q5）：扔掉最早未 drain 的一条；
    /// emit `Event::SteeringMessageDropped { reason: Capacity }`。
    DropOldest,
    /// 扔掉本次 push 的新一条。
    DropNewest,
    /// 阻塞 `push_steering(...)` 直到队列有空位（业务侧自负 backpressure）。
    BackPressure,
}

impl Default for SteeringPolicy {
    fn default() -> Self {
        Self {
            capacity: 8,
            ttl: Duration::from_secs(60),
            overflow: SteeringOverflow::DropOldest,
            dedup_window: Duration::from_millis(1500),
        }
    }
}
```

> **设计要点**：
>
> 1. 数据结构与 `interrupt()` 互补——`SteeringMessage` 永远不可终止 Run；
>    任何"硬中断"语义都仍走 `Session::interrupt()` + `InterruptToken`，
>    保持 P3「Single Loop, Single Brain」。
> 2. `SteeringBody::Structured.addenda` 走"展开为文本"路径，不向 LLM 注入新 schema，
>    与 ADR-0003 Prompt Cache Hard Constraint 兼容。
> 3. `SteeringSource::Plugin` 必须由 plugin manifest 显式声明
>    `capabilities.steering = true` 且为 `AdminTrusted` 才生效；运行期由
>    `harness-plugin::PluginActivationContext.steering` 派发。
> 4. 默认丢弃事件 `reason` 取值集合：`Capacity / TtlExpired / DedupHit / RunEnded /
>    SessionEnded / PluginDenied`（具体 enum 在 `event-schema.md §3.5.1`）。

### 3.5 消息与内容

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum MessageContent {
    Text(String),
    Structured(Value),
    Multimodal(Vec<MessagePart>),
}

#[non_exhaustive]
pub enum MessagePart {
    Text(String),
    Image { mime_type: String, blob_ref: BlobRef },
    ToolUse { id: ToolUseId, name: String, input: Value },
    ToolResult { tool_use_id: ToolUseId, content: ToolResult },
    /// Thinking / Reasoning 块。**必须保留 provider-native 字段**，否则在 cache 命中
    /// 的下一轮请求里 Anthropic 会因 `signature` 缺失拒绝；OpenAI Responses 也需要
    /// `encrypted_content` 才能正确 cache reasoning。详见 `crates/harness-model.md` §2.2 `ThinkingDelta`。
    Thinking(ThinkingBlock),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ThinkingBlock {
    /// 人类可读文本（可选；部分 provider 仅返回 encrypted 内容）
    pub text: Option<String>,
    /// `provider_id`，用于约束 `provider_native` 只能回灌给同一家 provider
    pub provider_id: String,
    /// Provider 原生结构（Anthropic `thinking` / OpenAI reasoning item / Gemini thought）。
    /// 跨 provider 不可互换；序列化时整段保留。
    pub provider_native: Option<Value>,
    /// Anthropic `signature`，用于 prompt-cache 续接；其他 provider 为 `None`
    pub signature: Option<String>,
}

pub enum ToolResult {
    Text(String),
    Structured(Value),
    Blob { content_type: String, blob_ref: BlobRef },
    Mixed(Vec<ToolResultPart>),
}

/// 工具结果的结构化内容块（ADR-0002 §6 + 本节正面白名单）。
///
/// **设计原则**（与 ADR-0002 协同）：
/// - 仅承载**语义**：每个变体描述"内容是什么"，不描述"怎么渲染"
/// - 渲染层（`ToolViewRenderer` 等）从语义变体推导 UI；本枚举不引入任何 UI 词汇
/// - 受 ADR-0002 §6 反向黑名单保护：禁止出现 `Html / ReactElement / VueComponent /
///   TauriCommand / EguiNode / Markdown` 等渲染相关 variant
/// - 任何新增 variant 必须能被 `harness-tool::ToolOrchestrator` 在不依赖 UI crate
///   的前提下序列化、走 `ResultBudget`、写入 EventStore
///
/// 业务侧若需要新增 variant：走 ADR + `harness-contracts` 增量评审，不得在 SDK
/// 默认实现里偷偷加。
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ToolResultPart {
    /// 纯文本片段（最常见）。
    Text { text: String },
    /// 受限结构化数据（适用于已知 schema 的工具输出，如 `Glob` 命中列表）。
    /// `schema_ref` 可选：指向 `tool/output_schema` 内的 JSON Pointer，便于
    /// 调用方校验或 UI 反查 ToolDescriptor 中的 schema 片段。
    Structured {
        value: Value,
        schema_ref: Option<String>,
    },
    /// 二进制 / 大文本落盘引用（与 ADR-0010 ResultBudget 共用 `BlobRef`）。
    /// `summary` 可选：当 budget overflow 时由 orchestrator 注入头/尾摘要。
    Blob {
        content_type: String,
        blob_ref: BlobRef,
        summary: Option<String>,
    },
    /// 代码片段（脚本输出 / 模型阅读 / `execute_code` 步骤产物）。
    /// `language` 取值受 `harness-skill::CodeLanguage` 同一字符集约束（如 "rust"
    /// / "python" / "shell" / "lua" / "json"）。**不**触发 UI 高亮——是否高亮
    /// 完全由渲染层决策。
    Code {
        language: String,
        text: String,
    },
    /// 引用类内容（链接 / 文件 / 子代理 transcript）。`reference_kind` 见 `ReferenceKind`。
    /// 字段不能命名为 `kind`，因为外层 `#[serde(tag = "kind")]` 已占用该 key。
    Reference {
        reference_kind: ReferenceKind,
        title: Option<String>,
        summary: Option<String>,
    },
    /// 表格化数据（语义为「行 × 列」，非 UI Table）。
    /// `headers.len() == row.len()` 必须成立，否则 orchestrator fail-closed
    /// `ToolError::MalformedToolResult`。
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<Value>>,
        caption: Option<String>,
    },
    /// 工具自身在执行过程中产生的进度 / 时序数据（用于 UI 时间线、Audit）。
    /// 不参与 LLM 上下文（默认 orchestrator 在转 `MessagePart::ToolResult` 时
    /// 折叠为 `summary` 字段）；必须保留以便事件溯源。
    Progress {
        stage: String,
        ratio: Option<f32>,
        detail: Option<String>,
    },
    /// 错误片段（与 `ToolResultEnvelope.is_error` 不冲突；本变体用于
    /// `Mixed` 中"部分成功部分失败"场景，外层 envelope 仍记总错误位）。
    Error {
        code: String,
        message: String,
        retriable: bool,
    },
}

/// `ToolResultPart::Reference` 的子类型；在 §3.5 内紧邻 `ToolResultPart` 定义。
///
/// 渲染层依据本枚举挑选合适的 widget，但 SDK 内核不做任何 UI 假设。
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "ref_kind", rename_all = "snake_case")]
pub enum ReferenceKind {
    /// 外部 URL；`url` 必须是可序列化字符串（`url::Url` 不引入 contracts 依赖）。
    Url { url: String },
    /// 工作区相对或绝对文件；`path` 与 `harness-permission::PathPrefix` 同口径。
    File {
        path: PathBuf,
        line_range: Option<(u32, u32)>,
    },
    /// 子代理 / Team transcript（`harness-journal` 落盘）；与
    /// `Event::SubagentAnnounced.transcript_ref` 同型。
    Transcript(TranscriptRef),
    /// 上一条工具调用引用（自身 Run 内）；用于跨步骤回指。
    ToolUse { tool_use_id: ToolUseId },
    /// 记忆条目；用于让模型展示"我引用了这条 memory"，不含原文负载。
    Memory { memory_id: MemoryId },
}
```

> **`ToolResult::Mixed` 的 LLM 适配**：`harness-tool::ToolOrchestrator` 在最终
> 回灌 `MessagePart::ToolResult` 时，按 ADR-0002 §6 把 `ToolResultPart::Progress`
> 折叠、把 `Error` 与 `Text/Code/Reference` 重新拼为面向 LLM 的可读字符串；
> 整体仍受 `ToolResultEnvelope.usage` 与 `ResultBudget` 约束。

/// 工具执行的最终回执（ADR-010 · §2.3）。
///
/// 包裹 `ToolResult` 与可选的 `OverflowMetadata`。
/// `harness-tool::ToolOrchestrator` 在收尾时构造，用于：
/// - 给上层注入"完整内容已落盘"的元数据，避免静默截断；
/// - 让 `Event::ToolUseCompleted` / `Event::ToolResultOffloaded` 共享同一来源。
pub struct ToolResultEnvelope {
    pub result: ToolResult,
    pub usage: Option<UsageSnapshot>,
    pub is_error: bool,
    pub overflow: Option<OverflowMetadata>,
}

pub struct BlobRef {
    pub id: BlobId,
    pub size: u64,
    pub content_hash: [u8; 32],
    pub content_type: Option<String>,
}

/// 子代理 / 多代理 transcript 持久化引用；指向 BlobStore 中的完整对话流落盘。
/// `Event::SubagentAnnounced.transcript_ref` / `Event::TeamTurnCompleted` 等
/// 引用本类型；BlobStore 写入由 `harness-journal` 在 `AnnounceMode::FullTranscript`
/// 路径上完成。
pub struct TranscriptRef {
    pub blob: BlobRef,
    /// 起止事件偏移；用于按区间回溯而无需读完整 BlobStore 对象。
    pub from_offset: JournalOffset,
    pub to_offset: JournalOffset,
}

/// 子代理终结原因；与 `SubagentStatus` 互补——前者是"被如何终结"，
/// 后者是"自我陈述的终态"。详见 `event-schema.md §3.9.3`。
#[non_exhaustive]
pub enum SubagentTerminationReason {
    NaturalCompletion,
    ParentCancelled,
    AdminInterrupted { admin_id: String },
    Stalled { silent_for_ms: u64 },
    BridgeBroken,
    Failed { detail: String },
}
```

### 3.6 Memory 共享类型

事件层 `MemoryUpserted / MemoryRecalled / MemoryThreatDetected` 引用的扁平字段类型集中在此；具体的 `MemoryStore` / `MemoryLifecycle` trait 与 `MemoryRecord` / `MemorySummary` / `MemoryMetadata` 复合结构定义在 `harness-memory` crate（详见 `crates/harness-memory.md` §2）。

```rust
/// 用于 Event 与 ThreatScanReport 的内容哈希；不携带 raw content 防泄漏。
pub struct ContentHash(pub [u8; 32]);

impl ContentHash {
    pub fn of(bytes: &[u8]) -> Self {
        // SHA-256；具体在 `harness-contracts::hash` 模块实装
        Self(sha2_hash(bytes))
    }
}

/// MemoryQuery 使用的鉴权主体；`harness-memory` 在 recall 路径上做 visibility 过滤。
pub struct MemoryActor {
    pub tenant_id: TenantId,
    pub user_id: Option<String>,
    pub team_id: Option<TeamId>,
    pub session_id: Option<SessionId>,
}

/// 写入路径的目标描述。`MemdirFile` 仅由 `harness-memory` 暴露具体常量集合。
pub struct MemoryWriteTarget {
    pub kind: MemoryKind,
    pub visibility: MemoryVisibility,
    pub destination: WriteDestination,
}

#[non_exhaustive]
pub enum WriteDestination {
    Memdir(MemdirFileTag),
    External { provider_id: String },
}

/// Memdir 文件枚举的契约层影子；`harness-memory::MemdirFile` 与之 1:1 对应。
#[non_exhaustive]
pub enum MemdirFileTag {
    Memory,
    User,
    Dreams,
}
```

> **为何拆到 contracts**：`MemoryUpsertedEvent` 等事件必须能在没有 `harness-memory` 依赖的下游使用（例如 `octopus-server` 的 Webhook 转发器）。把 enum 类型放契约层、复合 `MemoryRecord` 结构留 crate 内是 BlobRef / BlobStore 的同款分层。

### 3.7 `BlobStore` trait（大块数据存取）

```rust
#[async_trait]
pub trait BlobStore: Send + Sync + 'static {
    fn store_id(&self) -> &str;

    async fn put(
        &self,
        tenant: TenantId,
        bytes: Bytes,
        meta: BlobMeta,
    ) -> Result<BlobRef, BlobError>;

    async fn get(
        &self,
        tenant: TenantId,
        blob: &BlobRef,
    ) -> Result<BoxStream<Bytes>, BlobError>;

    async fn head(
        &self,
        tenant: TenantId,
        blob: &BlobRef,
    ) -> Result<Option<BlobMeta>, BlobError>;

    async fn delete(
        &self,
        tenant: TenantId,
        blob: &BlobRef,
    ) -> Result<(), BlobError>;
}

pub struct BlobMeta {
    pub content_type: Option<String>,
    pub size: u64,
    pub content_hash: [u8; 32],
    pub created_at: DateTime<Utc>,
    pub retention: BlobRetention,
}

pub enum BlobRetention {
    SessionScoped(SessionId),
    TenantScoped,
    RetainForever,
    TtlDays(u32),
}

#[derive(Debug, thiserror::Error)]
pub enum BlobError {
    #[error("blob not found: {0:?}")]
    NotFound(BlobId),
    #[error("content hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },
    #[error("size exceeds limit: {size} > {limit}")]
    TooLarge { size: u64, limit: u64 },
    #[error("tenant denied: {0:?}")]
    TenantDenied(TenantId),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("backend: {0}")]
    Backend(String),
}
```

**设计说明**：

- `BlobRef` 只在 `Event` / `Message` / `ToolResult` 里流转；实际字节由 `BlobStore` 实现存取（文件 / S3 / OSS / SQLite BLOB）。
- `BlobStore::put` 返回 `BlobRef` 包含 `content_hash`，业务方拿到 `BlobRef` 就能 `get` 回原内容。
- `BlobRetention` 约束 blob 生命周期，与 Session / Tenant 的清理联动（Journal prune 触发 blob GC）。
- **内容寻址（可选语义）**：实现可选择以 `content_hash` 作为 `BlobId`（`BlobId = blake3(bytes)`），让 `put` 在哈希冲突时直接返回既有 `BlobRef` 而不重复写入。是否启用由具体实现的构造参数决定（详见 `harness-journal.md` §3.1 `FileBlobStore::content_addressed`）。Trait 层保持中立：调用方**不得**假设两次相同字节的 `put` 返回相同的 `BlobId`，但**必须**接受这种结果。
- 内置实现：`FileBlobStore`（L1 `harness-journal` crate）、`InMemoryBlobStore`（testing）。业务可实现 `S3BlobStore` 等。

### 3.8 错误根类型

```rust
#[derive(Debug, thiserror::Error)]
pub enum HarnessError {
    #[error("prompt cache locked — use Session::reload_with()")]
    PromptCacheLocked,

    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("tool not found: {0}")]
    ToolNotFound(String),

    #[error("invalid tenant: {0}")]
    InvalidTenant(TenantId),

    #[error("budget exhausted: {0:?}")]
    BudgetExhausted(BudgetKind),

    #[error("interrupted by user")]
    Interrupted,

    #[error("model error: {0}")]
    Model(#[from] ModelError),

    #[error("sandbox error: {0}")]
    Sandbox(#[from] SandboxError),

    #[error("journal error: {0}")]
    Journal(#[from] JournalError),

    #[error("internal error: {0}")]
    Internal(String),
}
```

子 crate 定义各自的 `XxxError`，但必须 `impl From<XxxError> for HarnessError`。

### 3.8.1 `BudgetKind`（共享于错误与事件）

`BudgetKind` 同时被 `HarnessError::BudgetExhausted` 与 `Event::ContextBudgetExceeded`（`event-schema.md §3.8.2`）引用，故定义于 L0 contracts：

```rust
#[non_exhaustive]
pub enum BudgetKind {
    /// 越过 `TokenBudget::soft_budget_ratio`
    SoftBudget,
    /// 越过 `TokenBudget::hard_budget_ratio`
    HardBudget,
    /// 越过 `TokenBudget::max_tokens_per_turn`
    PerTurnTokens,
    /// 越过 `TokenBudget::max_tokens_per_session`
    PerSessionTokens,
    /// 单 Tool 结果体越过 `ToolProperties::max_result_size_chars`
    PerToolMaxChars { tool_name: String },
}
```

### 3.9 Schema 导出

```rust
pub fn generate_schema() -> RootSchema {
    schemars::schema_for!(Event)
}

pub fn generate_openapi_components() -> OpenApiComponents {
    /* 从 schemars 输出转换为 OpenAPI 3.x components */
}
```

用于 `contracts/openapi/` 与 `packages/schema/src/generated.ts` 的源。

## 4. 内置能力

### 4.1 `TypedUlid` 的编译期类型安全

```rust
let session_id: SessionId = SessionId::new();
let run_id: RunId = session_id;   // ❌ 编译失败（不同 scope 不可转换）
```

### 4.2 `schemars` 派生

所有公共 struct 标注 `#[derive(JsonSchema)]`；Event variants 标注 `#[serde(tag = "type", rename_all = "snake_case")]` 以便前端强类型解析。

### 4.3 `TenantId::SINGLE`

单租户场景默认值：

```rust
impl Default for TenantId {
    fn default() -> Self {
        Self::SINGLE
    }
}
```

## 5. 版本兼容策略

- 所有 Event variant / enum variant 标注 `#[non_exhaustive]`
- 新增 variant 为 minor bump
- 重命名 variant 为 major bump
- 保留 `harness_contracts::migrate` 模块，跨 major 版本提供迁移器

## 6. 使用示例

### 6.1 业务层

```rust
use octopus_harness_sdk::prelude::*;
// prelude re-export 了 contracts 所有类型

let session_id = SessionId::new();
let event = Event::RunStarted(RunStartedEvent {
    run_id: RunId::new(),
    session_id,
    tenant_id: TenantId::SINGLE,
    /* ... */
});
```

### 6.2 生成 JSON Schema

```rust
use octopus_harness_contracts::generate_schema;

let schema = generate_schema();
let json = serde_json::to_string_pretty(&schema)?;
std::fs::write("docs/architecture/harness/event.schema.json", json)?;
```

## 7. 测试策略

| 测试类 | 范围 |
|---|---|
| 单元测试 | ID 解析 / Display 往返；Event 序列化往返；Schema 稳定性快照 |
| 属性测试 | Event 的 json 序列化是幂等且可解析 |
| Golden | `docs/architecture/harness/event.schema.json` 变动即视为破坏性修改，CI 阻止合并 |

## 8. 反模式

| 反模式 | 原因 |
|---|---|
| 在 Event 里嵌入大 blob（> 10KB）| 落盘 Event 膨胀；应用 `BlobRef` |
| 把 `String` 当 ID（比如 `pub tool_name: String`）| 应用 newtype `ToolName(String)` |
| 让 L1+ crate 定义新的 Id 类型 | 破坏单一契约源 |
| `#[non_exhaustive]` 缺失 | 加 variant 就破坏兼容 |

## 9. 相关

- D3 · `api-contracts.md` §1 总览
- D4 · `event-schema.md`（每个 Event 字段详情）
- ADR-001 Event Sourcing
- `packages/schema/` 业务前端 schema（由 `harness-contracts` 生成）
