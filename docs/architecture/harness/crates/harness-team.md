# `octopus-harness-team` · L3 · 多 Agent 协同编排 SPEC

> 层级：L3 · 状态：Accepted
> 依赖：`harness-contracts` + `harness-engine`（via trait） + `harness-session` + `harness-journal`

## 1. 职责

实现 **多 Agent 长期协同**：Team 成员作为一级公民，消息路由，共享记忆，可选 coordinator。对齐 CC-12 / OC-08 / OC-26。

**核心能力**：

- Team 创建与生命周期
- 三种拓扑：Coordinator-Worker / Peer-to-Peer / Role-Routed
- 消息总线 + 路由策略
- Context Visibility（与触发授权正交）
- Shared Memory
- Coordinator 专用 Toolset

## 2. 对外 API

### 2.1 Team

```rust
pub struct Team {
    pub id: TeamId,
    pub spec: TeamSpec,
    inner: Arc<TeamInner>,
}

impl Team {
    pub async fn dispatch(&self, input: TeamInput) -> Result<EventStream, TeamError>;
    pub async fn post(&self, from: AgentId, msg: AgentMessage) -> Result<(), TeamError>;
    pub fn events(&self) -> EventStream;
    pub async fn pause(&self, agent: AgentId) -> Result<(), TeamError>;
    pub async fn resume(&self, agent: AgentId) -> Result<(), TeamError>;
    pub async fn terminate(&self, reason: TerminationReason) -> Result<TeamReport, TeamError>;

    /// 运行期向 Team 加入新成员（由 Coordinator 工具 `spawn_worker` 或业务面
    /// 主动调用）。装配期校验：`agent_id` 不得与现有成员重复；`engine_config`
    /// 必须满足 `harness-permission` 同租户授权；spawn 完成后**必须**写出一
    /// 条 `Event::TeamMemberJoined`（详见 `event-schema.md §3.10`），其
    /// `spec_snapshot_id` 指向新成员 spec 在 BlobStore 的不可变快照——
    /// Replay 据此重建动态成员，**不允许**仅靠 `TeamCreated.member_specs_hash`
    /// 还原后续加入者。
    pub async fn add_member(&self, spec: TeamMemberSpec) -> Result<AgentId, TeamError>;

    /// 运行期移除成员；幂等：未知 `agent` 直接返回 Ok(())。`reason` 落入
    /// `Event::TeamMemberLeft.reason`，supports `MemberLeaveReason` 全枚举。
    /// 移除期**不**回收成员 Session 历史（仍可作为 `TranscriptRef` 引用），
    /// 仅断开 MessageBus 订阅与 `TopologyStrategy::route` 输入。
    pub async fn remove_member(
        &self,
        agent: AgentId,
        reason: MemberLeaveReason,
    ) -> Result<(), TeamError>;
}
```

### 2.2 TeamSpec

```rust
pub struct TeamSpec {
    pub id: TeamId,
    pub name: String,
    pub members: Vec<TeamMemberSpec>,
    pub topology: TeamTopology,
    pub routing: RoutingPolicy,
    pub message_bus: MessageBusSpec,
    pub shared_memory: Option<SharedMemorySpec>,
    pub lifecycle: TeamLifecycle,
    pub observability: Option<TeamObservability>,

    /// **回合硬上限**：自单次 `Team::dispatch` 起，Team 内 `TeamTurnCompleted`
    /// 累计达到该值即强制收尾。`None` 表示不设回合上限（**仅在
    /// `TeamLifecycle::OneShot` + `CoordinatorWorker` 拓扑且 Coordinator 自带
    /// `StopTeamTool` 决策权**时允许；其余拓扑必须显式给出有限上限）。
    /// `PeerToPeer` 拓扑下未设此值视为反模式（§12）。
    pub max_turns_per_goal: Option<u32>,

    /// **同 correlation 路由环防御**：MessageBus 在路由 `AgentMessage` 时累计
    /// 同一 `correlation_id` 的消息数，超出该值即视为路由环：当前消息按
    /// `RouteFallback` 处理且写入 `Event::EngineFailed { kind: CyclicRouting }`。
    /// `None` 表示沿用全局默认 `harness-team` 内置 `CYCLIC_ROUTING_LIMIT = 64`。
    pub max_messages_per_correlation: Option<u32>,
}

pub struct TeamMemberSpec {
    pub agent_id: AgentId,
    pub role: String,
    pub engine_config: EngineConfig,
    pub visibility: ContextVisibility,
    pub quota: ResourceQuota,
}

pub struct EngineConfig {
    pub model_ref: ModelRef,
    pub toolset: ToolsetSelector,
    pub permission_mode: PermissionMode,
    pub sandbox_policy: SandboxPolicy,
    pub max_iterations: u32,
    pub token_budget: TokenBudget,
    pub system_prompt_addendum: Option<String>,
}
```

> `ResourceQuota` 的形状定义在 `agents-design.md §7.1`，与 `SubagentSpec.quota`
> 共享同一类型。本 crate **不重复声明**——任何对 `ResourceQuota` 的字段调整都
> 必须改 §7.1，避免 Subagent / Team 双轨漂移（与 `harness-subagent.md §2.2` 的
> cross-ref 处理对齐）。

### 2.3 Topology

```rust
pub enum TeamTopology {
    CoordinatorWorker {
        coordinator: AgentId,
        workers: Vec<AgentId>,
    },
    PeerToPeer,
    RoleRouted(RoleRoutingTable),
    Custom(Arc<dyn TopologyStrategy>),
}

pub struct RoleRoutingTable {
    pub rules: Vec<RoleRoutingRule>,
    pub fallback: RouteFallback,
}

pub struct RoleRoutingRule {
    pub pattern: RoutingPattern,
    pub target_role: String,
    pub priority: i32,
}

pub enum RoutingPattern {
    RegexMatch(Regex),
    KeywordAny(Vec<String>),
    Classifier(Arc<dyn MessageClassifier>),
}

pub enum RouteFallback {
    DropMessage,
    SendToCoordinator,
    Broadcast,
}

#[async_trait]
pub trait TopologyStrategy: Send + Sync + 'static {
    fn strategy_id(&self) -> &str;
    async fn route(&self, message: &AgentMessage, team: &TeamSpec) -> Vec<AgentId>;
}

/// 自定义路由分类器。装配期注册到 `RoleRoutingTable`，运行期由
/// `harness-team` 在 `RoutingPattern::Classifier` 路径上调用。
#[async_trait]
pub trait MessageClassifier: Send + Sync + 'static {
    /// 分类器稳定 ID（指标 label / 审计 / replay diff 用）；同一 ID 必须返回
    /// 一致结果，否则 replay 不确定。
    fn classifier_id(&self) -> &str;

    /// 单次分类的硬超时上限。`harness-team` 用 `tokio::time::timeout` 包裹
    /// `classify`；命中即按 fallback 处理（**不抛错**），并落
    /// `team_classifier_timeout_total{classifier_id}`。
    fn timeout(&self) -> Duration;

    /// 分类失败可选地携带原因（仅用于日志 / 指标）；返回 `Err` 与超时同款，
    /// 一律走 `RoleRoutingTable.fallback`。
    async fn classify(
        &self,
        message: &AgentMessage,
        team: &TeamSpec,
    ) -> Result<ClassifierVerdict, ClassifierError>;
}

pub struct ClassifierVerdict {
    /// 命中的目标角色集合；空集等价于 `Err(ClassifierError::NoMatch)`，
    /// 由 `RouteFallback` 兜底。
    pub roles: Vec<String>,
    /// 置信度（0.0 ~ 1.0）。`harness-team` 仅做 metric 直方图记录；
    /// 业务可在自定义 `TopologyStrategy` 里据此二次裁决。
    pub confidence: f32,
}

#[derive(Debug, thiserror::Error)]
pub enum ClassifierError {
    #[error("no match")]
    NoMatch,
    #[error("classifier failed: {0}")]
    Failed(String),
}
```

**不变量**：

1. `MessageClassifier` 的 `timeout` / 错误 / 空 verdict 一律走 `RoleRoutingTable.fallback`，**绝不**让单次分类失败导致 `Team::dispatch` 失败——避免某个第三方分类器把整个 Team 卡住。
2. `classifier_id` 必须**全 Team 唯一**；重复注册视为装配期错误（`TeamError::Internal`）。
3. `classify` 不得有可观察副作用（不可写 Memory、不可发消息）；replay 期会以"装配期相同 classifier 注入相同消息"为前提复现路由结果。

### 2.4 Routing Policy

```rust
pub enum RoutingPolicy {
    Mention,       // @agent_id 显式提及
    RoleBased,     // 按角色路由
    Explicit,      // 消息 to 字段强制
    Broadcast,     // 所有成员
    Coordinator,   // 始终回 Coordinator
}
```

### 2.5 Context Visibility（对齐 OC-26）

```rust
pub enum ContextVisibility {
    All,                                    // 看所有 Team 消息
    Allowlist(Vec<AgentId>),                // 只看列表中的
    AllowlistQuote(Vec<AgentId>),           // 只看列表的 + 被 quote 的
    Private,                                // 只看自己发的 + 收到的
}
```

**关键**：触发授权与上下文可见性**正交**。

### 2.6 Message Bus

```rust
pub struct MessageBusSpec {
    pub buffer_size: usize,
    pub persistence: BusPersistence,
    pub ordering: MessageOrdering,

    /// **新成员加入或网络恢复后 subscribe 时**对历史消息的回看窗口。
    /// 解决"动态 add_member / 短暂掉线后重连，是否能看到此前 Team 上下文"
    /// 的语义模糊。具体策略由 `ReplayWindow` 给出（默认 `Bounded(50)`）。
    /// 与 `BusPersistence` 的关系见下方表格——`Ephemeral` 模式下
    /// `Bounded` / `SinceJoin` 仍可工作（依靠 broadcast ring buffer），
    /// `FullSinceTeamCreated` 强制要求 `JournalOnly` 或 `JournalPlusBroadcast`。
    pub replay_window: ReplayWindow,
}

pub enum BusPersistence {
    JournalOnly,           // 仅 Event Journal
    JournalPlusBroadcast,  // Journal + tokio::sync::broadcast
    Ephemeral,             // 仅 broadcast（不持久）
}

pub enum MessageOrdering {
    CausalPerSender,   // 同一 sender 按序
    TotalOrder,        // 全局序（严格）
    BestEffort,        // 尽力而为
}

/// 成员 subscribe 时回看 Team 历史消息的策略。仅作用于"加入/重连那一刻"，
/// 不影响后续实时投递；与 `ContextVisibility` 串联裁剪（visibility 是过滤器，
/// replay_window 是窗口）。
pub enum ReplayWindow {
    /// 不回放任何历史。新成员只看到 subscribe 之后的消息（OpenClaw 默认行为）。
    None,
    /// 回放最近 N 条 Team 消息；仍受 `ContextVisibility` 进一步过滤。
    Bounded(u32),
    /// 仅回放成员"首次 join"以后的 Team 消息（重连场景常用，避免新成员被
    /// 历史包淹没）；首次加入即等价于 `None`。
    SinceJoin,
    /// 自 Team 创建起的全部历史；仅在 `BusPersistence` 含 Journal 时合法
    /// （装配期校验，否则 `TeamError::Internal`）。
    FullSinceTeamCreated,
}

pub struct AgentMessage {
    pub message_id: MessageId,
    pub team_id: TeamId,
    pub from: AgentId,
    pub to: Recipient,
    pub payload: MessagePayload,
    pub sent_at: DateTime<Utc>,
    pub correlation_id: CorrelationId,
}

pub enum Recipient {
    Agent(AgentId),
    Role(String),
    Broadcast,
    Coordinator,
}

pub enum MessagePayload {
    Text(String),
    Structured(Value),
    Request { reply_to: MessageId },
    Response { in_reply_to: MessageId, body: Value },
    Handoff { to: AgentId, summary: String },
}
```

### 2.7 Lifecycle

```rust
pub enum TeamLifecycle {
    OneShot { goal: String },
    Persistent { max_idle: Duration },
    ExplicitTerminate,
}

pub enum TerminationReason {
    GoalAchieved,
    Timeout,
    UserRequested,
    Error(String),
}

pub struct TeamReport {
    pub team_id: TeamId,
    pub members_usage: HashMap<AgentId, UsageSnapshot>,
    pub message_count: u64,
    pub duration: Duration,
    pub final_state: Value,
}
```

### 2.8 Coordinator Trait

```rust
#[async_trait]
pub trait Coordinator: Send + Sync + 'static {
    async fn dispatch(
        &self,
        team: &TeamSpec,
        input: TeamInput,
    ) -> Result<EventStream, TeamError>;

    async fn route(
        &self,
        message: &AgentMessage,
    ) -> Result<Vec<AgentId>, TeamError>;

    async fn terminate(
        &self,
        reason: TerminationReason,
    ) -> Result<TeamReport, TeamError>;
}
```

内置 `DefaultCoordinator`；业务可替换。

## 3. Coordinator 专用 Toolset

```rust
pub const COORDINATOR_TOOLSET: &[&str] = &[
    "dispatch",       // 派发任务给 Worker
    "message",        // 发送消息
    "stop_team",      // 终止 Team
    "team_status",    // 查 Team 状态
    "spawn_worker",   // 运行期向 Team 加入新 Worker（薄包装 Team::add_member）
    "pause_worker",   // 暂停指定 Worker（薄包装 Team::pause）
    "resume_worker",  // 恢复指定 Worker（薄包装 Team::resume）
];
```

对齐 CC-12：Coordinator 不能直接执行任务（没有 Bash / FileEdit），只能调度。

**`spawn_worker` 的范围限制**：

1. 仅在 `TeamTopology::CoordinatorWorker` / `RoleRouted` / `Custom` 拓扑下注册；
   `PeerToPeer` 拓扑没有"Coordinator"概念，注册即装配期失败。
2. 创建的新成员上限受 `TeamSpec.members.len() + spawn_worker_call_count` 双层
   约束——任何业务自定义 toolset 不得绕过 `TeamPolicy.max_members`（默认 8）。
3. 动态加入的成员同样要求落 `Event::TeamMemberJoined` 与
   `TeamMemberJoinedEvent.spec_snapshot_id`，replay 据此重建。
4. `spawn_worker` 与 `harness-subagent::AgentTool` 在职能上互不替代：前者把
   "新参与者"加入长驻 Team 总线，后者把"一次性子任务"委派给临时子代理；
   Coordinator 想"用一次就丢"应当走 Subagent，而非反复 spawn/remove member。

## 4. 三种拓扑工作流

### 4.1 Coordinator-Worker

```text
TeamInput → Coordinator
   │
   ├─ Coordinator 决策：派发给 worker1
   │   └─ dispatch(worker1, task)
   │
   ├─ worker1 执行 → Response → Coordinator
   │
   └─ Coordinator 决策：done 或继续派发
```

**Worker→Coordinator 文本注入契约**（与 `harness-subagent` 共用同一渲染抽象）：

1. Worker 完成任务（payload 为 `MessagePayload::Response { .. }` 或 `Handoff`）
   后，发往 Coordinator 的消息会先走 `AnnouncementRenderer`（默认
   `XmlTaskNotificationRenderer`，与 `harness-subagent.md §7` 字面同款），把
   结构化结果包成 `<task-notification>` XML 注入到 Coordinator Engine 的
   user-role 上下文。
2. Renderer 仅作用于 **CoordinatorWorker 拓扑** 下"Worker→Coordinator"方向的
   消息；`PeerToPeer` / `RoleRouted` 不强制改写 user-role 文本（业务可在
   `TopologyStrategy::Custom` 里按需启用）。
3. Renderer 与 `harness-subagent` **共享同一份 trait 与默认实现**——业务通过
   `HarnessBuilder::with_announcement_renderer` 替换的渲染器同时影响 Subagent
   announcement 与 Team Worker 响应；这是有意为之的语义统一，避免两条管线
   生成风格不一致的内部信号。
4. `<rewrite-hint>` 段落在 Team 路径上同样生效，承载"Coordinator 不得把
   `<task-notification>` XML 原样回吐给终端用户"的硬约束（对齐 CC-12 / OC-27）。

### 4.2 Peer-to-Peer

```text
TeamInput → First Agent (seed)
   │
   ├─ AgentA → message(@AgentB, query)
   ├─ AgentB → message(@AgentA, response)
   ├─ AgentA → message(@AgentC, broadcast_help)
   └─ ...
```

### 4.3 Role-Routed

```text
User Message → RoleRouter
   │
   ├─ pattern: "bug" → BugFixer Agent
   ├─ pattern: "feat" → Planner Agent
   └─ pattern: "review" → Reviewer Agent
```

## 5. Shared Memory

```rust
pub struct SharedMemorySpec {
    pub provider: Arc<dyn MemoryProvider>,
    pub scopes: HashMap<AgentId, MemoryScope>,
    pub write_policy: SharedWritePolicy,
}

pub enum SharedWritePolicy {
    Unrestricted,
    CoordinatorOnly,
    PerMemberQuota,
    RoleGated(Vec<String>),
}
```

**审计与可重建不变量**（与 `harness-memory.md` 的 Memory 事件统一管线）：

1. Team 共享 Memory 的**任何写操作**都必须以 `Event::MemoryUpserted`
   （`event-schema.md §3.17.1`）形式落 Journal；不得在 Team 自己的事件流内
   "私写一份"——`harness-memory` 是共享 Memory 的唯一记录方。
2. **追溯锚点**：通过 envelope 的 `tenant_id` + `MemoryUpsertedEvent.session_id`
   即可定位到发起写操作的 Team 成员 Session；进一步通过该 Session 的
   `TeamMemberJoinedEvent`（`event-schema.md §3.10`）反查 `team_id` 与
   `agent_id`。无需在 `MemoryUpsertedEvent` 上额外冗余 `team_id` / `agent_id`
   字段——审计链通过 envelope + Team 事件交叉对账完成。
3. **`correlation_id` 沿用**：写发生在某次 `Team::dispatch` 或某条
   `AgentMessage` 的处理过程中，`MemoryUpserted` 的 envelope `correlation_id`
   必须等于触发写的上游消息（`AgentMessageSent` / `TeamInput`）的
   `correlation_id`，与 §6 "Team 全链 CorrelationId" 一致；据此审计可以从一次
   dispatch 反查所有衍生的共享内存写。
4. `SharedWritePolicy::CoordinatorOnly` / `RoleGated` / `PerMemberQuota` 由
   `harness-team` 在 Bus 入口做**前置**断言（参考成员的 `agent_id` 与
   `engine_config.toolset` 是否含 `memory_write`）；命中违反策略时拒绝消息
   并写出 `TeamError::ContextVisibilityViolation` 路径下的 fail-closed 事件
   （详见 §8）。Memory provider 不必再做二次策略校验。

## 6. Event 轨迹

完整事件链（所有事件在 `harness-contracts::Event` 声明、`event-schema.md` §3.10/3.11 定义字段）：

```text
TeamCreated { team_id, topology_kind, member_specs_hash, ... }
    │
    ├─ TeamMemberJoined { team_id, agent_id, role, session_id,
    │                     visibility, spec_snapshot_id, spec_hash } (×n)
    │
    ▼
AgentMessageSent → AgentMessageRouted
    │
    ▼
[成员 Session 的完整 Event 流：RunStarted / AssistantDelta / ToolUse / ...]
    │
    ├─ (可选 watchdog 触发) TeamMemberStalled { team_id, agent_id, last_activity_at,
    │                                            stalled_for, action } —— action 为
    │                                            `Reported` / `Interrupted` / `Removed`
    │
    ▼
TeamTurnCompleted { team_id, turn_id, participating_agents }
    │
    ▼ (动态 add_member / remove_member 触发，或成员退出时)
TeamMemberJoined { ... }   // Team::add_member
TeamMemberLeft { team_id, agent_id, reason }
    │
    ▼
TeamTerminated { team_id, reason, report_hash }
```

**Replay 约束**：

- `TeamProjection::replay` 必须能从完整 Event 流重建**任意时刻的成员列表**、
  访问可见性配置、消息历史。`TeamCreated` + `TeamMemberJoined`（含初始成员
  与运行期 `Team::add_member` 加入者）两类事件**不得省略**。
- `TeamMemberJoined.spec_snapshot_id`（`event-schema.md §3.10`）指向 BlobStore
  的不可变 spec 快照；replay 可据此重建动态成员的 `EngineConfig`。
  仅靠 `TeamCreated.member_specs_hash` 不足以覆盖动态加入者。
- `TeamMemberStalled` 是 watchdog 旁路事件（与 `SubagentStalled` 对称，
  详见 `harness-subagent.md §4.3`）；不参与 `TeamProjection` 重建，仅供
  审计与可观测性。

### 6.1 CorrelationId 全链不变量（与 Subagent 串联）

Team 因果链贯穿三层：`Team::dispatch` → 成员 Session → 成员 spawn 的
Subagent。`event-schema.md` 同名小节给出权威字段表，本节摘要不变量供
crate spec 自查：

1. `TeamInput` 创建时分配 `correlation_id: CorrelationId`；该 ID 写入
   首条 `AgentMessageSent` 的 envelope。
2. 成员收到消息后调用 Engine 处理，**该轮**所有事件（`RunStarted` /
   `AssistantDelta` / `ToolUseRequested` / ...）的 envelope `correlation_id`
   **必须沿用**入站 `AgentMessage.correlation_id`，**不得**自行生成新值。
3. 成员在该轮内 `spawn` 出的 Subagent，其 `SubagentSpawnedEvent` 与子
   Session 的所有事件 envelope `correlation_id` 同样沿用此值
   （与 `harness-subagent.md §6.1` 一致）。
4. 成员发出回复（`Team::post`）写出 `AgentMessageSent` 时，envelope
   `correlation_id` 仍为同一值；MessageBus 用该值做 §2.6 的环检测。
5. `TeamTurnCompleted.turn_id` 与 `correlation_id` **不**等价：一个
   correlation 在 `PeerToPeer` 下可能跨多个 turn。指标 / 审计需要二者
   并用——`correlation_id` 标识同一目标的因果链，`turn_id` 标识 Team
   消息周期。

**违反时的兜底**：MessageBus 在路由前比对 `from` 成员 Session 的当前
`correlation_id` 与待发消息 envelope 的 `correlation_id`，不等则按
`TeamError::Internal` fail-closed，并写 `Event::EngineFailed`；
不允许静默修正。

## 7. Feature Flags

```toml
[features]
default = ["coordinator-worker", "peer-to-peer"]
coordinator-worker = []
peer-to-peer = []
role-routed = ["dep:regex"]
```

## 8. 错误类型

```rust
#[derive(Debug, thiserror::Error)]
pub enum TeamError {
    #[error("unknown agent: {0:?}")]
    UnknownAgent(AgentId),

    #[error("routing failed: {0}")]
    RoutingFailed(String),

    #[error("coordinator rejected message: {0}")]
    CoordinatorRejected(String),

    #[error("quota exceeded for {agent:?}")]
    QuotaExceeded { agent: AgentId },

    /// 同 `correlation_id` 累计消息数超过 `TeamSpec.max_messages_per_correlation`
    /// 或全局默认 `CYCLIC_ROUTING_LIMIT`；MessageBus 据此降级到 fallback。
    #[error("cyclic routing detected (correlation={correlation_id:?}, depth={depth})")]
    CyclicRouting { correlation_id: CorrelationId, depth: u32 },

    /// 单次 `Team::dispatch` 的 `TeamTurnCompleted` 累计已达
    /// `TeamSpec.max_turns_per_goal`；Team 强制 `TerminationReason::Timeout` 收尾。
    #[error("turn limit exceeded: {limit} turns for team={team_id:?}")]
    TurnLimitExceeded { team_id: TeamId, limit: u32 },

    /// Watchdog 判定成员长时间无心跳 / 卡死；与 `TeamMemberStalled` 同步触发。
    #[error("member unresponsive: {agent:?}, stalled for {stalled_for:?}")]
    MemberUnresponsive { agent: AgentId, stalled_for: Duration },

    /// MessageBus `buffer_size` 已满且 `MessageOrdering` 不允许丢失；
    /// 调用方应回退（指数退避或换 `BestEffort`）。指标
    /// `team_message_bus_backpressure_total` 同步累计。
    #[error("message bus backpressure: team={team_id:?}, depth={depth}")]
    MessageBusBackpressure { team_id: TeamId, depth: u32 },

    /// 消息触发了违反 `ContextVisibility` 的传递路径，或 SharedMemory 写
    /// 命中了 `SharedWritePolicy` 拒绝条件；fail-closed，不静默剥离。
    #[error("context visibility / shared-memory policy violated: {0}")]
    ContextVisibilityViolation(String),

    #[error("engine: {0}")]
    Engine(#[from] EngineError),

    #[error("internal: {0}")]
    Internal(String),
}
```

## 9. 使用示例

```rust
let team = harness.create_team(
    TeamBuilder::new("pr-review-team")
        .topology(TeamTopology::CoordinatorWorker {
            coordinator: "orchestrator".into(),
            workers: vec!["coder".into(), "reviewer".into()],
        })
        .member(TeamMemberBuilder::new("orchestrator")
            .role("Planner")
            .toolset(ToolsetSelector::Preset("coordinator"))
            .model("claude-sonnet-4.5")
            .build())
        .member(TeamMemberBuilder::new("coder")
            .role("Coder")
            .toolset(ToolsetSelector::Preset("fs-edit"))
            .sandbox_policy(SandboxPolicy {
                mode: SandboxMode::Container,
                scope: SandboxScope::WorkspaceOnly,
                network: NetworkAccess::AllowList(vec![]),
                resource_limits: ResourceLimits::default(),
                denied_host_paths: vec![],
            })
            .visibility(ContextVisibility::AllowlistQuote(vec!["orchestrator".into()]))
            .build())
        .member(TeamMemberBuilder::new("reviewer")
            .role("Reviewer")
            .toolset(ToolsetSelector::Preset("read-only"))
            .visibility(ContextVisibility::All)
            .build())
        .build()
).await?;

let mut events = team.dispatch(TeamInput::goal("review PR #123")).await?;
while let Some(ev) = events.next().await {
    match ev? {
        Event::AgentMessageSent { from, to, payload, .. } => {
            ui.render_message(from, to, payload);
        }
        Event::TeamTerminated { .. } => break,
        _ => {}
    }
}
```

## 10. 测试策略

| 类 | 覆盖 |
|---|---|
| 单元 | 每种 Topology 的 route 正确 |
| 消息总线 | `CausalPerSender` / `TotalOrder` / `BestEffort` 语义；`buffer_size` 满时回压触发 `MessageBusBackpressure` |
| Visibility | `Private` / `Allowlist` / `AllowlistQuote` 隔离；过滤计数命中 `team_context_visibility_blocked_total` 而非新事件 |
| Replay | 完整 Event 流重建任意时刻 `TeamProjection`；动态加入者必须有 `TeamMemberJoined.spec_snapshot_id` 才能还原 `EngineConfig` |
| Coordinator | 专用 toolset 检查；`spawn_worker` 在非 `CoordinatorWorker` 拓扑装配期失败 |
| 配额 | 超配触发 `QuotaExceeded` |
| 回合上限 | `max_turns_per_goal` 命中 → `TurnLimitExceeded` + `TerminationReason::Timeout` 收尾 |
| 路由环 | 同 `correlation_id` 累计超 `max_messages_per_correlation` → `CyclicRouting` + 走 `RouteFallback` |
| CorrelationId | 成员事件、Subagent 事件、`MemoryUpserted` 的 envelope `correlation_id` 与入站 `AgentMessage` 一致；不一致 fail-closed |
| 动态成员 | `Team::add_member` / `Team::remove_member` 幂等；写出 `TeamMemberJoined`（含 `spec_snapshot_id`） / `TeamMemberLeft`；replay 重建一致 |
| MessageClassifier | `timeout` 命中走 `RouteFallback` 不抛错；`classifier_id` 重复注册装配期失败 |
| ReplayWindow | `Bounded(N)` / `SinceJoin` / `FullSinceTeamCreated` 三种语义在 `Ephemeral` / `JournalOnly` 下的合法/非法组合 |
| AnnouncementRenderer | `CoordinatorWorker` 路径上 worker→coordinator 的 user-role 文本含 `<task-notification>` + `<rewrite-hint>` |
| Watchdog | 模拟成员长时间无心跳 → `TeamMemberStalled` 写入；`StalledAction::Removed` 触发 `TeamMemberLeft.reason = StalledRemoved` |
| SharedMemory | `CoordinatorOnly` / `RoleGated` / `PerMemberQuota` 拒绝路径走 `ContextVisibilityViolation`；写入 envelope `correlation_id` 与触发消息一致 |

## 11. 可观测性

| 指标 | 说明 |
|---|---|
| `team_created_total` | 按 topology 分桶 |
| `team_message_throughput` | 每秒消息数 |
| `team_routing_duration_ms` | 路由耗时 |
| `team_member_usage` | 按成员 token / 成本 |
| `team_idle_seconds` | 当前空闲时长 |
| `team_cyclic_routing_detected_total` | 命中 `max_messages_per_correlation` 或默认环上限的次数 |
| `team_turn_limit_exhausted_total` | 命中 `max_turns_per_goal` 的次数 |
| `team_member_stalled_total{action}` | watchdog 上报；`action ∈ reported / interrupted / removed` |
| `team_classifier_timeout_total{classifier_id}` | `MessageClassifier::classify` 超时回退次数 |
| `team_context_visibility_blocked_total` | 因 `ContextVisibility` 被过滤掉的消息计数（替代独立 `TeamMessageFiltered` 事件） |
| `team_message_bus_backpressure_total` | Bus 满时的回压次数（与 `TeamError::MessageBusBackpressure` 对账） |
| `team_dynamic_member_change_total{action}` | `add_member` / `remove_member` 调用计数 |
| `team_replay_window_serve_bytes` | 新成员 subscribe 时 `ReplayWindow` 回放字节量直方图 |

## 12. 反模式

- Team 没有 Visibility 配置（默认 All 可能泄漏）
- Coordinator 被赋予 Bash 工具
- Team 成员数量无上限（建议 ≤ 8；任何超出值需在 ADR 中显式记录）
- MessageBus Persistence 选 Ephemeral 但要求 Replay 或 `ReplayWindow::FullSinceTeamCreated`
- `PeerToPeer` 拓扑未设 `max_turns_per_goal` 或 `max_messages_per_correlation`
  （等价于裸放无限循环风险，命中 `CyclicRouting` 时已经太晚）
- 长驻 `Persistent` Team 缺少 watchdog（`team_idle_seconds` 没有上界监控、
  `TeamMemberStalled` 没有处理）
- 通过自定义 `TopologyStrategy` 在路由阶段写 SharedMemory（违反"路由层不
  产生副作用"，破坏 replay 确定性，应在成员 Engine 内显式写）
- 运行期热改成员的 `EngineConfig`（model_ref / toolset / sandbox_policy）：
  违反 `TeamMemberJoined.spec_snapshot_id` 不可变约定；正确做法是
  `remove_member` + 新 spec 的 `add_member`
- `spawn_worker` 频繁 spawn / remove 同名 Worker 来"做一次性子任务"
  （应改用 `harness-subagent::AgentTool`）
- 引入跨成员共享的 sandbox 实例（Team 不提供 `TeamSandboxScope` 抽象；
  每成员 `engine_config.sandbox_policy` 各自独立——共享只能通过同一
  `BackendId` 在业务层显式约定，且需在 ADR 记录）
- 自定义 `MessageClassifier::classify` 中执行 IO（破坏 replay 确定性；
  `classifier_id` 一致性也无法保证）
- Coordinator 把 `<task-notification>` XML 原样转述给终端用户
  （与 Subagent 同款反模式；对齐 CC-12 / OC-27）

## 13. 相关

- D6 · `agents-design.md` §4
- ADR-004 Agent Team 拓扑
- `crates/harness-subagent.md`
- Evidence: CC-12, OC-08, OC-26
