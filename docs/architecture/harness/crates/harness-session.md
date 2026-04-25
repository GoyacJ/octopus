# `octopus-harness-session` · L2 复合能力 · Session Lifecycle SPEC

> 层级：L2 · 状态：Accepted
> 依赖：`harness-contracts` + 全部 L1 + L2（tool/skill/mcp/hook/context）

## 1. 职责

实现 **Session 生命周期管理**：创建、运行、中断、Fork、Compact、Hot Reload（via Fork）、Snapshot。是 L2 层的"聚合者"。

**核心能力**：

- Session 生命周期（Create → Turn → Compact → End）
- Projection（从 Event 重建当前状态）
- Session Fork（克隆 session + 增量 ConfigDelta）
- Hot Reload via Fork（ADR-003）
- Workspace Bootstrap 绑定
- Multi-tenant scoping

## 2. 对外 API

### 2.1 Session

```rust
pub struct Session {
    pub id: SessionId,
    pub tenant_id: TenantId,
    pub options: SessionOptions,
    inner: Arc<SessionInner>,
}

pub struct SessionOptions {
    pub permission_mode: PermissionMode,
    pub model_ref: ModelRef,
    pub sandbox_policy: SandboxPolicy,
    pub workspace_ref: Option<WorkspaceId>,   // 新增：绑定到 Workspace
    pub workspace_bootstrap: Option<WorkspaceBootstrap>,
    pub system_prompt_addendum: Option<String>,
    pub max_iterations: u32,
    pub token_budget: TokenBudget,
    pub hooks: Vec<HookRef>,
    pub skills: Vec<SkillRef>,
    pub mcp_servers: Vec<McpServerRef>,
    pub memory_scope: MemoryScope,
    /// Tool Search 策略（ADR-009 §2.3）。默认 `Auto { ratio: 0.10, min_absolute_tokens: 4_000 }`。
    pub tool_search: ToolSearchMode,
    /// 权限决策时注入到 `PermissionContext` 的三件套默认值；
    /// 由业务装配 Session 时按调用通路（CLI / Cron / Gateway / Subagent）显式设置。
    /// 装配责任表 + fail-closed 检查见 `crates/harness-engine.md §6.3`；
    /// 语义见 `permission-model.md §3.2`。
    pub permission_defaults: PermissionDefaults,
}

impl Session {
    pub async fn run_turn(&self, input: TurnInput) -> Result<EventStream, SessionError>;
    pub async fn interrupt(&self) -> Result<(), SessionError>;
    pub async fn fork(&self, reason: ForkReason) -> Result<Session, SessionError>;
    pub async fn compact(&self, strategy: CompactStrategy) -> Result<Session, SessionError>;
    pub async fn reload_with(&self, delta: ConfigDelta) -> Result<ReloadOutcome, SessionError>;
    pub fn projection(&self) -> SessionProjection;
    pub fn snapshot_id(&self) -> SnapshotId;
    pub async fn end(&self, reason: EndReason) -> Result<(), SessionError>;

    /// 软引导（ADR-0017 §2.4）：与 `interrupt()` 互补，永远不终止 Run。
    /// 队列规则、合并语义、容量与 TTL 见 §2.7 `SteeringQueue`。
    pub async fn push_steering(
        &self,
        msg: SteeringRequest,
    ) -> Result<SteeringId, SessionError>;

    /// 取出当前 Steering 队列的快照（仅用于 UI 展示，不修改队列）。
    pub fn steering_snapshot(&self) -> SteeringSnapshot;

    /// 仅 testing feature：业务直接 cancel 一条已入队但未 drain 的消息。
    #[cfg(feature = "testing")]
    pub fn cancel_steering(&self, id: SteeringId) -> Result<(), SessionError>;
}
```

### 2.2 TurnInput

```rust
pub enum TurnInput {
    User(String),
    UserMultiModal(Vec<MessagePart>),
    ResumeFromInterrupt,
    Continue,
}

impl TurnInput {
    pub fn user(text: impl Into<String>) -> Self {
        Self::User(text.into())
    }
}
```

### 2.3 EventStream

```rust
pub type EventStream = BoxStream<'static, Result<Event, SessionError>>;
```

#### 2.3.1 背压与缓冲策略

EventStream 的生产者（Engine）与消费者（业务 UI/日志/存储）速度不匹配时，由 `SessionEventSinkPolicy` 显式约束：

```rust
pub struct SessionEventSinkPolicy {
    pub buffer_size: usize,             // 默认 256
    pub overflow: OverflowPolicy,
    pub lossy_event_kinds: HashSet<EventKind>,
    pub slow_consumer_timeout: Duration, // 默认 5s
}

pub enum OverflowPolicy {
    BlockProducer,           // 默认：消费者慢 → Engine 暂停
    DropLossy,               // lossy_event_kinds 内的事件可丢；其他走 BlockProducer
    ErrorOnOverflow,         // 直接向下游返回 Err(SessionError::EventSinkOverflow)
    LaggingConsumerDropAll,  // 仅在 Replay 只读场景使用
}

pub enum EventKind {
    AssistantDeltaProduced,
    ToolUseStreamChunk,
    UsageAccumulated,
    TraceSpanCompleted,
    // 其他一般不允许列为 lossy（ToolUseRequested / PermissionRequested / RunEnded 等必须全部送达）
}
```

**默认语义**（`SessionEventSinkPolicy::default()`）：

- `buffer_size = 256`：Engine 与消费者之间是 `mpsc::channel(256)`
- `overflow = BlockProducer`：消费者滞后时 Engine 暂停（但 sandbox 活动心跳不受影响，详见 `harness-sandbox.md` §5）
- `lossy_event_kinds = {}`（默认无事件可丢，保证审计完整性）
- 业务若启用 `DropLossy + lossy_event_kinds = {AssistantDeltaProduced, UsageAccumulated}`，可在渲染压力下丢弃流式增量与计量增量，但 **关键生命周期事件**（ToolUseRequested / PermissionRequested / RunEnded 等）永远保证送达
- `slow_consumer_timeout`：连续阻塞超过该时长会发 `Event::EventSinkStalled { consumer_id, elapsed }`，业务可据此切换降级模式

**不可丢事件（硬编码白名单）**：
`SessionCreated / SessionForked / SessionEnded / RunStarted / RunEnded / ToolUseRequested / ToolUseApproved / ToolUseDenied / PermissionRequested / PermissionResolved / PluginLoaded / PluginRejected / MemoryThreatDetected / SubagentSpawned / SubagentAnnounced / TeamCreated / TeamTerminated / EngineFailed / UnexpectedError`

### 2.4 Hot Reload API（对齐 ADR-003）

```rust
pub struct ConfigDelta {
    pub add_tools: Vec<ToolRegistration>,
    pub remove_tools: Vec<String>,
    pub add_skills: Vec<SkillRegistration>,
    pub add_mcp_servers: Vec<McpServerSpec>,
    pub remove_mcp_servers: Vec<McpServerId>,
    pub update_memory: Option<MemoryPatch>,
    pub permission_rule_patch: Option<RulePatch>,
    pub system_prompt_addendum: Option<String>,
    pub model_ref: Option<ModelRef>,
}

pub struct ReloadOutcome {
    pub mode: ReloadMode,
    pub new_session: Option<Session>,
    pub effective_from: ReloadEffect,
    pub cache_impact: CacheImpact,
}

pub enum ReloadMode {
    AppliedInPlace,
    ForkedNewSession { parent: SessionId, child: SessionId },
    Rejected { reason: String },
}

pub enum ReloadEffect {
    NextTurn,
    NextMessage,
    Immediate,
}

pub enum CacheImpact {
    NoInvalidation,
    OneShotInvalidation {
        reason: CacheInvalidationReason,
        affected_breakpoints: Vec<BreakpointId>,
    },
    FullReset,
}

pub enum CacheInvalidationReason {
    ToolsetAppended,
    SkillsAppended,
    McpServerAdded,
    MemdirContentChanged,
    SystemPromptChanged,
    ToolRemoved,
    ModelSwitched,
}
```

**分类规则**：

| ConfigDelta 内容 | ReloadMode | CacheImpact |
|---|---|---|
| 仅 `permission_rule_patch`（纯 SDK 侧） | `AppliedInPlace` | `NoInvalidation` |
| 仅 `add_tools`（`DeferPolicy::AlwaysLoad`） | `AppliedInPlace` | `OneShotInvalidation { reason: ToolsetAppended }` |
| 仅 `add_tools`（全部 `DeferPolicy::AutoDefer` / `ForceDefer`） | `AppliedInPlace` | `NoInvalidation`（入 deferred 集，仅 `Event::ToolDeferredPoolChanged`） |
| 仅 `add_skills` | `AppliedInPlace` | `OneShotInvalidation { reason: SkillsAppended }` |
| 仅 `add_mcp_servers`（含 `AutoDefer` 工具） | `AppliedInPlace` | `NoInvalidation`（工具入 deferred 集） |
| 仅 `add_mcp_servers`（含 `AlwaysLoad` 工具） | `AppliedInPlace` | `OneShotInvalidation { reason: McpServerAdded }` |
| `remove_tools` / `remove_mcp_servers` | `ForkedNewSession` | `FullReset` |
| `update_memory` | `ForkedNewSession` | `FullReset` |
| `model_ref` 变更 | `ForkedNewSession` | `FullReset` |
| `system_prompt_addendum` 非空 | `ForkedNewSession` | `FullReset` |
| 尝试切换 `tool_search` 模式 | `Rejected` | — · ADR-009 §2.3：ToolSearchMode 只允许创建期 / fork 时设定 |
| 上述多种安全增量组合 | `AppliedInPlace` | `OneShotInvalidation`（affected_breakpoints 合并） |
| 安全增量 + 破坏性变更组合 | `ForkedNewSession` | `FullReset` |
| 跨租户迁移尝试 | `Rejected` | — |
| 删除 run 过程中被引用的 Tool | `Rejected` | — |

> **重要语义**：`AppliedInPlace` 意味着"SDK 的 `Session` 对象层面无需 fork",但**不保证** LLM Prompt Cache 命中率不下降：只有 `NoInvalidation` 才代表 cache 零影响，`OneShotInvalidation` 会产生一次 cache miss（下一 turn 重新 build cache）。详见 ADR-003 §2.3。

### 2.5 Fork

```rust
pub enum ForkReason {
    UserRequested,
    Compaction,
    HotReload,
    Isolation,
    RetryFromCheckpoint(JournalOffset),
}

pub struct ForkSpec {
    pub reason: ForkReason,
    pub config_delta: Option<ConfigDelta>,
    pub start_offset: Option<JournalOffset>,
}
```

Fork 语义：

- 新 session 继承父 session 的 Event 直到 `start_offset`
- 新 session 可携带 `ConfigDelta` 变更配置
- 父 session 写 `Event::RunEnded { reason: Forked }`（可选）或继续运行（并行 fork）
- 新 session 写 `Event::SessionForked { parent, reason }`

### 2.6 Projection

```rust
pub struct SessionProjection {
    pub session_id: SessionId,
    pub messages: Vec<Message>,
    pub tool_uses: HashMap<ToolUseId, ToolUseRecord>,
    pub permission_log: Vec<PermissionRecord>,
    pub usage: UsageSnapshot,
    pub allowlist: AllowList,
    pub end_reason: Option<EndReason>,
    pub last_offset: JournalOffset,
    pub snapshot_id: SnapshotId,
    /// 已材化的 deferred 工具名集合（ADR-009 §2.7）
    pub discovered_tools: DiscoveredToolProjection,
}

/// Deferred 工具的"已材化"状态，由 `Event::ToolSchemaMaterialized` /
/// `Event::SessionForked` / `Event::SessionCompacted` 三类事件驱动。
#[derive(Debug, Clone, Default)]
pub struct DiscoveredToolProjection {
    materialized: HashSet<ToolName>,
    last_delta_at: Option<DateTime<Utc>>,
}

impl DiscoveredToolProjection {
    /// 是否已被 materialize（含 AnthropicToolReferenceBackend 与 InlineReinjectionBackend 两条路径）
    pub fn contains(&self, name: &ToolName) -> bool { self.materialized.contains(name) }
    pub fn iter(&self) -> impl Iterator<Item = &ToolName> { self.materialized.iter() }
    pub fn len(&self) -> usize { self.materialized.len() }
}

impl SessionProjection {
    pub fn replay(events: impl Iterator<Item = Event>) -> Result<Self, ProjectionError>;
}
```

**Projection 规则**（摘自 ADR-009 §2.7）：

| 事件 | 对 `discovered_tools` 的影响 |
|---|---|
| `Event::ToolSchemaMaterialized { names, .. }` | `materialized += names` |
| `Event::SessionForked { parent_snapshot, .. }` | 从 parent 继承 `materialized` 集 |
| `Event::SessionCompacted { .. }` | 清空 `materialized`（compact 视作 fresh start） |
| `Event::ToolDeferredPoolChanged { removed, .. }` | `materialized -= removed`（工具已从 registry 删除） |

### 2.7 SteeringQueue（ADR-0017）

> 软引导队列：与 `interrupt()`（硬中断）正交。所有 `Session::push_steering(...)`
> 调用最终落到本节描述的容器；Engine 在主循环 Safe Merge Point（`harness-engine §3`
> 主循环图新增的 `[steering.drain_and_merge()]` 阶段）按规则 drain。

#### 2.7.1 数据结构

```rust
pub struct SteeringQueue {
    inner: Mutex<VecDeque<SteeringMessage>>,
    policy: SteeringPolicy,
    notify: tokio::sync::Notify,
}

pub struct SteeringRequest {
    pub kind: SteeringKind,
    pub body: SteeringBody,
    pub priority: Option<SteeringPriority>,
    pub correlation_id: Option<CorrelationId>,
    pub source: SteeringSource,
}

pub struct SteeringSnapshot {
    pub items: Vec<SteeringMessage>,
    pub dropped_recent: Vec<(SteeringId, SteeringDropReason)>,
    pub policy: SteeringPolicy,
}
```

`SteeringMessage / SteeringKind / SteeringBody / SteeringPriority / SteeringSource /
SteeringPolicy / SteeringOverflow / SteeringId` 一律使用 `harness-contracts §3.4.3`
的定义；本 crate 不再重复声明。`SteeringDropReason` 与 `event-schema.md §3.5.1`
的 `Event::SteeringMessageDropped.reason` 同源。

#### 2.7.2 装配

`SessionBuilder` 增量：

```rust
impl SessionBuilder {
    /// 显式覆盖默认 `SteeringPolicy`（默认值由 contracts 提供，与 ADR-0017 §2.3 一致）。
    pub fn with_steering_policy(self, policy: SteeringPolicy) -> Self;
}
```

启用条件：

| 维度 | 行为 |
|---|---|
| feature flag | `feature_flags.steering_queue` 默认 **on**；关闭时 `push_steering` 返回 `SessionError::FeatureDisabled`，不写事件 |
| capacity = 0 | 等价于"功能在但永远 DropNewest"；`push_steering` 返回 `SteeringId` 后立刻 emit `Event::SteeringMessageDropped { reason: Capacity }` |
| `PermissionMode::DontAsk` 等无人值守模式 | 队列正常工作；ADR-0017 §2.7 的 source 速率限制不变 |

#### 2.7.3 push_steering 规则

```text
push_steering(req)
  ├─ feature_flag 关闭 → Err(FeatureDisabled)
  ├─ source == Plugin 且 plugin 未声明 capabilities.steering=true 或非 AdminTrusted
  │     → Err(SteeringSourceDenied) + Event::SteeringMessageDropped { reason: PluginDenied }
  ├─ dedup_window 内已有同 body_hash 同 source 消息（priority != High）
  │     → 命中 dedup；返回既存 Id；Event::SteeringMessageDropped { reason: DedupHit }
  ├─ queue.len == capacity →
  │     ├─ DropOldest    → 弹出最早一条；Event::SteeringMessageDropped { reason: Capacity }
  │     ├─ DropNewest    → 拒绝本次；Event::SteeringMessageDropped { reason: Capacity }
  │     └─ BackPressure  → await notify until 有空位（业务自负超时）
  └─ queue.push_back(msg); Event::SteeringMessageQueued { id, kind, priority, source }
```

drain 阶段（由 Engine 调用）规则：

```text
drain_and_merge() -> Option<SynthesizedUserMessage>
  ├─ 当 queue 为空 → 返回 None；不写事件
  ├─ 否则按 FIFO 取出 visible_at <= now 且未过 ttl 的消息
  ├─ 应用合并语义（Append 拼接 / Replace 覆盖 / NudgeOnly 仅留痕）
  ├─ 对剩余消息中超 ttl 的 → Event::SteeringMessageDropped { reason: TtlExpired }
  └─ 写 Event::SteeringMessageApplied { ids, merged_into_message_id?, kind_distribution }
```

`drain_and_merge` 是 Engine 在 Iteration Loop 入口调用的内部方法；业务侧 **不**
直接调用，仅通过 `push_steering` / `steering_snapshot` 与队列交互。

#### 2.7.4 RunEnded 后的残留处理

- `Event::RunEnded` 触发后，队列中所有未 drain 的消息：
  - `priority == Normal` 且 `kind != NudgeOnly` → 留在队列等待下一次 `run_turn`
  - 命中 ttl → `Event::SteeringMessageDropped { reason: RunEnded }`
- `Session::end(...)` 触发后，队列中所有消息一律 drop 并 emit
  `Event::SteeringMessageDropped { reason: SessionEnded }`，与 ADR-0001 EventStream
  关闭顺序一致

#### 2.7.5 与 `Session::reload_with` / `compact` / `fork` 的关系

| 操作 | 队列处理 |
|---|---|
| `reload_with(ConfigDelta)` | 队列**保留**（policy 改变后立即生效；新 policy 的容量可能立即触发 DropOldest） |
| `compact(strategy)` | 队列**保留**；compact 仅影响历史消息，与软引导队列正交 |
| `fork(reason)` | 队列**不**继承（fork 出的新 Session 起空队列，避免 Replay 双写） |

> **设计要点**：本节只暴露 Session 容器侧的形态。Engine 侧的 drain / safe merge
> 流水线在 `crates/harness-engine.md §3`；plugin 侧的 capability handle 在
> `crates/harness-plugin.md §2.4`；对应事件在 `event-schema.md §3.5.1`。

## 3. 内部状态

```rust
struct SessionInner {
    id: SessionId,
    tenant_id: TenantId,
    options: SessionOptions,

    store: Arc<dyn EventStore>,
    model: Arc<dyn ModelProvider>,
    sandbox: Arc<dyn SandboxBackend>,
    permission_broker: Arc<dyn PermissionBroker>,
    hook_registry: HookRegistrySnapshot,
    tool_registry: ToolRegistrySnapshot,
    skill_registry: SkillRegistrySnapshot,
    mcp_registry: Arc<McpRegistry>,
    memory: Arc<MemoryManager>,
    context_engine: Arc<ContextEngine>,

    run_state: tokio::sync::Mutex<RunState>,
    interrupt_token: InterruptToken,
    /// 软引导队列（ADR-0017 §2.3 / §2.7）；与 `interrupt_token` 互补，永远不终止 Run。
    /// 由 `Session::push_steering` 入队，Engine 在主循环 Safe Merge Point drain。
    steering_queue: Arc<SteeringQueue>,
    observer: Arc<Observer>,
}

enum RunState {
    Idle,
    Running { run_id: RunId, started_at: Instant },
    Interrupted { run_id: RunId },
    Ended { reason: EndReason, ended_at: Instant },
}
```

## 4. 生命周期事件

```text
SessionCreated(SessionCreatedEvent {
    session_id,
    tenant_id,
    options_hash,
    snapshot_id,
    created_at,
})
    │
    ▼
[run_turn 调用]
RunStarted(RunStartedEvent { run_id, ... })
    │
    ▼
[Engine 执行] → [Tool / Hook / Permission / Context events]
    │
    ▼
RunEnded(RunEndedEvent { run_id, reason, usage })
    │
    ▼ (可选)
SessionReloadRequested → SessionReloadApplied
    │
    ▼ (触发 Fork)
SessionForked { parent, child, reason }
    │
    ▼ (结束)
SessionEnded(SessionEndedEvent { session_id, reason })
```

## 5. 运行期禁止修改（ADR-003）

```rust
impl Session {
    pub fn set_system_prompt(&mut self, _: String) -> Result<(), SessionError> {
        Err(SessionError::PromptCacheLocked)
    }
    pub fn set_toolset(&mut self, _: BuiltinToolset) -> Result<(), SessionError> {
        Err(SessionError::PromptCacheLocked)
    }
    // ...
}
```

所有修改必须走 `reload_with`，由 ConfigDelta 分类器决定 in-place 或 fork。

## 6. 中断

```rust
impl Session {
    pub async fn interrupt(&self) -> Result<(), SessionError> {
        self.inner.interrupt_token.trigger();
        // Engine 检查 interrupt_token 并在下一个 safe point 停止
        Ok(())
    }
}

pub struct InterruptToken {
    flag: Arc<AtomicBool>,
    notify: Arc<tokio::sync::Notify>,
}
```

Engine 在以下 safe point 检查：

- Tool invocation 开始前
- Model stream 的每个 delta
- Hook dispatch 之间

## 7. Workspace 抽象与 Bootstrap 绑定

### 7.1 Workspace 概念

```rust
pub struct Workspace {
    pub id: WorkspaceId,
    pub tenant_id: TenantId,
    pub root_path: PathBuf,
    pub display_name: String,
    pub bootstrap_files: Vec<BootstrapFileSpec>,
    pub default_session_options: Option<SessionOptions>,
    pub created_at: DateTime<Utc>,
}

pub struct WorkspaceRegistry { /* ... */ }

impl Harness {
    pub async fn create_workspace(&self, spec: WorkspaceSpec) -> Result<Workspace, HarnessError>;
    pub async fn list_workspaces(&self, tenant: TenantId) -> Result<Vec<Workspace>, HarnessError>;
    pub async fn get_workspace(&self, id: WorkspaceId) -> Result<Option<Workspace>, HarnessError>;
}
```

### 7.2 Session 绑定 Workspace

Session 可通过 `SessionOptions::workspace_ref: Option<WorkspaceId>` 绑定到 workspace：

- 绑定后：Session 创建期自动加载 Workspace 的 bootstrap 文件 + 合并 `default_session_options`
- 未绑定：Session 独立运行，无 workspace 上下文
- 运行期切 workspace → 视为 `ReloadMode::ForkedNewSession`（fork 出新 session）

```rust
impl SessionBuilder {
    pub fn with_workspace(mut self, workspace: WorkspaceId) -> Self;

    pub fn with_workspace_bootstrap(
        mut self,
        root: impl Into<PathBuf>,
    ) -> Self {
        self.workspace_bootstrap = Some(WorkspaceBootstrap {
            workspace_root: root.into(),
            files: WorkspaceBootstrap::default_files(),
        });
        self
    }
}
```

### 7.3 多级 Bootstrap 继承（类似 CC-09 `SETTING_SOURCES`）

Bootstrap 文件按以下优先级加载（后者覆盖前者同名段落）：

1. `~/.octopus/AGENTS.md`（用户级）
2. Workspace `root/AGENTS.md`（工作空间级）
3. Workspace 子目录 `AGENTS.md`（若 `cwd` 在子目录下）
4. `.octopus/AGENTS.md`（项目级，一般同 workspace root）
5. Session options 里显式指定的 `system_prompt_addendum`（session 级）

Bootstrap 文件在 session **创建时**读取，此后不再重读（保 Prompt Cache）。Workspace 的 bootstrap 文件修改需要 `reload_with` 才会生效（且会触发 fork）。

## 8. Feature Flags

```toml
[features]
default = ["workspace-bootstrap", "hot-reload-fork"]
workspace-bootstrap = []
hot-reload-fork = []
```

## 9. 错误类型

```rust
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("prompt cache locked — use reload_with()")]
    PromptCacheLocked,

    #[error("session already ended")]
    AlreadyEnded,

    #[error("no active run")]
    NoActiveRun,

    #[error("fork rejected: {0}")]
    ForkRejected(String),

    #[error("journal: {0}")]
    Journal(#[from] JournalError),

    #[error("interrupted")]
    Interrupted,
}
```

## 10. 使用示例

### 10.1 基本创建与运行

```rust
let session = harness.create_session(
    SessionOptions::default()
        .with_permission_mode(PermissionMode::Default)
        .with_workspace_bootstrap("data/workspace")
).await?;

let mut events = session.run_turn(TurnInput::user("请 review 一下 auth 模块")).await?;
while let Some(event) = events.next().await {
    match event? {
        Event::AssistantDeltaProduced(e) => print!("{}", e.delta),
        Event::ToolUseRequested(e) => ui.tool_start(&e.tool_name),
        Event::RunEnded(e) => break,
        _ => {}
    }
}
```

### 10.2 Hot Reload（in-place）

```rust
let outcome = session.reload_with(ConfigDelta {
    add_tools: vec![my_new_tool_registration()],
    ..Default::default()
}).await?;

match outcome.mode {
    ReloadMode::AppliedInPlace => {
        match outcome.cache_impact {
            CacheImpact::NoInvalidation => {
                tracing::info!("下一 turn 起生效，prompt cache 完全保留");
            }
            CacheImpact::OneShotInvalidation { reason, .. } => {
                tracing::info!(?reason, "下一 turn 起生效；会产生一次 cache miss");
            }
            _ => {}
        }
    }
    _ => unreachable!(),
}
```

### 10.3 Hot Reload（fork）

```rust
let outcome = session.reload_with(ConfigDelta {
    system_prompt_addendum: Some("现在切换到 code-review 模式".into()),
    ..Default::default()
}).await?;

match outcome.mode {
    ReloadMode::ForkedNewSession { parent: _, child } => {
        let new_session = outcome.new_session.unwrap();
        // 使用 new_session 继续
    }
    _ => unreachable!(),
}
```

## 11. 测试策略

| 类 | 覆盖 |
|---|---|
| 单元 | SessionOptions 校验、Projection replay |
| 生命周期 | Create → Run → Interrupt → End |
| Fork | 父子 Event 链正确 |
| Hot Reload | 三档 ReloadMode 分类 |
| 中断 | 各 safe point 响应 |
| Projection | 同一 Event 序列多次 replay 结果一致 |

## 12. 可观测性

| 指标 | 说明 |
|---|---|
| `session_created_total` | 按 tenant 分桶 |
| `session_duration_seconds` | 存活时间 |
| `session_turn_duration_ms` | 每 turn 耗时 |
| `session_forked_total` | 按 reason 分桶 |
| `session_interrupts_total` | |
| `session_reload_mode` | 按三档分桶 |

## 13. 反模式

- 直接修改 `session.options`（应走 reload_with）
- 忘记调 `session.end()`（导致 Journal 无结束 marker）
- Session 对象跨租户共享（`TenantId` 隔离被破坏）
- 在 `run_turn` 内并发调用 `interrupt`（应用 InterruptToken 机制）

## 14. 相关

- D3 · `api-contracts.md` §Session
- D4 · `event-schema.md` §Session 事件
- ADR-003 Prompt Cache 硬约束
- ADR-009 Deferred Tool Loading（`SessionOptions.tool_search` + `DiscoveredToolProjection`）
- `crates/harness-engine.md`
- `crates/harness-context.md`
- `crates/harness-tool-search.md`
